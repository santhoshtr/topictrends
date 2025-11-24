use std::{
    fs::File,
    io::{BufWriter, Write},
};

use byteorder::{LittleEndian, WriteBytesExt};

fn generate_bin_dump(dense_page_views: Vec<u32>, output_path: String) -> Result<()> {
    // A. Prepare Memory Vector (All zeros)
    // Size = 7 Million * 4 bytes ~= 28MB (Very cheap)
    let size = dense_page_views.len();
    let mut views = vec![0u32; size];

    // D. Write Binary File
    let out_file = File::create(output_path)?;
    let mut writer = BufWriter::new(out_file);

    // Header: Magic (4) + Version (4) + Size (8)
    writer.write_all(b"VIEW")?;
    writer.write_u32::<LittleEndian>(1)?;
    writer.write_u64::<LittleEndian>(size as u64)?;

    // Body: The raw array
    for count in views {
        writer.write_u32::<LittleEndian>(count)?;
    }

    writer.flush()?;
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

fn get_daily_pageviews(wiki: String, year: i8, month: i8, day: i8) -> Vec<u32> {
    // 1. read data_dir/pageviews-{year}-{month}-{day}.parquet
    // Find all records where project == wiki
    // Calculate page_id : daily_views (aggregate)
    // Convert the page_id to dense_id
    // with dense_id as vector index, create a u32 dense vector  with daily_views value
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let wiki = args[1];
    let graph_builder = GraphBuilder::new(wiki);
    let graph = graph_builder.build().expect("Error while building graph");

    Ok(())
}
