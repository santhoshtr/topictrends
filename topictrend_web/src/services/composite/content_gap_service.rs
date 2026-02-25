use crate::models::{AppState, ContentGapResult, ContentGapWikiResult};
use crate::services::core::{CoreServiceError, EngineService};
use std::sync::Arc;

pub struct ContentGapService;

impl ContentGapService {
    pub async fn get_content_gap(
        state: Arc<AppState>,
        category_qid: u32,
        category_label: &str,
        wikis: Vec<String>,
        depth: u32,
    ) -> Result<ContentGapResult, CoreServiceError> {
        let mut results: Vec<ContentGapWikiResult> = Vec::new();

        for wiki in &wikis {
            let graph = EngineService::get_or_build_graph_engine(Arc::clone(&state), wiki).await?;
            let article_count = {
                let graph_lock = graph.read().map_err(|_| {
                    CoreServiceError::InternalError("Failed to acquire graph lock".to_string())
                })?;
                graph_lock
                    .get_articles_in_category(category_qid, depth)
                    .map_err(|err| CoreServiceError::EngineError(err))?
                    .len()
            };

            results.push(ContentGapWikiResult {
                wiki: wiki.clone(),
                article_count,
            });
        }

        Ok(ContentGapResult {
            category: category_label.to_string(),
            category_qid,
            depth,
            wikis: results,
        })
    }
}
