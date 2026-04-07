use std::sync::Arc;

use rmcp::{ErrorData, tool};
use rmcp::handler::server::wrapper::Parameters;

use crate::mcp::TopicTrendMcpServer;
use crate::mcp::tools::CategorySearchInput;
use crate::models::{CategorySearchItemResponse, CategorySearchResponse};
use crate::services::core::QidService;

impl TopicTrendMcpServer {
    /// Search Wikipedia categories using embedding-based semantic similarity.
    ///
    /// Returns matching categories above the similarity threshold, each with its Wikidata QID,
    /// English title, target-wiki title, and match score. Useful for discovering category QIDs
    /// before querying trends.
    #[tool(
        name = "topictrends_search_categories",
        description = "Semantic search for Wikipedia categories by embedding similarity. Returns QIDs and titles for matched categories.",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = true)
    )]
    pub async fn search_categories(
        &self,
        Parameters(p): Parameters<CategorySearchInput>,
    ) -> Result<rmcp::handler::server::wrapper::Json<CategorySearchResponse>, ErrorData> {
        let limit = p.limit.unwrap_or(1000);
        let match_threshold = p.match_threshold.unwrap_or(0.6);

        let results = topictrend_taxonomy::search(
            p.query, "enwiki".to_string(), limit, match_threshold,
        ).await.map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        let mut categories: Vec<CategorySearchItemResponse> = results.into_iter().map(|r| {
            CategorySearchItemResponse {
                category_qid: r.qid,
                category_title_en: r.page_title,
                category_title: String::new(),
                match_score: r.score,
            }
        }).collect();

        if p.wiki != "enwiki" {
            let qids: Vec<u32> = categories.iter().map(|c| c.category_qid).collect();
            let titles = QidService::get_titles_by_qids(Arc::clone(&self.state), &p.wiki, &qids)
                .await.unwrap_or_default();
            categories.retain_mut(|cat| {
                if let Some(title) = titles.get(&cat.category_qid) {
                    cat.category_title = title.clone();
                    true
                } else {
                    false
                }
            });
        } else {
            for cat in &mut categories {
                cat.category_title = cat.category_title_en.clone();
            }
        }

        Ok(rmcp::handler::server::wrapper::Json(CategorySearchResponse { categories }))
    }
}
