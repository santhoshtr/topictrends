// Coverage matrix ETL: per-wiki depth-0 coverage, all measures in one pass.
//
// `direct_coverage(C)` = number of distinct articles filed *directly* under
// category C in this wiki — a group-by on the local `article_category.parquet`
// (already filtered to valid article/category QIDs, see get-article_category).
//
// `qid_overlap_coverage(C)` = number of C's globally-known articles that exist
// as articles in this wiki, independent of how (or whether) this wiki
// categorises them — the pure *content* measure. The canonical projection
// `article_category_canonical.parquet` already materializes exactly that
// intersection (canonical_set(C) ∩ articles(W), built by canonical-projection
// from the gated canonical snapshot), so this too is a plain group-by.
//
// `overlap_pageviews(C)` = sum of windowed pageviews over the same canonical
// overlap article set (the per-wiki `pageviews/{Y}/{M}/{D}.parquet` files for
// the `window_days` ending at the snapshot date, joined to the canonical
// relation). Scattering over the canonical projection — not the local relation
// — keeps its article universe identical to `qid_overlap_coverage`, so
// `overlap_pageviews / qid_overlap_coverage` is a self-consistent average
// views-per-overlap-article. This is the popularity weight gap discovery uses.
//
// Every local direct edge is also a canonical edge over this wiki's articles,
// so direct_coverage ≤ qid_overlap_coverage per category and the overlap key
// set covers the direct key set: merging is a left join, zero-filling
// direct_coverage where the wiki has articles for C but no local category.
//
// Output: Parquet of (category_qid, direct_coverage, qid_overlap_coverage,
// overlap_pageviews), sorted by category_qid. The wiki is implied by the
// partition path (data/{wiki}/coverage/{date}.parquet), so it is not stored as
// a column, mirroring the pageview/pageedit layout.

use chrono::{Datelike, Duration, NaiveDate};
use parquet::file::writer::SerializedFileWriter;
use parquet::{file::properties::WriterProperties, record::RecordWriter as _};
use parquet_derive::ParquetRecordWriter;
use polars::prelude::*;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

const DEFAULT_WINDOW_DAYS: i64 = 30;

#[derive(Debug, ParquetRecordWriter)]
struct CoverageRecord {
    category_qid: u32,
    direct_coverage: u32,
    qid_overlap_coverage: u32,
    overlap_pageviews: u64,
}

fn count_per_category(path: &str, out_name: &str) -> Result<LazyFrame, Box<dyn std::error::Error>> {
    let p = PlRefPath::try_from_path(Path::new(path))?;
    Ok(LazyFrame::scan_parquet(p, Default::default())?
        .group_by([col("category_qid")])
        .agg([col("article_qid").n_unique().alias(out_name)]))
}

/// Sum each category's overlap-article pageviews over the `window_days` ending
/// at `date`. Scatters the windowed per-article view totals through the
/// canonical relation (same universe as `qid_overlap_coverage`). Returns `None`
/// when no per-day pageview file in the window exists (small / unindexed wikis),
/// leaving `overlap_pageviews` zero-filled — such a wiki is rarely a *reference*,
/// and weighting only ever reads the reference column.
fn pageviews_per_category(
    pageviews_dir: &str,
    canonical_input: &str,
    date: NaiveDate,
    window_days: i64,
) -> Result<Option<LazyFrame>, Box<dyn std::error::Error>> {
    let mut day_lfs: Vec<LazyFrame> = Vec::new();
    for back in 0..window_days {
        let d = date - Duration::days(back);
        let path = format!(
            "{}/{}/{:02}/{:02}.parquet",
            pageviews_dir,
            d.year(),
            d.month(),
            d.day()
        );
        if Path::new(&path).exists() {
            let p = PlRefPath::try_from_path(Path::new(&path))?;
            day_lfs.push(LazyFrame::scan_parquet(p, Default::default())?);
        }
    }
    if day_lfs.is_empty() {
        return Ok(None);
    }

    // Windowed views per article qid.
    let article_views = concat(&day_lfs, UnionArgs::default())?
        .group_by([col("qid")])
        .agg([col("views").sum().alias("views")]);

    // Scatter into categories via the canonical (article_qid, category_qid) relation.
    let canonical = {
        let p = PlRefPath::try_from_path(Path::new(canonical_input))?;
        LazyFrame::scan_parquet(p, Default::default())?
    };
    let per_cat = canonical
        .join(
            article_views,
            [col("article_qid")],
            [col("qid")],
            JoinArgs::new(JoinType::Inner),
        )
        .group_by([col("category_qid")])
        .agg([col("views")
            .sum()
            .cast(DataType::UInt64)
            .alias("overlap_pageviews")]);
    Ok(Some(per_cat))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 6 {
        eprintln!(
            "Usage: {} <article_category.parquet> <article_category_canonical.parquet> <output_file> <pageviews_dir> <date YYYY-MM-DD> [window_days]",
            args[0]
        );
        std::process::exit(1);
    }
    let local_input = &args[1];
    let canonical_input = &args[2];
    let output_file = &args[3];
    let pageviews_dir = &args[4];
    let date = NaiveDate::parse_from_str(&args[5], "%Y-%m-%d")?;
    let window_days = args
        .get(6)
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_WINDOW_DAYS);

    let overlap = count_per_category(canonical_input, "qid_overlap_coverage")?;
    let direct = count_per_category(local_input, "direct_coverage")?;

    let merged = overlap
        .join(
            direct,
            [col("category_qid")],
            [col("category_qid")],
            JoinArgs::new(JoinType::Left),
        )
        .with_column(col("direct_coverage").fill_null(0));

    // Attach the per-category pageview weight (left join, zero-filling categories
    // with no overlap views, or all-zero if the wiki has no pageviews in window).
    let merged = match pageviews_per_category(pageviews_dir, canonical_input, date, window_days)? {
        Some(per_cat) => merged
            .join(
                per_cat,
                [col("category_qid")],
                [col("category_qid")],
                JoinArgs::new(JoinType::Left),
            )
            .with_column(col("overlap_pageviews").fill_null(0)),
        None => {
            eprintln!(
                "No pageview files in the {}-day window ending {} under {}; overlap_pageviews=0",
                window_days, date, pageviews_dir
            );
            merged.with_column(lit(0u64).alias("overlap_pageviews"))
        }
    };

    let merged = merged
        .select([
            col("category_qid").cast(DataType::UInt32),
            col("direct_coverage").cast(DataType::UInt32),
            col("qid_overlap_coverage").cast(DataType::UInt32),
            col("overlap_pageviews").cast(DataType::UInt64),
        ])
        .sort(["category_qid"], Default::default())
        .collect()?;

    let cat_col = merged.column("category_qid")?.u32()?;
    let direct_col = merged.column("direct_coverage")?.u32()?;
    let overlap_col = merged.column("qid_overlap_coverage")?.u32()?;
    let pageviews_col = merged.column("overlap_pageviews")?.u64()?;

    let records: Vec<CoverageRecord> = cat_col
        .iter()
        .zip(direct_col.iter())
        .zip(overlap_col.iter())
        .zip(pageviews_col.iter())
        .filter_map(|(((c, d), o), pv)| {
            Some(CoverageRecord {
                category_qid: c?,
                direct_coverage: d?,
                qid_overlap_coverage: o?,
                overlap_pageviews: pv.unwrap_or(0),
            })
        })
        .collect();

    let schema = records.as_slice().schema()?;
    let props = Arc::new(
        WriterProperties::builder()
            .set_compression(parquet::basic::Compression::SNAPPY)
            .build(),
    );
    if let Some(parent) = Path::new(output_file).parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = File::create(output_file)?;
    let mut writer = SerializedFileWriter::new(file, schema, props)?;
    let mut row_group = writer.next_row_group()?;
    records.as_slice().write_to_row_group(&mut row_group)?;
    row_group.close()?;
    writer.close()?;

    println!("Wrote {} categories to {}", records.len(), output_file);
    Ok(())
}
