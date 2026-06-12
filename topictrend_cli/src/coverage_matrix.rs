// Coverage matrix ETL: per-wiki depth-0 coverage, both measures in one pass.
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
// Every local direct edge is also a canonical edge over this wiki's articles,
// so direct_coverage ≤ qid_overlap_coverage per category and the overlap key
// set covers the direct key set: merging is a left join, zero-filling
// direct_coverage where the wiki has articles for C but no local category.
//
// Output: Parquet of (category_qid, direct_coverage, qid_overlap_coverage),
// sorted by category_qid. The wiki is implied by the partition path
// (data/{wiki}/coverage/{date}.parquet), so it is not stored as a column,
// mirroring the pageview/pageedit layout.

use parquet::file::writer::SerializedFileWriter;
use parquet::{file::properties::WriterProperties, record::RecordWriter as _};
use parquet_derive::ParquetRecordWriter;
use polars::prelude::*;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, ParquetRecordWriter)]
struct CoverageRecord {
    category_qid: u32,
    direct_coverage: u32,
    qid_overlap_coverage: u32,
}

fn count_per_category(path: &str, out_name: &str) -> Result<LazyFrame, Box<dyn std::error::Error>> {
    let p = PlRefPath::try_from_path(Path::new(path))?;
    Ok(LazyFrame::scan_parquet(p, Default::default())?
        .group_by([col("category_qid")])
        .agg([col("article_qid").n_unique().alias(out_name)]))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        eprintln!(
            "Usage: {} <article_category.parquet> <article_category_canonical.parquet> <output_file>",
            args[0]
        );
        std::process::exit(1);
    }
    let local_input = &args[1];
    let canonical_input = &args[2];
    let output_file = &args[3];

    let overlap = count_per_category(canonical_input, "qid_overlap_coverage")?;
    let direct = count_per_category(local_input, "direct_coverage")?;

    let merged = overlap
        .join(
            direct,
            [col("category_qid")],
            [col("category_qid")],
            JoinArgs::new(JoinType::Left),
        )
        .with_column(col("direct_coverage").fill_null(0))
        .select([
            col("category_qid").cast(DataType::UInt32),
            col("direct_coverage").cast(DataType::UInt32),
            col("qid_overlap_coverage").cast(DataType::UInt32),
        ])
        .sort(["category_qid"], Default::default())
        .collect()?;

    let cat_col = merged.column("category_qid")?.u32()?;
    let direct_col = merged.column("direct_coverage")?.u32()?;
    let overlap_col = merged.column("qid_overlap_coverage")?.u32()?;

    let records: Vec<CoverageRecord> = cat_col
        .into_iter()
        .zip(direct_col)
        .zip(overlap_col)
        .filter_map(|((c, d), o)| {
            Some(CoverageRecord {
                category_qid: c?,
                direct_coverage: d?,
                qid_overlap_coverage: o?,
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
