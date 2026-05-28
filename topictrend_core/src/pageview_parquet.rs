use parquet::file::writer::SerializedFileWriter;
use parquet::{file::properties::WriterProperties, record::RecordWriter as _};
use parquet_derive::ParquetRecordWriter;
use polars::prelude::*;
use std::{error::Error, fs::File, path::Path, sync::Arc};

use crate::direct_map::DirectMap;

#[derive(Debug, ParquetRecordWriter)]
struct PageviewRecord {
    qid: u32,
    views: u32,
}

/// Write a sorted (qid, views) list to a Parquet file.
pub fn write_pageview_parquet(
    pairs: &[(u32, u32)],
    output_path: &str,
) -> Result<(), Box<dyn Error>> {
    let records: Vec<PageviewRecord> = pairs
        .iter()
        .map(|&(qid, views)| PageviewRecord { qid, views })
        .collect();

    let schema = records.as_slice().schema()?;
    let props = Arc::new(
        WriterProperties::builder()
            .set_compression(parquet::basic::Compression::SNAPPY)
            .build(),
    );
    let file = File::create(output_path)?;
    let mut writer = SerializedFileWriter::new(file, schema, props)?;
    let mut row_group = writer.next_row_group()?;
    records.as_slice().write_to_row_group(&mut row_group)?;
    row_group.close()?;
    writer.close()?;

    Ok(())
}

/// Read the daily raw pageview parquet for the given date, filter to `wiki`,
/// map page_id → qid via `articles.parquet`, drop unknown / zero-view rows,
/// and return a `(qid, views)` list sorted by qid.
pub fn get_daily_pageviews(wiki: &str, year: i16, month: i8, day: i8) -> Vec<(u32, u32)> {
    let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string());

    let pageviews_path = format!(
        "{}/pageviews/{}/{:02}/{:02}.parquet",
        data_dir, year, month, day
    );
    let articles_path = format!("{}/{}/articles.parquet", data_dir, wiki);

    if !Path::new(&pageviews_path).exists() {
        eprintln!("Pageview file not found: {}", pageviews_path);
        return Vec::new();
    }

    let df: DataFrame = LazyFrame::scan_parquet(
        PlRefPath::try_from_path(Path::new(&pageviews_path)).unwrap(),
        Default::default(),
    )
    .expect("Failed to read pageviews parquet")
    .collect()
    .expect("Failed to collect DataFrame");

    let grouped_df = df
        .lazy()
        .filter(col("wiki").eq(lit(wiki)))
        .group_by([col("page_id")])
        .agg([col("daily_views").sum().alias("daily_views")])
        .collect()
        .expect("Failed to group DataFrame");

    let page_ids = grouped_df.column("page_id").unwrap().u32().unwrap();
    let daily_views = grouped_df.column("daily_views").unwrap().u32().unwrap();

    let articles_df: DataFrame = LazyFrame::scan_parquet(
        PlRefPath::try_from_path(Path::new(&articles_path)).unwrap(),
        Default::default(),
    )
    .unwrap()
    .collect()
    .unwrap();

    let article_id_to_qid: DirectMap = articles_df
        .column("page_id")
        .unwrap()
        .u32()
        .unwrap()
        .into_iter()
        .zip(articles_df.column("qid").unwrap().u32().unwrap())
        .filter_map(|(id, qid)| Some((id?, qid?)))
        .collect();

    let mut pairs: Vec<(u32, u32)> = Vec::new();
    for (opt_page_id, opt_views) in page_ids.into_iter().zip(daily_views) {
        if let (Some(page_id), Some(views)) = (opt_page_id, opt_views)
            && views > 0
            && let Some(qid) = article_id_to_qid.get(page_id)
        {
            pairs.push((qid, views));
        }
    }

    pairs.sort_unstable_by_key(|&(qid, _)| qid);
    pairs
}
