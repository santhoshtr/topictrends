use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_macros::debug_handler;
use std::sync::Arc;

use crate::services::{
    composite::DeltaService,
    core::{CoreServiceError, QidService},
};
use crate::{
    models::{
        AppState, ArticleDeltaParams, ArticleDeltaResponse, ArticleItem, ArticleTrendParams,
        ArticlesInCategoryResponse, CategoryDeltaParams, CategoryDeltaResponse,
        CategoryRankResponse, CategorySearchItemResponse, CategorySearchParams,
        CategorySearchResponse, CategoryTrendParams, DailyViews, ListArticlesInCategoryParams,
        SubCategoryParams, TopArticle, TopCategoriesParams, TopCategory,
    },
    services::core::CategoryService,
};
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

pub async fn search_categories(
    Query(params): Query<CategorySearchParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<CategorySearchResponse>, ApiError> {
    use crate::services::core::QidService;

    let limit: u64 = params.limit.unwrap_or(1000u64);
    let match_threshold = params.match_threshold.unwrap_or(0.5);

    let search_results: Vec<topictrend_taxonomy::SearchResult> =
        topictrend_taxonomy::search(params.query.clone(), "enwiki".to_string(), limit)
            .await
            .map_err(|e| {
                ApiError::ServiceError(crate::services::ServiceError::CoreError(
                    crate::services::core::CoreServiceError::InternalError(e.to_string()),
                ))
            })?;

    let mut categories: Vec<CategorySearchItemResponse> = search_results
        .into_iter()
        .filter(|result| result.score >= match_threshold)
        .map(|result| CategorySearchItemResponse {
            category_qid: result.qid,
            category_title_en: result.page_title,
            category_title: "".to_string(),
            match_score: result.score,
        })
        .collect();

    if params.wiki != "enwiki" {
        let qids: Vec<u32> = categories.iter().map(|cat| cat.category_qid).collect();

        let titles_in_target_wiki =
            QidService::get_titles_by_qids(Arc::clone(&state), &params.wiki, &qids)
                .await
                .unwrap_or_default();

        categories.retain_mut(|category| {
            if let Some(title) = titles_in_target_wiki.get(&category.category_qid) {
                category.category_title = title.clone();
                true
            } else {
                false
            }
        });
    } else {
        for category in &mut categories {
            category.category_title = category.category_title_en.clone();
        }
    }

    Ok(Json(CategorySearchResponse { categories }))
}

pub async fn get_articles_in_category(
    Query(params): Query<ListArticlesInCategoryParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<ArticlesInCategoryResponse>, ApiError> {
    let category_qid = if let Some(qid) = params.category_qid {
        qid
    } else {
        let category = params.category.ok_or_else(|| {
            CoreServiceError::InternalError(
                "Either category or category_qid must be provided".to_string(),
            )
        })?;
        QidService::get_qid_by_title(Arc::clone(&state), params.wiki.as_str(), &category, 14)
            .await?
    };

    // Get all articles in the category (depth 0 = direct members only)
    let article_qids = CategoryService::get_category_articles(
        Arc::clone(&state),
        params.wiki.as_str(),
        category_qid,
        0,
    )
    .await?;

    // Get titles for all articles
    let titles_map =
        QidService::get_titles_by_qids(Arc::clone(&state), params.wiki.as_str(), &article_qids)
            .await?;

    // Get view data for each article
    let mut articles_in_category = Vec::new();

    for article_qid in article_qids {
        let title = titles_map
            .get(&article_qid)
            .cloned()
            .unwrap_or_else(|| format!("Q{}", article_qid));

        articles_in_category.push(ArticleItem {
            qid: article_qid,
            title,
        });
    }
    Ok(Json(ArticlesInCategoryResponse {
        articles: articles_in_category,
    }))
}
