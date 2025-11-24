use clap::{Arg, Command};
use polars::frame::DataFrame;
use polars::prelude::*;
use std::{
    error::Error,
    io::{self, Write},
    path::Path,
    sync::Arc,
};
use topictrend::{graphbuilder::GraphBuilder, wikigraph::WikiGraph};

use byteorder::{LittleEndian, WriteBytesExt};

fn generate_bin_dump(views: Vec<u32>) -> Result<(), Box<dyn Error>> {
    //  Write Binary File
    let mut stdout = io::stdout();

    // Header: Magic (4) + Version (4) + Size (8)
    stdout.write_all(b"VIEW")?;
    stdout.write_u32::<LittleEndian>(1)?;
    stdout.write_u64::<LittleEndian>(views.len() as u64)?;

    // Body: The raw array
    for count in views {
        stdout.write_u32::<LittleEndian>(count)?;
    }

    stdout.flush()?;
    Ok(())
}

#[derive(Debug, Clone)]
struct PageView {
    project: String,
    page_title: String,
    page_id: i64,
    access_method: String,
    daily_views: i64,
}

fn get_daily_pageviews(wiki: &str, year: &i8, month: &i8, day: &i8) -> Vec<u32> {
    let graph_builder = GraphBuilder::new(wiki);
    let graph: WikiGraph = graph_builder.build().expect("Error while building graph");

    // 1. Read data_dir/pageviews-{year}-{month}-{day}.parquet
    let data_dir = std::env::var("DATA_DIR").expect("DATA_DIR not set in .env");
    let file_path = format!("{}/pageviews-{}-{}-{}.parquet", data_dir, year, month, day);

    if !std::path::Path::new(&file_path).exists() {
        eprintln!("Pageview file not found: {}", file_path);
        return Vec::new();
    }

    let path: PlPath = PlPath::Local(Arc::from(Path::new(&file_path)));
    let df: DataFrame = LazyFrame::scan_parquet(path, Default::default())
        .expect("Failed to read Parquet file")
        .collect()
        .expect("Failed to collect DataFrame");

    // 2. Find all records where project == wiki
    let filtered_df = df
        .lazy()
        .filter(col("project").eq(lit(wiki)))
        .collect()
        .expect("Failed to filter DataFrame");

    // 3. Calculate page_id : daily_views (aggregate)
    let grouped_df = filtered_df
        .lazy()
        .group_by([col("page_id")])
        .agg([col("daily_views").sum().alias("daily_views")])
        .collect()
        .expect("Failed to group DataFrame");

    let page_ids = grouped_df
        .column("page_id")
        .expect("Missing column: page_id")
        .u32()
        .unwrap();
    let daily_views = grouped_df
        .column("daily_views")
        .expect("Missing column: daily_views")
        .u32()
        .unwrap();

    // 4. Convert the page_id to dense_id
    let mut dense_vector = vec![0u32; graph.art_dense_to_original.len()];

    for (opt_page_id, opt_views) in page_ids.into_iter().zip(daily_views.into_iter()) {
        if let (Some(page_id), Some(views)) = (opt_page_id, opt_views)
            && let Some(&dense_id) = graph.art_original_to_dense.get(&page_id)
        {
            // 5. With dense_id as vector index, create a u32 dense vector with daily_views value
            dense_vector[dense_id as usize] = views;
        }
    }

    dense_vector
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("Per Day Wiki Stats")
        .about("Generates per-day wiki statistics")
        .arg(
            Arg::new("wiki")
                .long("wiki")
                .short('w')
                .help("The wiki ID (e.g., enwiki)")
                .required(true)
                .value_parser(clap::value_parser!(String)),
        )
        .arg(
            Arg::new("year")
                .long("year")
                .short('y')
                .help("The year (e.g., 2025)")
                .required(true)
                .value_parser(clap::value_parser!(i8)),
        )
        .arg(
            Arg::new("month")
                .long("month")
                .short('m')
                .help("The month (e.g., 11)")
                .required(true)
                .value_parser(clap::value_parser!(i8)),
        )
        .arg(
            Arg::new("day")
                .long("day")
                .short('d')
                .help("The day (e.g., 24)")
                .required(true)
                .value_parser(clap::value_parser!(i8)),
        )
        .get_matches();

    let wiki = matches.get_one::<String>("wiki").unwrap();
    let year: &i8 = matches.get_one::<i8>("year").unwrap();
    let month: &i8 = matches.get_one::<i8>("month").unwrap();
    let day: &i8 = matches.get_one::<i8>("day").unwrap();

    println!(
        "Processing stats for wiki: {}, date: {}-{}-{}",
        wiki, year, month, day
    );
    let page_views_dense_vector = get_daily_pageviews(wiki, year, month, day);
    generate_bin_dump(page_views_dense_vector)
}
