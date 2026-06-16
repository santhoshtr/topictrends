// Build the global category-label table: one label per category QID across
// all editions, English-first.
//
// Categories share Wikidata QIDs across editions but a QID may have no page
// (and hence no title) in a given wiki. The web layer resolves labels from
// the local wiki's categories.parquet first and falls back to this table for
// categories projected in from other editions. Label priority: enwiki, then
// wikipedia.list order.
//
// Writes data/canonical/{date}/category_labels.parquet (qid: u32, label: str),
// sorted by qid.
//
// Usage: canonical-labels --date YYYY-MM-DD [wiki ...]
//   trailing wikis : override the wiki universe (defaults to data/wikipedia.list).

use parquet::file::properties::WriterProperties;
use parquet::file::writer::SerializedFileWriter;
use parquet::record::RecordWriter as _;
use parquet_derive::ParquetRecordWriter;
use polars::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

const ROW_GROUP_SIZE: usize = 4_000_000;

#[derive(Debug, ParquetRecordWriter)]
struct CategoryLabel {
    qid: u32,
    label: String,
}

fn data_dir() -> String {
    std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string())
}

fn read_parquet(path: &str) -> Result<DataFrame, Box<dyn std::error::Error>> {
    let p = PlRefPath::try_from_path(Path::new(path))?;
    Ok(LazyFrame::scan_parquet(p, Default::default())?.collect()?)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let date = args
        .iter()
        .position(|a| a == "--date")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(|| {
            eprintln!("Usage: canonical-labels --date YYYY-MM-DD [wiki ...]");
            std::process::exit(1);
        });
    let positional: Vec<String> = args
        .iter()
        .skip(1)
        .filter(|a| !a.starts_with("--") && **a != date)
        .cloned()
        .collect();

    let data = data_dir();
    let wikis: Vec<String> = if positional.is_empty() {
        std::fs::read_to_string(format!("{}/wikipedia.list", data))?
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect()
    } else {
        positional
    };

    let start = Instant::now();

    // enwiki labels win; after that, first edition in wikipedia.list order.
    let mut read_order: Vec<&str> = vec!["enwiki"];
    read_order.extend(wikis.iter().map(String::as_str).filter(|w| *w != "enwiki"));

    let mut label: HashMap<u32, String> = HashMap::new();
    for wiki in read_order {
        let path = format!("{}/{}/categories.parquet", data, wiki);
        if !Path::new(&path).exists() {
            eprintln!("  WARNING: skipping {} (no categories.parquet)", wiki);
            continue;
        }
        let df = read_parquet(&path)?;
        let qids = df.column("qid")?.u32()?;
        let titles = df.column("page_title")?.str()?;
        for (q, t) in qids.iter().zip(titles.iter()) {
            if let (Some(q), Some(t)) = (q, t) {
                label.entry(q).or_insert_with(|| t.to_string());
            }
        }
    }

    let mut records: Vec<CategoryLabel> = label
        .into_iter()
        .map(|(qid, label)| CategoryLabel { qid, label })
        .collect();
    records.sort_unstable_by_key(|r| r.qid);

    let out_dir = format!("{}/canonical/{}", data, date);
    std::fs::create_dir_all(&out_dir)?;
    let out_path = format!("{}/category_labels.parquet", out_dir);

    let probe = [CategoryLabel { qid: 0, label: String::new() }];
    let schema = probe.as_slice().schema()?;
    let props = Arc::new(
        WriterProperties::builder()
            .set_compression(parquet::basic::Compression::SNAPPY)
            .build(),
    );
    let mut writer = SerializedFileWriter::new(File::create(&out_path)?, schema, props)?;
    for chunk in records.chunks(ROW_GROUP_SIZE) {
        let mut rg = writer.next_row_group()?;
        chunk.write_to_row_group(&mut rg)?;
        rg.close()?;
    }
    writer.close()?;

    println!(
        "Category labels {}: {} distinct categories over {} wikis in {:.0?}",
        date,
        records.len(),
        wikis.len(),
        start.elapsed()
    );
    println!("Wrote {}", out_path);
    Ok(())
}
