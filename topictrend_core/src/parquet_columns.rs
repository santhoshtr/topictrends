//! Bulk columnar reads for the per-day metric Parquet files.
//!
//! The `parquet` crate's row-based `get_row_iter` materializes a dynamically
//! typed `Row` per record, which measures 12–15x slower than decoding each
//! column in bulk with the typed column readers on enwiki-sized files
//! (~5M rows/day). These helpers stay on the raw synchronous `parquet` API
//! (no Polars, no Arrow) so they remain safe to call from inside an async
//! runtime.
//!
//! Columns are assumed REQUIRED (no nulls) — which is what our ETL writers
//! produce (`parquet_derive` over non-`Option` fields). A nullable column
//! fails the read with an error rather than being silently misread.

use parquet::column::reader::get_typed_column_reader;
use parquet::data_type::{DataType, Int32Type, Int64Type};
use parquet::file::reader::{FileReader, SerializedFileReader};
use std::error::Error;
use std::fs::File;

fn read_column<T: DataType>(
    reader: &SerializedFileReader<File>,
    col: usize,
) -> Result<Vec<T::T>, Box<dyn Error>> {
    let metadata = reader.metadata();
    let total_rows = metadata.file_metadata().num_rows() as usize;
    let mut values: Vec<T::T> = Vec::with_capacity(total_rows);

    for rg_idx in 0..metadata.num_row_groups() {
        let rg = reader.get_row_group(rg_idx)?;
        let rg_rows = metadata.row_group(rg_idx).num_rows() as usize;
        let mut col_reader = get_typed_column_reader::<T>(rg.get_column_reader(col)?);
        let (records, _, _) = col_reader.read_records(rg_rows, None, None, &mut values)?;
        if records != rg_rows {
            return Err(format!(
                "column {col}: read {records} records, row group has {rg_rows}"
            )
            .into());
        }
    }
    Ok(values)
}

/// Read column `col` (physical INT32, our `u32` fields) across all row groups.
pub fn read_u32_column(
    reader: &SerializedFileReader<File>,
    col: usize,
) -> Result<Vec<u32>, Box<dyn Error>> {
    Ok(read_column::<Int32Type>(reader, col)?
        .into_iter()
        .map(|v| v as u32)
        .collect())
}

/// Read column `col` (physical INT64, our `u64` fields) across all row groups.
pub fn read_u64_column(
    reader: &SerializedFileReader<File>,
    col: usize,
) -> Result<Vec<u64>, Box<dyn Error>> {
    Ok(read_column::<Int64Type>(reader, col)?
        .into_iter()
        .map(|v| v as u64)
        .collect())
}
