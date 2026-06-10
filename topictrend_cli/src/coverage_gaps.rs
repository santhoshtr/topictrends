// Gap-discovery ranking over the coverage matrix.
//
// For a target wiki W, rank categories by how many of a topic's articles W lacks
// relative to a reference edition (default enwiki):
//
//   gap(C, W) = qid_overlap(C, reference) - qid_overlap(C, W)
//
// qid_overlap(C, X) is the count of category C's canonical articles that exist as
// articles in X. So the gap is "articles about topic C that the reference edition
// has and W does not" — a translation/creation worklist. `direct > 0` for W tells
// you whether W even has the category structure, separating a content gap from a
// structure gap.
//
// Reads the dated coverage snapshots produced by `make coverage` and resolves
// category labels from the reference wiki's categories.parquet (page_title).
//
// Usage: coverage-gaps --wiki mlwiki [--reference enwiki] [--date YYYY-MM-DD]
//                      [--top 50] [--min-ref 0]

use polars::prelude::*;
use std::collections::HashMap;
use std::path::Path;

fn data_dir() -> String {
    std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string())
}

fn arg(args: &[String], key: &str) -> Option<String> {
    args.iter().position(|a| a == key).and_then(|i| args.get(i + 1)).cloned()
}

fn read(path: &str) -> Result<DataFrame, Box<dyn std::error::Error>> {
    let p = PlRefPath::try_from_path(Path::new(path))?;
    Ok(LazyFrame::scan_parquet(p, Default::default())?.collect()?)
}

/// Newest snapshot date under data/<wiki>/coverage/ (lexicographic = chronological).
fn latest_snapshot(data: &str, wiki: &str) -> Option<String> {
    let dir = format!("{}/{}/coverage", data, wiki);
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().to_str()?.strip_suffix(".parquet").map(String::from))
        .max()
}

/// category_qid -> (direct_coverage, qid_overlap_coverage)
fn load_coverage(path: &str) -> Result<HashMap<u32, (u32, u32)>, Box<dyn std::error::Error>> {
    let df = read(path)?;
    let c = df.column("category_qid")?.u32()?;
    let d = df.column("direct_coverage")?.u32()?;
    let o = df.column("qid_overlap_coverage")?.u32()?;
    Ok(c.into_iter()
        .zip(d)
        .zip(o)
        .filter_map(|((c, d), o)| Some((c?, (d?, o?))))
        .collect())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let data = data_dir();

    let wiki = arg(&args, "--wiki").unwrap_or_else(|| {
        eprintln!("Usage: coverage-gaps --wiki <wiki> [--reference enwiki] [--date YYYY-MM-DD] [--top 50] [--min-ref 0]");
        std::process::exit(1);
    });
    let reference = arg(&args, "--reference").unwrap_or_else(|| "enwiki".to_string());
    let top: usize = arg(&args, "--top").and_then(|s| s.parse().ok()).unwrap_or(50);
    let min_ref: u32 = arg(&args, "--min-ref").and_then(|s| s.parse().ok()).unwrap_or(0);
    // Upper bound on reference overlap. Cross-cutting meta/maintenance categories
    // (Living people, stubs, disambiguation, surnames, by-year) have huge overlap
    // and dominate the raw gap ranking without being editathon-actionable; cap to
    // surface mid-tier topical gaps. 0 = no cap.
    let max_ref: u32 = arg(&args, "--max-ref").and_then(|s| s.parse().ok()).unwrap_or(0);
    let date = arg(&args, "--date")
        .or_else(|| latest_snapshot(&data, &wiki))
        .expect("no coverage snapshot found; pass --date");

    let w_cov = load_coverage(&format!("{}/{}/coverage/{}.parquet", data, wiki, date))?;
    let ref_cov = load_coverage(&format!("{}/{}/coverage/{}.parquet", data, reference, date))?;

    // Labels: prefer enwiki (English, most complete) for readability; fall back
    // to the reference wiki's categories.parquet for categories enwiki lacks.
    let load_labels = |w: &str| -> Result<HashMap<u32, String>, Box<dyn std::error::Error>> {
        let cats = read(&format!("{}/{}/categories.parquet", data, w))?;
        Ok(cats
            .column("qid")?
            .u32()?
            .into_iter()
            .zip(cats.column("page_title")?.str()?)
            .filter_map(|(q, t)| Some((q?, t?.to_string())))
            .collect())
    };
    let labels = load_labels("enwiki")?;
    let fallback = if reference == "enwiki" {
        HashMap::new()
    } else {
        load_labels(&reference)?
    };

    // gap = ref_overlap - w_overlap, over categories the reference covers.
    let mut ranked: Vec<(u32, u32, u32, u32, u32)> = Vec::new(); // (gap, ref_ovl, w_ovl, w_direct, qid)
    for (&qid, &(_ref_direct, ref_ovl)) in &ref_cov {
        if ref_ovl < min_ref || (max_ref > 0 && ref_ovl > max_ref) {
            continue;
        }
        let (w_direct, w_ovl) = w_cov.get(&qid).copied().unwrap_or((0, 0));
        if ref_ovl > w_ovl {
            ranked.push((ref_ovl - w_ovl, ref_ovl, w_ovl, w_direct, qid));
        }
    }
    ranked.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));

    println!(
        "# Gap discovery for {wiki} vs {reference} (snapshot {date})",
    );
    println!("# gap = {reference} qid_overlap - {wiki} qid_overlap = canonical articles {wiki} lacks");
    println!("# has_cat = '{wiki}' files >=1 article directly under the category (structure present)");
    println!("{:>4}  {:>9}  {:>9}  {:>9}  {:>6}  {:>7}  {:>11}  title", "rank", "gap", "ref_ovl", "w_ovl", "cov%", "has_cat", "qid");
    for (i, &(gap, ref_ovl, w_ovl, w_direct, qid)) in ranked.iter().take(top).enumerate() {
        let cov = 100.0 * w_ovl as f64 / ref_ovl as f64;
        let has = if w_direct > 0 { "yes" } else { "no" };
        let title = labels.get(&qid).or_else(|| fallback.get(&qid)).map(String::as_str).unwrap_or("?");
        println!(
            "{:>4}  {:>9}  {:>9}  {:>9}  {:>5.1}%  {:>7}  {:>11}  {}",
            i + 1, gap, ref_ovl, w_ovl, cov, has, qid, title
        );
    }
    eprintln!("\n{} categories with a positive gap (min-ref={}); showing top {}.", ranked.len(), min_ref, top.min(ranked.len()));
    Ok(())
}
