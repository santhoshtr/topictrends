use polars::prelude::*;
use std::collections::HashMap;
use std::io::{self, BufRead};
use std::path::Path;
use topictrend::pageedit_parquet::write_pageedit_parquet;

/// Build a single day's pageedit Parquet from the replica.
///
/// Reads the day-aggregated query output `(page_id, edit_count)` on stdin
/// (piped from `queries/day-pageedits.sql` via the mariadb client), maps
/// `page_id → qid` through `articles.parquet`, re-aggregates by qid, and
/// writes `(qid, edit_count)` to the per-day Parquet path — the same layout
/// and schema the dump backfill produces, so both converge on identical files.
///
/// Single-file producer: it always writes the path it is given. Idempotency
/// (don't rebuild an existing day) is handled by the Makefile, which only
/// invokes this when the target file is missing — the same convention as
/// `get-pageviews` / `get-per_day_wiki_stats`. The dump backfill's
/// `get-pageedits`, by contrast, writes many files in one run and so guards
/// each with write-if-missing itself.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <wiki> <output_path>", args[0]);
        eprintln!("Example: get-day-pageedits mlwiki data/mlwiki/pageedits/2026/05/26.parquet");
        std::process::exit(1);
    }
    let wiki = &args[1];
    let output_path = &args[2];

    let pageid_to_qid = load_pageid_to_qid_mapping(wiki)?;

    // Aggregate by qid: multiple page_ids mapping to the same qid is rare but
    // possible (e.g. merges), so sum rather than overwrite.
    let mut by_qid: HashMap<u32, u32> = HashMap::new();
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line?;
        let mut parts = line.split('\t');
        // The mariadb header row ("page_id\tedit_count") fails u32 parsing and
        // is skipped, as do any malformed rows.
        let Some(page_id) = parts.next().and_then(|s| s.parse::<u32>().ok()) else {
            continue;
        };
        let Some(edit_count) = parts.next().and_then(|s| s.parse::<u32>().ok()) else {
            continue;
        };
        if let Some(&qid) = pageid_to_qid.get(&page_id) {
            *by_qid.entry(qid).or_insert(0) += edit_count;
        }
    }

    let mut pairs: Vec<(u32, u32)> = by_qid.into_iter().collect();
    pairs.sort_unstable_by_key(|&(qid, _)| qid);

    write_pageedit_parquet(&pairs, output_path)?;
    println!("Wrote {} articles with edits to {}", pairs.len(), output_path);

    Ok(())
}

/// Load `articles.parquet` to get the `page_id → qid` mapping.
fn load_pageid_to_qid_mapping(wiki: &str) -> Result<HashMap<u32, u32>, Box<dyn std::error::Error>> {
    let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string());
    let articles_path = format!("{}/{}/articles.parquet", data_dir, wiki);

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

    Ok(mapping)
}
