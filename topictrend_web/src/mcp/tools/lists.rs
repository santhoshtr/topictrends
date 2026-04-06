use std::sync::Arc;
use std::collections::HashMap;

use rmcp::{ErrorData, tool};
use rmcp::handler::server::wrapper::Parameters;

use crate::mcp::TopicTrendMcpServer;
use crate::mcp::tools::{ContentGapTopicInput, ListArticlesInput, SubCategoriesInput, core_err};
use crate::models::{ArticleItem, ArticlesInCategoryResponse, ContentGapResult};
use crate::services::{ContentGapService, PageViewsService};
use crate::services::core::{CategoryService, QidService};

impl TopicTrendMcpServer {
    /// List direct subcategories of a Wikipedia category.
    ///
    /// Returns a map of Wikidata QID → category title for all immediate children.
    #[tool(
        name = "topictrends_list_subcategories",
        description = "List direct subcategories of a Wikipedia category, returning QID→title pairs.",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = true)
    )]
    pub async fn list_subcategories(
        &self,
        Parameters(p): Parameters<SubCategoriesInput>,
    ) -> Result<rmcp::handler::server::wrapper::Json<HashMap<String, String>>, ErrorData> {
        let map = PageViewsService::get_sub_categories(
            Arc::clone(&self.state), &p.wiki, &p.category, p.category_qid,
        ).await.map_err(|e| crate::mcp::tools::service_err(e))?;

        // Convert u32 keys to strings as the OpenAPI spec uses string keys
        let string_map: HashMap<String, String> = map.into_iter()
            .map(|(qid, title)| (qid.to_string(), title))
            .collect();

        Ok(rmcp::handler::server::wrapper::Json(string_map))
    }

    /// List articles that are direct members of a Wikipedia category.
    ///
    /// At least one of `category` or `category_qid` must be provided.
    #[tool(
        name = "topictrends_list_articles_in_category",
        description = "List articles that are direct members of a Wikipedia category (QID and title for each).",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = true)
    )]
    pub async fn list_articles_in_category(
        &self,
        Parameters(p): Parameters<ListArticlesInput>,
    ) -> Result<rmcp::handler::server::wrapper::Json<ArticlesInCategoryResponse>, ErrorData> {
        let category_qid = if let Some(qid) = p.category_qid {
            qid
        } else {
            let category = p.category.ok_or_else(|| {
                ErrorData::invalid_params(
                    "Either category or category_qid must be provided".to_string(),
                    None,
                )
            })?;
            QidService::get_qid_by_title(Arc::clone(&self.state), &p.wiki, &category, 14)
                .await.map_err(core_err)?
        };

        let article_qids = CategoryService::get_category_articles(
            Arc::clone(&self.state), &p.wiki, category_qid, 0,
        ).await.map_err(core_err)?;

        let titles = QidService::get_titles_by_qids(
            Arc::clone(&self.state), &p.wiki, &article_qids,
        ).await.map_err(core_err)?;

        let articles = article_qids.into_iter().map(|qid| {
            let title = titles.get(&qid).cloned().unwrap_or_else(|| format!("Q{}", qid));
            ArticleItem { qid, title }
        }).collect();

        Ok(rmcp::handler::server::wrapper::Json(ArticlesInCategoryResponse { articles }))
    }

    /// Analyse content coverage gaps across Wikipedia language editions for a semantic topic.
    ///
    /// Performs semantic search for categories matching `topic`, then returns the article count
    /// for each requested wiki. Useful for identifying which language editions have sparse coverage.
    #[tool(
        name = "topictrends_get_content_gap_topic",
        description = "Content coverage gap analysis across Wikipedia language editions for a semantic topic.",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = true)
    )]
    pub async fn get_content_gap_topic(
        &self,
        Parameters(p): Parameters<ContentGapTopicInput>,
    ) -> Result<rmcp::handler::server::wrapper::Json<ContentGapResult>, ErrorData> {
        let wikis: Vec<String> = p.wikis
            .split(',')
            .map(|w| w.trim().to_string())
            .filter(|w| !w.is_empty())
            .collect();

        let result = ContentGapService::get_topic_content_gap(
            Arc::clone(&self.state), &p.topic, wikis, p.depth,
        ).await.map_err(core_err)?;

        Ok(rmcp::handler::server::wrapper::Json(result))
    }
}
