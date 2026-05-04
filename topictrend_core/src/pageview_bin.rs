use byteorder::{LittleEndian, WriteBytesExt};
use polars::prelude::*;
use std::{
    error::Error,
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

use crate::{direct_map::DirectMap, graphbuilder::GraphBuilder, wikigraph::WikiGraph};

pub fn generate_bin_dump(views: Vec<u32>, output_path: &str) -> Result<(), Box<dyn Error>> {
    let out_file = File::create(output_path)?;
    let mut writer = BufWriter::new(out_file);

    // Header: Magic (4) + Version (4) + Size (8)
    writer.write_all(b"VIEW")?;
    writer.write_u32::<LittleEndian>(1)?;
    writer.write_u64::<LittleEndian>(views.len() as u64)?;

    for count in views {
        writer.write_u32::<LittleEndian>(count)?;
    }

    writer.flush()?;
    Ok(())
}

pub fn get_daily_pageviews(wiki: &str, year: i16, month: i8, day: i8) -> Vec<u32> {
    let graph: WikiGraph = GraphBuilder::new(wiki)
        .build()
        .expect("Error while building graph");

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

    let mut dense_vector = vec![0u32; graph.art_dense_to_original.len()];

    for (opt_page_id, opt_views) in page_ids.into_iter().zip(daily_views) {
        if let (Some(page_id), Some(views)) = (opt_page_id, opt_views)
            && let Some(qid) = article_id_to_qid.get(page_id)
                && let Some(dense_id) = graph.art_original_to_dense.get(qid) {
                    dense_vector[dense_id as usize] = views;
                }
    }

    dense_vector
}
