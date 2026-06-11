use super::{CoreServiceError, EngineService};
use crate::models::AppState;
use std::sync::Arc;

pub struct ArticleService;

impl ArticleService {
    /// Categories of an article as `(category_qid, wiki_count)`, ranked by
    /// cross-wiki agreement (weight 1 everywhere on local topology).
    pub async fn get_article_categories(
        state: Arc<AppState>,
        wiki: &str,
        article_qid: u32,
    ) -> Result<Vec<(u32, u16)>, CoreServiceError> {
        let engine = EngineService::get_or_build_pageview_engine(state, wiki).await?;

        let ranked = {
            let engine_lock = engine.read().map_err(|e| {
                CoreServiceError::InternalError(format!("Failed to acquire read lock: {}", e))
            })?;

            let wikigraph = engine_lock.get_wikigraph();
            wikigraph
                .art_original_to_dense
                .get(article_qid)
                .ok_or(CoreServiceError::NotFound)?;
            wikigraph.get_categories_for_article_ranked(article_qid)
        };

        Ok(ranked)
    }

    pub async fn validate_article_exists(
        state: Arc<AppState>,
        wiki: &str,
        article_qid: u32,
    ) -> Result<bool, CoreServiceError> {
        let engine = EngineService::get_or_build_pageview_engine(state, wiki).await?;

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
