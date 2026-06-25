use std::sync::Arc;

use rmcp::{ErrorData, tool};
use rmcp::handler::server::wrapper::Parameters;

use crate::mcp::TopicTrendMcpServer;
use crate::mcp::tools::{ClusterArticlesInput, core_err};
use crate::models::ClusterArticlesResponse;
use crate::services::core::ClusterService;

impl TopicTrendMcpServer {
    /// Group a set of Wikipedia articles into category-topics.
    ///
    /// Uses the trending-topics clustering (reverse scatter + greedy coverage):
    /// each article is assigned to its single broadest-coverage topic, so
    /// near-duplicate categories collapse. Returns the clusters plus any
    /// articles that resolved to no QID or to no local category.
    #[tool(
        name = "topictrends_cluster_articles",
        description = "Group a set of Wikipedia articles into category-topics using the trending-topics clustering (reverse scatter + greedy coverage). Each article is placed in its single best topic.",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = true)
    )]
    pub async fn cluster_articles(
        &self,
        Parameters(p): Parameters<ClusterArticlesInput>,
    ) -> Result<rmcp::handler::server::wrapper::Json<ClusterArticlesResponse>, ErrorData> {
        let response = ClusterService::cluster(
            Arc::clone(&self.state),
            &p.wiki,
            p.articles,
            p.max_clusters.map(|n| n as usize),
        )
        .await
        .map_err(core_err)?;

        Ok(rmcp::handler::server::wrapper::Json(response))
    }
}
