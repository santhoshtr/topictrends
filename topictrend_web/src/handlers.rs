use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_macros::debug_handler;
use std::sync::Arc;

use crate::models::{
    AppState, ArticleTrendParams, CategoryRankResponse, CategoryTrendParams, DailyViews,
    SubCategoryParams, TopArticle, TopCategoriesParams, TopCategory,
};
use crate::{
    models::ArticleTrendResponse, models::CategoryTrendResponse, services::PageViewsService,
};

// Custom error type for API handlers
#[derive(Debug)]
pub enum ApiError {
    ServiceError(crate::services::pageviews_service::ServiceError),
}

impl From<crate::services::pageviews_service::ServiceError> for ApiError {
    fn from(err: crate::services::pageviews_service::ServiceError) -> Self {
        ApiError::ServiceError(err)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ApiError::ServiceError(e) => match e {
                crate::services::pageviews_service::ServiceError::DatabaseError(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Database error: {}", e),
                ),
                crate::services::pageviews_service::ServiceError::EngineError(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Engine error: {}", e),
                ),
                crate::services::pageviews_service::ServiceError::NotFound => {
                    (StatusCode::NOT_FOUND, "Resource not found".to_string())
                }
                crate::services::pageviews_service::ServiceError::InternalError(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Internal server error: {}", e),
                ),
            },
        };

        (status, Json(serde_json::json!({ "error": error_message }))).into_response()
    }
}

pub async fn get_category_trend_handler(
    Query(params): Query<CategoryTrendParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<CategoryTrendResponse>, ApiError> {
    let result = PageViewsService::get_category_trend(
        state,
        &params.wiki,
        &params.category,
        params.category_qid,
        params.depth,
        params.start_date,
        params.end_date,
    )
    .await?;

    let daily_views: Vec<DailyViews> = result
        .views
        .into_iter()
        .map(|(date, views)| DailyViews { date, views })
        .collect();

    let top_articles: Vec<TopArticle> = result
        .top_articles
        .into_iter()
        .map(|art| TopArticle {
            qid: art.qid,
            title: art.title,
            views: art.views,
        })
        .collect();

    Ok(Json(CategoryTrendResponse {
        qid: result.qid,
        title: result.title,
        views: daily_views,
        top_articles,
    }))
}

pub async fn get_article_trend_handler(
    Query(params): Query<ArticleTrendParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<ArticleTrendResponse>, ApiError> {
    let result = PageViewsService::get_article_trend(
        state,
        &params.wiki,
        &params.article,
        params.article_qid,
        params.start_date,
        params.end_date,
    )
    .await?;

    let daily_views = result
        .views
        .into_iter()
        .map(|(date, views)| DailyViews { date, views })
        .collect();

    Ok(Json(ArticleTrendResponse {
        qid: result.qid,
        title: result.title,
        views: daily_views,
    }))
}

pub async fn get_sub_categories(
    Query(params): Query<SubCategoryParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<std::collections::HashMap<u32, String>>, ApiError> {
    let titles_map = PageViewsService::get_sub_categories(
        state,
        &params.wiki,
        &params.category,
        params.category_qid,
    )
    .await?;

    Ok(Json(titles_map))
}

#[debug_handler]
pub async fn get_top_categories_handler(
    Query(params): Query<TopCategoriesParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<CategoryRankResponse>, ApiError> {
    let categories = PageViewsService::get_top_categories(
        state,
        &params.wiki,
        params.start_date,
        params.end_date,
        params.top_n,
    )
    .await?;

    let top_categories_with_titles: Vec<TopCategory> = categories
        .into_iter()
        .map(|cat| {
            let top_articles: Vec<TopArticle> = cat
                .top_articles
                .into_iter()
                .map(|art| TopArticle {
                    qid: art.qid,
                    title: art.title,
                    views: art.views,
                })
                .collect();

            TopCategory {
                qid: cat.qid,
                title: cat.title,
                views: cat.views,
                top_articles,
            }
        })
        .collect();

    let response = CategoryRankResponse {
        categories: top_categories_with_titles,
    };

    Ok(Json(response))
}
