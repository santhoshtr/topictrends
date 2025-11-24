use dotenv::dotenv;

use mysql::prelude::*;
use mysql::*;
use parquet::file::writer::SerializedFileWriter;
use parquet::{file::properties::WriterProperties, record::RecordWriter as _};
use parquet_derive::ParquetRecordWriter;
use std::fs::File;
use std::sync::Arc;

#[derive(Debug, ParquetRecordWriter)]
struct PageRecord {
    page_id: u32,
    page_title: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let db_user = std::env::var("DB_USER").expect("DB_USER not set in .env");
    let db_password = std::env::var("DB_PASSWORD").expect("DB_PASSWORD not set in .env");
    let db_host = std::env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_string());
    let db_port = std::env::var("DB_PORT").unwrap_or_else(|_| "3306".to_string());
    let db_name = std::env::var("DB_NAME").unwrap_or_else(|_| "enwiki".to_string());

    let opts = OptsBuilder::new()
        .user(Some(db_user))
        .pass(Some(db_password))
        .ip_or_hostname(Some(db_host))
        .tcp_port(db_port.parse().expect("Failed to parse string to i32"))
        .db_name(Some(db_name));

    let pool = Pool::new(opts)?;
    let mut conn = pool.get_conn().expect("Connection failed");

    // Execute the query
    let query = "SELECT page_id, page_title FROM page WHERE page_namespace = 14 ";
    let results: Vec<PageRecord> = conn.query_map(query, |(page_id, page_title)| PageRecord {
        page_id,
        page_title,
    })?;

    println!("Retrieved {} records", results.len());

    let schema = results.as_slice().schema().unwrap();
    let props = Arc::new(WriterProperties::builder().build());
    let file = File::create("data/categories.parquet").unwrap();
    let mut writer = SerializedFileWriter::new(file, schema, props).unwrap();
    let mut row_group = writer.next_row_group().unwrap();
    results
        .as_slice()
        .write_to_row_group(&mut row_group)
        .unwrap();
    row_group.close().unwrap();

    writer.close().unwrap();
    println!("Successfully wrote data to categories.parquet");

    Ok(())
}
