use axum::{
    Json,
    extract::{Query, State},
};
use std::{
    sync::{Arc, RwLock},
    time::Instant,
};
use topictrend::pageview_engine::PageViewEngine;

use crate::models::{AppState, ArticleTrendParams, CategoryTrendParams, SubCategoryParams};
use crate::{models::TrendResponse, wiki::get_qid_by_title};

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

    let category_qid =
        match get_qid_by_title(Arc::clone(&state), &params.wiki, &params.category, &14_i8).await {
            Ok(qid) => qid,
            Err(_) => return Json(vec![]),
        };

    // Wrap the entire blocking operation
    let now = Instant::now();
    let engine = get_or_build_engine(state, &params.wiki).await;

    println!("Engine build completed in {:.2?}s", now.elapsed());

    // Acquire a write lock to access the engine mutably
    let raw_data = {
        let mut engine_lock = engine.write().unwrap();
        engine_lock.get_category_trend(category_qid, depth, start, end)
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

    let article_qid =
        match get_qid_by_title(Arc::clone(&state), &params.wiki, &params.article, &0_i8).await {
            Ok(id) => id,
            Err(_) => return Json(vec![]),
        };

    // Wrap the entire blocking operation

    let engine = get_or_build_engine(state, &params.wiki).await;

    // Acquire a write lock to access the engine mutably
    let raw_data = {
        let mut engine_lock = engine.write().unwrap();
        engine_lock.get_article_trend(article_qid, start, end)
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
) -> Json<Vec<u32>> {
    let category_qid =
        match get_qid_by_title(Arc::clone(&state), &params.wiki, &params.category, &14_i8).await {
            Ok(id) => id,
            Err(_) => return Json(vec![]),
        };

    let engine = get_or_build_engine(state, &params.wiki).await;

    let results: Result<Vec<u32>, String> = {
        let engine_lock = engine.write().unwrap();

        engine_lock
            .get_wikigraph()
            .get_child_categories(category_qid)
    };

    match results {
        Ok(categories) => Json(categories),
        Err(_) => Json(Vec::new()),
    }
}
