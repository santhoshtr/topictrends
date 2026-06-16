//! Reader for the materialized coverage-matrix snapshots produced by the
//! `coverage-matrix` ETL binary.
//!
//! Each per-wiki dated file `data/{wiki}/coverage/{YYYY-MM-DD}.parquet` has the
//! schema `(category_qid: u32, direct_coverage: u32, qid_overlap_coverage: u32,
//! overlap_pageviews: u64)` and is written sorted by `category_qid`. We hold it
//! as parallel arrays so a cross-wiki gap computation is a cache-friendly
//! two-pointer merge and a single-category lookup is an `O(log n)` binary search.
//!
//! `overlap_pageviews` was added after the first snapshots shipped; pre-existing
//! 3-column files load with `has_pageviews = false` and a zero-filled column, so
//! the reader never panics on an old snapshot.
//!
//! Like `pageview_parquet`/`pageview_engine`, this reads with the raw `parquet`
//! crate (purely synchronous) rather than Polars, so it is safe to call from
//! inside `spawn_blocking` in the async web layer.

use chrono::NaiveDate;
use parquet::file::reader::{FileReader, SerializedFileReader};
use parquet::record::RowAccessor;
use std::error::Error;
use std::fs::File;
use std::path::Path;

/// One wiki's coverage snapshot, sorted by `category_qid`.
#[derive(Debug)]
pub struct CoverageMatrix {
    category_qids: Vec<u32>, // sorted ascending
    direct: Vec<u32>,        // direct_coverage[i]
    overlap: Vec<u32>,       // qid_overlap_coverage[i]
    pageviews: Vec<u64>,     // overlap_pageviews[i] (all-zero for legacy 3-col snapshots)
    has_pageviews: bool,     // false when the snapshot predates the pageview column
}

impl CoverageMatrix {
    /// `O(log n)` lookup. Returns `(direct_coverage, qid_overlap_coverage,
    /// overlap_pageviews)`, or `(0, 0, 0)` if the category is absent.
    #[inline]
    pub fn get(&self, category_qid: u32) -> (u32, u32, u64) {
        match self.category_qids.binary_search(&category_qid) {
            Ok(i) => (self.direct[i], self.overlap[i], self.pageviews[i]),
            Err(_) => (0, 0, 0),
        }
    }

    /// Yields `(category_qid, direct, overlap, pageviews)` in `category_qid` order.
    pub fn iter(&self) -> impl Iterator<Item = (u32, u32, u32, u64)> + '_ {
        self.category_qids
            .iter()
            .copied()
            .zip(self.direct.iter().copied())
            .zip(self.overlap.iter().copied())
            .zip(self.pageviews.iter().copied())
            .map(|(((c, d), o), p)| (c, d, o, p))
    }

    /// Whether this snapshot carries the `overlap_pageviews` column. When false,
    /// pageview weighting cannot be applied (the column is uniformly zero).
    pub fn has_pageviews(&self) -> bool {
        self.has_pageviews
    }

    pub fn len(&self) -> usize {
        self.category_qids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.category_qids.is_empty()
    }

    /// Build from `(category_qid, direct, overlap, pageviews)` rows. Sorts by
    /// `category_qid` so `get`/`iter` can rely on the ordering invariant even
    /// if a future writer changes its output order.
    fn from_rows(mut rows: Vec<(u32, u32, u32, u64)>, has_pageviews: bool) -> Self {
        rows.sort_unstable_by_key(|(c, _, _, _)| *c);
        let mut category_qids = Vec::with_capacity(rows.len());
        let mut direct = Vec::with_capacity(rows.len());
        let mut overlap = Vec::with_capacity(rows.len());
        let mut pageviews = Vec::with_capacity(rows.len());
        for (c, d, o, p) in rows {
            category_qids.push(c);
            direct.push(d);
            overlap.push(o);
            pageviews.push(p);
        }
        Self {
            category_qids,
            direct,
            overlap,
            pageviews,
            has_pageviews,
        }
    }
}

/// Load a coverage-matrix Parquet. The first three columns
/// (category_qid, direct_coverage, qid_overlap_coverage) are read positionally,
/// matching the `CoverageRecord` written by the ETL. The fourth,
/// `overlap_pageviews`, is presence-checked by name in the file schema so that
/// pre-pageview 3-column snapshots load cleanly (zero-filled, `has_pageviews=false`).
pub fn load_coverage_parquet(path: &str) -> Result<CoverageMatrix, Box<dyn Error>> {
    let file = File::open(path)?;
    let reader = SerializedFileReader::new(file)?;

    let has_pageviews = reader
        .metadata()
        .file_metadata()
        .schema_descr()
        .columns()
        .iter()
        .any(|c| c.name() == "overlap_pageviews");

    let row_iter = reader.get_row_iter(None)?;
    let mut rows: Vec<(u32, u32, u32, u64)> = Vec::new();
    for row_result in row_iter {
        let row = row_result?;
        let category_qid = row.get_uint(0)?;
        let direct = row.get_uint(1)?;
        let overlap = row.get_uint(2)?;
        let pageviews = if has_pageviews { row.get_ulong(3)? } else { 0 };
        rows.push((category_qid, direct, overlap, pageviews));
    }

    Ok(CoverageMatrix::from_rows(rows, has_pageviews))
}

/// Discover the newest coverage snapshot for a wiki under
/// `{DATA_DIR}/{wiki}/coverage/`. Filenames are ISO dates (`YYYY-MM-DD.parquet`)
/// which sort chronologically; returns the latest `(date, full_path)`, or `None`
/// if the directory is missing or holds no parseable snapshot.
pub fn latest_coverage_snapshot(wiki: &str) -> Option<(NaiveDate, String)> {
    let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string());
    let dir = format!("{}/{}/coverage", data_dir, wiki);

    let mut best: Option<(NaiveDate, String)> = None;
    for entry in std::fs::read_dir(&dir).ok()?.flatten() {
        let name = entry.file_name();
        let name = name.to_str()?;
        let Some(stem) = name.strip_suffix(".parquet") else {
            continue;
        };
        let Ok(date) = NaiveDate::parse_from_str(stem, "%Y-%m-%d") else {
            continue;
        };
        let path = Path::new(&dir).join(name).to_string_lossy().into_owned();
        match &best {
            Some((best_date, _)) if *best_date >= date => {}
            _ => best = Some((date, path)),
        }
    }
    best
}
