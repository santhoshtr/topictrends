use axum::{
    Json,
    extract::{Path, Query, State},
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

    // Clone what you need for the blocking task
    let state_clone = state.clone();
    let wiki_clone = params.wiki.clone();

    // Wrap the entire blocking operation
    let raw_data = tokio::task::spawn_blocking(move || {
        let mut engine = {
            let mut engines = state_clone.engines.write().unwrap();
            engines
                .entry(wiki_clone.clone())
                .or_insert_with(|| PageViewEngine::new(wiki_clone.as_str()))
                .clone()
        };

        engine.get_category_trend(&params.category, depth, start, end)
    })
    .await
    .unwrap_or_else(|err| {
        eprintln!("Error: Failed to execute blocking task: {}", err);
        vec![] // Return an empty vector in case of failure
    });

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

    // Clone what you need for the blocking task
    let state_clone = state.clone();
    let wiki_clone = params.wiki.clone();

    // Wrap the entire blocking operation
    let raw_data = tokio::task::spawn_blocking(move || {
        let mut engine = {
            let mut engines = state_clone.engines.write().unwrap();
            engines
                .entry(wiki_clone.clone())
                .or_insert_with(|| PageViewEngine::new(wiki_clone.as_str()))
                .clone()
        };

        engine.get_article_trend(&params.article, depth, start, end)
    })
    .await
    .unwrap(); // Handle JoinError properly in production

    let response = raw_data
        .into_iter()
        .map(|(date, views)| TrendResponse { date, views })
        .collect();

    Json(response)
}
