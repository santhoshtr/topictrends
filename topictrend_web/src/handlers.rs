use axum::{
    Json,
    extract::{Query, State},
};
use std::sync::Arc;
use topictrend::pageview_engine::PageViewEngine;

use crate::models::CategoryTrendParams;
use crate::models::TrendResponse;
use crate::models::{AppState, ArticleTrendParams};

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
    let mut engine = get_or_build_engine(state, &params.wiki).await;

    let raw_data = engine.get_category_trend(&params.category, depth, start, end);

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
    let depth = params.depth.unwrap_or(0);
    let start = params
        .start_date
        .unwrap_or_else(|| chrono::Local::now().date_naive() - chrono::Duration::days(30));
    let end = params
        .end_date
        .unwrap_or_else(|| chrono::Local::now().date_naive());

    // Wrap the entire blocking operation
    let mut engine = get_or_build_engine(state, &params.wiki).await;

    let raw_data = engine.get_article_trend(&params.article, depth, start, end);

    let response = raw_data
        .into_iter()
        .map(|(date, views)| TrendResponse { date, views })
        .collect();

    Json(response)
}

async fn get_or_build_engine(state: Arc<AppState>, wiki: &str) -> PageViewEngine {
    let state_clone = state.clone();
    let wiki_clone = wiki.to_string();

    tokio::task::spawn_blocking(move || {
        let mut engines = state_clone.engines.write().unwrap();
        engines
            .entry(wiki_clone.clone())
            .or_insert_with(|| PageViewEngine::new(wiki_clone.as_str()))
            .clone()
    })
    .await
    .expect("Failed to spawn blocking task")
}
