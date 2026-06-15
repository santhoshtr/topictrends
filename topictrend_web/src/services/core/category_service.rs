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
        let engine = EngineService::get_or_build_pageview_engine(state, wiki).await?;

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
        let engine = EngineService::get_or_build_pageview_engine(state, wiki).await?;

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
        min_agreement: u16,
    ) -> Result<Vec<u32>, CoreServiceError> {
        let engine = EngineService::get_or_build_pageview_engine(state, wiki).await?;

        let article_qids = {
            let engine_lock = engine.read().map_err(|e| {
                CoreServiceError::InternalError(format!("Failed to acquire read lock: {}", e))
            })?;

            engine_lock
                .get_wikigraph()
                .get_articles_in_category_filtered(category_qid, 0, min_agreement)
                .map_err(|e| {
                    CoreServiceError::EngineError(format!(
                        "Failed to get articles in category: {}",
                        e
                    ))
                })?
        };

        Ok(article_qids)
    }

    /// Drop categories whose direct (canonical) membership exceeds
    /// `max_fraction` of the wiki's articles. Broad hypernyms (e.g.
    /// "Geography") saturate the article union with globally-popular members
    /// unrelated to the searched topic; this removes them from the topic path
    /// while leaving direct category lookups untouched. Categories absent from
    /// the graph contribute nothing to the union and are kept as no-ops.
    pub async fn filter_saturated_categories(
        state: Arc<AppState>,
        wiki: &str,
        category_qids: Vec<u32>,
        max_fraction: f64,
    ) -> Result<Vec<u32>, CoreServiceError> {
        let engine = EngineService::get_or_build_pageview_engine(state, wiki).await?;

        let kept = {
            let engine_lock = engine.read().map_err(|e| {
                CoreServiceError::InternalError(format!("Failed to acquire read lock: {}", e))
            })?;
            let graph = engine_lock.get_wikigraph();
            let total = graph.article_count() as f64;
            let max_members = (total * max_fraction).ceil() as u64;

            category_qids
                .into_iter()
                .filter(|qid| {
                    graph
                        .category_member_count(*qid)
                        .is_none_or(|members| members <= max_members)
                })
                .collect()
        };

        Ok(kept)
    }

    pub async fn validate_category_exists(
        state: Arc<AppState>,
        wiki: &str,
        category_qid: u32,
    ) -> Result<bool, CoreServiceError> {
        let engine = EngineService::get_or_build_pageview_engine(state, wiki).await?;

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
