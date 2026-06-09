// Stage 1 of the coverage matrix ETL: per-wiki depth-0 `direct_coverage`.
//
// `direct_coverage(C)` = number of distinct articles filed *directly* under
// category C in this wiki. Because `article_category.parquet` is already
// filtered to valid article/category QIDs (see get-article_category), this is a
// faithful group-by on that file — no need to build the full WikiGraph.
//
// Output: a Parquet of (category_qid, direct_coverage), sorted by category_qid.
// The wiki is implied by the partition path (data/{wiki}/coverage/{date}.parquet),
// so it is not stored as a column, mirroring the pageview/pageedit layout.
//
// qid_overlap_coverage (the cross-wiki content measure) is a separate stage that
// joins all wikis' direct-member sets; it is not produced here.

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
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <article_category.parquet> <output_file>", args[0]);
        std::process::exit(1);
    }
    let input = &args[1];
    let output_file = &args[2];

    let path = PlRefPath::try_from_path(Path::new(input))?;
    let grouped = LazyFrame::scan_parquet(path, Default::default())?
        .group_by([col("category_qid")])
        .agg([col("article_qid").n_unique().alias("direct_coverage")])
        .sort(["category_qid"], Default::default())
        .collect()?;

    let cat_col = grouped.column("category_qid")?.u32()?;
    let cov_col = grouped.column("direct_coverage")?.u32()?;

    let records: Vec<CoverageRecord> = cat_col
        .into_iter()
        .zip(cov_col)
        .filter_map(|(c, n)| {
            Some(CoverageRecord {
                category_qid: c?,
                direct_coverage: n?,
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
