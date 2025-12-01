use axum::{
    Json,
    extract::{Query, State},
};
use std::{
    sync::{Arc, RwLock},
    time::Instant,
};
use topictrend::pageview_engine::PageViewEngine;

use crate::models::TrendResponse;
use crate::models::{AppState, ArticleTrendParams, CategoryTrendParams, SubCategoryParams};

pub async fn get_category_trend_handler(
    Query(params): Query<CategoryTrendParams>,
    State(state): State<Arc<AppState>>,
) -> Json<Vec<TrendResponse>> {
    let depth = params.depth.unwrap_or(0);
    let start = params
        .start_date
        .unwrap_or_else(|| chrono::Local::now().date_naive() - chrono::Duration::days(30));
    let end = params
        .end_date
        .unwrap_or_else(|| chrono::Local::now().date_naive());

    // Wrap the entire blocking operation
    let now = Instant::now();
    let engine = get_or_build_engine(state, &params.wiki).await;

    println!("Engine build completed in {:.2?}s", now.elapsed());

    // Acquire a write lock to access the engine mutably
    let raw_data = {
        let mut engine_lock = engine.write().unwrap();
        engine_lock.get_category_trend(&params.category, depth, start, end)
    };
    let response = raw_data
        .into_iter()
        .map(|(date, views)| TrendResponse { date, views })
        .collect();

    Json(response)
}

pub async fn get_article_trend_handler(
    Query(params): Query<ArticleTrendParams>,
    State(state): State<Arc<AppState>>,
) -> Json<Vec<TrendResponse>> {
    let start = params
        .start_date
        .unwrap_or_else(|| chrono::Local::now().date_naive() - chrono::Duration::days(30));
    let end = params
        .end_date
        .unwrap_or_else(|| chrono::Local::now().date_naive());

    // Wrap the entire blocking operation

    let engine = get_or_build_engine(state, &params.wiki).await;

    // Acquire a write lock to access the engine mutably
    let raw_data = {
        let mut engine_lock = engine.write().unwrap();
        engine_lock.get_article_trend(&params.article, start, end)
    };
    let response = raw_data
        .into_iter()
        .map(|(date, views)| TrendResponse { date, views })
        .collect();

    Json(response)
}

async fn get_or_build_engine(state: Arc<AppState>, wiki: &str) -> Arc<RwLock<PageViewEngine>> {
    let wiki = wiki.to_string(); // Avoid cloning inside the blocking task

    tokio::task::spawn_blocking(move || {
        let mut engines = state.engines.write().unwrap();
        if let Some(engine) = engines.get(&wiki) {
            Arc::clone(engine) // Return the existing Arc<RwLock<PageViewEngine>>
        } else {
            let new_engine = Arc::new(RwLock::new(PageViewEngine::new(&wiki)));
            engines.insert(wiki.clone(), Arc::clone(&new_engine)); // Insert the new Arc<RwLock<PageViewEngine>>
            new_engine
        }
    })
    .await
    .expect("Failed to spawn blocking task")
}

pub async fn get_sub_categories(
    Query(params): Query<SubCategoryParams>,
    State(state): State<Arc<AppState>>,
) -> Json<Vec<String>> {
    let category_title = params.category;
    let wiki = params.wiki;

    let engine = get_or_build_engine(state, &wiki).await;

    let results: Result<Vec<(u32, String)>, String> = {
        let engine_lock = engine.write().unwrap();

        engine_lock
            .get_wikigraph()
            .get_child_categories(&category_title)
    };

    let string_results: Vec<String> = match results {
        Ok(categories) => categories.into_iter().map(|(_, name)| name).collect(),
        Err(_) => Vec::new(),
    };

    Json(string_results)
}
