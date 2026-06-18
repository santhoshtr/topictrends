// Step-0 study harness for the v2 plan (.plans/topictrends-v2.md): measure the
// canonical (cross-wiki union) category membership against v1 depth traversal.
// Throwaway — not part of the ETL pipeline; delete when step 0 concludes.
//
// Modes:
//   v2-study compare --wiki enwiki --categories 558331,1457982,...
//     For each category C: |canonical_set(C)|, |canonical ∩ W| (k=1 and k>=2),
//     v1 |get_articles_in_category(C, depth 0/1/2)|, Jaccard(canonical∩W, d2),
//     top members by wiki_count, recovered (canonical − d2) and lost
//     (d2 − canonical) samples with titles. Markdown to stdout.
//
//   v2-study histogram
//     wiki_count distribution over all canonical (article, category) edges
//     across the wiki universe — the k=1 noise-surface number.
//
// Wiki universe: data/wikipedia.list, skipping wikis without topology files.

use polars::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use topictrend::graphbuilder::GraphBuilder;

fn data_dir() -> String {
    std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string())
}

fn arg(args: &[String], key: &str) -> Option<String> {
    args.iter().position(|a| a == key).and_then(|i| args.get(i + 1)).cloned()
}

fn read_parquet(path: &str) -> Result<DataFrame, Box<dyn std::error::Error>> {
    let p = PlRefPath::try_from_path(Path::new(path))?;
    Ok(LazyFrame::scan_parquet(p, Default::default())?.collect()?)
}

fn wiki_list(data: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(std::fs::read_to_string(format!("{}/wikipedia.list", data))?
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("compare") => compare(&args),
        Some("histogram") => histogram(),
        _ => {
            eprintln!("Usage: v2-study compare --wiki <wiki> --categories <qid,qid,...>");
            eprintln!("       v2-study histogram");
            std::process::exit(1);
        }
    }
}

fn histogram() -> Result<(), Box<dyn std::error::Error>> {
    let data = data_dir();
    let wikis = wiki_list(&data)?;

    // (article, category) packed into u64 so sort+run-length gives multiplicity.
    let mut pairs: Vec<u64> = Vec::new();
    let mut raw_total: u64 = 0;
    for wiki in &wikis {
        let path = format!("{}/{}/article_category.parquet", data, wiki);
        if !Path::new(&path).exists() {
            eprintln!("  skipping {} (no article_category.parquet)", wiki);
            continue;
        }
        let df = read_parquet(&path)?;
        let a = df.column("article_qid")?.u32()?;
        let c = df.column("category_qid")?.u32()?;
        for (oa, oc) in a.into_iter().zip(c) {
            if let (Some(a), Some(c)) = (oa, oc) {
                pairs.push(((a as u64) << 32) | c as u64);
                raw_total += 1;
            }
        }
        eprintln!("  loaded {} ({} cumulative)", wiki, pairs.len());
    }
    pairs.sort_unstable();

    let mut hist: HashMap<u32, u64> = HashMap::new();
    let mut distinct: u64 = 0;
    let mut i = 0;
    while i < pairs.len() {
        let mut j = i + 1;
        while j < pairs.len() && pairs[j] == pairs[i] {
            j += 1;
        }
        *hist.entry((j - i) as u32).or_insert(0) += 1;
        distinct += 1;
        i = j;
    }

    println!("# wiki_count histogram over canonical edges");
    println!();
    println!("- wikis scanned: {}", wikis.len());
    println!("- raw (article, category) rows: {}", raw_total);
    println!("- distinct canonical edges: {}", distinct);
    println!();
    println!("| wiki_count | edges | % of edges | cumulative % |");
    println!("|---:|---:|---:|---:|");
    let mut ks: Vec<u32> = hist.keys().copied().collect();
    ks.sort_unstable();
    let mut cum: u64 = 0;
    for k in ks {
        let n = hist[&k];
        cum += n;
        println!(
            "| {} | {} | {:.2}% | {:.2}% |",
            k,
            n,
            n as f64 * 100.0 / distinct as f64,
            cum as f64 * 100.0 / distinct as f64
        );
    }
    Ok(())
}

struct EdgeInfo {
    wiki_count: u32,
    in_target: bool, // target wiki itself files the article under the category
}

fn compare(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let target = arg(args, "--wiki").unwrap_or_else(|| {
        eprintln!("Usage: v2-study compare --wiki <wiki> --categories <qid,qid,...>");
        std::process::exit(1);
    });
    let cats: Vec<u32> = arg(args, "--categories")
        .map(|s| s.split(',').filter_map(|t| t.trim().trim_start_matches('Q').parse().ok()).collect())
        .unwrap_or_default();
    if cats.is_empty() {
        eprintln!("--categories is required (comma-separated QIDs)");
        std::process::exit(1);
    }

    let data = data_dir();
    let wikis = wiki_list(&data)?;

    eprintln!("Building {} graph...", target);
    let graph = GraphBuilder::new(&target).build()?;

    // Titles in the target wiki, for readable samples.
    let arts = read_parquet(&format!("{}/{}/articles.parquet", data, target))?;
    let title: HashMap<u32, String> = arts
        .column("qid")?
        .u32()?
        .into_iter()
        .zip(arts.column("page_title")?.str()?)
        .filter_map(|(q, t)| Some((q?, t?.replace('_', " "))))
        .collect();

    // English-first category labels for headers (fallback: target wiki's own).
    let mut cat_label: HashMap<u32, String> = HashMap::new();
    let needed: HashSet<u32> = cats.iter().copied().collect();
    for w in ["enwiki", target.as_str()] {
        let path = format!("{}/{}/categories.parquet", data, w);
        if !Path::new(&path).exists() {
            continue;
        }
        let df = read_parquet(&path)?;
        for (q, t) in df.column("qid")?.u32()?.into_iter().zip(df.column("page_title")?.str()?) {
            if let (Some(q), Some(t)) = (q, t)
                && needed.contains(&q)
                && !cat_label.contains_key(&q)
            {
                cat_label.insert(q, t.replace('_', " "));
            }
        }
    }

    // Canonical edges for the requested categories across the universe.
    eprintln!("Scanning {} wikis for canonical membership...", wikis.len());
    let cat_filter = cats
        .iter()
        .map(|c| col("category_qid").eq(lit(*c)))
        .reduce(|a, b| a.or(b))
        .unwrap();
    let mut edges: HashMap<(u32, u32), EdgeInfo> = HashMap::new();
    for wiki in &wikis {
        let path = format!("{}/{}/article_category.parquet", data, wiki);
        if !Path::new(&path).exists() {
            continue;
        }
        let p = PlRefPath::try_from_path(Path::new(&path))?;
        let df = LazyFrame::scan_parquet(p, Default::default())?
            .filter(cat_filter.clone())
            .select([col("article_qid"), col("category_qid")])
            .collect()?;
        let a = df.column("article_qid")?.u32()?;
        let c = df.column("category_qid")?.u32()?;
        let is_target = wiki == &target;
        for (oa, oc) in a.into_iter().zip(c) {
            if let (Some(a), Some(c)) = (oa, oc) {
                let e = edges.entry((c, a)).or_insert(EdgeInfo { wiki_count: 0, in_target: false });
                e.wiki_count += 1;
                e.in_target |= is_target;
            }
        }
    }

    println!("# v2-study compare — target: {}", target);
    for &c in &cats {
        let label = cat_label.get(&c).map(String::as_str).unwrap_or("?");
        let canonical: Vec<(u32, u32, bool)> = edges
            .iter()
            .filter(|((cat, _), _)| *cat == c)
            .map(|((_, a), e)| (*a, e.wiki_count, e.in_target))
            .collect();
        let canonical_total = canonical.len();
        let mut in_w: Vec<(u32, u32, bool)> = canonical
            .into_iter()
            .filter(|(a, _, _)| graph.art_original_to_dense.get(*a).is_some())
            .collect();
        in_w.sort_unstable_by(|x, y| y.1.cmp(&x.1).then(x.0.cmp(&y.0)));
        let in_w_set: HashSet<u32> = in_w.iter().map(|(a, _, _)| *a).collect();
        let k2 = in_w.iter().filter(|(_, n, _)| *n >= 2).count();

        let v1: Vec<HashSet<u32>> = (0..=2)
            .map(|d| graph.get_articles_in_category(c, d).unwrap().into_iter().collect())
            .collect();
        let d2 = &v1[2];

        let inter = in_w_set.intersection(d2).count();
        let union = in_w_set.union(d2).count();
        let jaccard = if union > 0 { inter as f64 / union as f64 } else { 0.0 };

        let mut recovered: Vec<&(u32, u32, bool)> =
            in_w.iter().filter(|(a, _, _)| !d2.contains(a)).collect();
        // already sorted by wiki_count desc
        let mut lost: Vec<u32> = d2.difference(&in_w_set).copied().collect();
        lost.sort_unstable();

        println!();
        println!("## Q{} — {}", c, label);
        println!();
        println!("| metric | value |");
        println!("|---|---:|");
        println!("| canonical (all wikis) | {} |", canonical_total);
        println!("| canonical ∩ {} (k=1) | {} |", target, in_w.len());
        println!("| canonical ∩ {} (k>=2) | {} |", target, k2);
        println!("| v1 depth 0 | {} |", v1[0].len());
        println!("| v1 depth 1 | {} |", v1[1].len());
        println!("| v1 depth 2 | {} |", v1[2].len());
        println!("| Jaccard(canonical∩W, d2) | {:.3} |", jaccard);
        println!("| recovered (canonical − d2) | {} |", recovered.len());
        println!("| lost (d2 − canonical) | {} |", lost.len());

        println!();
        println!("Top members by wiki_count (self = {} files it directly):", target);
        for (a, n, self_) in in_w.iter().take(20) {
            let t = title.get(a).map(String::as_str).unwrap_or("?");
            println!("- {} wikis, self={} — Q{} {}", n, if *self_ { "y" } else { "n" }, a, t);
        }

        recovered.truncate(5);
        println!();
        println!("Recovered samples (in canonical∩W, missed by v1 depth 2):");
        for (a, n, _) in recovered {
            let t = title.get(a).map(String::as_str).unwrap_or("?");
            println!("- {} wikis — Q{} {}", n, a, t);
        }

        println!();
        println!("Lost samples (in v1 depth 2, not in canonical):");
        for a in lost.iter().take(5) {
            let t = title.get(a).map(String::as_str).unwrap_or("?");
            println!("- Q{} {}", a, t);
        }
    }
    Ok(())
}
