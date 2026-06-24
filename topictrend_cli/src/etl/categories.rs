use parquet::file::writer::SerializedFileWriter;
use parquet::{file::properties::WriterProperties, record::RecordWriter as _};
use parquet_derive::ParquetRecordWriter;
use polars::prelude::{LazyFrame, PlRefPath};
use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, ParquetRecordWriter)]
struct PageRecord {
    page_id: u32,
    qid: u32,
    page_title: String,
}

/// Load the category-exclusion denylist (column `qid`). A missing file means no
/// filtering (first bootstrap, before `make excluded-categories` has run).
/// Dropping a category here is enough: get-categorygraph and get-article_category
/// validate every category against categories.parquet's id↔qid map, so their
/// edges to a dropped category prune automatically.
fn load_excluded(path: &str) -> Result<HashSet<u32>, Box<dyn std::error::Error>> {
    if !Path::new(path).exists() {
        eprintln!("WARNING: denylist {path} not found; categories unfiltered");
        return Ok(HashSet::new());
    }
    let p = PlRefPath::try_from_path(Path::new(path))?;
    let df = LazyFrame::scan_parquet(p, Default::default())?.collect()?;
    Ok(df.column("qid")?.u32()?.iter().flatten().collect())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <output_file> [excluded_qids_parquet]", args[0]);
        std::process::exit(1);
    }
    let output_file = &args[1];
    let excluded = match args.get(2) {
        Some(path) => load_excluded(path)?,
        None => HashSet::new(),
    };

    let stdin = io::stdin();
    let results: Vec<PageRecord> = stdin
        .lock()
        .lines()
        .filter_map(|line| {
            let line = line.ok()?;
            let mut parts = line.split('\t');
            let page_id = parts.next()?.parse::<u32>().ok()?;
            let qid = parts.next()?.parse::<u32>().ok()?;
            if excluded.contains(&qid) {
                return None;
            }
            let page_title = parts.next()?.to_string();
            Some(PageRecord {
                page_id,
                qid,
                page_title,
            })
        })
        .collect();

    println!(
        "Retrieved {} records ({} excluded qids)",
        results.len(),
        excluded.len()
    );

    let schema = results.as_slice().schema().unwrap();
    let props = Arc::new(
        WriterProperties::builder()
            .set_compression(parquet::basic::Compression::SNAPPY)
            .build(),
    );

    let file = File::create(output_file).unwrap();
    let mut writer = SerializedFileWriter::new(file, schema, props).unwrap();
    let mut row_group = writer.next_row_group().unwrap();
    results
        .as_slice()
        .write_to_row_group(&mut row_group)
        .unwrap();
    row_group.close().unwrap();

    writer.close().unwrap();

    println!("Successfully wrote data to {}", output_file);

    Ok(())
}
