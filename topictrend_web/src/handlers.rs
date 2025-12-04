use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_macros::debug_handler;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::Instant,
};
use topictrend::pageview_engine::PageViewEngine;

use crate::models::{
    AppState, ArticleTrendParams, CategoryRankResponse, CategoryTrendParams, DailyViews,
    SubCategoryParams, TopArticle, TopCategoriesParams, TopCategory,
};
use crate::{
    models::ArticleTrendResponse,
    models::CategoryTrendResponse,
    wiki::{get_qid_by_title, get_titles_by_qids},
};

// Custom error type for API handlers
#[derive(Debug)]
pub enum ApiError {
    DatabaseError(sqlx::Error),
    EngineError(String),
    NotFound,
    InternalError(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ApiError::DatabaseError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            ),
            ApiError::EngineError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Engine error: {}", e),
            ),
            ApiError::NotFound => (StatusCode::NOT_FOUND, "Resource not found".to_string()),
            ApiError::InternalError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Internal server error: {}", e),
            ),
        };

        (status, Json(serde_json::json!({ "error": error_message }))).into_response()
    }
}

pub async fn get_category_trend_handler(
    Query(params): Query<CategoryTrendParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<CategoryTrendResponse>, ApiError> {
    let depth = params.depth.unwrap_or(0);
    let start = params
        .start_date
        .unwrap_or_else(|| chrono::Local::now().date_naive() - chrono::Duration::days(30));
    let end = params
        .end_date
        .unwrap_or_else(|| chrono::Local::now().date_naive());

    let category_qid = if let Some(qid) = params.category_qid {
        qid
    } else {
        get_qid_by_title(Arc::clone(&state), &params.wiki, &params.category, &14_i8)
            .await
            .map_err(|e| ApiError::DatabaseError(e))?
    };

    // Wrap the entire blocking operation
    let now = Instant::now();
    let engine = get_or_build_engine(Arc::clone(&state), &params.wiki)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to build engine: {}", e)))?;

    println!("Engine build completed in {:.2?}s", now.elapsed());

    // Acquire a write lock to access the engine mutably
    let (raw_data, category_rank) = {
        let mut engine_lock = engine
            .write()
            .map_err(|e| ApiError::InternalError(format!("Failed to acquire write lock: {}", e)))?;

        let trend_data = engine_lock.get_category_trend(category_qid, depth, start, end);
        let top_articles = engine_lock
            .get_top_articles_in_category(category_qid, start, end, depth, 10)
            .map_err(|e| ApiError::EngineError(format!("Failed to get top articles: {}", e)))?;

        (trend_data, top_articles)
    };

    let daily_views: Vec<DailyViews> = raw_data
        .into_iter()
        .map(|(date, views)| DailyViews { date, views })
        .collect();

    // Collect all article QIDs to fetch titles
    let article_qids: Vec<u32> = category_rank
        .top_articles
        .iter()
        .map(|a| a.article_qid)
        .collect();

    // Fetch titles for all articles
    let titles_map = get_titles_by_qids(Arc::clone(&state), &params.wiki, article_qids)
        .await
        .map_err(|e| ApiError::DatabaseError(e))?;

    // Transform to TopArticle with titles
    let top_articles: Vec<TopArticle> = category_rank
        .top_articles
        .into_iter()
        .map(|art| {
            let article_title = titles_map
                .get(&art.article_qid)
                .cloned()
                .unwrap_or_else(|| format!("Q{}", art.article_qid));

            TopArticle {
                qid: art.article_qid,
                title: article_title,
                views: art.total_views as u32,
            }
        })
        .collect();

    Ok(Json(CategoryTrendResponse {
        qid: category_qid,
        title: params.category,
        views: daily_views,
        top_articles,
    }))
}

pub async fn get_article_trend_handler(
    Query(params): Query<ArticleTrendParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<ArticleTrendResponse>, ApiError> {
    let start = params
        .start_date
        .unwrap_or_else(|| chrono::Local::now().date_naive() - chrono::Duration::days(30));
    let end = params
        .end_date
        .unwrap_or_else(|| chrono::Local::now().date_naive());

    let article_qid = if let Some(qid) = params.article_qid {
        qid
    } else {
        get_qid_by_title(Arc::clone(&state), &params.wiki, &params.article, &0_i8)
            .await
            .map_err(|e| ApiError::DatabaseError(e))?
    };

    // Wrap the entire blocking operation
    let engine = get_or_build_engine(state, &params.wiki)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to build engine: {}", e)))?;

    // Acquire a write lock to access the engine mutably
    let raw_data = {
        let mut engine_lock = engine
            .write()
            .map_err(|e| ApiError::InternalError(format!("Failed to acquire write lock: {}", e)))?;
        engine_lock.get_article_trend(article_qid, start, end)
    };
    let daily_views = raw_data
        .into_iter()
        .map(|(date, views)| DailyViews { date, views })
        .collect();

    Ok(Json(ArticleTrendResponse {
        qid: article_qid,
        title: params.article,
        views: daily_views,
    }))
}

async fn get_or_build_engine(
    state: Arc<AppState>,
    wiki: &str,
) -> Result<Arc<RwLock<PageViewEngine>>, Box<dyn std::error::Error + Send + Sync>> {
    let wiki = wiki.to_string(); // Avoid cloning inside the blocking task

    tokio::task::spawn_blocking(move || {
        let mut engines = state
            .engines
            .write()
            .map_err(|_| "Failed to acquire engines lock")?;
        if let Some(engine) = engines.get(&wiki) {
            Ok(Arc::clone(engine)) // Return the existing Arc<RwLock<PageViewEngine>>
        } else {
            let new_engine = Arc::new(RwLock::new(PageViewEngine::new(&wiki)));
            engines.insert(wiki.clone(), Arc::clone(&new_engine)); // Insert the new Arc<RwLock<PageViewEngine>>
            Ok(new_engine)
        }
    })
    .await
    .map_err(|_| "Failed to spawn blocking task")?
}

pub async fn get_sub_categories(
    Query(params): Query<SubCategoryParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<HashMap<u32, String>>, ApiError> {
    let category_qid = if let Some(qid) = params.category_qid {
        qid
    } else {
        get_qid_by_title(Arc::clone(&state), &params.wiki, &params.category, &14_i8)
            .await
            .map_err(|e| ApiError::DatabaseError(e))?
    };

    let engine = get_or_build_engine(Arc::clone(&state), &params.wiki)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to build engine: {}", e)))?;

    let category_qids: Result<Vec<u32>, String> = {
        let engine_lock = engine
            .read()
            .map_err(|e| ApiError::InternalError(format!("Failed to acquire read lock: {}", e)))?;

        engine_lock
            .get_wikigraph()
            .get_child_categories(category_qid)
    };

    match category_qids {
        Ok(categories) => {
            let titles_map = get_titles_by_qids(state, &params.wiki, categories)
                .await
                .map_err(|e| ApiError::DatabaseError(e))?;
            Ok(Json(titles_map))
        }
        Err(e) => Err(ApiError::EngineError(format!(
            "Failed to get child categories: {}",
            e
        ))),
    }
}

#[debug_handler]
pub async fn get_top_categories_handler(
    Query(params): Query<TopCategoriesParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<CategoryRankResponse>, ApiError> {
    let top_n = params.top_n.unwrap_or(10);

    let start = params
        .start_date
        .unwrap_or_else(|| chrono::Local::now().date_naive() - chrono::Duration::days(30));
    let end = params
        .end_date
        .unwrap_or_else(|| chrono::Local::now().date_naive());

    let engine = get_or_build_engine(Arc::clone(&state), &params.wiki)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to build engine: {}", e)))?;

    // Do the blocking operation first
    let categories = {
        let mut engine_lock = engine
            .write()
            .map_err(|e| ApiError::InternalError(format!("Failed to acquire write lock: {}", e)))?;

        engine_lock
            .get_top_categories(start, end, top_n as usize)
            .map_err(|e| ApiError::EngineError(format!("Failed to get top categories: {}", e)))?
    };

    // Now collect QIDs and fetch titles asynchronously
    let mut all_qids = Vec::new();

    for category in &categories {
        all_qids.push(category.category_qid);
        for article in &category.top_articles {
            all_qids.push(article.article_qid);
        }
    }

    // Fetch all titles in one batch (this is async and outside the blocking section)
    let titles_map = get_titles_by_qids(Arc::clone(&state), &params.wiki, all_qids)
        .await
        .map_err(|e| ApiError::DatabaseError(e))?;

    // Transform CategoryRank to TopCategory with titles
    let top_categories_with_titles: Vec<TopCategory> = categories
        .into_iter()
        .map(|cat| {
            let category_title = titles_map
                .get(&cat.category_qid)
                .cloned()
                .unwrap_or_else(|| format!("Q{}", cat.category_qid));

            let top_articles: Vec<TopArticle> = cat
                .top_articles
                .into_iter()
                .map(|art| {
                    let article_title = titles_map
                        .get(&art.article_qid)
                        .cloned()
                        .unwrap_or_else(|| format!("Q{}", art.article_qid));

                    TopArticle {
                        qid: art.article_qid,
                        title: article_title,
                        views: art.total_views as u32,
                    }
                })
                .collect();

            TopCategory {
                qid: cat.category_qid,
                title: category_title,
                views: cat.total_views as u32,
                top_articles,
            }
        })
        .collect();

    let response = CategoryRankResponse {
        categories: top_categories_with_titles,
    };

    Ok(Json(response))
}
