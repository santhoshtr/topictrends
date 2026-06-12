use std::sync::Arc;

use rmcp::{ErrorData, tool};
use rmcp::handler::server::wrapper::Parameters;

use crate::mcp::TopicTrendMcpServer;
use crate::mcp::tools::{
    ArticleTrendInput, CategoriesSearchTrendInput, CategoryTrendInput, TopNInput, TopicTrendInput,
    parse_date_opt, service_err,
};
use crate::models::{
    ArticleTrendResponse, CategoryRankResponse, CategoryTrendResponse, CategoriesTrendResponse,
    DailyViews, PageViewTopArticle, PageViewTopArticlesResponse, TopArticle, TopArticleCategory,
    TopCategory, CategoryInfo,
};
use crate::services::PageViewsService;
use crate::services::core::QidService;
use std::collections::HashMap;
use crate::services::composite::pageviews_service::{ArticleRank, CategoryRank};

impl TopicTrendMcpServer {
    /// Get daily Wikipedia pageviews for a category over a date range.
    ///
    /// Returns a time series of daily view counts plus the top viewed articles in the category.
    /// Use `depth` to include subcategory articles (0 = direct members only).
    #[tool(
        name = "topictrends_get_category_pageview_trend",
        description = "Daily Wikipedia pageviews for a category (direct members), with top articles.",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = true)
    )]
    pub async fn get_category_pageview_trend(
        &self,
        Parameters(p): Parameters<CategoryTrendInput>,
    ) -> Result<rmcp::handler::server::wrapper::Json<CategoryTrendResponse>, ErrorData> {
        let start = parse_date_opt(p.start_date)?;
        let end = parse_date_opt(p.end_date)?;
        let r = PageViewsService::get_category_trend(
            Arc::clone(&self.state), &p.wiki, &p.category, p.category_qid, start, end,
        ).await.map_err(service_err)?;

        let mut en_qids = article_rank_qids(&r.top_articles);
        en_qids.push(r.qid);
        let en = QidService::get_english_titles(Arc::clone(&self.state), &p.wiki, &en_qids).await;

        Ok(rmcp::handler::server::wrapper::Json(CategoryTrendResponse {
            qid: r.qid,
            title_en: en.get(&r.qid).cloned(),
            title: r.title,
            views: r.views.into_iter().map(|(date, views)| DailyViews { date, views }).collect(),
            top_articles: build_top_articles(r.top_articles, &en),
        }))
    }

    /// Get daily Wikipedia pageviews for a single article over a date range.
    #[tool(
        name = "topictrends_get_article_pageview_trend",
        description = "Daily Wikipedia pageviews for a single article.",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = true)
    )]
    pub async fn get_article_pageview_trend(
        &self,
        Parameters(p): Parameters<ArticleTrendInput>,
    ) -> Result<rmcp::handler::server::wrapper::Json<ArticleTrendResponse>, ErrorData> {
        let start = parse_date_opt(p.start_date)?;
        let end = parse_date_opt(p.end_date)?;
        let r = PageViewsService::get_article_trend(
            Arc::clone(&self.state), &p.wiki, &p.article, p.article_qid, start, end,
        ).await.map_err(service_err)?;

        let en =
            QidService::get_english_titles(Arc::clone(&self.state), &p.wiki, &[r.qid]).await;

        Ok(rmcp::handler::server::wrapper::Json(ArticleTrendResponse {
            qid: r.qid,
            title_en: en.get(&r.qid).cloned(),
            title: r.title,
            views: r.views.into_iter().map(|(date, views)| DailyViews { date, views }).collect(),
        }))
    }

    /// Get daily Wikipedia pageviews for a semantic topic.
    ///
    /// Performs embedding-based search against the English Wikipedia category taxonomy,
    /// then aggregates daily pageviews across all matched categories mapped to the target wiki.
    /// The topic does not need to match an exact category title.
    #[tool(
        name = "topictrends_get_topic_pageview_trend",
        description = "Daily Wikipedia pageviews for a semantic topic query (embedding-based, cross-language).",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = true)
    )]
    pub async fn get_topic_pageview_trend(
        &self,
        Parameters(p): Parameters<TopicTrendInput>,
    ) -> Result<rmcp::handler::server::wrapper::Json<CategoryTrendResponse>, ErrorData> {
        let start = parse_date_opt(p.start_date)?;
        let end = parse_date_opt(p.end_date)?;
        let r = PageViewsService::get_topic_trend(
            Arc::clone(&self.state), &p.wiki, &p.topic, start, end,
        ).await.map_err(service_err)?;

        let en_qids = article_rank_qids(&r.top_articles);
        let en = QidService::get_english_titles(Arc::clone(&self.state), &p.wiki, &en_qids).await;

        Ok(rmcp::handler::server::wrapper::Json(CategoryTrendResponse {
            qid: r.qid,
            title_en: None,
            title: r.title,
            views: r.views.into_iter().map(|(date, views)| DailyViews { date, views }).collect(),
            top_articles: build_top_articles(r.top_articles, &en),
        }))
    }

    /// Get the top Wikipedia categories ranked by total pageviews in a period.
    #[tool(
        name = "topictrends_get_pageviews_top_categories",
        description = "Top Wikipedia categories ranked by total pageviews in the given period.",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = true)
    )]
    pub async fn get_pageviews_top_categories(
        &self,
        Parameters(p): Parameters<TopNInput>,
    ) -> Result<rmcp::handler::server::wrapper::Json<CategoryRankResponse>, ErrorData> {
        let start = parse_date_opt(p.start_date)?;
        let end = parse_date_opt(p.end_date)?;
        let cats = PageViewsService::get_top_categories(
            Arc::clone(&self.state), &p.wiki, start, end, p.top_n,
        ).await.map_err(service_err)?;

        let mut en_qids: Vec<u32> = Vec::new();
        for cat in &cats {
            en_qids.push(cat.qid);
            en_qids.extend(article_rank_qids(&cat.top_articles));
        }
        let en = QidService::get_english_titles(Arc::clone(&self.state), &p.wiki, &en_qids).await;

        Ok(rmcp::handler::server::wrapper::Json(CategoryRankResponse {
            categories: cats.into_iter().map(|c| build_top_category(c, &en)).collect(),
        }))
    }

    /// Get the top Wikipedia articles globally ranked by total pageviews in a period.
    #[tool(
        name = "topictrends_get_pageviews_top_articles",
        description = "Top Wikipedia articles globally ranked by total pageviews in the given period.",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = true)
    )]
    pub async fn get_pageviews_top_articles(
        &self,
        Parameters(p): Parameters<TopNInput>,
    ) -> Result<rmcp::handler::server::wrapper::Json<PageViewTopArticlesResponse>, ErrorData> {
        let start = parse_date_opt(p.start_date)?;
        let end = parse_date_opt(p.end_date)?;
        let arts = PageViewsService::get_top_articles_global(
            Arc::clone(&self.state), &p.wiki, start, end, p.top_n,
        ).await.map_err(service_err)?;

        let mut en_qids: Vec<u32> = Vec::new();
        for art in &arts {
            en_qids.push(art.qid);
            en_qids.extend(art.categories.iter().map(|c| c.qid));
        }
        let en = QidService::get_english_titles(Arc::clone(&self.state), &p.wiki, &en_qids).await;

        Ok(rmcp::handler::server::wrapper::Json(PageViewTopArticlesResponse {
            articles: arts.into_iter().map(|art| PageViewTopArticle {
                qid: art.qid,
                title_en: en.get(&art.qid).cloned(),
                title: art.title,
                views: art.views,
                categories: art.categories.into_iter()
                    .map(|c| TopArticleCategory {
                        qid: c.qid,
                        title_en: en.get(&c.qid).cloned(),
                        title: c.title,
                    })
                    .collect(),
            }).collect(),
        }))
    }

    /// Get pageview trends for categories found via semantic search.
    ///
    /// Searches categories by `category_query` (embedding-based), then returns a combined
    /// cumulative trend plus the top articles across all matched categories.
    #[tool(
        name = "topictrends_get_categories_pageview_trend",
        description = "Combined pageview trend for all categories matching a semantic search query.",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = true)
    )]
    pub async fn get_categories_pageview_trend(
        &self,
        Parameters(p): Parameters<CategoriesSearchTrendInput>,
    ) -> Result<rmcp::handler::server::wrapper::Json<CategoriesTrendResponse>, ErrorData> {
        let start = parse_date_opt(p.start_date)?;
        let end = parse_date_opt(p.end_date)?;
        let limit = p.limit.unwrap_or(1000);
        let match_threshold = p.match_threshold.unwrap_or(0.6);

        let search_results = topictrend_taxonomy::search(
            p.category_query.clone(), "enwiki".to_string(), limit, match_threshold,
        ).await.map_err(|e| ErrorData::internal_error(e.to_string(), None))?;

        let qids: Vec<u32> = search_results.into_iter().map(|r| r.qid).collect();

        let r = PageViewsService::get_categories_trend(
            Arc::clone(&self.state), &p.wiki, qids, start, end,
        ).await.map_err(service_err)?;

        let mut en_qids: Vec<u32> = r.categories.iter().map(|c| c.qid).collect();
        en_qids.extend(article_rank_qids(&r.top_articles));
        let en = QidService::get_english_titles(Arc::clone(&self.state), &p.wiki, &en_qids).await;

        Ok(rmcp::handler::server::wrapper::Json(CategoriesTrendResponse {
            categories: r.categories.into_iter()
                .map(|c| CategoryInfo {
                    qid: c.qid,
                    title_en: en.get(&c.qid).cloned(),
                    title: c.title,
                })
                .collect(),
            cumulative_views: r.cumulative_views.into_iter()
                .map(|(date, views)| DailyViews { date, views })
                .collect(),
            top_articles: build_top_articles(r.top_articles, &en),
        }))
    }
}

fn build_top_articles(arts: Vec<ArticleRank>, en: &HashMap<u32, String>) -> Vec<TopArticle> {
    arts.into_iter().map(|art| TopArticle {
        qid: art.qid,
        title_en: en.get(&art.qid).cloned(),
        title: art.title,
        views: art.views,
        source_categories: art.source_categories.into_iter()
            .map(|(qid, title)| TopArticleCategory {
                qid,
                title,
                title_en: en.get(&qid).cloned(),
            })
            .collect(),
    }).collect()
}

fn build_top_category(cat: CategoryRank, en: &HashMap<u32, String>) -> TopCategory {
    TopCategory {
        qid: cat.qid,
        title_en: en.get(&cat.qid).cloned(),
        title: cat.title,
        views: cat.views,
        top_articles: build_top_articles(cat.top_articles, en),
    }
}

/// QIDs appearing in a list of ranked articles (articles + their source categories).
fn article_rank_qids(arts: &[ArticleRank]) -> Vec<u32> {
    let mut qids = Vec::new();
    for art in arts {
        qids.push(art.qid);
        qids.extend(art.source_categories.iter().map(|(qid, _)| *qid));
    }
    qids
}
