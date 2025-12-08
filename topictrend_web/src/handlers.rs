use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_macros::debug_handler;
use std::sync::Arc;

use crate::models::{
    AppState, ArticleDeltaParams, ArticleDeltaResponse, ArticleTrendParams, CategoryDeltaParams,
    CategoryDeltaResponse, CategoryRankResponse, CategoryTrendParams, DailyViews,
    SubCategoryParams, TopArticle, TopCategoriesParams, TopCategory,
};
use crate::services::composite::DeltaService;
use crate::{
    models::{ArticleTrendResponse, CategoryTrendResponse},
    services::PageViewsService,
};

// Custom error type for API handlers
#[derive(Debug)]
pub enum ApiError {
    ServiceError(crate::services::ServiceError),
    DeltaError(crate::services::core::CoreServiceError),
}

impl From<crate::services::ServiceError> for ApiError {
    fn from(err: crate::services::ServiceError) -> Self {
        ApiError::ServiceError(err)
    }
}

impl From<crate::services::core::CoreServiceError> for ApiError {
    fn from(err: crate::services::core::CoreServiceError) -> Self {
        ApiError::DeltaError(err)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ApiError::ServiceError(e) => match e {
                crate::services::ServiceError::CoreError(core_err) => match core_err {
                    crate::services::core::CoreServiceError::DatabaseError(e) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Database error: {}", e),
                    ),
                    crate::services::core::CoreServiceError::EngineError(e) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Engine error: {}", e),
                    ),
                    crate::services::core::CoreServiceError::NotFound => {
                        (StatusCode::NOT_FOUND, "Resource not found".to_string())
                    }
                    crate::services::core::CoreServiceError::InternalError(e) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Internal server error: {}", e),
                    ),
                },
            },
            ApiError::DeltaError(core_err) => match core_err {
                crate::services::core::CoreServiceError::DatabaseError(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Database error: {}", e),
                ),
                crate::services::core::CoreServiceError::EngineError(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Engine error: {}", e),
                ),
                crate::services::core::CoreServiceError::NotFound => {
                    (StatusCode::NOT_FOUND, "Resource not found".to_string())
                }
                crate::services::core::CoreServiceError::InternalError(e) => (
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

pub async fn get_category_delta_handler(
    Query(params): Query<CategoryDeltaParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<CategoryDeltaResponse>, ApiError> {
    let limit = params.limit.unwrap_or(100) as usize;
    let depth = params.depth.unwrap_or(0);

    let delta_items = DeltaService::get_category_delta(
        Arc::clone(&state),
        &params.wiki,
        params.baseline_start_date,
        params.baseline_end_date,
        params.impact_start_date,
        params.impact_end_date,
        limit,
        depth,
    )
    .await?;

    let categories: Vec<crate::models::CategoryDeltaItemResponse> = delta_items
        .into_iter()
        .map(|item| crate::models::CategoryDeltaItemResponse {
            category_qid: item.category_qid,
            category_title: item.category_title,
            baseline_views: item.baseline_views,
            impact_views: item.impact_views,
            delta_percentage: item.delta_percentage,
            absolute_delta: item.absolute_delta,
        })
        .collect();

    let baseline_period = format!(
        "{} to {}",
        params.baseline_start_date, params.baseline_end_date
    );
    let impact_period = format!("{} to {}", params.impact_start_date, params.impact_end_date);

    Ok(Json(CategoryDeltaResponse {
        categories,
        baseline_period,
        impact_period,
    }))
}

pub async fn get_article_delta_handler(
    Query(params): Query<ArticleDeltaParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<ArticleDeltaResponse>, ApiError> {
    use crate::services::core::QidService;

    let limit = params.limit.unwrap_or(100) as usize;
    let depth = params.depth.unwrap_or(0);

    let delta_items = DeltaService::get_article_delta(
        Arc::clone(&state),
        &params.wiki,
        params.category_qid,
        params.baseline_start_date,
        params.baseline_end_date,
        params.impact_start_date,
        params.impact_end_date,
        limit,
        depth,
    )
    .await?;

    let articles: Vec<crate::models::ArticleDeltaItemResponse> = delta_items
        .into_iter()
        .map(|item| crate::models::ArticleDeltaItemResponse {
            article_qid: item.article_qid,
            article_title: item.article_title,
            baseline_views: item.baseline_views,
            impact_views: item.impact_views,
            delta_percentage: item.delta_percentage,
            absolute_delta: item.absolute_delta,
        })
        .collect();

    // Get category title
    let category_title =
        QidService::get_title_by_qid(Arc::clone(&state), &params.wiki, params.category_qid)
            .await
            .unwrap_or_else(|_| format!("Q{}", params.category_qid));

    let baseline_period = format!(
        "{} to {}",
        params.baseline_start_date, params.baseline_end_date
    );
    let impact_period = format!("{} to {}", params.impact_start_date, params.impact_end_date);

    Ok(Json(ArticleDeltaResponse {
        articles,
        category_qid: params.category_qid,
        category_title,
        baseline_period,
        impact_period,
    }))
}
