use parquet::file::writer::SerializedFileWriter;
use parquet::{file::properties::WriterProperties, record::RecordWriter as _};
use parquet_derive::ParquetRecordWriter;
use std::{error::Error, fs::File, path::Path, sync::Arc};

#[derive(Debug, ParquetRecordWriter)]
struct PageEditRecord {
    qid: u32,
    edit_count: u32,
}

/// Write a `(qid, edit_count)` list to a per-day Parquet file. Same schema
/// shape as the per-day pageview files (`pageview_parquet::write_pageview_parquet`),
/// so `PageEditsEngine` can load it through the same raw-parquet reader path.
pub fn write_pageedit_parquet(
    pairs: &[(u32, u32)],
    output_path: &str,
) -> Result<(), Box<dyn Error>> {
    let records: Vec<PageEditRecord> = pairs
        .iter()
        .map(|&(qid, edit_count)| PageEditRecord { qid, edit_count })
        .collect();

    let schema = records.as_slice().schema()?;
    let props = Arc::new(
        WriterProperties::builder()
            .set_compression(parquet::basic::Compression::SNAPPY)
            .build(),
    );
    if let Some(parent) = Path::new(output_path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = File::create(output_path)?;
    let mut writer = SerializedFileWriter::new(file, schema, props)?;
    let mut row_group = writer.next_row_group()?;
    records.as_slice().write_to_row_group(&mut row_group)?;
    row_group.close()?;
    writer.close()?;

    Ok(())
}
