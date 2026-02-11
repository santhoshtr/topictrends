use polars::prelude::*;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone)]
struct PageEditRecord {
    article_id: u32,
    date: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    let (wiki, output_file, chunk_size) = if args.len() >= 3 {
        let wiki = &args[1];
        let output = &args[2];
        let chunk = if args.len() >= 4 {
            args[3].parse().unwrap_or(100_000)
        } else {
            100_000
        };
        (wiki.as_str(), output.as_str(), chunk)
    } else {
        eprintln!("Usage: <program> <wiki> <output_file> [chunk_size]");
        eprintln!("Example: get-pageedits mlwiki output.parquet 100000");
        std::process::exit(1);
    };

    println!("=== Wikipedia Page Edits to Parquet Converter ===");
    println!("Wiki: {}", wiki);
    println!("Output: {}", output_file);
    println!("Chunk size: {}", chunk_size);

    convert_pageedits_to_parquet(wiki, output_file, chunk_size)?;

    Ok(())
}

/// Parse a TSV line from MediaWiki history dump
/// Returns (article_id, date) for revision-create events only
fn parse_line(line: &str) -> Result<PageEditRecord, Box<dyn std::error::Error>> {
    let parts: Vec<&str> = line.split('\t').collect();

    // MediaWiki history dumps have 76 columns
    if parts.len() < 76 {
        return Err("Invalid line format".into());
    }

    // Column mapping (0-indexed):
    // 1: event_entity (must be "revision")
    // 2: event_type (must be "create")
    // 3: event_timestamp
    // 26: page_id (0-indexed, so column 27 in 1-indexed)

    // Filter: only process revision-create events
    if parts[1] != "revision" || parts[2] != "create" {
        return Err("Not a revision-create event".into());
    }

    // Extract page_id (column 27, 0-indexed = 26)
    let article_id: u32 = parts[26].parse().map_err(|_| "Invalid page_id")?;

    // Extract date from timestamp (column 4, 0-indexed = 3)
    // Format: "2001-01-15 00:00:00.0" -> extract "2001-01-15"
    let timestamp = parts[3];
    let date = timestamp
        .split(' ')
        .next()
        .ok_or("Invalid timestamp format")?
        .to_string();

    Ok(PageEditRecord { article_id, date })
}

/// Load articles.parquet to get page_id → qid mapping
fn load_pageid_to_qid_mapping(wiki: &str) -> Result<HashMap<u32, u32>, Box<dyn std::error::Error>> {
    let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string());
    let articles_path = format!("{}/{}/articles.parquet", data_dir, wiki);

    println!("Loading article mappings from: {}", articles_path);

    let path = PlRefPath::try_from_path(Path::new(&articles_path))?;
    let df = LazyFrame::scan_parquet(path, Default::default())?.collect()?;

    let page_ids = df.column("page_id")?.u32()?;
    let qids = df.column("qid")?.u32()?;

    let mut mapping = HashMap::new();
    for i in 0..df.height() {
        if let (Some(page_id), Some(qid)) = (page_ids.get(i), qids.get(i)) {
            mapping.insert(page_id, qid);
        }
    }

    println!("Loaded {} page_id → qid mappings", mapping.len());
    Ok(mapping)
}

/// Process a chunk of edit records and aggregate by (article_qid, date)
/// Translates page_id to qid using the provided mapping
fn process_chunk(
    records: Vec<PageEditRecord>,
    pageid_to_qid: &HashMap<u32, u32>,
) -> Result<DataFrame, PolarsError> {
    // Aggregate: count edits per (article_qid, date) pair
    let mut aggregates: HashMap<(u32, String), u32> = HashMap::new();

    for record in records {
        // Translate page_id to qid
        if let Some(&qid) = pageid_to_qid.get(&record.article_id) {
            *aggregates.entry((qid, record.date)).or_insert(0) += 1;
        }
        // Skip records without qid mapping (articles without Wikidata items)
    }

    // Convert aggregated data to vectors for DataFrame
    let mut article_qids = Vec::with_capacity(aggregates.len());
    let mut dates = Vec::with_capacity(aggregates.len());
    let mut edit_counts = Vec::with_capacity(aggregates.len());

    for ((qid, date), count) in aggregates {
        article_qids.push(qid);
        dates.push(date);
        edit_counts.push(count);
    }

    let height = article_qids.len();

    DataFrame::new(
        height,
        vec![
            Column::new("article_qid".into(), article_qids),
            Column::new("date".into(), dates),
            Column::new("edit_count".into(), edit_counts),
        ],
    )
}

fn convert_pageedits_to_parquet(
    wiki: &str,
    output_path: &str,
    chunk_size: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting conversion...");

    // Load page_id → qid mapping first
    let pageid_to_qid = load_pageid_to_qid_mapping(wiki)?;
    let pageid_to_qid = Arc::new(pageid_to_qid);

    let stdin = std::io::stdin();
    let reader = BufReader::new(stdin.lock());

    let mut chunks = Vec::new();
    let mut current_chunk = Vec::with_capacity(chunk_size);
    let mut lines_processed = 0;
    let mut lines_filtered = 0;
    let bytes_read = Arc::new(AtomicUsize::new(0));

    println!("Reading and chunking data...");

    for line in reader.lines() {
        let line = line?;
        let line_bytes = line.len() + 1; // +1 for newline
        bytes_read.fetch_add(line_bytes, Ordering::Relaxed);

        lines_processed += 1;

        // Progress indicator every 100k lines
        if lines_processed % 100_000 == 0 {
            println!(
                "  Processed {} lines ({} filtered)",
                lines_processed, lines_filtered
            );
        }

        match parse_line(&line) {
            Ok(record) => {
                current_chunk.push(record);
                lines_filtered += 1;

                if current_chunk.len() >= chunk_size {
                    chunks.push(current_chunk);
                    current_chunk = Vec::with_capacity(chunk_size);
                }
            }
            Err(_) => {
                // Silently skip non-revision-create events and malformed lines
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

    println!("\nProcessed {} total lines", lines_processed);
    println!("Filtered {} revision-create events", lines_filtered);
    println!("Processing {} chunks in parallel...", chunks.len());

    // Process chunks in parallel
    let dataframes: Vec<DataFrame> = chunks
        .into_par_iter()
        .filter_map(|chunk| {
            let result = process_chunk(chunk, &pageid_to_qid);
            if let Err(e) = &result {
                eprintln!("Error processing chunk: {}", e);
            }
            result.ok()
        })
        .collect();

    if dataframes.is_empty() {
        return Err("No valid dataframes created".into());
    }

    println!("\nCombining {} dataframes...", dataframes.len());

    // Convert DataFrame to LazyFrame
    let lazy_frames: Vec<LazyFrame> = dataframes.into_iter().map(|df| df.lazy()).collect();

    // Combine all chunks and aggregate again (in case same article_qid+date across chunks)
    let combined = concat(&lazy_frames, UnionArgs::default())?
        .group_by([col("article_qid"), col("date")])
        .agg([col("edit_count").sum()]);

    println!("Writing to parquet file {} ", &output_path);

    // Create output file with error handling
    let mut file = File::create(output_path)
        .map_err(|e| format!("Failed to create output file '{}': {}", output_path, e))?;

    // Collect the final dataframe
    let mut dataframe = combined
        .collect()
        .map_err(|e| format!("Failed to collect final dataframe: {}", e))?;

    // Write to parquet with error handling
    ParquetWriter::new(&mut file)
        .with_compression(ParquetCompression::Snappy)
        .finish(&mut dataframe)
        .map_err(|e| format!("Failed to write parquet file '{}': {}", output_path, e))?;

    // Ensure all data is written to disk
    file.sync_all()
        .map_err(|e| format!("Failed to sync file to disk '{}': {}", output_path, e))?;

    println!("\n✓ Conversion complete!");
    println!("  Lines processed: {}", lines_processed);
    println!("  Revision-create events: {}", lines_filtered);
    println!("  Output records: {}", dataframe.height());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_revision_create_line() {
        let line = "mlwiki\trevision\tcreate\t2001-01-15 00:00:00.0\t\t\t\t\t\t\t\t\t\t\t\t\t\t\ttrue\t\t\t\t\t\t\t\t42410\tTestpage\tTestpage\t0\ttrue\t0\ttrue\t\ttrue\t2001-01-15 00:00:00.0\t2001-01-15 00:00:00.0\t1\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t208037\t\tfalse\t\tfalse\t396\t396\t8bj5cor742c5miko1b9fj7q6klg4kw6\t\t\ttrue\t2008-06-14 14:38:03.0\tfalse\t\t\tfalse\tfalse\t";
        let result = parse_line(line);
        assert!(result.is_ok());
        let record = result.unwrap();
        assert_eq!(record.article_id, 42410);
        assert_eq!(record.date, "2001-01-15");
    }

    #[test]
    fn test_skip_non_revision_events() {
        let line = "mlwiki\tpage\tcreate\t2001-01-15 00:00:00.0\t\t\t\t\t\t\t\t\t\t\t\t\t\t\ttrue\t\t\t\t\t\t\t\t42410\tTestpage\tTestpage\t0\ttrue\t0\ttrue\t\ttrue\t2001-01-15 00:00:00.0\t2001-01-15 00:00:00.0\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t";
        let result = parse_line(line);
        assert!(result.is_err());
    }

    #[test]
    fn test_skip_non_create_events() {
        let line = "mlwiki\trevision\tdelete\t2001-01-15 00:00:00.0\t\t\t\t\t\t\t\t\t\t\t\t\t\t\ttrue\t\t\t\t\t\t\t\t42410\tTestpage\tTestpage\t0\ttrue\t0\ttrue\t\ttrue\t2001-01-15 00:00:00.0\t2001-01-15 00:00:00.0\t1\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t208037\t\tfalse\t\tfalse\t396\t396\t8bj5cor742c5miko1b9fj7q6klg4kw6\t\t\ttrue\t2008-06-14 14:38:03.0\tfalse\t\t\tfalse\tfalse\t";
        let result = parse_line(line);
        assert!(result.is_err());
    }

    #[test]
    fn test_date_extraction() {
        let line = "mlwiki\trevision\tcreate\t2025-12-31 23:59:59.0\t\t\t\t\t\t\t\t\t\t\t\t\t\t\ttrue\t\t\t\t\t\t\t\t42410\tTestpage\tTestpage\t0\ttrue\t0\ttrue\t\ttrue\t2001-01-15 00:00:00.0\t2001-01-15 00:00:00.0\t1\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t208037\t\tfalse\t\tfalse\t396\t396\t8bj5cor742c5miko1b9fj7q6klg4kw6\t\t\ttrue\t2008-06-14 14:38:03.0\tfalse\t\t\tfalse\tfalse\t";
        let result = parse_line(line);
        assert!(result.is_ok());
        let record = result.unwrap();
        assert_eq!(record.date, "2025-12-31");
    }

    #[test]
    fn test_invalid_page_id() {
        let line = "mlwiki\trevision\tcreate\t2001-01-15 00:00:00.0\t\t\t\t\t\t\t\t\t\t\t\t\t\t\ttrue\t\t\t\t\t\t\t\tNOT_A_NUMBER\tTestpage\tTestpage\t0\ttrue\t0\ttrue\t\ttrue\t2001-01-15 00:00:00.0\t2001-01-15 00:00:00.0\t1\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t208037\t\tfalse\t\tfalse\t396\t396\t8bj5cor742c5miko1b9fj7q6klg4kw6\t\t\ttrue\t2008-06-14 14:38:03.0\tfalse\t\t\tfalse\tfalse\t";
        let result = parse_line(line);
        assert!(result.is_err());
    }
}
