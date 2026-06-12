// Category-consensus probe for a single article: across every wiki that files
// the given article QID under at least one category, count how many wikis agree
// on each category QID and print them frequency-ranked.
//
// Categories share a Wikidata QID across editions, so "2 wikis gave the same
// category" is just two editions mapping the article under the same category_qid.
// Labels are resolved in English (enwiki's categories.parquet), falling back to
// the first edition — in wikipedia.list order — that has a label for that QID.
//
// The aim is to see whether a topic for the article can be predicted from the
// consensus of categories the communities assigned.
//
// Usage: article-categories --qid 311615 [wiki ...]
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let qid: u32 = arg(&args, "--qid")
        .and_then(|s| s.trim_start_matches('Q').parse().ok())
        .unwrap_or_else(|| {
            eprintln!("Usage: article-categories --qid <article_qid> [wiki ...]");
            std::process::exit(1);
        });

    let data = data_dir();
    // Trailing positional wikis (skipping the --qid value) override the universe.
    let mut positional: Vec<String> = Vec::new();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--qid" => i += 2,
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

    // Pass 1: for each wiki, the set of categories it files this article under.
    // We accumulate a per-category wiki count and remember which wikis touched
    // the article at all (so label fallback only scans relevant editions).
    let mut cat_count: HashMap<u32, usize> = HashMap::new();
    let mut relevant_wikis: Vec<String> = Vec::new();
    for wiki in &wikis {
        let path = format!("{}/{}/article_category.parquet", data, wiki);
        if !Path::new(&path).exists() {
            continue;
        }
        let p = PlRefPath::try_from_path(Path::new(&path))?;
        let df = LazyFrame::scan_parquet(p, Default::default())?
            .filter(col("article_qid").eq(lit(qid)))
            .select([col("category_qid")])
            .collect()?;
        let cats: HashSet<u32> = df.column("category_qid")?.u32()?.iter().flatten().collect();
        if cats.is_empty() {
            continue;
        }
        relevant_wikis.push(wiki.clone());
        for c in cats {
            *cat_count.entry(c).or_insert(0) += 1;
        }
    }

    let needed: HashSet<u32> = cat_count.keys().copied().collect();

    // Label resolution. Prefer enwiki's English labels for every category QID it
    // knows — even categories enwiki did not itself file this article under —
    // then fall back, in wikipedia.list order, to the editions that touched the
    // article. Because article_category only carries categories that have a QID,
    // every needed QID exists in at least one relevant edition, so a label is
    // always found.
    let mut read_order: Vec<String> = vec!["enwiki".to_string()];
    read_order.extend(relevant_wikis.iter().cloned());

    let mut label: HashMap<u32, (String, String)> = HashMap::new(); // qid -> (label, source wiki)
    for wiki in &read_order {
        if label.len() == needed.len() {
            break;
        }
        let path = format!("{}/{}/categories.parquet", data, wiki);
        if !Path::new(&path).exists() {
            continue;
        }
        let p = PlRefPath::try_from_path(Path::new(&path))?;
        let df = LazyFrame::scan_parquet(p, Default::default())?
            .select([col("qid"), col("page_title")])
            .collect()?;
        let qids = df.column("qid")?.u32()?;
        let titles = df.column("page_title")?.str()?;
        for (q, t) in qids.iter().zip(titles.iter()) {
            if let (Some(q), Some(t)) = (q, t)
                && needed.contains(&q)
                && !label.contains_key(&q)
            {
                label.insert(q, (t.replace('_', " "), wiki.clone()));
            }
        }
    }

    // Frequency-ranked: most-agreed category first, ties broken by QID.
    let mut rows: Vec<(usize, u32)> = cat_count.into_iter().map(|(q, c)| (c, q)).collect();
    rows.sort_unstable_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));

    println!(
        "# Categories assigned to article Q{} across {} editions ({} file it under >=1 category)",
        qid,
        wikis.len(),
        relevant_wikis.len()
    );
    println!("# count\tcategory_qid\tlabel\t(label source)");
    for (count, q) in &rows {
        let (lbl, src) = label
            .get(q)
            .map(|(l, s)| (l.as_str(), s.as_str()))
            .unwrap_or(("?", "?"));
        println!("{}\tQ{}\t{}\t({})", count, q, lbl, src);
    }
    eprintln!("\n{} distinct categories over {} editions", rows.len(), relevant_wikis.len());
    Ok(())
}
