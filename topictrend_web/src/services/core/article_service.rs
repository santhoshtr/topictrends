use super::{CoreServiceError, EngineService};
use crate::models::AppState;
use std::sync::Arc;

pub struct ArticleService;

impl ArticleService {
    pub async fn get_article_categories(
        state: Arc<AppState>,
        wiki: &str,
        article_qid: u32,
    ) -> Result<Vec<u32>, CoreServiceError> {
        let engine = EngineService::get_or_build_engine(state, wiki).await?;

        let category_qids = {
            let engine_lock = engine.read().map_err(|e| {
                CoreServiceError::InternalError(format!("Failed to acquire read lock: {}", e))
            })?;

            let wikigraph = engine_lock.get_wikigraph();

            // Convert original QID to dense ID
            let dense_id = wikigraph
                .art_original_to_dense
                .get(article_qid)
                .ok_or_else(|| CoreServiceError::NotFound)?;

            // Get category dense IDs for this article
            let category_dense_ids = wikigraph.article_cats.get(dense_id);

            // Convert dense IDs back to original QIDs
            let mut category_qids = Vec::new();
            for &dense_id in category_dense_ids {
                if let Some(&original_qid) = wikigraph.cat_dense_to_original.get(dense_id as usize)
                {
                    category_qids.push(original_qid);
                }
            }

            category_qids
        };

        Ok(category_qids)
    }

    pub async fn validate_article_exists(
        state: Arc<AppState>,
        wiki: &str,
        article_qid: u32,
    ) -> Result<bool, CoreServiceError> {
        let engine = EngineService::get_or_build_engine(state, wiki).await?;

        let exists = {
            let engine_lock = engine.read().map_err(|e| {
                CoreServiceError::InternalError(format!("Failed to acquire read lock: {}", e))
            })?;

            engine_lock
                .get_wikigraph()
                .art_original_to_dense
                .get(article_qid)
                .is_some()
        };

        Ok(exists)
    }
}
