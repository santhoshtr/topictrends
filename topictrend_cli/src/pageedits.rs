use polars::prelude::*;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone)]
struct PageEditRecord {
    article_id: u32,
    date: String,
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

    println!("=== Wikipedia Page Edits to Parquet Converter ===");
    println!("Output: {}", output_file);
    println!("Chunk size: {}", chunk_size);

    convert_pageedits_to_parquet(output_file, chunk_size)?;

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

/// Process a chunk of edit records and aggregate by (article_id, date)
fn process_chunk(records: Vec<PageEditRecord>) -> Result<DataFrame, PolarsError> {
    // Aggregate: count edits per (article_id, date) pair
    let mut aggregates: HashMap<(u32, String), u32> = HashMap::new();

    for record in records {
        *aggregates
            .entry((record.article_id, record.date))
            .or_insert(0) += 1;
    }

    // Convert aggregated data to vectors for DataFrame
    let mut article_ids = Vec::with_capacity(aggregates.len());
    let mut dates = Vec::with_capacity(aggregates.len());
    let mut edit_counts = Vec::with_capacity(aggregates.len());

    for ((article_id, date), count) in aggregates {
        article_ids.push(article_id);
        dates.push(date);
        edit_counts.push(count);
    }

    let height = article_ids.len();

    DataFrame::new(
        height,
        vec![
            Column::new("article_id".into(), article_ids),
            Column::new("date".into(), dates),
            Column::new("edit_count".into(), edit_counts),
        ],
    )
}

fn convert_pageedits_to_parquet(
    output_path: &str,
    chunk_size: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting conversion...");

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
            let result = process_chunk(chunk);
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

    // Combine all chunks and aggregate again (in case same article_id+date across chunks)
    let combined = concat(&lazy_frames, UnionArgs::default())?
        .group_by([col("article_id"), col("date")])
        .agg([col("edit_count").sum()]);

    println!("Writing to parquet file {} ", &output_path);
    let mut file = File::create(output_path)?;
    let mut dataframe = combined.collect()?;

    ParquetWriter::new(&mut file)
        .with_compression(ParquetCompression::Snappy)
        .finish(&mut dataframe)?;

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
