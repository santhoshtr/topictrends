// Materialize the canonical cross-wiki article→category relation.
//
// Articles and categories share Wikidata QIDs across editions, so the
// per-wiki article_category relations can be unioned into one global
// relation, with a per-edge count of how many wikis assert the assignment
// (the categorization-consensus signal). Streams every wiki's
// article_category.parquet and writes a dated snapshot:
//
//   data/canonical/{date}/article_category.parquet
//     (article_qid: u32, category_qid: u32, wiki_count: u32),
//     sorted by (article_qid, category_qid), chunked row groups
//   data/canonical/{date}/manifest.tsv
//     per-wiki input row counts (the sanity-gate baseline for the next run)
//
// Sanity gate: against the most recent previous snapshot's manifest, abort if
// any wiki's input row count dropped below 50% (a truncated replica fetch
// shrinking everyone's canonical sets is the known failure mode). --force
// overrides.
//
// Usage: canonical-membership --date YYYY-MM-DD [--force] [wiki ...]
//   trailing wikis : override the wiki universe (defaults to data/wikipedia.list).

use parquet::file::properties::WriterProperties;
use parquet::file::writer::SerializedFileWriter;
use parquet::record::RecordWriter as _;
use parquet_derive::ParquetRecordWriter;
use polars::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

const ROW_GROUP_SIZE: usize = 16_000_000;
const MIN_INPUT_RATIO: f64 = 0.5;

// wiki_count is u32 (not u16) deliberately: the workspace polars cannot scan
// UInt16 parquet columns, and every downstream consumer reads via polars.
#[derive(Debug, ParquetRecordWriter)]
struct CanonicalEdge {
    article_qid: u32,
    category_qid: u32,
    wiki_count: u32,
}

fn data_dir() -> String {
    std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string())
}

fn read_parquet(path: &str) -> Result<DataFrame, Box<dyn std::error::Error>> {
    let p = PlRefPath::try_from_path(Path::new(path))?;
    Ok(LazyFrame::scan_parquet(p, Default::default())?.collect()?)
}

/// Most recent snapshot date (lexicographic on YYYY-MM-DD dir names) strictly
/// before `date` that has a manifest, with its per-wiki row counts.
fn previous_manifest(
    canonical_dir: &str,
    date: &str,
) -> Result<Option<(String, HashMap<String, u64>)>, Box<dyn std::error::Error>> {
    let dir = Path::new(canonical_dir);
    if !dir.is_dir() {
        return Ok(None);
    }
    let mut dates: Vec<String> = std::fs::read_dir(dir)?
        .filter_map(|e| {
            let e = e.ok()?;
            let name = e.file_name().into_string().ok()?;
            (e.path().join("manifest.tsv").is_file() && name.as_str() < date).then_some(name)
        })
        .collect();
    dates.sort_unstable();
    let Some(prev) = dates.pop() else {
        return Ok(None);
    };
    let mut counts = HashMap::new();
    for line in std::fs::read_to_string(format!("{}/{}/manifest.tsv", canonical_dir, prev))?.lines()
    {
        let mut parts = line.split('\t');
        if let (Some(wiki), Some(n)) = (parts.next(), parts.next())
            && let Ok(n) = n.parse::<u64>()
        {
            counts.insert(wiki.to_string(), n);
        }
    }
    Ok(Some((prev, counts)))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let date = args
        .iter()
        .position(|a| a == "--date")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(|| {
            eprintln!("Usage: canonical-membership --date YYYY-MM-DD [--force] [wiki ...]");
            std::process::exit(1);
        });
    let force = args.iter().any(|a| a == "--force");
    let positional: Vec<String> = args
        .iter()
        .skip(1)
        .filter(|a| !a.starts_with("--") && **a != date)
        .cloned()
        .collect();

    let data = data_dir();
    let wikis: Vec<String> = if positional.is_empty() {
        std::fs::read_to_string(format!("{}/wikipedia.list", data))?
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect()
    } else {
        positional
    };

    let canonical_dir = format!("{}/canonical", data);
    let start = Instant::now();

    // Load: pack (article, category) into u64 so sort + run-length gives the
    // per-edge wiki count. ~240M pairs over 337 wikis ≈ 2GB resident.
    let mut pairs: Vec<u64> = Vec::new();
    let mut input_rows: Vec<(String, u64)> = Vec::with_capacity(wikis.len());
    for wiki in &wikis {
        let path = format!("{}/{}/article_category.parquet", data, wiki);
        if !Path::new(&path).exists() {
            eprintln!("  WARNING: skipping {} (no article_category.parquet)", wiki);
            input_rows.push((wiki.clone(), 0));
            continue;
        }
        let df = read_parquet(&path)?;
        let a = df.column("article_qid")?.u32()?;
        let c = df.column("category_qid")?.u32()?;
        let mut rows: u64 = 0;
        for (oa, oc) in a.iter().zip(c.iter()) {
            if let (Some(a), Some(c)) = (oa, oc) {
                pairs.push(((a as u64) << 32) | c as u64);
                rows += 1;
            }
        }
        input_rows.push((wiki.clone(), rows));
        eprintln!("  loaded {} ({} rows, {} cumulative)", wiki, rows, pairs.len());
    }

    // Sanity gate against the previous snapshot's manifest.
    if let Some((prev_date, prev)) = previous_manifest(&canonical_dir, &date)? {
        let current: HashMap<&str, u64> =
            input_rows.iter().map(|(w, n)| (w.as_str(), *n)).collect();
        let offenders: Vec<String> = prev
            .iter()
            .filter(|(wiki, prev_n)| {
                **prev_n > 0
                    && (*current.get(wiki.as_str()).unwrap_or(&0) as f64)
                        < **prev_n as f64 * MIN_INPUT_RATIO
            })
            .map(|(wiki, &prev_n)| {
                format!(
                    "  {}: {} -> {} rows",
                    wiki,
                    prev_n,
                    current.get(wiki.as_str()).unwrap_or(&0)
                )
            })
            .collect();
        if !offenders.is_empty() {
            eprintln!(
                "Sanity gate vs snapshot {}: {} wiki(s) lost >{}% of article_category rows:",
                prev_date,
                offenders.len(),
                ((1.0 - MIN_INPUT_RATIO) * 100.0) as u32
            );
            for o in &offenders {
                eprintln!("{}", o);
            }
            if force {
                eprintln!("--force given, continuing anyway.");
            } else {
                eprintln!("Aborting (re-run with --force to override).");
                std::process::exit(2);
            }
        } else {
            eprintln!("Sanity gate vs snapshot {}: OK", prev_date);
        }
    } else {
        eprintln!("No previous snapshot manifest found; sanity gate skipped.");
    }

    let raw_total = pairs.len();
    eprintln!("Sorting {} raw pairs...", raw_total);
    pairs.sort_unstable();

    // Write: run-length encode into chunked row groups.
    let out_dir = format!("{}/{}", canonical_dir, date);
    std::fs::create_dir_all(&out_dir)?;
    let out_path = format!("{}/article_category.parquet", out_dir);

    // Schema is derived from a throwaway record slice (parquet_derive's
    // RecordWriter exposes it per-slice, not per-type).
    let probe = [CanonicalEdge { article_qid: 0, category_qid: 0, wiki_count: 0 }];
    let schema = probe.as_slice().schema()?;
    let props = Arc::new(
        WriterProperties::builder()
            .set_compression(parquet::basic::Compression::SNAPPY)
            .build(),
    );
    let mut writer = SerializedFileWriter::new(File::create(&out_path)?, schema, props)?;

    let mut buffer: Vec<CanonicalEdge> = Vec::with_capacity(ROW_GROUP_SIZE);
    let mut distinct: u64 = 0;
    let mut max_count: u32 = 0;
    let mut i = 0;
    while i < pairs.len() {
        let mut j = i + 1;
        while j < pairs.len() && pairs[j] == pairs[i] {
            j += 1;
        }
        let wiki_count = (j - i) as u32;
        max_count = max_count.max(wiki_count);
        buffer.push(CanonicalEdge {
            article_qid: (pairs[i] >> 32) as u32,
            category_qid: pairs[i] as u32,
            wiki_count,
        });
        distinct += 1;
        if buffer.len() == ROW_GROUP_SIZE {
            let mut rg = writer.next_row_group()?;
            buffer.as_slice().write_to_row_group(&mut rg)?;
            rg.close()?;
            buffer.clear();
        }
        i = j;
    }
    if !buffer.is_empty() {
        let mut rg = writer.next_row_group()?;
        buffer.as_slice().write_to_row_group(&mut rg)?;
        rg.close()?;
    }
    writer.close()?;

    // Manifest last, so a crashed build never becomes a gate baseline.
    let mut manifest = String::new();
    for (wiki, rows) in &input_rows {
        manifest.push_str(&format!("{}\t{}\n", wiki, rows));
    }
    std::fs::write(format!("{}/manifest.tsv", out_dir), manifest)?;

    println!(
        "Canonical snapshot {}: {} wikis, {} raw rows -> {} distinct edges (max wiki_count {}) in {:.0?}",
        date,
        wikis.len(),
        raw_total,
        distinct,
        max_count,
        start.elapsed()
    );
    println!("Wrote {}", out_path);
    Ok(())
}
