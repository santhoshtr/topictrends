// Generate the category-exclusion denylist used by the topology ETL.
//
// Scans the latest data/canonical/<date>/category_labels.parquet (English-first
// labels) and keeps every category QID whose normalized label (underscores to
// spaces, lowercased) contains a maintenance FRAGMENT, then unions CURATED_HEAD
// — QIDs with no clean label fragment (and some that are hiddencat on enwiki and
// so never appear in the labels at all). Writes data/excluded_categories.parquet
// (qid: u32, sorted, distinct), which get-categories filters against.
//
// FRAGMENTS and CURATED_HEAD are the single source of truth for the denylist;
// regenerate occasionally (`make excluded-categories`), it changes slowly.
//
// Usage: gen-excluded-categories   (honors DATA_DIR, default "data")

use parquet::file::properties::WriterProperties;
use parquet::file::writer::SerializedFileWriter;
use parquet::record::RecordWriter as _;
use parquet_derive::ParquetRecordWriter;
use polars::prelude::*;
use std::collections::BTreeSet;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Maintenance label fragments. Matched as substrings of the normalized label.
/// `cs1` (not bare `error`) covers CS1 errors + maint without catching
/// "Terrorism in ...".
const FRAGMENTS: &[&str] = &[
    "articles",
    "pages",
    "stub",
    "disambiguation",
    "by alphabet",
    "cs1",
    "categories",
    "list of",
    "template",
    "magic link",
];

/// The original hand-curated denylist, kept verbatim. The FRAGMENTS above add
/// the unbounded maintenance/assessment/stub population; this preserves the
/// editorial entries that fragments do not reliably reproduce — whole-population
/// classifiers, by-name/by-alphabet containers, CS1-maint variants, and
/// categories that are hiddencat on enwiki (so absent from the label table) or
/// carry non-English labels. Overlap with fragment matches is deduped.
const CURATED_HEAD: &[u32] = &[
    // Whole-population / biographical classification
    5312304, 4047087, 9507857, 7473085, 6697530, 7045213, // Disambiguation
    1982926, 9700479, 4671251, 4671284, 8379354, // Stubs
    2944440, 7046360, 7046440, 5834688, 130866438,
    // Wikipedia maintenance / templates
    130251703, 3740, 6332021, 18285010, 22165254,
    // Tracking categories observed in canonical-topology trending
    27892622, 10152088, 7478359, 10051136, 4989282, 9806171, 8922197, 8922195, 27825420, 8181072,
    4387444, 6157677, // By-alphabet / by-name organizational containers
    32889963, 6547581, 9961681, 54860644, 9700775, 8691757, 7046062, 7580371, 9989549, 104844397,
    62273066, 99593300, 14768888, // CS1 errors / maint
    10862576, 21515029, 21684313, 18913694, 11679498, 72844866, 72899059, 10862669, 72837064,
    16794092, 19369547, 18707556, 8709092, 8709091, 29605259, 72836974, 100319655, 72837310,
    16794084, 21714667, 8544371, // India stubs
];

#[derive(Debug, ParquetRecordWriter)]
struct ExcludedCategory {
    qid: u32,
}

fn data_dir() -> String {
    std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string())
}

/// Latest data/canonical/<date>/ that has a category_labels.parquet.
fn latest_labels_parquet(data: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let canonical = Path::new(data).join("canonical");
    let mut dates: Vec<String> = std::fs::read_dir(&canonical)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().join("category_labels.parquet").is_file())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    dates.sort();
    let latest = dates.last().ok_or_else(|| {
        format!(
            "no <date>/category_labels.parquet under {}",
            canonical.display()
        )
    })?;
    Ok(canonical.join(latest).join("category_labels.parquet"))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data = data_dir();
    let parquet = latest_labels_parquet(&data)?;
    eprintln!("Reading labels from {}", parquet.display());

    let path = PlRefPath::try_from_path(&parquet)?;
    let df = LazyFrame::scan_parquet(path, Default::default())?.collect()?;
    let qids = df.column("qid")?.u32()?;
    let labels = df.column("label")?.str()?;

    let mut excluded: BTreeSet<u32> = BTreeSet::new();
    let mut per_fragment = vec![0usize; FRAGMENTS.len()];
    for (q, l) in qids.iter().zip(labels.iter()) {
        let (Some(qid), Some(label)) = (q, l) else {
            continue;
        };
        let normalized = label.replace('_', " ").to_lowercase();
        if let Some(i) = FRAGMENTS.iter().position(|f| normalized.contains(f)) {
            per_fragment[i] += 1;
            excluded.insert(qid);
        }
    }
    let matched = excluded.len();
    excluded.extend(CURATED_HEAD.iter().copied());

    let records: Vec<ExcludedCategory> = excluded
        .iter()
        .map(|&qid| ExcludedCategory { qid })
        .collect();

    let out_path = format!("{}/excluded_categories.parquet", data);
    let schema = records.as_slice().schema()?;
    let props = Arc::new(
        WriterProperties::builder()
            .set_compression(parquet::basic::Compression::SNAPPY)
            .build(),
    );
    let mut writer = SerializedFileWriter::new(File::create(&out_path)?, schema, props)?;
    let mut rg = writer.next_row_group()?;
    records.as_slice().write_to_row_group(&mut rg)?;
    rg.close()?;
    writer.close()?;

    for (frag, n) in FRAGMENTS.iter().zip(&per_fragment) {
        eprintln!("  {frag:>16}: {n}");
    }
    eprintln!(
        "Wrote {}: {} QIDs ({} fragment-matched + {} curated)",
        out_path,
        records.len(),
        matched,
        CURATED_HEAD.len()
    );
    Ok(())
}
