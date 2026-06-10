use axum::{
    Json,
    extract::{Query, State},
};
use std::sync::Arc;

use crate::models::{
    AppState, GapDiscoveryItemResponse, GapDiscoveryParams, GapDiscoveryResponse,
};
use crate::services::GapDiscoveryService;

use super::ApiError;

const DEFAULT_LIMIT: usize = 50;
const MAX_LIMIT: usize = 200;

pub async fn get_gap_discovery_handler(
    Query(params): Query<GapDiscoveryParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<GapDiscoveryResponse>, ApiError> {
    let offset = params.offset.unwrap_or(0);
    let limit = params.limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT);

    let outcome = GapDiscoveryService::discover(
        Arc::clone(&state),
        &params.reference,
        &params.target,
        params.min_ref,
        params.has_category,
        offset,
        limit,
    )
    .await?;

    let categories = outcome
        .rows
        .into_iter()
        .map(|r| GapDiscoveryItemResponse {
            category_qid: r.category_qid,
            category_title: r.category_title,
            direct_coverage_target: r.direct_coverage_target,
            overlap_target: r.overlap_target,
            overlap_reference: r.overlap_reference,
            gap: r.gap,
            coverage_pct: r.coverage_pct,
            has_category: r.has_category,
        })
        .collect();

    Ok(Json(GapDiscoveryResponse {
        reference: params.reference,
        target: params.target,
        reference_date: outcome.reference_date.to_string(),
        target_date: outcome.target_date.to_string(),
        total: outcome.total,
        with_category: outcome.with_category,
        without_category: outcome.without_category,
        offset,
        limit,
        categories,
    }))
}
