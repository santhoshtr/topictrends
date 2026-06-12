use axum::{
    Json,
    extract::{Query, State},
};
use std::sync::Arc;

use crate::models::{
    AppState, GapDiscoveryItemResponse, GapDiscoveryParams, GapDiscoveryResponse,
};
use crate::services::GapDiscoveryService;
use crate::services::core::QidService;

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

    // Display titles come from the reference wiki, so the English hover is
    // relative to the reference too.
    let en_qids: Vec<u32> = outcome.rows.iter().map(|r| r.category_qid).collect();
    let en =
        QidService::get_english_titles(Arc::clone(&state), &params.reference, &en_qids).await;

    let categories = outcome
        .rows
        .into_iter()
        .map(|r| GapDiscoveryItemResponse {
            category_qid: r.category_qid,
            category_title: r.category_title,
            category_title_en: en.get(&r.category_qid).cloned(),
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
