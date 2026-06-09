// Stage 2 of the coverage matrix ETL: cross-wiki `qid_overlap_coverage`.
//
// canonical_set(C) = union over all wikis of articles filed directly under C.
// qid_overlap(C, W) = |{ a in canonical_set(C) : a exists as an article in W }|.
//
// This is the pure *content* gap: how many of a category's globally-known
// articles a wiki has, independent of how (or whether) that wiki categorises
// them. Computed by a reverse scatter: hold canonical (article_qid,
// category_qid) membership once, then for each wiki walk its article set and
// tally each article's categories-anywhere.
//
// The pass enriches each per-wiki stage-1 snapshot in place, producing the final
// (category_qid, direct_coverage, qid_overlap_coverage) matrix. A category can
// have qid_overlap > 0 while direct_coverage = 0 (the category is absent in W but
// some of its canonical articles exist there), so the merge is a full outer union
// of the two key sets, zero-filling the missing side.

use parquet::file::writer::SerializedFileWriter;
use parquet::{file::properties::WriterProperties, record::RecordWriter as _};
use parquet_derive::ParquetRecordWriter;
use polars::prelude::*;
use roaring::RoaringBitmap;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, ParquetRecordWriter)]
struct CoverageRecord {
    category_qid: u32,
    direct_coverage: u32,
    qid_overlap_coverage: u32,
}

fn data_dir() -> String {
    std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string())
}

fn u32_col(df: &DataFrame, name: &str) -> Result<Vec<u32>, Box<dyn std::error::Error>> {
    Ok(df.column(name)?.u32()?.into_iter().flatten().collect())
}

fn read_parquet(path: &str) -> Result<DataFrame, Box<dyn std::error::Error>> {
    let p = PlRefPath::try_from_path(Path::new(path))?;
    Ok(LazyFrame::scan_parquet(p, Default::default())?.collect()?)
}

/// Build canonical membership across all wikis as a Vec of (article_qid,
/// category_qid) pairs, deduplicated and sorted by (article_qid, category_qid)
/// so an article's categories-anywhere form a contiguous block.
fn build_canonical(
    data: &str,
    wikis: &[String],
) -> Result<Vec<(u32, u32)>, Box<dyn std::error::Error>> {
    let mut pairs: Vec<(u32, u32)> = Vec::new();
    for wiki in wikis {
        let path = format!("{}/{}/article_category.parquet", data, wiki);
        let df = read_parquet(&path)?;
        let a = df.column("article_qid")?.u32()?;
        let c = df.column("category_qid")?.u32()?;
        for (oa, oc) in a.into_iter().zip(c) {
            if let (Some(a), Some(c)) = (oa, oc) {
                pairs.push((a, c));
            }
        }
        eprintln!("  loaded {} ({} cumulative pairs)", wiki, pairs.len());
    }
    pairs.sort_unstable();
    pairs.dedup();
    eprintln!("Canonical membership: {} distinct (article, category) pairs", pairs.len());
    Ok(pairs)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    // --date YYYY-MM-DD is required; trailing positional args override the wiki
    // list (defaults to data/wikipedia.list).
    let date = args
        .iter()
        .position(|a| a == "--date")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(|| {
            eprintln!("Usage: coverage-overlap --date YYYY-MM-DD [wiki ...]");
            std::process::exit(1);
        });
    let wikis: Vec<String> = args
        .iter()
        .skip(1)
        .filter(|a| !a.starts_with("--") && *a != &date)
        .cloned()
        .collect();

    let data = data_dir();
    let wikis = if wikis.is_empty() {
        std::fs::read_to_string(format!("{}/wikipedia.list", data))?
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect()
    } else {
        wikis
    };

    eprintln!("Building canonical membership from {} wikis...", wikis.len());
    let canonical = build_canonical(&data, &wikis)?;

    for wiki in &wikis {
        // Article set for this wiki.
        let arts = read_parquet(&format!("{}/{}/articles.parquet", data, wiki))?;
        let a_w: RoaringBitmap = u32_col(&arts, "qid")?.into_iter().collect();

        // Reverse scatter: for each article in W, tally its categories-anywhere.
        let mut overlap: HashMap<u32, u32> = HashMap::new();
        for a in &a_w {
            let lo = canonical.partition_point(|&(art, _)| art < a);
            for &(art, cat) in &canonical[lo..] {
                if art != a {
                    break;
                }
                *overlap.entry(cat).or_insert(0) += 1;
            }
        }

        // Stage-1 direct_coverage for this wiki's snapshot.
        let cov_path = format!("{}/{}/coverage/{}.parquet", data, wiki, date);
        let cov = read_parquet(&cov_path)?;
        let cats = u32_col(&cov, "category_qid")?;
        let directs = u32_col(&cov, "direct_coverage")?;
        let direct: HashMap<u32, u32> = cats.into_iter().zip(directs).collect();

        // Full outer union of the two key sets.
        let mut keys: Vec<u32> = direct.keys().chain(overlap.keys()).copied().collect();
        keys.sort_unstable();
        keys.dedup();

        let records: Vec<CoverageRecord> = keys
            .into_iter()
            .map(|category_qid| CoverageRecord {
                category_qid,
                direct_coverage: *direct.get(&category_qid).unwrap_or(&0),
                qid_overlap_coverage: *overlap.get(&category_qid).unwrap_or(&0),
            })
            .collect();

        let schema = records.as_slice().schema()?;
        let props = Arc::new(
            WriterProperties::builder()
                .set_compression(parquet::basic::Compression::SNAPPY)
                .build(),
        );
        let file = File::create(&cov_path)?;
        let mut writer = SerializedFileWriter::new(file, schema, props)?;
        let mut row_group = writer.next_row_group()?;
        records.as_slice().write_to_row_group(&mut row_group)?;
        row_group.close()?;
        writer.close()?;
        eprintln!("Enriched {} ({} categories)", cov_path, records.len());
    }

    Ok(())
}
