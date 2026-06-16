#![allow(dead_code, unused_imports)]

use chrono::NaiveDate;
use rmcp::ErrorData;
use schemars::JsonSchema;
use serde::Deserialize;

pub mod delta;
pub mod gap_discovery;
pub mod googlesearch;
pub mod lists;
pub mod pageedits;
pub mod pageviews;
pub mod search;

// ---------------------------------------------------------------------------
// Shared input structs
// ---------------------------------------------------------------------------

/// Input for category-level trend queries.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CategoryTrendInput {
    /// Wikipedia edition database name (e.g. "enwiki", "frwiki").
    pub wiki: String,
    /// Category title without the "Category:" prefix (e.g. "Physics").
    pub category: String,
    /// Wikidata QID as a plain integer (e.g. 42 for Q42). Optional; resolved from title if omitted.
    pub category_qid: Option<u32>,
    /// Start date inclusive, YYYY-MM-DD. Defaults to 30 days ago.
    pub start_date: Option<String>,
    /// End date inclusive, YYYY-MM-DD. Defaults to today.
    pub end_date: Option<String>,
}

/// Input for article-level trend queries.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ArticleTrendInput {
    /// Wikipedia edition database name (e.g. "enwiki").
    pub wiki: String,
    /// Article title as it appears in the wiki.
    pub article: String,
    /// Wikidata QID as a plain integer. Optional.
    pub article_qid: Option<u32>,
    /// Start date inclusive, YYYY-MM-DD. Defaults to 30 days ago.
    pub start_date: Option<String>,
    /// End date inclusive, YYYY-MM-DD. Defaults to today.
    pub end_date: Option<String>,
}

/// Input for top-N category/article ranking queries.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct TopNInput {
    /// Wikipedia edition database name (e.g. "enwiki").
    pub wiki: String,
    /// Start date inclusive, YYYY-MM-DD. Defaults to 30 days ago.
    pub start_date: Option<String>,
    /// End date inclusive, YYYY-MM-DD. Defaults to today.
    pub end_date: Option<String>,
    /// Number of top results to return.
    pub top_n: Option<u32>,
}

/// Input for category-level delta queries between two time periods.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CategoryDeltaInput {
    /// Wikipedia edition database name (e.g. "enwiki").
    pub wiki: String,
    /// Baseline period start date inclusive, YYYY-MM-DD.
    pub baseline_start_date: String,
    /// Baseline period end date inclusive, YYYY-MM-DD.
    pub baseline_end_date: String,
    /// Impact period start date inclusive, YYYY-MM-DD.
    pub impact_start_date: String,
    /// Impact period end date inclusive, YYYY-MM-DD.
    pub impact_end_date: String,
    /// Maximum number of categories to return.
    pub limit: Option<u32>,
}

/// Input for article-level delta queries within a specific category.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ArticleDeltaInput {
    /// Wikipedia edition database name (e.g. "enwiki").
    pub wiki: String,
    /// Wikidata QID of the category (required).
    pub category_qid: u32,
    /// Baseline period start date inclusive, YYYY-MM-DD.
    pub baseline_start_date: String,
    /// Baseline period end date inclusive, YYYY-MM-DD.
    pub baseline_end_date: String,
    /// Impact period start date inclusive, YYYY-MM-DD.
    pub impact_start_date: String,
    /// Impact period end date inclusive, YYYY-MM-DD.
    pub impact_end_date: String,
    /// Maximum number of articles to return.
    pub limit: Option<u32>,
}

/// Input for semantic category search.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CategorySearchInput {
    /// Wikipedia edition database name (e.g. "enwiki").
    pub wiki: String,
    /// Search query string for embedding-based category search.
    pub query: String,
    /// Minimum similarity score threshold (0.0–1.0). Defaults to 0.6.
    pub match_threshold: Option<f32>,
    /// Maximum number of candidates to consider.
    pub limit: Option<u64>,
}

/// Input for subcategory listing.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SubCategoriesInput {
    /// Wikipedia edition database name (e.g. "enwiki").
    pub wiki: String,
    /// Category title without the "Category:" prefix.
    pub category: String,
    /// Wikidata QID as a plain integer. Optional.
    pub category_qid: Option<u32>,
}

/// Input for listing articles in a category.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListArticlesInput {
    /// Wikipedia edition database name (e.g. "enwiki").
    pub wiki: String,
    /// Category title without the "Category:" prefix. At least one of category or category_qid required.
    pub category: Option<String>,
    /// Wikidata QID as a plain integer. At least one of category or category_qid required.
    pub category_qid: Option<u32>,
    /// Keep only members at least this many wikis agree on (default 1).
    pub min_agreement: Option<u16>,
}


/// Input for listing categories an article belongs to.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListArticleCategoriesInput {
    /// Wikipedia edition database name (e.g. "enwiki").
    pub wiki: String,
    /// Article title. At least one of article or article_qid required.
    pub article: Option<String>,
    /// Wikidata QID as a plain integer. At least one of article or article_qid required.
    pub article_qid: Option<u32>,
}

/// Input for cross-wiki content-gap discovery (ranked categories).
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GapDiscoveryInput {
    /// Reference (baseline) wiki whose coverage sets the bar, e.g. "enwiki". Acts
    /// as the editorial lens — a sister-language wiki surfaces region-relevant gaps.
    pub reference: String,
    /// Target wiki to improve. Gap = reference overlap − target overlap, e.g. "hiwiki".
    pub target: String,
    /// Drop categories with fewer than this many reference-side overlap articles
    /// (a noise floor). Optional.
    pub min_ref: Option<u32>,
    /// Structure filter: false = category absent in target, true = present but
    /// under-populated, omit = all gaps.
    pub has_category: Option<bool>,
    /// Rank by estimated missing readership (pageview-weighted) instead of raw
    /// missing-article count. Defaults to false. Ignored with a warning flag in the
    /// response if the reference snapshot predates pageview data.
    pub weight: Option<bool>,
    /// Skip the top N ranked rows (pagination). Defaults to 0.
    pub offset: Option<u32>,
    /// Maximum rows to return (default 50, capped at 200).
    pub limit: Option<u32>,
}

// ---------------------------------------------------------------------------
// Date parsing
// ---------------------------------------------------------------------------

pub fn parse_date(s: &str) -> Result<NaiveDate, ErrorData> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| {
        ErrorData::invalid_params(
            format!("Invalid date '{}'. Expected YYYY-MM-DD.", s),
            None,
        )
    })
}

pub fn parse_date_opt(s: Option<String>) -> Result<Option<NaiveDate>, ErrorData> {
    s.as_deref().map(parse_date).transpose()
}

// ---------------------------------------------------------------------------
// Error conversion
// ---------------------------------------------------------------------------

pub fn service_err(e: crate::services::ServiceError) -> ErrorData {
    use crate::services::core::CoreServiceError;
    let msg = match e {
        crate::services::ServiceError::CoreError(CoreServiceError::NotFound) => {
            "Resource not found".to_string()
        }
        crate::services::ServiceError::CoreError(CoreServiceError::EngineError(m)) => {
            format!("Engine error: {}", m)
        }
        crate::services::ServiceError::CoreError(CoreServiceError::InternalError(m)) => {
            format!("Internal error: {}", m)
        }
    };
    ErrorData::internal_error(msg, None)
}

pub fn core_err(e: crate::services::core::CoreServiceError) -> ErrorData {
    service_err(crate::services::ServiceError::CoreError(e))
}
