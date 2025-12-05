use super::{CoreServiceError, EngineService};
use crate::models::AppState;
use std::sync::Arc;

pub struct CategoryService;

impl CategoryService {
    pub async fn get_child_categories(
        state: Arc<AppState>,
        wiki: &str,
        category_qid: u32,
    ) -> Result<Vec<u32>, CoreServiceError> {
        let engine = EngineService::get_or_build_engine(state, wiki).await?;

        let category_qids = {
            let engine_lock = engine.read().map_err(|e| {
                CoreServiceError::InternalError(format!("Failed to acquire read lock: {}", e))
            })?;

            engine_lock
                .get_wikigraph()
                .get_child_categories(category_qid)
                .map_err(|e| {
                    CoreServiceError::EngineError(format!("Failed to get child categories: {}", e))
                })?
        };

        Ok(category_qids)
    }

    pub async fn get_parent_categories(
        state: Arc<AppState>,
        wiki: &str,
        category_qid: u32,
    ) -> Result<Vec<u32>, CoreServiceError> {
        let engine = EngineService::get_or_build_engine(state, wiki).await?;

        let category_qids = {
            let engine_lock = engine.read().map_err(|e| {
                CoreServiceError::InternalError(format!("Failed to acquire read lock: {}", e))
            })?;

            engine_lock
                .get_wikigraph()
                .get_parent_categories(category_qid)
                .map_err(|e| {
                    CoreServiceError::EngineError(format!("Failed to get parent categories: {}", e))
                })?
        };

        Ok(category_qids)
    }

    pub async fn get_category_articles(
        state: Arc<AppState>,
        wiki: &str,
        category_qid: u32,
        depth: u32,
    ) -> Result<Vec<u32>, CoreServiceError> {
        let engine = EngineService::get_or_build_engine(state, wiki).await?;

        let article_qids = {
            let engine_lock = engine.read().map_err(|e| {
                CoreServiceError::InternalError(format!("Failed to acquire read lock: {}", e))
            })?;

            engine_lock
                .get_wikigraph()
                .get_articles_in_category(category_qid, depth)
                .map_err(|e| {
                    CoreServiceError::EngineError(format!(
                        "Failed to get articles in category: {}",
                        e
                    ))
                })?
        };

        Ok(article_qids)
    }

    pub async fn validate_category_exists(
        state: Arc<AppState>,
        wiki: &str,
        category_qid: u32,
    ) -> Result<bool, CoreServiceError> {
        let engine = EngineService::get_or_build_engine(state, wiki).await?;

        let exists = {
            let engine_lock = engine.read().map_err(|e| {
                CoreServiceError::InternalError(format!("Failed to acquire read lock: {}", e))
            })?;

            engine_lock
                .get_wikigraph()
                .cat_original_to_dense
                .get(category_qid)
                .is_some()
        };

        Ok(exists)
    }
}
