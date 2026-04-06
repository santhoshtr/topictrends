use std::sync::Arc;

use rmcp::{ErrorData, tool};
use rmcp::handler::server::wrapper::Parameters;

use crate::mcp::TopicTrendMcpServer;
use crate::mcp::tools::{
    ArticleTrendInput, CategoryTrendInput, TopNInput, TopicTrendInput,
    parse_date_opt, core_err,
};
use crate::models::{
    ArticleEditTrendResponse, CategoryEditRankResponse, CategoryEditTrendResponse,
    DailyEdits, PageEditTopArticle, PageEditTopArticlesResponse,
    TopArticleByEdits, TopArticleCategory, TopArticleEdits, TopCategoryByEdits,
};
use crate::services::PageEditsService;
use crate::services::composite::pageedits_service::{ArticleEditRank, CategoryEditRank};

impl TopicTrendMcpServer {
    /// Get daily Wikipedia page edit counts for a category over a date range.
    ///
    /// Returns a time series of daily edit counts plus the most-edited articles in the category.
    #[tool(
        name = "topictrends_get_category_pageedit_trend",
        description = "Daily Wikipedia page edit counts for a category, with most-edited articles.",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = true)
    )]
    pub async fn get_category_pageedit_trend(
        &self,
        Parameters(p): Parameters<CategoryTrendInput>,
    ) -> Result<rmcp::handler::server::wrapper::Json<CategoryEditTrendResponse>, ErrorData> {
        let start = parse_date_opt(p.start_date)?;
        let end = parse_date_opt(p.end_date)?;
        let r = PageEditsService::get_category_edit_trend(
            Arc::clone(&self.state), &p.wiki, &p.category, p.category_qid, p.depth, start, end,
        ).await.map_err(core_err)?;

        Ok(rmcp::handler::server::wrapper::Json(CategoryEditTrendResponse {
            qid: r.qid,
            title: r.title,
            edits: r.edits.into_iter().map(|(date, edits)| DailyEdits { date, edits }).collect(),
            top_articles: build_top_article_edits(r.top_articles),
        }))
    }

    /// Get daily Wikipedia page edit counts for a single article over a date range.
    #[tool(
        name = "topictrends_get_article_pageedit_trend",
        description = "Daily Wikipedia page edit counts for a single article.",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = true)
    )]
    pub async fn get_article_pageedit_trend(
        &self,
        Parameters(p): Parameters<ArticleTrendInput>,
    ) -> Result<rmcp::handler::server::wrapper::Json<ArticleEditTrendResponse>, ErrorData> {
        let start = parse_date_opt(p.start_date)?;
        let end = parse_date_opt(p.end_date)?;
        let r = PageEditsService::get_article_edit_trend(
            Arc::clone(&self.state), &p.wiki, &p.article, p.article_qid, start, end,
        ).await.map_err(core_err)?;

        Ok(rmcp::handler::server::wrapper::Json(ArticleEditTrendResponse {
            qid: r.qid,
            title: r.title,
            edits: r.edits.into_iter().map(|(date, edits)| DailyEdits { date, edits }).collect(),
        }))
    }

    /// Get daily Wikipedia page edit counts for a semantic topic.
    ///
    /// Performs embedding-based search against the English Wikipedia category taxonomy,
    /// then aggregates daily edit counts across all matched categories.
    #[tool(
        name = "topictrends_get_topic_pageedit_trend",
        description = "Daily Wikipedia page edit counts for a semantic topic query.",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = true)
    )]
    pub async fn get_topic_pageedit_trend(
        &self,
        Parameters(p): Parameters<TopicTrendInput>,
    ) -> Result<rmcp::handler::server::wrapper::Json<CategoryEditTrendResponse>, ErrorData> {
        let start = parse_date_opt(p.start_date)?;
        let end = parse_date_opt(p.end_date)?;
        let r = PageEditsService::get_topic_edit_trend(
            Arc::clone(&self.state), &p.wiki, &p.topic, p.depth, start, end,
        ).await.map_err(core_err)?;

        Ok(rmcp::handler::server::wrapper::Json(CategoryEditTrendResponse {
            qid: r.qid,
            title: r.title,
            edits: r.edits.into_iter().map(|(date, edits)| DailyEdits { date, edits }).collect(),
            top_articles: build_top_article_edits(r.top_articles),
        }))
    }

    /// Get the top Wikipedia categories ranked by total page edits in a period.
    #[tool(
        name = "topictrends_get_pageedits_top_categories",
        description = "Top Wikipedia categories ranked by total page edit count in the given period.",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = true)
    )]
    pub async fn get_pageedits_top_categories(
        &self,
        Parameters(p): Parameters<TopNInput>,
    ) -> Result<rmcp::handler::server::wrapper::Json<CategoryEditRankResponse>, ErrorData> {
        let start = parse_date_opt(p.start_date)?;
        let end = parse_date_opt(p.end_date)?;
        let cats = PageEditsService::get_top_categories(
            Arc::clone(&self.state), &p.wiki, start, end, p.top_n,
        ).await.map_err(core_err)?;

        Ok(rmcp::handler::server::wrapper::Json(CategoryEditRankResponse {
            categories: cats.into_iter().map(build_category_edit_rank).collect(),
        }))
    }

    /// Get the top Wikipedia articles globally ranked by total page edits in a period.
    #[tool(
        name = "topictrends_get_pageedits_top_articles",
        description = "Top Wikipedia articles globally ranked by total page edit count in the given period.",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = true)
    )]
    pub async fn get_pageedits_top_articles(
        &self,
        Parameters(p): Parameters<TopNInput>,
    ) -> Result<rmcp::handler::server::wrapper::Json<PageEditTopArticlesResponse>, ErrorData> {
        let start = parse_date_opt(p.start_date)?;
        let end = parse_date_opt(p.end_date)?;
        let arts = PageEditsService::get_top_articles_global(
            Arc::clone(&self.state), &p.wiki, start, end, p.top_n,
        ).await.map_err(core_err)?;

        Ok(rmcp::handler::server::wrapper::Json(PageEditTopArticlesResponse {
            articles: arts.into_iter().map(|art| PageEditTopArticle {
                qid: art.qid,
                title: art.title,
                edits: art.edits,
                categories: art.categories.into_iter()
                    .map(|c| TopArticleCategory { qid: c.qid, title: c.title })
                    .collect(),
            }).collect(),
        }))
    }
}

fn build_top_article_edits(arts: Vec<ArticleEditRank>) -> Vec<TopArticleEdits> {
    arts.into_iter().map(|art| TopArticleEdits {
        qid: art.qid,
        title: art.title,
        edits: art.edits,
        source_categories: art.source_categories.into_iter()
            .map(|(qid, title)| TopArticleCategory { qid, title })
            .collect(),
    }).collect()
}

fn build_category_edit_rank(cat: CategoryEditRank) -> TopCategoryByEdits {
    TopCategoryByEdits {
        qid: cat.qid,
        title: cat.title,
        edits: cat.edits,
        top_articles: cat.top_articles.into_iter().map(|art| TopArticleByEdits {
            qid: art.qid,
            title: art.title,
            edits: art.edits,
        }).collect(),
    }
}
