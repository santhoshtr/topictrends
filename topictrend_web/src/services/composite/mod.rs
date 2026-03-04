pub mod content_gap_service;
pub mod pageedit_delta_service;
pub mod pageedits_service;
pub mod pageview_delta_service;
pub mod pageviews_service;

pub use content_gap_service::ContentGapService;
pub use pageedit_delta_service::PageEditDeltaService;
pub use pageedits_service::PageEditsService;
pub use pageview_delta_service::PageViewDeltaService;
pub use pageviews_service::PageViewsService;
pub use pageviews_service::ServiceError;

use crate::services::core::CoreServiceError;

/// Search the taxonomy for categories matching `category` and return all QIDs
/// with a score at or above `MATCH_THRESHOLD`. Returns `NotFound` if none match.
pub async fn taxonomy_search_category_qids(category: &str) -> Result<Vec<u32>, CoreServiceError> {
    const LIMIT: u64 = 1000;
    const MATCH_THRESHOLD: f32 = 0.6;

    let results = topictrend_taxonomy::search(category.to_string(), "enwiki".to_string(), LIMIT)
        .await
        .map_err(|e| CoreServiceError::InternalError(format!("Taxonomy search failed: {}", e)))?;

    let qids: Vec<u32> = results
        .into_iter()
        .filter(|r| 1.0 - r.score >= MATCH_THRESHOLD)
        .map(|r| r.qid)
        .collect();

    if qids.is_empty() {
        return Err(CoreServiceError::NotFound);
    }

    Ok(qids)
}
