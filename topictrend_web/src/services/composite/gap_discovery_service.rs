//! Orchestrates cross-wiki gap discovery: fetch the cached ranking for a
//! `(reference, target)` pair, slice the requested window, and resolve the
//! window's category titles from the REFERENCE wiki (the target usually lacks
//! the category, so resolving against it would fail).

use crate::models::AppState;
use crate::services::core::{CoreServiceError, CoverageService, QidService};
use chrono::NaiveDate;
use std::sync::Arc;

pub struct GapDiscoveryService;

pub struct GapDiscoveryRow {
    pub category_qid: u32,
    pub category_title: String,
    pub direct_coverage_target: u32,
    pub overlap_target: u32,
    pub overlap_reference: u32,
    pub gap: i64,
    pub coverage_pct: f64,
    pub has_category: bool,
}

pub struct GapDiscoveryOutcome {
    pub rows: Vec<GapDiscoveryRow>,
    pub total: usize,
    pub with_category: usize,
    pub without_category: usize,
    pub reference_date: NaiveDate,
    pub target_date: NaiveDate,
}

impl GapDiscoveryService {
    #[allow(clippy::too_many_arguments)]
    pub async fn discover(
        state: Arc<AppState>,
        reference: &str,
        target: &str,
        min_ref: Option<u32>,
        max_ref: Option<u32>,
        has_category: Option<bool>,
        offset: usize,
        limit: usize,
    ) -> Result<GapDiscoveryOutcome, CoreServiceError> {
        if reference == target {
            return Err(CoreServiceError::InternalError(
                "reference and target wikis must differ".to_string(),
            ));
        }

        let ranking =
            CoverageService::get_or_build_ranking(Arc::clone(&state), reference, target).await?;
        let window = ranking.window(min_ref, max_ref, has_category, offset, limit);

        // Resolve titles for just this page, from the reference wiki. Degrade to
        // "Q{qid}" if the replica is unavailable rather than failing the request.
        let qids: Vec<u32> = window.rows.iter().map(|r| r.category_qid).collect();
        let titles = QidService::get_titles_by_qids(Arc::clone(&state), reference, &qids)
            .await
            .unwrap_or_default();

        let rows = window
            .rows
            .into_iter()
            .map(|r| {
                let category_title = titles
                    .get(&r.category_qid)
                    .cloned()
                    .unwrap_or_else(|| format!("Q{}", r.category_qid));
                let coverage_pct = if r.overlap_reference > 0 {
                    r.overlap_target as f64 / r.overlap_reference as f64
                } else {
                    0.0
                };
                GapDiscoveryRow {
                    category_qid: r.category_qid,
                    category_title,
                    direct_coverage_target: r.direct_target,
                    overlap_target: r.overlap_target,
                    overlap_reference: r.overlap_reference,
                    gap: r.gap,
                    coverage_pct,
                    has_category: r.has_category,
                }
            })
            .collect();

        Ok(GapDiscoveryOutcome {
            rows,
            total: window.filtered_total,
            with_category: window.with_category,
            without_category: window.without_category,
            reference_date: ranking.reference_date,
            target_date: ranking.target_date,
        })
    }
}
