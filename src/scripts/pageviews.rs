use byteorder::{LittleEndian, WriteBytesExt};
use polars::prelude::*;
use rayon::prelude::*;
use std::fs::File;
use std::io::Write;
use std::io::{BufRead, BufReader, BufWriter};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{path::Path, sync::Arc};

#[derive(Debug, Clone)]
struct PageView {
    project: String,
    page_title: String,
    page_id: i64,
    access_method: String,
    daily_views: i64,
}

fn parse_line(line: &str) -> Result<PageView, Box<dyn std::error::Error>> {
    let parts: Vec<&str> = line.splitn(6, ' ').collect();

    if parts.len() < 5 {
        return Err("Invalid line format".into());
    }

    Ok(PageView {
        project: parts[0].to_string(),
        page_title: parts[1].to_string(),
        page_id: parts[2].parse()?,
        access_method: parts[3].to_string(),
        daily_views: parts[4].parse()?,
    })
}

fn process_chunk(records: Vec<PageView>) -> Result<DataFrame, PolarsError> {
    let project: Vec<String> = records
        .iter()
        .map(|r| r.project.replace("pedia", ""))
        .collect();
    let page_title: Vec<String> = records.iter().map(|r| r.page_title.clone()).collect();
    let page_id: Vec<i64> = records.iter().map(|r| r.page_id).collect();
    let access_method: Vec<i64> = records
        .iter()
        .map(|r| match r.access_method.as_str() {
            "mobile-web" => 1,
            "desktop" => 0,
            _ => -1, // Default value for unexpected cases
        })
        .collect();
    let daily_views: Vec<i64> = records.iter().map(|r| r.daily_views).collect();

    DataFrame::new(vec![
        Column::new("project".into(), project),
        Column::new("page_id".into(), page_id),
        Column::new("access_method".into(), access_method),
        Column::new("daily_views".into(), daily_views),
    ])
}

fn get_file_size<P: AsRef<Path>>(path: P) -> std::io::Result<u64> {
    Ok(std::fs::metadata(path)?.len())
}

pub fn convert_pageviews_to_parquet(
    output_path: &str,
    chunk_size: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting conversion...");

    let stdin = std::io::stdin();
    let reader = BufReader::new(stdin.lock());

    let mut chunks = Vec::new();
    let mut current_chunk = Vec::with_capacity(chunk_size);
    let mut lines_processed = 0;
    let bytes_read = Arc::new(AtomicUsize::new(0));

    println!("Reading and chunking data...");

    for line in reader.lines() {
        let line = line?;
        let line_bytes = line.len() + 1; // +1 for newline
        bytes_read.fetch_add(line_bytes, Ordering::Relaxed);

        match parse_line(&line) {
            Ok(record) => {
                current_chunk.push(record);
                lines_processed += 1;

                if current_chunk.len() >= chunk_size {
                    chunks.push(current_chunk);
                    current_chunk = Vec::with_capacity(chunk_size);
                }
            }
            Err(_) => {
                // Silently skip malformed lines in production
                continue;
            }
        }
    }

    // Add remaining records
    if !current_chunk.is_empty() {
        chunks.push(current_chunk);
    }

    if chunks.is_empty() {
        return Err("No valid data to write".into());
    }

    println!("\nProcessing {} chunks in parallel...", chunks.len());

    let chunk_counter = Arc::new(AtomicUsize::new(0));

    // Process chunks in parallel
    let dataframes: Vec<DataFrame> = chunks
        .into_par_iter()
        .filter_map(|chunk| {
            let result = process_chunk(chunk);

            // Update progress
            let count = chunk_counter.fetch_add(1, Ordering::Relaxed);

            result.ok()
        })
        .collect();

    if dataframes.is_empty() {
        return Err("No valid dataframes created".into());
    }

    println!("\nCombining {} dataframes...", dataframes.len());
    // Convert DataFrame to LazyFrame
    let lazy_frames: Vec<LazyFrame> = dataframes.into_iter().map(|df| df.lazy()).collect();

    let combined = concat(&lazy_frames, UnionArgs::default())?;
    println!("Writing to parquet file {} ", &output_path);
    let mut file = File::create(output_path)?;
    let mut dataframe = combined.collect()?; // Collect LazyFrame into DataFrame
    ParquetWriter::new(&mut file)
        .with_compression(ParquetCompression::Snappy)
        .finish(&mut dataframe)?; // Pass the DataFrame
    println!("\n✓ Conversion complete!");
    println!("  Lines processed: {}", lines_processed);

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    let (output_file, chunk_size) = if args.len() >= 2 {
        let output = &args[1];
        let chunk = if args.len() >= 3 {
            args[2].parse().unwrap_or(100_000)
        } else {
            100_000
        };
        (output.as_str(), chunk)
    } else {
        eprintln!("Usage: <program> <output_file> [chunk_size]");
        std::process::exit(1);
    };

    println!("=== Wikipedia Pageviews to Parquet Converter ===");
    println!("Output: {}", output_file);
    println!("Chunk size: {}\n", chunk_size);

    convert_pageviews_to_parquet(output_file, chunk_size)?;

    Ok(())
}
