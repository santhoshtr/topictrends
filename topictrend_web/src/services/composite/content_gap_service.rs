use crate::models::{AppState, ContentGapResult, ContentGapWikiResult};
use crate::services::core::{CoreServiceError, EngineService};
use crate::services::composite::taxonomy_search_category_qids;
use std::sync::Arc;

pub struct ContentGapService;

impl ContentGapService {
    pub async fn get_content_gap(
        state: Arc<AppState>,
        category_qid: u32,
        category_label: &str,
        wikis: Vec<String>,
    ) -> Result<ContentGapResult, CoreServiceError> {
        let mut results: Vec<ContentGapWikiResult> = Vec::new();

        for wiki in &wikis {
            let graph = EngineService::get_or_build_graph_engine(Arc::clone(&state), wiki).await?;
            let article_count = graph
                .get_articles_in_category(category_qid, 0)
                .map_err(CoreServiceError::EngineError)?
                .len();

            results.push(ContentGapWikiResult {
                wiki: wiki.clone(),
                article_count,
            });
        }

        Ok(ContentGapResult {
            category: category_label.to_string(),
            category_qid,
            wikis: results,
        })
    }

    pub async fn get_topic_content_gap(
        state: Arc<AppState>,
        topic: &str,
        wikis: Vec<String>,
    ) -> Result<ContentGapResult, CoreServiceError> {
        let category_qids = taxonomy_search_category_qids(topic).await?;

        let mut results: Vec<ContentGapWikiResult> = Vec::new();

        for wiki in &wikis {
            let graph = EngineService::get_or_build_graph_engine(Arc::clone(&state), wiki).await?;
            let mut total_article_count = 0;

            for qid in &category_qids {
                let articles = graph
                    .get_articles_in_category(*qid, 0)
                    .map_err(CoreServiceError::EngineError)?;
                total_article_count += articles.len();
            }

            results.push(ContentGapWikiResult {
                wiki: wiki.clone(),
                article_count: total_article_count,
            });
        }

        Ok(ContentGapResult {
            category: topic.to_string(),
            category_qid: 0,
            wikis: results,
        })
    }
}
