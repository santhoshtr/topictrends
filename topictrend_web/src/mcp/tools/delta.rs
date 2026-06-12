use std::sync::Arc;

use rmcp::{ErrorData, tool};
use rmcp::handler::server::wrapper::Parameters;

use crate::mcp::TopicTrendMcpServer;
use crate::mcp::tools::{ArticleDeltaInput, CategoryDeltaInput, parse_date, core_err};
use crate::models::{
    GoogleSearchArticleDeltaItemResponse, GoogleSearchArticleDeltaResponse,
    GoogleSearchCategoryDeltaItemResponse, GoogleSearchCategoryDeltaResponse,
    PageEditArticleDeltaItemResponse, PageEditArticleDeltaResponse,
    PageEditCategoryDeltaItemResponse, PageEditCategoryDeltaResponse,
    PageViewArticleDeltaItemResponse, PageViewArticleDeltaResponse,
    PageViewCategoryDeltaItemResponse, PageViewCategoryDeltaResponse,
};
use crate::services::composite::{
    GoogleSearchDeltaService, PageEditDeltaService, PageViewDeltaService,
};

impl TopicTrendMcpServer {
    /// Compare Wikipedia category pageviews between a baseline period and an impact period.
    ///
    /// Returns categories sorted by absolute change, showing which topics saw the largest
    /// shifts in reader interest between the two periods.
    #[tool(
        name = "topictrends_get_category_pageview_delta",
        description = "Compare category pageviews between a baseline and impact period to identify trending topics.",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = true)
    )]
    pub async fn get_category_pageview_delta(
        &self,
        Parameters(p): Parameters<CategoryDeltaInput>,
    ) -> Result<rmcp::handler::server::wrapper::Json<PageViewCategoryDeltaResponse>, ErrorData> {
        let baseline_start = parse_date(&p.baseline_start_date)?;
        let baseline_end   = parse_date(&p.baseline_end_date)?;
        let impact_start   = parse_date(&p.impact_start_date)?;
        let impact_end     = parse_date(&p.impact_end_date)?;
        let limit = p.limit.unwrap_or(100) as usize;

        let items = PageViewDeltaService::get_category_delta(
            Arc::clone(&self.state), &p.wiki,
            baseline_start, baseline_end, impact_start, impact_end,
            limit,
        ).await.map_err(core_err)?;

        Ok(rmcp::handler::server::wrapper::Json(PageViewCategoryDeltaResponse {
            baseline_period: format!("{} to {}", baseline_start, baseline_end),
            impact_period: format!("{} to {}", impact_start, impact_end),
            categories: items.into_iter().map(|item| PageViewCategoryDeltaItemResponse {
                category_qid: item.category_qid,
                category_title: item.category_title,
                baseline_views: item.baseline_views,
                impact_views: item.impact_views,
                delta_percentage: item.delta_percentage,
                absolute_delta: item.absolute_delta,
            }).collect(),
        }))
    }

    /// Compare Wikipedia article pageviews within a category between two periods.
    #[tool(
        name = "topictrends_get_article_pageview_delta",
        description = "Compare article pageviews within a category between a baseline and impact period.",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = true)
    )]
    pub async fn get_article_pageview_delta(
        &self,
        Parameters(p): Parameters<ArticleDeltaInput>,
    ) -> Result<rmcp::handler::server::wrapper::Json<PageViewArticleDeltaResponse>, ErrorData> {
        let baseline_start = parse_date(&p.baseline_start_date)?;
        let baseline_end   = parse_date(&p.baseline_end_date)?;
        let impact_start   = parse_date(&p.impact_start_date)?;
        let impact_end     = parse_date(&p.impact_end_date)?;
        let limit = p.limit.unwrap_or(100) as usize;

        let items = PageViewDeltaService::get_article_delta(
            Arc::clone(&self.state), &p.wiki, p.category_qid,
            baseline_start, baseline_end, impact_start, impact_end,
            limit,
        ).await.map_err(core_err)?;

        Ok(rmcp::handler::server::wrapper::Json(PageViewArticleDeltaResponse {
            category_qid: p.category_qid,
            category_title: String::new(),
            baseline_period: format!("{} to {}", baseline_start, baseline_end),
            impact_period: format!("{} to {}", impact_start, impact_end),
            articles: items.into_iter().map(|item| PageViewArticleDeltaItemResponse {
                article_qid: item.article_qid,
                article_title: item.article_title,
                baseline_views: item.baseline_views,
                impact_views: item.impact_views,
                delta_percentage: item.delta_percentage,
                absolute_delta: item.absolute_delta,
            }).collect(),
        }))
    }

    /// Compare Wikipedia category page edit counts between two periods.
    #[tool(
        name = "topictrends_get_category_pageedit_delta",
        description = "Compare category page edit counts between a baseline and impact period.",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = true)
    )]
    pub async fn get_category_pageedit_delta(
        &self,
        Parameters(p): Parameters<CategoryDeltaInput>,
    ) -> Result<rmcp::handler::server::wrapper::Json<PageEditCategoryDeltaResponse>, ErrorData> {
        let baseline_start = parse_date(&p.baseline_start_date)?;
        let baseline_end   = parse_date(&p.baseline_end_date)?;
        let impact_start   = parse_date(&p.impact_start_date)?;
        let impact_end     = parse_date(&p.impact_end_date)?;
        let limit = p.limit.unwrap_or(100) as usize;

        let items = PageEditDeltaService::get_category_delta(
            Arc::clone(&self.state), &p.wiki,
            baseline_start, baseline_end, impact_start, impact_end,
            limit,
        ).await.map_err(core_err)?;

        Ok(rmcp::handler::server::wrapper::Json(PageEditCategoryDeltaResponse {
            baseline_period: format!("{} to {}", baseline_start, baseline_end),
            impact_period: format!("{} to {}", impact_start, impact_end),
            categories: items.into_iter().map(|item| PageEditCategoryDeltaItemResponse {
                category_qid: item.category_qid,
                category_title: item.category_title,
                baseline_edits: item.baseline_edits,
                impact_edits: item.impact_edits,
                delta_percentage: item.delta_percentage,
                absolute_delta: item.absolute_delta,
            }).collect(),
        }))
    }

    /// Compare Wikipedia article page edit counts within a category between two periods.
    #[tool(
        name = "topictrends_get_article_pageedit_delta",
        description = "Compare article page edit counts within a category between a baseline and impact period.",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = true)
    )]
    pub async fn get_article_pageedit_delta(
        &self,
        Parameters(p): Parameters<ArticleDeltaInput>,
    ) -> Result<rmcp::handler::server::wrapper::Json<PageEditArticleDeltaResponse>, ErrorData> {
        let baseline_start = parse_date(&p.baseline_start_date)?;
        let baseline_end   = parse_date(&p.baseline_end_date)?;
        let impact_start   = parse_date(&p.impact_start_date)?;
        let impact_end     = parse_date(&p.impact_end_date)?;
        let limit = p.limit.unwrap_or(100) as usize;

        let items = PageEditDeltaService::get_article_delta(
            Arc::clone(&self.state), &p.wiki, p.category_qid,
            baseline_start, baseline_end, impact_start, impact_end,
            limit,
        ).await.map_err(core_err)?;

        Ok(rmcp::handler::server::wrapper::Json(PageEditArticleDeltaResponse {
            category_qid: p.category_qid,
            category_title: String::new(),
            baseline_period: format!("{} to {}", baseline_start, baseline_end),
            impact_period: format!("{} to {}", impact_start, impact_end),
            articles: items.into_iter().map(|item| PageEditArticleDeltaItemResponse {
                article_qid: item.article_qid,
                article_title: item.article_title,
                baseline_edits: item.baseline_edits,
                impact_edits: item.impact_edits,
                delta_percentage: item.delta_percentage,
                absolute_delta: item.absolute_delta,
            }).collect(),
        }))
    }

    /// Compare Wikipedia category Google Search clicks between two periods.
    #[tool(
        name = "topictrends_get_category_googlesearch_delta",
        description = "Compare category Google Search clicks and impressions between a baseline and impact period.",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = true)
    )]
    pub async fn get_category_googlesearch_delta(
        &self,
        Parameters(p): Parameters<CategoryDeltaInput>,
    ) -> Result<rmcp::handler::server::wrapper::Json<GoogleSearchCategoryDeltaResponse>, ErrorData> {
        let baseline_start = parse_date(&p.baseline_start_date)?;
        let baseline_end   = parse_date(&p.baseline_end_date)?;
        let impact_start   = parse_date(&p.impact_start_date)?;
        let impact_end     = parse_date(&p.impact_end_date)?;
        let limit = p.limit.unwrap_or(100) as usize;

        let items = GoogleSearchDeltaService::get_category_delta(
            Arc::clone(&self.state), &p.wiki,
            baseline_start, baseline_end, impact_start, impact_end,
            limit,
        ).await.map_err(core_err)?;

        Ok(rmcp::handler::server::wrapper::Json(GoogleSearchCategoryDeltaResponse {
            baseline_period: format!("{} to {}", baseline_start, baseline_end),
            impact_period: format!("{} to {}", impact_start, impact_end),
            categories: items.into_iter().map(|item| GoogleSearchCategoryDeltaItemResponse {
                category_qid: item.category_qid,
                category_title: item.category_title,
                baseline_clicks: item.baseline_clicks,
                impact_clicks: item.impact_clicks,
                baseline_impressions: item.baseline_impressions,
                impact_impressions: item.impact_impressions,
                delta_percentage: item.delta_percentage,
                absolute_delta: item.absolute_delta,
            }).collect(),
        }))
    }

    /// Compare Wikipedia article Google Search clicks within a category between two periods.
    #[tool(
        name = "topictrends_get_article_googlesearch_delta",
        description = "Compare article Google Search clicks within a category between a baseline and impact period.",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = true)
    )]
    pub async fn get_article_googlesearch_delta(
        &self,
        Parameters(p): Parameters<ArticleDeltaInput>,
    ) -> Result<rmcp::handler::server::wrapper::Json<GoogleSearchArticleDeltaResponse>, ErrorData> {
        let baseline_start = parse_date(&p.baseline_start_date)?;
        let baseline_end   = parse_date(&p.baseline_end_date)?;
        let impact_start   = parse_date(&p.impact_start_date)?;
        let impact_end     = parse_date(&p.impact_end_date)?;
        let limit = p.limit.unwrap_or(100) as usize;

        let items = GoogleSearchDeltaService::get_article_delta(
            Arc::clone(&self.state), &p.wiki, p.category_qid,
            baseline_start, baseline_end, impact_start, impact_end,
            limit,
        ).await.map_err(core_err)?;

        Ok(rmcp::handler::server::wrapper::Json(GoogleSearchArticleDeltaResponse {
            category_qid: p.category_qid,
            category_title: String::new(),
            baseline_period: format!("{} to {}", baseline_start, baseline_end),
            impact_period: format!("{} to {}", impact_start, impact_end),
            articles: items.into_iter().map(|item| GoogleSearchArticleDeltaItemResponse {
                article_qid: item.article_qid,
                article_title: item.article_title,
                baseline_clicks: item.baseline_clicks,
                impact_clicks: item.impact_clicks,
                baseline_impressions: item.baseline_impressions,
                impact_impressions: item.impact_impressions,
                delta_percentage: item.delta_percentage,
                absolute_delta: item.absolute_delta,
            }).collect(),
        }))
    }
}
