use super::{CoreServiceError, EngineService};
use crate::models::AppState;
use chrono::NaiveDate;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct ArticleViews {
    pub article_qid: u32,
    pub total_views: u64,
}

#[derive(Clone, Debug)]
pub struct CategoryViews {
    pub category_qid: u32,
    pub total_views: u64,
    pub top_articles: Vec<ArticleViews>,
}

pub struct PageViewService;

impl PageViewService {
    pub async fn get_category_views(
        state: Arc<AppState>,
        wiki: &str,
        category_qid: u32,
        start_date: NaiveDate,
        end_date: NaiveDate,
        depth: u32,
    ) -> Result<Vec<(NaiveDate, u64)>, CoreServiceError> {
        let engine = EngineService::get_or_build_engine(state, wiki).await?;

        let raw_data = {
            let mut engine_lock = engine.write().map_err(|e| {
                CoreServiceError::InternalError(format!("Failed to acquire write lock: {}", e))
            })?;

            engine_lock.get_category_trend(category_qid, depth, start_date, end_date)
        };

        Ok(raw_data)
    }

    pub async fn get_article_views(
        state: Arc<AppState>,
        wiki: &str,
        article_qid: u32,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<(NaiveDate, u64)>, CoreServiceError> {
        let engine = EngineService::get_or_build_engine(state, wiki).await?;

        let raw_data = {
            let mut engine_lock = engine.write().map_err(|e| {
                CoreServiceError::InternalError(format!("Failed to acquire write lock: {}", e))
            })?;

            engine_lock.get_article_trend(article_qid, start_date, end_date)
        };

        Ok(raw_data)
    }

    pub async fn get_top_articles(
        state: Arc<AppState>,
        wiki: &str,
        category_qid: u32,
        start_date: NaiveDate,
        end_date: NaiveDate,
        depth: u32,
        limit: usize,
    ) -> Result<Vec<ArticleViews>, CoreServiceError> {
        let engine = EngineService::get_or_build_engine(state, wiki).await?;

        let top_articles = {
            let mut engine_lock = engine.write().map_err(|e| {
                CoreServiceError::InternalError(format!("Failed to acquire write lock: {}", e))
            })?;

            engine_lock
                .get_top_articles_in_category(category_qid, start_date, end_date, depth, limit)
                .map_err(|e| {
                    CoreServiceError::EngineError(format!("Failed to get top articles: {}", e))
                })?
        };

        let raw_articles: Vec<ArticleViews> = top_articles
            .top_articles
            .into_iter()
            .map(|art| ArticleViews {
                article_qid: art.article_qid,
                total_views: art.total_views,
            })
            .collect();

        Ok(raw_articles)
    }

    pub async fn get_top_categories(
        state: Arc<AppState>,
        wiki: &str,
        start_date: NaiveDate,
        end_date: NaiveDate,
        limit: usize,
    ) -> Result<Vec<CategoryViews>, CoreServiceError> {
        let engine = EngineService::get_or_build_engine(state, wiki).await?;

        let categories = {
            let mut engine_lock = engine.write().map_err(|e| {
                CoreServiceError::InternalError(format!("Failed to acquire write lock: {}", e))
            })?;

            engine_lock
                .get_top_categories(start_date, end_date, limit)
                .map_err(|e| {
                    CoreServiceError::EngineError(format!("Failed to get top categories: {}", e))
                })?
        };

        let raw_categories: Vec<CategoryViews> = categories
            .into_iter()
            .map(|cat| {
                let top_articles: Vec<ArticleViews> = cat
                    .top_articles
                    .into_iter()
                    .map(|art| ArticleViews {
                        article_qid: art.article_qid,
                        total_views: art.total_views,
                    })
                    .collect();

                CategoryViews {
                    category_qid: cat.category_qid,
                    total_views: cat.total_views,
                    top_articles,
                }
            })
            .collect();

        Ok(raw_categories)
    }
}
