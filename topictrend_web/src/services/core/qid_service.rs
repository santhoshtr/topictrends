//! Title↔QID resolution over the topology parquets (no database).
//!
//! Primary source: the wiki's own title maps (`title_store::WikiTitleStore`,
//! bounded per-wiki cache). Fallback for category QIDs with no local page:
//! the global category-label table from the canonical snapshot. QIDs that
//! resolve nowhere are simply absent from the returned maps — callers already
//! degrade to `"Q{qid}"`.

use super::CoreServiceError;
use super::title_store::{CategoryLabelTable, WikiTitleStore, normalize_title};
use crate::models::AppState;
use std::{collections::HashMap, sync::Arc};

pub struct QidService;

impl QidService {
    /// Get (and lazily load/cache) a wiki's title maps.
    async fn store(state: &Arc<AppState>, wiki: &str) -> Result<Arc<WikiTitleStore>, CoreServiceError> {
        if let Some(store) = state
            .title_stores
            .read()
            .map_err(|_| CoreServiceError::InternalError("title_stores lock poisoned".into()))?
            .get(&wiki.to_string())
        {
            return Ok(store);
        }

        let wiki_owned = wiki.to_string();
        let loaded = tokio::task::spawn_blocking(move || WikiTitleStore::load(&wiki_owned))
            .await
            .map_err(|e| CoreServiceError::InternalError(format!("title store load join: {}", e)))?
            .map_err(CoreServiceError::InternalError)?;

        let store = Arc::new(loaded);
        state
            .title_stores
            .write()
            .map_err(|_| CoreServiceError::InternalError("title_stores lock poisoned".into()))?
            .insert(wiki.to_string(), store.clone());
        Ok(store)
    }

    /// The global category-label fallback, loaded once. `None` when no
    /// canonical snapshot with labels exists.
    async fn label_table(state: &Arc<AppState>) -> Option<Arc<CategoryLabelTable>> {
        if let Some(slot) = state.category_labels.get() {
            return slot.clone();
        }
        let loaded = tokio::task::spawn_blocking(CategoryLabelTable::load_latest)
            .await
            .ok()
            .flatten()
            .map(Arc::new);
        if loaded.is_none() {
            tracing::warn!("no canonical category_labels.parquet found; non-local category labels disabled");
        }
        // First writer wins; on a race, read back the stored slot.
        let _ = state.category_labels.set(loaded.clone());
        state.category_labels.get().cloned().unwrap_or(loaded)
    }

    pub async fn get_qid_by_title(
        state: Arc<AppState>,
        wiki: &str,
        title: &str,
        namespace: i8,
    ) -> Result<u32, CoreServiceError> {
        let store = Self::store(&state, wiki).await?;
        store
            .qid_of(&normalize_title(title), namespace)
            .ok_or(CoreServiceError::NotFound)
    }

    pub async fn get_title_by_qid(
        state: Arc<AppState>,
        wiki: &str,
        qid: u32,
    ) -> Result<String, CoreServiceError> {
        let titles = Self::get_titles_by_qids(state, wiki, &vec![qid]).await?;
        titles.get(&qid).cloned().ok_or(CoreServiceError::NotFound)
    }

    pub async fn get_titles_by_qids(
        state: Arc<AppState>,
        wiki: &str,
        qids: &Vec<u32>,
    ) -> Result<HashMap<u32, String>, CoreServiceError> {
        if qids.is_empty() {
            return Ok(HashMap::new());
        }
        let store = Self::store(&state, wiki).await?;

        let mut result = HashMap::with_capacity(qids.len());
        let mut missing: Vec<u32> = Vec::new();
        for &qid in qids {
            match store.title_of(qid) {
                Some(title) => {
                    result.insert(qid, title.to_string());
                }
                None => missing.push(qid),
            }
        }

        if !missing.is_empty()
            && let Some(labels) = Self::label_table(&state).await
        {
            for qid in missing {
                if let Some(label) = labels.get(qid) {
                    result.insert(qid, label.to_string());
                }
            }
        }

        Ok(result)
    }

    pub async fn get_qids_by_titles(
        state: Arc<AppState>,
        wiki: &str,
        titles: Vec<String>,
        namespace: i8,
    ) -> Result<HashMap<String, u32>, CoreServiceError> {
        if titles.is_empty() {
            return Ok(HashMap::new());
        }
        let store = Self::store(&state, wiki).await?;

        let mut result = HashMap::with_capacity(titles.len());
        for title in titles {
            if let Some(qid) = store.qid_of(&normalize_title(&title), namespace) {
                result.insert(title, qid);
            }
        }
        Ok(result)
    }
}
