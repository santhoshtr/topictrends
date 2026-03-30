use axum::{
    Json,
    extract::{Query, State},
};
use std::sync::Arc;

use crate::services::PageEditsService;
use crate::models::{
    AppState, ArticleTrendParams, ArticleEditTrendResponse, CategoryTrendParams,
    CategoryEditTrendResponse, DailyEdits, TopArticleEdits, TopCategoriesParams,
    CategoryEditRankResponse, TopCategoryByEdits, TopArticleByEdits, PageEditTopArticlesResponse,
    PageEditTopArticle, TopArticleCategory, TopicTrendParams,
};

use super::ApiError;

pub async fn get_category_edit_trend_handler(
    Query(params): Query<CategoryTrendParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<CategoryEditTrendResponse>, ApiError> {
    let result = PageEditsService::get_category_edit_trend(
        state,
        &params.wiki,
        &params.category,
        params.category_qid,
        params.depth,
        params.start_date,
        params.end_date,
    )
    .await?;

    let daily_edits: Vec<DailyEdits> = result
        .edits
        .into_iter()
        .map(|(date, edits)| DailyEdits { date, edits })
        .collect();

    let top_articles: Vec<TopArticleEdits> = result
        .top_articles
        .into_iter()
        .map(|art| TopArticleEdits {
            qid: art.qid,
            title: art.title,
            edits: art.edits,
            source_categories: art.source_categories.into_iter()
                .map(|(qid, title)| TopArticleCategory { qid, title })
                .collect(),
        })
        .collect();

    Ok(Json(CategoryEditTrendResponse {
        qid: result.qid,
        title: result.title,
        edits: daily_edits,
        top_articles,
    }))
}

pub async fn get_article_edit_trend_handler(
    Query(params): Query<ArticleTrendParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<ArticleEditTrendResponse>, ApiError> {
    let result = PageEditsService::get_article_edit_trend(
        state,
        &params.wiki,
        &params.article,
        params.article_qid,
        params.start_date,
        params.end_date,
    )
    .await?;

    let daily_edits = result
        .edits
        .into_iter()
        .map(|(date, edits)| DailyEdits { date, edits })
        .collect();

    Ok(Json(ArticleEditTrendResponse {
        qid: result.qid,
        title: result.title,
        edits: daily_edits,
    }))
}

pub async fn get_topic_edit_trend_handler(
    Query(params): Query<TopicTrendParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<CategoryEditTrendResponse>, ApiError> {
    let result = PageEditsService::get_topic_edit_trend(
        state,
        &params.wiki,
        &params.topic,
        params.depth,
        params.start_date,
        params.end_date,
    )
    .await?;

    let daily_edits: Vec<DailyEdits> = result
        .edits
        .into_iter()
        .map(|(date, edits)| DailyEdits { date, edits })
        .collect();

    let top_articles: Vec<TopArticleEdits> = result
        .top_articles
        .into_iter()
        .map(|art| TopArticleEdits {
            qid: art.qid,
            title: art.title,
            edits: art.edits,
            source_categories: art.source_categories.into_iter()
                .map(|(qid, title)| TopArticleCategory { qid, title })
                .collect(),
        })
        .collect();

    Ok(Json(CategoryEditTrendResponse {
        qid: result.qid,
        title: result.title,
        edits: daily_edits,
        top_articles,
    }))
}

pub async fn get_pageedits_top_categories_handler(
    Query(params): Query<TopCategoriesParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<CategoryEditRankResponse>, ApiError> {
    let categories = PageEditsService::get_top_categories(
        state,
        &params.wiki,
        params.start_date,
        params.end_date,
        params.top_n,
    )
    .await?;

    let response = CategoryEditRankResponse {
        categories: categories
            .into_iter()
            .map(|cat| TopCategoryByEdits {
                qid: cat.qid,
                title: cat.title,
                edits: cat.edits,
                top_articles: cat
                    .top_articles
                    .into_iter()
                    .map(|art| TopArticleByEdits {
                        qid: art.qid,
                        title: art.title,
                        edits: art.edits,
                    })
                    .collect(),
            })
            .collect(),
    };

    Ok(Json(response))
}

pub async fn get_pageedits_top_articles_handler(
    Query(params): Query<TopCategoriesParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<PageEditTopArticlesResponse>, ApiError> {
    let articles = PageEditsService::get_top_articles_global(
        state,
        &params.wiki,
        params.start_date,
        params.end_date,
        params.top_n,
    )
    .await?;

    let response = PageEditTopArticlesResponse {
        articles: articles
            .into_iter()
            .map(|article| PageEditTopArticle {
                qid: article.qid,
                title: article.title,
                edits: article.edits,
                categories: article
                    .categories
                    .into_iter()
                    .map(|category| TopArticleCategory {
                        qid: category.qid,
                        title: category.title,
                    })
                    .collect(),
            })
            .collect(),
    };

    Ok(Json(response))
}
