//! Parquet-backed title↔QID resolution — the web layer's only title source.
//!
//! Primary: the wiki's own `articles.parquet` / `categories.parquet`, loaded
//! lazily per wiki into a bounded FIFO cache (enwiki costs ~1GB across both
//! directions, so the cap matters; `TOPICTREND_TITLE_WIKIS`, default 4).
//! Fallback for category QIDs with no local page (the canonical topology
//! projects categories in from other editions): the global
//! `data/canonical/<date>/category_labels.parquet` table built by
//! `make canonical` — English-first labels for every category QID, loaded
//! once on first miss. Articles never need the fallback: the canonical
//! projection only surfaces articles that exist locally.
//!
//! Titles are exactly as fresh as the topology snapshot — consistent with
//! the graph, which only knows snapshot articles anyway.

use polars::prelude::*;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

fn data_dir() -> String {
    std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string())
}

const TITLE_CACHE_ENV: &str = "TOPICTREND_TITLE_WIKIS";
const DEFAULT_TITLE_CACHE: usize = 4;

pub fn title_cache_capacity() -> usize {
    std::env::var(TITLE_CACHE_ENV)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_TITLE_CACHE)
}

/// page_title convention: underscores, not spaces.
pub fn normalize_title(title: &str) -> String {
    title.trim().replace(' ', "_")
}

/// Both directions for one namespace, sharing the title allocations.
struct NamespaceMaps {
    title_by_qid: HashMap<u32, Arc<str>>,
    qid_by_title: HashMap<Arc<str>, u32>,
}

impl NamespaceMaps {
    fn load(path: &str) -> Result<Self, String> {
        let p = PlRefPath::try_from_path(Path::new(path)).map_err(|e| e.to_string())?;
        let df = LazyFrame::scan_parquet(p, Default::default())
            .and_then(|lf| lf.collect())
            .map_err(|e| format!("{}: {}", path, e))?;
        let qids = df.column("qid").and_then(|c| c.u32()).map_err(|e| e.to_string())?;
        let titles = df.column("page_title").and_then(|c| c.str()).map_err(|e| e.to_string())?;

        let mut title_by_qid = HashMap::with_capacity(df.height());
        let mut qid_by_title = HashMap::with_capacity(df.height());
        for (q, t) in qids.iter().zip(titles.iter()) {
            if let (Some(q), Some(t)) = (q, t) {
                let t: Arc<str> = Arc::from(t);
                title_by_qid.insert(q, t.clone());
                qid_by_title.insert(t, q);
            }
        }
        Ok(Self { title_by_qid, qid_by_title })
    }
}

/// One wiki's article + category title maps.
pub struct WikiTitleStore {
    articles: NamespaceMaps,
    categories: NamespaceMaps,
}

impl WikiTitleStore {
    /// Blocking (parquet I/O + map build) — call from `spawn_blocking`.
    pub fn load(wiki: &str) -> Result<Self, String> {
        let data = data_dir();
        Ok(Self {
            articles: NamespaceMaps::load(&format!("{}/{}/articles.parquet", data, wiki))?,
            categories: NamespaceMaps::load(&format!("{}/{}/categories.parquet", data, wiki))?,
        })
    }

    /// QID → title, articles first, then categories.
    pub fn title_of(&self, qid: u32) -> Option<&str> {
        self.articles
            .title_by_qid
            .get(&qid)
            .or_else(|| self.categories.title_by_qid.get(&qid))
            .map(|t| t.as_ref())
    }

    /// Title → QID in MediaWiki namespace terms (0 = article, 14 = category).
    pub fn qid_of(&self, title: &str, namespace: i8) -> Option<u32> {
        let maps = if namespace == 14 { &self.categories } else { &self.articles };
        maps.qid_by_title.get(title).copied()
    }
}

/// Global English-first category-label fallback, qid-sorted for binary search.
pub struct CategoryLabelTable {
    entries: Vec<(u32, Box<str>)>,
}

impl CategoryLabelTable {
    /// Loads the latest `data/canonical/<date>/category_labels.parquet`.
    /// Blocking — call from `spawn_blocking`. `None` when no snapshot exists
    /// (fallback disabled; non-local categories render as bare QIDs).
    pub fn load_latest() -> Option<Self> {
        let canonical_dir = format!("{}/canonical", data_dir());
        let mut dates: Vec<String> = std::fs::read_dir(&canonical_dir)
            .ok()?
            .filter_map(|e| {
                let e = e.ok()?;
                let name = e.file_name().into_string().ok()?;
                e.path().join("category_labels.parquet").is_file().then_some(name)
            })
            .collect();
        dates.sort_unstable();
        let latest = dates.pop()?;
        let path = format!("{}/{}/category_labels.parquet", canonical_dir, latest);

        let p = PlRefPath::try_from_path(Path::new(&path)).ok()?;
        let df = LazyFrame::scan_parquet(p, Default::default()).and_then(|lf| lf.collect()).ok()?;
        let qids = df.column("qid").and_then(|c| c.u32()).ok()?;
        let labels = df.column("label").and_then(|c| c.str()).ok()?;
        let mut entries: Vec<(u32, Box<str>)> = qids
            .iter()
            .zip(labels.iter())
            .filter_map(|(q, l)| Some((q?, Box::from(l?))))
            .collect();
        entries.sort_unstable_by_key(|(q, _)| *q); // written sorted; cheap insurance
        tracing::info!("category label table loaded: {} entries from {}", entries.len(), path);
        Some(Self { entries })
    }

    pub fn get(&self, qid: u32) -> Option<&str> {
        self.entries
            .binary_search_by_key(&qid, |(q, _)| *q)
            .ok()
            .map(|i| self.entries[i].1.as_ref())
    }
}
