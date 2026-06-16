// Project the canonical cross-wiki article→category relation onto each wiki.
//
// Reads a canonical snapshot (data/canonical/{date}/article_category.parquet,
// sorted by article_qid, category_qid) and, per wiki, keeps the edges whose
// article exists in that wiki. Two outputs per wiki:
//
//   data/{wiki}/article_category_canonical.parquet
//     (article_qid: u32, category_qid: u32, wiki_count: u32) — same shape as
//     article_category.parquet plus the cross-wiki agreement count, so it can
//     feed the graph builder as a drop-in alternative relation.
//   data/{wiki}/categories_canonical.parquet
//     (qid: u32) — the wiki's category node universe under the canonical
//     relation: local category QIDs unioned with category QIDs appearing in
//     the projection. QIDs only; labels resolve at the web edge.
//
// Usage: canonical-projection --date YYYY-MM-DD [wiki ...]
//   --date         : canonical snapshot to project (must exist).
//   trailing wikis : override the wiki universe (defaults to data/wikipedia.list).

use parquet::file::properties::WriterProperties;
use parquet::file::writer::SerializedFileWriter;
use parquet::record::RecordWriter as _;
use parquet_derive::ParquetRecordWriter;
use polars::prelude::*;
use roaring::RoaringBitmap;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

const ROW_GROUP_SIZE: usize = 16_000_000;

#[derive(Debug, ParquetRecordWriter)]
struct CanonicalEdge {
    article_qid: u32,
    category_qid: u32,
    wiki_count: u32,
}

#[derive(Debug, ParquetRecordWriter)]
struct CategoryNode {
    qid: u32,
}

fn data_dir() -> String {
    std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string())
}

fn read_parquet(path: &str) -> Result<DataFrame, Box<dyn std::error::Error>> {
    let p = PlRefPath::try_from_path(Path::new(path))?;
    Ok(LazyFrame::scan_parquet(p, Default::default())?.collect()?)
}

fn u32_col(df: &DataFrame, name: &str) -> Result<Vec<u32>, Box<dyn std::error::Error>> {
    Ok(df.column(name)?.u32()?.iter().flatten().collect())
}

fn writer_props() -> Arc<WriterProperties> {
    Arc::new(
        WriterProperties::builder()
            .set_compression(parquet::basic::Compression::SNAPPY)
            .build(),
    )
}

fn write_chunked<T>(path: &str, records: &[T]) -> Result<(), Box<dyn std::error::Error>>
where
    for<'a> &'a [T]: parquet::record::RecordWriter<T>,
{
    let schema = records.schema()?;
    let mut writer = SerializedFileWriter::new(File::create(path)?, schema, writer_props())?;
    for chunk in records.chunks(ROW_GROUP_SIZE) {
        let mut rg = writer.next_row_group()?;
        chunk.write_to_row_group(&mut rg)?;
        rg.close()?;
    }
    writer.close()?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let date = args
        .iter()
        .position(|a| a == "--date")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(|| {
            eprintln!("Usage: canonical-projection --date YYYY-MM-DD [wiki ...]");
            std::process::exit(1);
        });
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

    let start = Instant::now();
    let canonical_path = format!("{}/canonical/{}/article_category.parquet", data, date);
    eprintln!("Loading canonical snapshot {}...", canonical_path);
    let df = read_parquet(&canonical_path)?;
    let arts = u32_col(&df, "article_qid")?;
    let cats = u32_col(&df, "category_qid")?;
    let counts = u32_col(&df, "wiki_count")?;
    drop(df);
    eprintln!("  {} canonical edges", arts.len());

    for wiki in &wikis {
        let articles_path = format!("{}/{}/articles.parquet", data, wiki);
        let categories_path = format!("{}/{}/categories.parquet", data, wiki);
        if !Path::new(&articles_path).exists() || !Path::new(&categories_path).exists() {
            eprintln!("  WARNING: skipping {} (missing topology parquets)", wiki);
            continue;
        }

        let mut wiki_articles = u32_col(&read_parquet(&articles_path)?, "qid")?;
        wiki_articles.sort_unstable();
        wiki_articles.dedup();

        // The canonical snapshot is sorted by article_qid, so each article's
        // edges form a contiguous block found by binary search.
        let mut edges: Vec<CanonicalEdge> = Vec::new();
        let mut projected_cats = RoaringBitmap::new();
        for &a in &wiki_articles {
            let lo = arts.partition_point(|&x| x < a);
            for i in lo..arts.len() {
                if arts[i] != a {
                    break;
                }
                edges.push(CanonicalEdge {
                    article_qid: a,
                    category_qid: cats[i],
                    wiki_count: counts[i],
                });
                projected_cats.insert(cats[i]);
            }
        }

        // Category node universe: local categories ∪ categories projected in.
        let mut universe: RoaringBitmap =
            u32_col(&read_parquet(&categories_path)?, "qid")?.into_iter().collect();
        let local = universe.len();
        universe |= &projected_cats;
        let nodes: Vec<CategoryNode> = universe.iter().map(|qid| CategoryNode { qid }).collect();

        write_chunked(
            &format!("{}/{}/article_category_canonical.parquet", data, wiki),
            &edges,
        )?;
        write_chunked(&format!("{}/{}/categories_canonical.parquet", data, wiki), &nodes)?;
        eprintln!(
            "  {}: {} edges, {} categories ({} local + {} projected-only)",
            wiki,
            edges.len(),
            nodes.len(),
            local,
            nodes.len() as u64 - local
        );
    }

    println!(
        "Projected canonical snapshot {} onto {} wikis in {:.0?}",
        date,
        wikis.len(),
        start.elapsed()
    );
    Ok(())
}
