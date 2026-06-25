use axum::{Json, extract::State};
use std::sync::Arc;

use crate::models::{AppState, ClusterArticlesRequest, ClusterArticlesResponse};
use crate::services::core::ClusterService;

use super::ApiError;

/// Group a POSTed set of article titles into category-topics, using the same
/// reverse-scatter + greedy-coverage clustering as the trending top-categories
/// surface. Each article is assigned to its single best topic.
pub async fn cluster_articles_handler(
    State(state): State<Arc<AppState>>,
    Json(body): Json<ClusterArticlesRequest>,
) -> Result<Json<ClusterArticlesResponse>, ApiError> {
    let response = ClusterService::cluster(
        state,
        &body.wiki,
        body.articles,
        body.max_clusters,
        body.min_agreement.unwrap_or(3),
    )
    .await?;
    Ok(Json(response))
}
