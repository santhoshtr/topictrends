use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::Instant,
};
use topictrend::pageview_engine::PageViewEngine;

use crate::models::{AppState, ArticleTrendParams, CategoryTrendParams, SubCategoryParams};
use crate::{
    models::TrendResponse,
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
) -> Result<Json<Vec<TrendResponse>>, ApiError> {
    let depth = params.depth.unwrap_or(0);
    let start = params
        .start_date
        .unwrap_or_else(|| chrono::Local::now().date_naive() - chrono::Duration::days(30));
    let end = params
        .end_date
        .unwrap_or_else(|| chrono::Local::now().date_naive());

    let category_qid = get_qid_by_title(Arc::clone(&state), &params.wiki, &params.category, &14_i8)
        .await
        .map_err(|e| ApiError::DatabaseError(e))?;

    // Wrap the entire blocking operation
    let now = Instant::now();
    let engine = get_or_build_engine(state, &params.wiki)
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to build engine: {}", e)))?;

    println!("Engine build completed in {:.2?}s", now.elapsed());

    // Acquire a write lock to access the engine mutably
    let raw_data = {
        let mut engine_lock = engine
            .write()
            .map_err(|e| ApiError::InternalError(format!("Failed to acquire write lock: {}", e)))?;
        engine_lock.get_category_trend(category_qid, depth, start, end)
    };
    let response = raw_data
        .into_iter()
        .map(|(date, views)| TrendResponse { date, views })
        .collect();

    Ok(Json(response))
}

pub async fn get_article_trend_handler(
    Query(params): Query<ArticleTrendParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<TrendResponse>>, ApiError> {
    let start = params
        .start_date
        .unwrap_or_else(|| chrono::Local::now().date_naive() - chrono::Duration::days(30));
    let end = params
        .end_date
        .unwrap_or_else(|| chrono::Local::now().date_naive());

    let article_qid = get_qid_by_title(Arc::clone(&state), &params.wiki, &params.article, &0_i8)
        .await
        .map_err(|e| ApiError::DatabaseError(e))?;

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
    let response = raw_data
        .into_iter()
        .map(|(date, views)| TrendResponse { date, views })
        .collect();

    Ok(Json(response))
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
    let category_qid = get_qid_by_title(Arc::clone(&state), &params.wiki, &params.category, &14_i8)
        .await
        .map_err(|e| ApiError::DatabaseError(e))?;

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
