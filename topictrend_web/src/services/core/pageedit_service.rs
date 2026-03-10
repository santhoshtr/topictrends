use super::{CoreServiceError, EngineService};
use crate::models::AppState;
use chrono::NaiveDate;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct ArticleEdits {
    pub article_qid: u32,
    pub total_edits: u64,
}

#[derive(Clone, Debug)]
pub struct CategoryEdits {
    pub category_qid: u32,
    pub total_edits: u64,
    pub top_articles: Vec<ArticleEdits>,
}

pub struct PageEditService;

impl PageEditService {
    pub async fn get_category_edits(
        state: Arc<AppState>,
        wiki: &str,
        category_qid: u32,
        start_date: NaiveDate,
        end_date: NaiveDate,
        depth: u32,
    ) -> Result<Vec<(NaiveDate, u64)>, CoreServiceError> {
        let engine = EngineService::get_or_build_pageedit_engine(state, wiki).await?;

        let raw_data = {
            let engine_lock = engine.read().map_err(|e| {
                CoreServiceError::InternalError(format!("Failed to acquire read lock: {}", e))
            })?;

            engine_lock.get_category_trend(category_qid, depth, start_date, end_date)
        };

        Ok(raw_data)
    }

    pub async fn get_article_edits(
        state: Arc<AppState>,
        wiki: &str,
        article_qid: u32,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<Vec<(NaiveDate, u64)>, CoreServiceError> {
        let engine = EngineService::get_or_build_pageedit_engine(state, wiki).await?;

        let raw_data = {
            let engine_lock = engine.read().map_err(|e| {
                CoreServiceError::InternalError(format!("Failed to acquire read lock: {}", e))
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
    ) -> Result<Vec<ArticleEdits>, CoreServiceError> {
        let engine = EngineService::get_or_build_pageedit_engine(state, wiki).await?;

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

        let raw_articles: Vec<ArticleEdits> = top_articles
            .top_articles
            .into_iter()
            .map(|art| ArticleEdits {
                article_qid: art.article_qid,
                total_edits: art.total_edits,
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
    ) -> Result<Vec<CategoryEdits>, CoreServiceError> {
        let engine = EngineService::get_or_build_pageedit_engine(state, wiki).await?;

        let categories = {
            let engine_lock = engine.read().map_err(|e| {
                CoreServiceError::InternalError(format!("Failed to acquire read lock: {}", e))
            })?;

            engine_lock
                .get_top_categories(start_date, end_date, limit)
                .map_err(|e| {
                    CoreServiceError::EngineError(format!("Failed to get top categories: {}", e))
                })?
        };

        let raw_categories: Vec<CategoryEdits> = categories
            .into_iter()
            .map(|cat| {
                let top_articles: Vec<ArticleEdits> = cat
                    .top_articles
                    .into_iter()
                    .map(|art| ArticleEdits {
                        article_qid: art.article_qid,
                        total_edits: art.total_edits,
                    })
                    .collect();

                CategoryEdits {
                    category_qid: cat.category_qid,
                    total_edits: cat.total_edits,
                    top_articles,
                }
            })
            .collect();

        Ok(raw_categories)
    }
}
