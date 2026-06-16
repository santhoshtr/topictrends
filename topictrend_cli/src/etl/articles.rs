use parquet::file::writer::SerializedFileWriter;
use parquet::{file::properties::WriterProperties, record::RecordWriter as _};
use parquet_derive::ParquetRecordWriter;
use std::fs::File;
use std::io::{self, BufRead};
use std::sync::Arc;

#[derive(Debug, ParquetRecordWriter)]
struct PageRecord {
    page_id: u32,
    qid: u32,
    page_title: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <output_file>", args[0]);
        std::process::exit(1);
    }
    let output_file = &args[1];
    let stdin = io::stdin();
    let results: Vec<PageRecord> = stdin
        .lock()
        .lines()
        .filter_map(|line| {
            let line = line.ok()?;
            let mut parts = line.split('\t');
            let page_id = parts.next()?.parse::<u32>().ok()?;
            let qid = parts.next()?.parse::<u32>().ok()?;
            let page_title = parts.next()?.to_string();
            Some(PageRecord {
                page_id,
                qid,
                page_title,
            })
        })
        .collect();

    println!("Retrieved {} records", results.len());

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
    println!("Successfully wrote data to {}", args[1]);

    Ok(())
}
