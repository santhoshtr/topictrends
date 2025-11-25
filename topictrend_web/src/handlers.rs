use axum::{
    extract::{Path, Query, State},
    Json,
};
use topictrend::pageview_engine::PageViewEngine;
use std::sync::Arc;


use crate::models::TrendParams;
use crate::models::TrendResponse;
use crate::models::AppState;

// Route: GET /api/trend/:wiki/:category_id
pub async fn get_category_trend_handler(
    Path((wiki, category_id)): Path<(String, u32)>,
    Query(params): Query<TrendParams>,
    State(state): State<Arc<AppState>>,
) -> Json<Vec<TrendResponse>> {
    let depth = params.depth.unwrap_or(0);
    let start = params.start_date.unwrap_or_else(|| /* 30 days ago */ 
        chrono::Local::now().date_naive() - chrono::Duration::days(30)
    );
    let end = params.end_date.unwrap_or_else(|| 
        chrono::Local::now().date_naive()
    );
    let engine: PageViewEngine = state.engines.get(&wiki).unwrap_or_else(||
        &PageViewEngine::new(wiki.as_str())
    );

    // We pass a reference (&) to the mask. 
    // The engine uses it to filter the huge daily vectors.
    let raw_data = engine.get_category_trend(category_id,depth, start, end);

    let response = raw_data
        .into_iter()
        .map(|(date, views)| TrendResponse { date, views })
        .collect();

    Json(response)
}
