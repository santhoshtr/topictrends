//! Semantic category search, run entirely in-process.
//!
//! Query and title encoding run through `fastembed` (ONNX `all-MiniLM-L12-v2`,
//! 384-d) and the vector store is the official `zvec` Rust SDK over the on-disk
//! collections at `<ZVEC_DIR>/<wiki>-categories`.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, Once, OnceLock};

use anyhow::{Context, Result};
use fastembed::{EmbeddingModel, TextEmbedding, TextInitOptions};
use polars::prelude::*;
use zvec::{
    Collection, CollectionOptions, CollectionSchema, DataType, Doc, FieldSchema, IndexParams,
    MetricType, SearchQuery,
};

pub use crate::models::SearchResult;
mod models;

const EMBEDDING_DIM: u32 = 384;
const INDEX_BATCH_SIZE: usize = 100;

fn zvec_dir() -> String {
    std::env::var("ZVEC_DIR").unwrap_or_else(|_| "data/embedding_store/zvec".to_string())
}

fn data_dir() -> String {
    std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string())
}

fn collection_path(wiki: &str) -> String {
    format!("{}/{}-categories", zvec_dir(), wiki)
}

/// Initialize the zvec library exactly once for the process lifetime.
fn ensure_zvec_init() {
    static INIT: Once = Once::new();
    INIT.call_once(|| zvec::initialize(None).expect("zvec initialize"));
}

/// Encode text with the shared embedding model. `fastembed::embed` needs
/// `&mut self`, so a single model is guarded by a mutex and lazily loaded on
/// first use (the model download/load is the one-time cost).
fn embed(texts: Vec<&str>) -> Result<Vec<Vec<f32>>> {
    static EMBEDDER: Mutex<Option<TextEmbedding>> = Mutex::new(None);
    let mut guard = EMBEDDER.lock().expect("embedder mutex poisoned");
    if guard.is_none() {
        let model = TextEmbedding::try_new(TextInitOptions::new(EmbeddingModel::AllMiniLML12V2))
            .context("loading fastembed all-MiniLM-L12-v2 model")?;
        *guard = Some(model);
    }
    guard
        .as_mut()
        .unwrap()
        .embed(texts, None)
        .context("fastembed inference failed")
}

/// Open (and cache) a wiki's collection read-only + mmap. Shared as `Arc` so
/// the cache lock is released before the query runs; zvec `Collection` is
/// `Send + Sync` and read queries are concurrent-safe.
fn get_collection(wiki: &str) -> Result<std::sync::Arc<Collection>> {
    ensure_zvec_init();
    static COLLECTIONS: OnceLock<Mutex<HashMap<String, std::sync::Arc<Collection>>>> =
        OnceLock::new();
    let cache = COLLECTIONS.get_or_init(|| Mutex::new(HashMap::new()));

    let mut guard = cache.lock().expect("collection cache mutex poisoned");
    if let Some(collection) = guard.get(wiki) {
        return Ok(std::sync::Arc::clone(collection));
    }

    let path = collection_path(wiki);
    let mut options = CollectionOptions::new().context("CollectionOptions::new")?;
    options.set_read_only(true).context("set_read_only")?;
    options.set_enable_mmap(true).context("set_enable_mmap")?;
    let collection = std::sync::Arc::new(
        Collection::open(&path, Some(&options))
            .with_context(|| format!("opening zvec collection at {path}"))?,
    );
    guard.insert(wiki.to_string(), std::sync::Arc::clone(&collection));
    Ok(collection)
}

/// Search a wiki's category collection for the `limit` nearest neighbours of
/// `query`, keeping only results whose cosine similarity is `>= match_threshold`.
pub async fn search(
    query: String,
    wiki: String,
    limit: u64,
    match_threshold: f32,
) -> Result<Vec<SearchResult>> {
    tokio::task::spawn_blocking(move || search_blocking(&query, &wiki, limit, match_threshold))
        .await
        .context("search task panicked")?
}

fn search_blocking(
    query: &str,
    wiki: &str,
    limit: u64,
    match_threshold: f32,
) -> Result<Vec<SearchResult>> {
    let query_vector = embed(vec![query])?
        .into_iter()
        .next()
        .context("embedding model returned no vector")?;

    let collection = get_collection(wiki)?;
    let search_query = SearchQuery::builder()
        .field_name("embedding")
        .vector(&query_vector)
        .topk(limit as i32)
        .output_fields(&["qid", "page_title"])
        .build()
        .context("building SearchQuery")?;

    let hits = collection.query(&search_query).context("zvec query")?;

    let mut results = Vec::with_capacity(hits.len());
    for hit in &hits {
        // zvec reports cosine DISTANCE for the COSINE metric; similarity = 1 - distance.
        let similarity = 1.0 - hit.get_score();
        if similarity < match_threshold {
            continue;
        }
        results.push(SearchResult::new(
            similarity,
            hit.get_u32("qid")?.unwrap_or(0),
            hit.get_string("page_title")?.unwrap_or_default(),
        ));
    }
    Ok(results)
}

/// Build a wiki's category collection from `<DATA_DIR>/<wiki>/categories.parquet`,
/// encoding page titles with fastembed and indexing them into zvec (HNSW cosine).
pub async fn injest(wiki: String) -> Result<()> {
    tokio::task::spawn_blocking(move || injest_blocking(&wiki))
        .await
        .context("injest task panicked")?
}

fn injest_blocking(wiki: &str) -> Result<()> {
    ensure_zvec_init();

    let parquet = format!("{}/{}/categories.parquet", data_dir(), wiki);
    let frame = LazyFrame::scan_parquet(
        PlRefPath::try_from_path(Path::new(&parquet))
            .with_context(|| format!("invalid path {parquet}"))?,
        Default::default(),
    )
    .with_context(|| format!("scanning {parquet}"))?
        .select([col("qid"), col("page_title")])
        .collect()
        .with_context(|| format!("reading {parquet}"))?;

    let qids = frame.column("qid")?.u32()?;
    let titles = frame.column("page_title")?.str()?;
    println!("Found {} records to process for {wiki}", frame.height());

    let path = collection_path(wiki);
    let schema = CollectionSchema::builder(&format!("{wiki}-categories"))
        .add_field(FieldSchema::new("qid", DataType::Uint32, false, 0)?)
        .add_field(FieldSchema::new("page_title", DataType::String, false, 0)?)
        .add_vector_field(
            "embedding",
            DataType::VectorFp32,
            EMBEDDING_DIM,
            IndexParams::hnsw(MetricType::Cosine, 16, 200)?,
        )
        .build()
        .context("building collection schema")?;
    let collection =
        Collection::create_and_open(&path, &schema, None).context("creating collection")?;

    let mut batch: Vec<(u32, &str)> = Vec::with_capacity(INDEX_BATCH_SIZE);
    let mut processed = 0usize;
    for (qid, title) in qids.iter().zip(titles.iter()) {
        let (Some(qid), Some(title)) = (qid, title) else {
            continue;
        };
        batch.push((qid, title));
        if batch.len() >= INDEX_BATCH_SIZE {
            processed += insert_batch(&collection, &batch)?;
            batch.clear();
        }
    }
    if !batch.is_empty() {
        processed += insert_batch(&collection, &batch)?;
    }

    collection.optimize().context("optimizing index")?;
    collection.close().context("closing collection")?;
    println!("Indexed {processed} records for {wiki}");
    Ok(())
}

fn insert_batch(collection: &Collection, batch: &[(u32, &str)]) -> Result<usize> {
    let titles: Vec<&str> = batch.iter().map(|(_, title)| *title).collect();
    let embeddings = embed(titles)?;

    let mut docs = Vec::with_capacity(batch.len());
    for ((qid, title), embedding) in batch.iter().zip(embeddings.iter()) {
        let mut doc = Doc::new()?;
        doc.set_pk(&qid.to_string());
        doc.add_u32("qid", *qid)?;
        doc.add_string("page_title", title)?;
        doc.add_vector_f32("embedding", embedding)?;
        docs.push(doc);
    }
    let refs: Vec<&Doc> = docs.iter().collect();
    collection.insert(&refs).context("inserting batch")?;
    Ok(batch.len())
}
