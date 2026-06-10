// Manual-inspection helper for the coverage matrix: list the actual articles
// behind a `qid_overlap` cell.
//
// For a category C and a target wiki W, prints every article that exists in W
// AND is filed *directly* under C in at least one Wikipedia edition — i.e. the
// canonical-set ∩ W that `qid_overlap(C, W)` only counts. Each row is annotated
// with how many (and which) wikis map it under C, and whether W maps it itself.
// Titles are resolved locally from W's articles.parquet (no MariaDB needed).
//
// Usage: coverage-articles --category 1410960 --wiki mlwiki [--only-foreign] [wiki ...]
//   --only-foreign : drop articles W already files directly under C (keep only
//                    the ones *other* wikis mapped).
//   trailing wikis : override the wiki universe (defaults to data/wikipedia.list).

use polars::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::Path;

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let category: u32 = arg(&args, "--category")
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| {
            eprintln!("Usage: coverage-articles --category <qid> --wiki <wiki> [--only-foreign] [wiki ...]");
            std::process::exit(1);
        });
    let target = arg(&args, "--wiki").unwrap_or_else(|| {
        eprintln!("Usage: coverage-articles --category <qid> --wiki <wiki> [--only-foreign] [wiki ...]");
        std::process::exit(1);
    });
    let only_foreign = args.iter().any(|a| a == "--only-foreign");

    let data = data_dir();
    // Collect trailing positional wikis, skipping flags by position (a token
    // following --category/--wiki is a flag value, not a wiki — and the target
    // may legitimately appear in the trailing list).
    let mut positional: Vec<String> = Vec::new();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--category" | "--wiki" => i += 2,
            s if s.starts_with("--") => i += 1,
            s => {
                positional.push(s.to_string());
                i += 1;
            }
        }
    }
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

    // Target article QID -> title (also the membership set we intersect against).
    let arts = read_parquet(&format!("{}/{}/articles.parquet", data, target))?;
    let title: HashMap<u32, String> = arts
        .column("qid")?
        .u32()?
        .into_iter()
        .zip(arts.column("page_title")?.str()?)
        .filter_map(|(q, t)| Some((q?, t?.to_string())))
        .collect();
    let target_qids: HashSet<u32> = title.keys().copied().collect();

    // article_qid -> wikis that file it directly under `category` (kept only for
    // articles present in the target, so the map stays small).
    let mut mapped: HashMap<u32, Vec<String>> = HashMap::new();
    for wiki in &wikis {
        let path = format!("{}/{}/article_category.parquet", data, wiki);
        let p = PlRefPath::try_from_path(Path::new(&path))?;
        let df = LazyFrame::scan_parquet(p, Default::default())?
            .filter(col("category_qid").eq(lit(category)))
            .select([col("article_qid")])
            .collect()?;
        for a in df.column("article_qid")?.u32()?.into_iter().flatten() {
            if target_qids.contains(&a) {
                mapped.entry(a).or_default().push(wiki.clone());
            }
        }
    }

    // (wiki_count, mapped_by_target, qid, wikis) rows.
    let lang = target.strip_suffix("wiki").unwrap_or(&target);
    let mut rows: Vec<(usize, bool, u32, Vec<String>)> = Vec::new();
    for (qid, mut wlist) in mapped {
        wlist.sort_unstable();
        wlist.dedup();
        let mapped_by_target = wlist.iter().any(|w| w == &target);
        if only_foreign && mapped_by_target {
            continue;
        }
        rows.push((wlist.len(), mapped_by_target, qid, wlist));
    }
    // Most-agreed mappings first; ties by qid for stable output.
    rows.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(a.2.cmp(&b.2)));

    println!(
        "# Articles in {} mapped under category Q{} by some wiki (union over {} wikis{})",
        target,
        category,
        wikis.len(),
        if only_foreign { ", foreign-only" } else { "" }
    );
    println!("{:>5}  {:>6}  {:>4}  {:>10}  {:<40}  wikis", "rank", "#wikis", "self", "qid", "title");
    for (i, (count, mapped_by_target, qid, wlist)) in rows.iter().enumerate() {
        let t = title.get(qid).map(String::as_str).unwrap_or("?");
        let url = format!("https://{}.wikipedia.org/wiki/{}", lang, t);
        let sample: Vec<&str> = wlist.iter().take(6).map(String::as_str).collect();
        let more = if wlist.len() > 6 { format!(" +{}", wlist.len() - 6) } else { String::new() };
        println!(
            "{:>5}  {:>6}  {:>4}  {:>10}  {:<40}  {}{}",
            i + 1,
            count,
            if *mapped_by_target { "y" } else { "n" },
            format!("Q{}", qid),
            format!("{}  {}", t, url),
            sample.join(","),
            more,
        );
    }
    eprintln!("\n{} articles ({} present in {})", rows.len(), title.len(), target);
    Ok(())
}
