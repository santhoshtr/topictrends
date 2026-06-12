//! Reader for the materialized coverage-matrix snapshots produced by the
//! `coverage-matrix` ETL binary.
//!
//! Each per-wiki dated file `data/{wiki}/coverage/{YYYY-MM-DD}.parquet` has the
//! schema `(category_qid: u32, direct_coverage: u32, qid_overlap_coverage: u32)`
//! and is written sorted by `category_qid`. We hold it as three parallel arrays
//! so a cross-wiki gap computation is a cache-friendly two-pointer merge and a
//! single-category lookup is an `O(log n)` binary search.
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
}

impl CoverageMatrix {
    /// `O(log n)` lookup. Returns `(direct_coverage, qid_overlap_coverage)`,
    /// or `(0, 0)` if the category is absent from this wiki's snapshot.
    #[inline]
    pub fn get(&self, category_qid: u32) -> (u32, u32) {
        match self.category_qids.binary_search(&category_qid) {
            Ok(i) => (self.direct[i], self.overlap[i]),
            Err(_) => (0, 0),
        }
    }

    /// Yields `(category_qid, direct, overlap)` in `category_qid` order.
    pub fn iter(&self) -> impl Iterator<Item = (u32, u32, u32)> + '_ {
        self.category_qids
            .iter()
            .copied()
            .zip(self.direct.iter().copied())
            .zip(self.overlap.iter().copied())
            .map(|((c, d), o)| (c, d, o))
    }

    pub fn len(&self) -> usize {
        self.category_qids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.category_qids.is_empty()
    }

    /// Build from `(category_qid, direct, overlap)` rows. Sorts by
    /// `category_qid` so `get`/`iter` can rely on the ordering invariant even
    /// if a future writer changes its output order.
    fn from_rows(mut rows: Vec<(u32, u32, u32)>) -> Self {
        rows.sort_unstable_by_key(|(c, _, _)| *c);
        let mut category_qids = Vec::with_capacity(rows.len());
        let mut direct = Vec::with_capacity(rows.len());
        let mut overlap = Vec::with_capacity(rows.len());
        for (c, d, o) in rows {
            category_qids.push(c);
            direct.push(d);
            overlap.push(o);
        }
        Self {
            category_qids,
            direct,
            overlap,
        }
    }
}

/// Load a coverage-matrix Parquet. Columns are read positionally
/// (0 = category_qid, 1 = direct_coverage, 2 = qid_overlap_coverage), matching
/// the `CoverageRecord` written by the ETL — the same positional convention as
/// `load_pageview_parquet`.
pub fn load_coverage_parquet(path: &str) -> Result<CoverageMatrix, Box<dyn Error>> {
    let file = File::open(path)?;
    let reader = SerializedFileReader::new(file)?;
    let row_iter = reader.get_row_iter(None)?;

    let mut rows: Vec<(u32, u32, u32)> = Vec::new();
    for row_result in row_iter {
        let row = row_result?;
        let category_qid = row.get_uint(0)?;
        let direct = row.get_uint(1)?;
        let overlap = row.get_uint(2)?;
        rows.push((category_qid, direct, overlap));
    }

    Ok(CoverageMatrix::from_rows(rows))
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
