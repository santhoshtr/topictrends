use std::sync::Arc;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::{ErrorData, tool};

use crate::mcp::TopicTrendMcpServer;
use crate::mcp::tools::{GapDiscoveryInput, core_err};
use crate::models::{GapDiscoveryItemResponse, GapDiscoveryResponse};
use crate::services::GapDiscoveryService;
use crate::services::core::QidService;

const DEFAULT_LIMIT: usize = 50;
const MAX_LIMIT: usize = 200;

impl TopicTrendMcpServer {
    /// Discover and rank the categories where a target Wikipedia most lags a
    /// reference edition — a "what to work on next" worklist, not a single-category
    /// check. Ranked by missing-article count, or (with `weight`) by estimated
    /// missing readership so popular topics outrank bot stub farms.
    #[tool(
        name = "topictrends_discover_content_gaps",
        description = "Rank the categories where a target Wikipedia lags a reference edition, by missing-article count or (weight=true) estimated missing readership. Use has_category to split structure gaps (category absent) from content gaps (present but under-populated).",
        annotations(read_only_hint = true, destructive_hint = false, idempotent_hint = true, open_world_hint = true)
    )]
    pub async fn discover_content_gaps(
        &self,
        Parameters(p): Parameters<GapDiscoveryInput>,
    ) -> Result<rmcp::handler::server::wrapper::Json<GapDiscoveryResponse>, ErrorData> {
        let offset = p.offset.unwrap_or(0) as usize;
        let limit = (p.limit.unwrap_or(DEFAULT_LIMIT as u32) as usize).min(MAX_LIMIT);
        let weighted = p.weight.unwrap_or(false);

        let outcome = GapDiscoveryService::discover(
            Arc::clone(&self.state),
            &p.reference,
            &p.target,
            p.min_ref,
            p.has_category,
            weighted,
            offset,
            limit,
        )
        .await
        .map_err(core_err)?;

        // Titles come from the reference wiki (the target usually lacks the
        // category), so the English hover is relative to the reference too.
        let en_qids: Vec<u32> = outcome.rows.iter().map(|r| r.category_qid).collect();
        let en =
            QidService::get_english_titles(Arc::clone(&self.state), &p.reference, &en_qids).await;

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
                overlap_pageviews: r.overlap_pageviews,
                weighted_score: r.weighted_score,
            })
            .collect();

        Ok(rmcp::handler::server::wrapper::Json(GapDiscoveryResponse {
            reference: p.reference,
            target: p.target,
            reference_date: outcome.reference_date.to_string(),
            target_date: outcome.target_date.to_string(),
            total: outcome.total,
            with_category: outcome.with_category,
            without_category: outcome.without_category,
            offset,
            limit,
            weighted,
            weighted_applied: outcome.weighted_applied,
            categories,
        }))
    }
}
