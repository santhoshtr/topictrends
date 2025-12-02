use polars::prelude::*;
use rayon::prelude::*;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use topictrend::direct_map::DirectMap;

#[derive(Debug, Clone)]
struct PageView {
    wiki: String,
    qid: u32,
    access_method: String,
    daily_views: u32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 3 {
        eprintln!(
            "Usage: {} <articles_parquet> <output_file> [chunk_size]",
            args[0]
        );
        std::process::exit(1);
    }

    let articles_parquet = &args[1];
    let output_file = &args[2];
    let chunk_size = if args.len() >= 4 {
        args[3].parse().unwrap_or(100_000)
    } else {
        100_000
    };

    println!("=== Wikipedia Pageviews to Parquet Converter ===");
    println!("Articles parquet: {}", articles_parquet);
    println!("Output: {}", output_file);
    println!("Chunk size: {}\n", chunk_size);

    // Load articles.parquet to get valid article IDs and their QIDs
    let articles_parquet_path: PlPath = PlPath::Local(Arc::from(Path::new(&articles_parquet)));
    let articles_df =
        LazyFrame::scan_parquet(articles_parquet_path, Default::default())?.collect()?;

    let article_qids = articles_df.column("page_id")?.u32()?;
    let article_qids = articles_df.column("qid")?.u32()?;

    // Build DirectMap for page_id -> qid conversion
    let article_qid_to_qid: DirectMap = article_qids
        .into_iter()
        .zip(article_qids.into_iter())
        .filter_map(|(id, qid)| Some((id?, qid?)))
        .collect();

    println!(
        "Loaded {} article ID mappings",
        article_qid_to_qid.keys().len()
    );

    convert_pageviews_to_parquet(output_file, chunk_size, article_qid_to_qid)?;

    Ok(())
}

fn parse_line(
    line: &str,
    id_to_qid_map: &DirectMap,
) -> Result<Option<PageView>, Box<dyn std::error::Error>> {
    let parts: Vec<&str> = line.splitn(6, ' ').collect();

    if parts.len() < 5 {
        return Err("Invalid line format".into());
    }

    let page_id: u32 = parts[2].parse()?;

    // Convert page_id to qid, return None if not found (filters out non-main namespace articles)
    let qid = match id_to_qid_map.get(page_id) {
        Some(qid) => qid,
        None => return Ok(None), // Skip articles not in main namespace
    };

    Ok(Some(PageView {
        wiki: parts[0].to_string(),
        qid,
        access_method: parts[3].to_string(),
        daily_views: parts[4].parse()?,
    }))
}

fn process_chunk(records: Vec<PageView>) -> Result<DataFrame, PolarsError> {
    // We consistantly use projects as enwiki, tawiki etc, however pageview dumps has en.wikipedia,
    // ta.wikipedia. Normalize.
    let wiki: Vec<String> = records
        .iter()
        .map(|r| r.wiki.replace(".wikipedia", "wiki"))
        .collect();
    let qid: Vec<u32> = records.iter().map(|r| r.qid).collect();
    // Save some space by mapping the access methods to numbers
    let access_method: Vec<i8> = records
        .iter()
        .map(|r| match r.access_method.as_str() {
            "mobile-web" => 1,
            "desktop" => 0,
            _ => -1, // Default value for unexpected cases
        })
        .collect();
    let daily_views: Vec<u32> = records.iter().map(|r| r.daily_views).collect();

    DataFrame::new(vec![
        Column::new("wiki".into(), wiki),
        Column::new("qid".into(), qid),
        Column::new("access_method".into(), access_method),
        Column::new("daily_views".into(), daily_views),
    ])
}

fn convert_pageviews_to_parquet(
    output_path: &str,
    chunk_size: usize,
    id_to_qid_map: DirectMap,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting conversion...");

    let stdin = std::io::stdin();
    let reader = BufReader::new(stdin.lock());

    let mut chunks = Vec::new();
    let mut current_chunk = Vec::with_capacity(chunk_size);
    let mut lines_processed = 0;
    let mut valid_records = 0;
    let bytes_read = Arc::new(AtomicUsize::new(0));

    println!("Reading and chunking data...");

    for line in reader.lines() {
        let line = line?;
        let line_bytes = line.len() + 1; // +1 for newline
        bytes_read.fetch_add(line_bytes, Ordering::Relaxed);
        lines_processed += 1;

        match parse_line(&line, &id_to_qid_map) {
            Ok(Some(record)) => {
                current_chunk.push(record);
                valid_records += 1;

                if current_chunk.len() >= chunk_size {
                    chunks.push(current_chunk);
                    current_chunk = Vec::with_capacity(chunk_size);
                }
            }
            Ok(None) => {
                // Skip records not in main namespace (silently)
                continue;
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

    println!(
        "\nProcessed {} lines, {} valid records in {} chunks",
        lines_processed,
        valid_records,
        chunks.len()
    );
    println!("Processing {} chunks in parallel...", chunks.len());

    // Process chunks in parallel
    let dataframes: Vec<DataFrame> = chunks
        .into_par_iter()
        .filter_map(|chunk| {
            let result = process_chunk(chunk);
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
    println!("  Valid records: {}", valid_records);

    Ok(())
}
