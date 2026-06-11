use super::{CoreServiceError, EngineService, excluded_categories};
use crate::models::AppState;
use chrono::NaiveDate;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct DailySearchMetrics {
    pub date: NaiveDate,
    pub clicks: u64,
    pub impressions: u64,
    pub ctr: f64,
    pub position: f64,
}

#[derive(Clone, Debug)]
pub struct ArticleSearchRank {
    pub article_qid: u32,
    pub total_clicks: u64,
    pub total_impressions: u64,
    pub ctr: f64,
}

#[derive(Clone, Debug)]
pub struct CategorySearchRank {
    pub category_qid: u32,
    pub total_clicks: u64,
    pub total_impressions: u64,
    pub ctr: f64,
    pub top_articles: Vec<ArticleSearchRank>,
}

pub struct GoogleSearchService;

impl GoogleSearchService {
    pub async fn get_category_search_trend(
        state: Arc<AppState>,
        wiki: &str,
        category_qid: u32,
        start_date: NaiveDate,
        end_date: NaiveDate,
        depth: u32,
    ) -> Result<Vec<DailySearchMetrics>, CoreServiceError> {
        let engine = EngineService::get_or_build_google_search_engine(state, wiki).await?;

        let raw_data = {
            let engine_lock = engine.read().map_err(|e| {
                CoreServiceError::InternalError(format!("Failed to acquire read lock: {}", e))
            })?;

            engine_lock.get_category_trend(category_qid, depth, start_date, end_date)
        };

        Ok(raw_data
            .into_iter()
            .map(|(date, metrics)| DailySearchMetrics {
                date,
                clicks: metrics.clicks,
                impressions: metrics.impressions,
                ctr: metrics.ctr,
                position: metrics.position,
            })
            .collect())
    }

    pub async fn get_article_search_trend(
        state: Arc<AppState>,
        wiki: &str,
        article_qid: u32,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<DailySearchMetrics>, CoreServiceError> {
        let engine = EngineService::get_or_build_google_search_engine(state, wiki).await?;

        let raw_data = {
            let engine_lock = engine.read().map_err(|e| {
                CoreServiceError::InternalError(format!("Failed to acquire read lock: {}", e))
            })?;

            engine_lock.get_article_trend(article_qid, start_date, end_date)
        };

        Ok(raw_data
            .into_iter()
            .map(|(date, metrics)| DailySearchMetrics {
                date,
                clicks: metrics.clicks,
                impressions: metrics.impressions,
                ctr: metrics.ctr,
                position: metrics.position,
            })
            .collect())
    }

    pub async fn get_top_articles(
        state: Arc<AppState>,
        wiki: &str,
        category_qid: u32,
        start_date: NaiveDate,
        end_date: NaiveDate,
        depth: u32,
        limit: usize,
    ) -> Result<Vec<ArticleSearchRank>, CoreServiceError> {
        let engine = EngineService::get_or_build_google_search_engine(state, wiki).await?;

        let top_articles = {
            let engine_lock = engine.read().map_err(|e| {
                CoreServiceError::InternalError(format!("Failed to acquire read lock: {}", e))
            })?;

            engine_lock
                .get_top_articles_in_category(category_qid, start_date, end_date, depth, limit)
                .map_err(|e| {
                    CoreServiceError::EngineError(format!("Failed to get top articles: {}", e))
                })?
        };

        Ok(top_articles
            .top_articles
            .into_iter()
            .map(|article| ArticleSearchRank {
                article_qid: article.article_qid,
                total_clicks: article.total_clicks,
                total_impressions: article.total_impressions,
                ctr: article.ctr,
            })
            .collect())
    }

    pub async fn get_top_categories(
        state: Arc<AppState>,
        wiki: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
        limit: usize,
    ) -> Result<Vec<CategorySearchRank>, CoreServiceError> {
        let engine = EngineService::get_or_build_google_search_engine(state, wiki).await?;

        // Oversample so dropping denylisted categories cannot shrink the page.
        let categories = {
            let engine_lock = engine.read().map_err(|e| {
                CoreServiceError::InternalError(format!("Failed to acquire read lock: {}", e))
            })?;

            engine_lock
                .get_top_categories(start_date, end_date, excluded_categories::oversampled(limit))
                .map_err(|e| {
                    CoreServiceError::EngineError(format!("Failed to get top categories: {}", e))
                })?
        };

        Ok(categories
            .into_iter()
            .filter(|category| {
                !excluded_categories::EXCLUDED_CATEGORY_QIDS.contains(&category.category_qid)
            })
            .take(limit)
            .map(|category| CategorySearchRank {
                category_qid: category.category_qid,
                total_clicks: category.total_clicks,
                total_impressions: category.total_impressions,
                ctr: category.ctr,
                top_articles: category
                    .top_articles
                    .into_iter()
                    .map(|article| ArticleSearchRank {
                        article_qid: article.article_qid,
                        total_clicks: article.total_clicks,
                        total_impressions: article.total_impressions,
                        ctr: article.ctr,
                    })
                    .collect(),
            })
            .collect())
    }

    pub async fn get_top_articles_global(
        state: Arc<AppState>,
        wiki: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
        limit: usize,
    ) -> Result<Vec<ArticleSearchRank>, CoreServiceError> {
        let engine = EngineService::get_or_build_google_search_engine(state, wiki).await?;

        let top_articles = {
            let engine_lock = engine.read().map_err(|e| {
                CoreServiceError::InternalError(format!("Failed to acquire read lock: {}", e))
            })?;

            engine_lock
                .get_top_articles(start_date, end_date, limit)
                .map_err(|e| {
                    CoreServiceError::EngineError(format!("Failed to get top articles: {}", e))
                })?
        };

        Ok(top_articles
            .into_iter()
            .map(|article| ArticleSearchRank {
                article_qid: article.article_qid,
                total_clicks: article.total_clicks,
                total_impressions: article.total_impressions,
                ctr: article.ctr,
            })
            .collect())
    }
}
