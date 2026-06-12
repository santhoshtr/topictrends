use std::sync::Arc;

use rmcp::{ErrorData, tool};
use rmcp::handler::server::wrapper::Parameters;

use crate::mcp::TopicTrendMcpServer;
use crate::mcp::tools::{
    ArticleTrendInput, CategoryTrendInput, TopNInput, TopicTrendInput,
    parse_date_opt, core_err,
};
use crate::models::{
    CategorySearchRankResponse, DailyGoogleSearch, GoogleSearchArticleTrendResponse,
    GoogleSearchCategoryTrendResponse, GoogleSearchTopArticle, GoogleSearchTopArticlesResponse,
    TopArticleBySearch, TopArticleCategory, TopArticleGoogleSearch, TopCategoryBySearch,
};
use crate::services::composite::GoogleSearchTrendsService;
use crate::services::core::QidService;
use std::collections::HashMap;
use crate::services::composite::google_search_service::{
    ArticleGoogleSearchRank, CategorySearchRankResult,
};

impl TopicTrendMcpServer {
    /// Get daily Google Search Console metrics for a Wikipedia category.
    ///
    /// Returns clicks, impressions, CTR and average position per day.
    #[tool(
        name = "topictrends_get_category_googlesearch_trend",
        description = "Daily Google Search clicks, impressions, CTR and position for a Wikipedia category.",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = true)
    )]
    pub async fn get_category_googlesearch_trend(
        &self,
        Parameters(p): Parameters<CategoryTrendInput>,
    ) -> Result<rmcp::handler::server::wrapper::Json<GoogleSearchCategoryTrendResponse>, ErrorData> {
        let start = parse_date_opt(p.start_date)?;
        let end = parse_date_opt(p.end_date)?;
        let r = GoogleSearchTrendsService::get_category_trend(
            Arc::clone(&self.state), &p.wiki, &p.category, p.category_qid, start, end,
        ).await.map_err(core_err)?;

        let mut en_qids = article_search_rank_qids(&r.top_articles);
        en_qids.push(r.qid);
        let en = QidService::get_english_titles(Arc::clone(&self.state), &p.wiki, &en_qids).await;

        Ok(rmcp::handler::server::wrapper::Json(GoogleSearchCategoryTrendResponse {
            qid: r.qid,
            title_en: en.get(&r.qid).cloned(),
            title: r.title,
            search: r.search.into_iter().map(|item| DailyGoogleSearch {
                date: item.date,
                clicks: item.clicks,
                impressions: item.impressions,
                ctr: item.ctr,
                position: item.position,
            }).collect(),
            top_articles: build_top_article_search(r.top_articles, &en),
        }))
    }

    /// Get daily Google Search Console metrics for a single Wikipedia article.
    #[tool(
        name = "topictrends_get_article_googlesearch_trend",
        description = "Daily Google Search clicks, impressions, CTR and position for a single Wikipedia article.",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = true)
    )]
    pub async fn get_article_googlesearch_trend(
        &self,
        Parameters(p): Parameters<ArticleTrendInput>,
    ) -> Result<rmcp::handler::server::wrapper::Json<GoogleSearchArticleTrendResponse>, ErrorData> {
        let start = parse_date_opt(p.start_date)?;
        let end = parse_date_opt(p.end_date)?;
        let r = GoogleSearchTrendsService::get_article_trend(
            Arc::clone(&self.state), &p.wiki, &p.article, p.article_qid, start, end,
        ).await.map_err(core_err)?;

        let en =
            QidService::get_english_titles(Arc::clone(&self.state), &p.wiki, &[r.qid]).await;

        Ok(rmcp::handler::server::wrapper::Json(GoogleSearchArticleTrendResponse {
            qid: r.qid,
            title_en: en.get(&r.qid).cloned(),
            title: r.title,
            search: r.search.into_iter().map(|item| DailyGoogleSearch {
                date: item.date,
                clicks: item.clicks,
                impressions: item.impressions,
                ctr: item.ctr,
                position: item.position,
            }).collect(),
        }))
    }

    /// Get daily Google Search Console metrics for a semantic topic.
    ///
    /// Performs embedding-based category search then aggregates Google Search data.
    #[tool(
        name = "topictrends_get_topic_googlesearch_trend",
        description = "Daily Google Search metrics for a semantic topic query.",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = true)
    )]
    pub async fn get_topic_googlesearch_trend(
        &self,
        Parameters(p): Parameters<TopicTrendInput>,
    ) -> Result<rmcp::handler::server::wrapper::Json<GoogleSearchCategoryTrendResponse>, ErrorData> {
        let start = parse_date_opt(p.start_date)?;
        let end = parse_date_opt(p.end_date)?;
        let r = GoogleSearchTrendsService::get_topic_google_search_trend(
            Arc::clone(&self.state), &p.wiki, &p.topic, start, end,
        ).await.map_err(core_err)?;

        let en_qids = article_search_rank_qids(&r.top_articles);
        let en = QidService::get_english_titles(Arc::clone(&self.state), &p.wiki, &en_qids).await;

        Ok(rmcp::handler::server::wrapper::Json(GoogleSearchCategoryTrendResponse {
            qid: r.qid,
            title_en: None,
            title: r.title,
            search: r.search.into_iter().map(|item| DailyGoogleSearch {
                date: item.date,
                clicks: item.clicks,
                impressions: item.impressions,
                ctr: item.ctr,
                position: item.position,
            }).collect(),
            top_articles: build_top_article_search(r.top_articles, &en),
        }))
    }

    /// Get the top Wikipedia categories ranked by total Google Search clicks in a period.
    #[tool(
        name = "topictrends_get_googlesearch_top_categories",
        description = "Top Wikipedia categories ranked by total Google Search clicks in the given period.",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = true)
    )]
    pub async fn get_googlesearch_top_categories(
        &self,
        Parameters(p): Parameters<TopNInput>,
    ) -> Result<rmcp::handler::server::wrapper::Json<CategorySearchRankResponse>, ErrorData> {
        let start = parse_date_opt(p.start_date)?;
        let end = parse_date_opt(p.end_date)?;
        let cats = GoogleSearchTrendsService::get_top_categories(
            Arc::clone(&self.state), &p.wiki, start, end, p.top_n,
        ).await.map_err(core_err)?;

        let mut en_qids: Vec<u32> = Vec::new();
        for cat in &cats {
            en_qids.push(cat.qid);
            en_qids.extend(cat.top_articles.iter().map(|a| a.qid));
        }
        let en = QidService::get_english_titles(Arc::clone(&self.state), &p.wiki, &en_qids).await;

        Ok(rmcp::handler::server::wrapper::Json(CategorySearchRankResponse {
            categories: cats.into_iter().map(|c| build_category_search_rank(c, &en)).collect(),
        }))
    }

    /// Get the top Wikipedia articles globally ranked by total Google Search clicks in a period.
    #[tool(
        name = "topictrends_get_googlesearch_top_articles",
        description = "Top Wikipedia articles globally ranked by total Google Search clicks in the given period.",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = true)
    )]
    pub async fn get_googlesearch_top_articles(
        &self,
        Parameters(p): Parameters<TopNInput>,
    ) -> Result<rmcp::handler::server::wrapper::Json<GoogleSearchTopArticlesResponse>, ErrorData> {
        let start = parse_date_opt(p.start_date)?;
        let end = parse_date_opt(p.end_date)?;
        let arts = GoogleSearchTrendsService::get_top_articles_global(
            Arc::clone(&self.state), &p.wiki, start, end, p.top_n,
        ).await.map_err(core_err)?;

        let mut en_qids: Vec<u32> = Vec::new();
        for art in &arts {
            en_qids.push(art.qid);
            en_qids.extend(art.categories.iter().map(|c| c.qid));
        }
        let en = QidService::get_english_titles(Arc::clone(&self.state), &p.wiki, &en_qids).await;

        Ok(rmcp::handler::server::wrapper::Json(GoogleSearchTopArticlesResponse {
            articles: arts.into_iter().map(|art| GoogleSearchTopArticle {
                qid: art.qid,
                title_en: en.get(&art.qid).cloned(),
                title: art.title,
                clicks: art.clicks,
                impressions: art.impressions,
                ctr: art.ctr,
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
}

fn build_top_article_search(
    arts: Vec<ArticleGoogleSearchRank>,
    en: &HashMap<u32, String>,
) -> Vec<TopArticleGoogleSearch> {
    arts.into_iter().map(|art| TopArticleGoogleSearch {
        qid: art.qid,
        title_en: en.get(&art.qid).cloned(),
        title: art.title,
        clicks: art.clicks,
        impressions: art.impressions,
        ctr: art.ctr,
        source_categories: art.source_categories.into_iter()
            .map(|(qid, title)| TopArticleCategory {
                qid,
                title,
                title_en: en.get(&qid).cloned(),
            })
            .collect(),
    }).collect()
}

fn build_category_search_rank(
    cat: CategorySearchRankResult,
    en: &HashMap<u32, String>,
) -> TopCategoryBySearch {
    TopCategoryBySearch {
        qid: cat.qid,
        title_en: en.get(&cat.qid).cloned(),
        title: cat.title,
        clicks: cat.clicks,
        impressions: cat.impressions,
        ctr: cat.ctr,
        top_articles: cat.top_articles.into_iter().map(|art| TopArticleBySearch {
            qid: art.qid,
            title_en: en.get(&art.qid).cloned(),
            title: art.title,
            clicks: art.clicks,
            impressions: art.impressions,
            ctr: art.ctr,
        }).collect(),
    }
}

/// QIDs appearing in a list of ranked articles (articles + their source categories).
fn article_search_rank_qids(arts: &[ArticleGoogleSearchRank]) -> Vec<u32> {
    let mut qids = Vec::new();
    for art in arts {
        qids.push(art.qid);
        qids.extend(art.source_categories.iter().map(|(qid, _)| *qid));
    }
    qids
}
