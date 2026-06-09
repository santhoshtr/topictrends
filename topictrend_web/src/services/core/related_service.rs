use super::{CoreServiceError, EngineService};
use crate::models::AppState;
use std::sync::Arc;

pub struct RelatedService;

impl RelatedService {
    /// Articles related to `article_qid` by shared-category overlap, as
    /// `(qid, shared_category_count)` pairs ordered by overlap (highest first).
    /// Runs entirely on the shared in-memory graph — no Parquet, no DB.
    pub async fn get_related_articles(
        state: Arc<AppState>,
        wiki: &str,
        article_qid: u32,
        top: usize,
    ) -> Result<Vec<(u32, u32)>, CoreServiceError> {
        let graph = EngineService::get_or_build_graph_engine(state, wiki).await?;
        Ok(graph.related_by_categories(article_qid, top))
    }
}
