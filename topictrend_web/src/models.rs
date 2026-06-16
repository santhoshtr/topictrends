use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use chrono::NaiveDate;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;
use crate::services::core::coverage_service::{
    BoundedCache, CoverageSnapshot, GapRanking, coverage_cache_capacity, ranking_cache_capacity,
};
use crate::services::core::title_store::{CategoryLabelTable, WikiTitleStore, title_cache_capacity};
use topictrend::google_search_engine::GoogleSearchEngine;
use topictrend::pageedits_engine::PageEditsEngine;
use topictrend::pageview_engine::PageViewEngine;
use topictrend::wikigraph::WikiGraph;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetricType {
    PageView,
    PageEdit,
    GoogleSearch,
    Graph,
}

pub enum MetricEngine {
    PageView(Arc<RwLock<PageViewEngine>>),
    PageEdit(Arc<RwLock<PageEditsEngine>>),
    GoogleSearch(Arc<RwLock<GoogleSearchEngine>>),
    // `Arc<WikiGraph>` (no RwLock): the graph is immutable once built and
    // is shared by all metric engines for the same wiki.
    Graph(Arc<WikiGraph>),
}

impl MetricEngine {
    pub fn as_pageview(&self) -> Option<&Arc<RwLock<PageViewEngine>>> {
        match self {
            MetricEngine::PageView(engine) => Some(engine),
            MetricEngine::PageEdit(_) | MetricEngine::GoogleSearch(_) | MetricEngine::Graph(_) => {
                None
            }
        }
    }

    pub fn as_pageedit(&self) -> Option<&Arc<RwLock<PageEditsEngine>>> {
        match self {
            MetricEngine::PageEdit(engine) => Some(engine),
            MetricEngine::PageView(_) | MetricEngine::GoogleSearch(_) | MetricEngine::Graph(_) => {
                None
            }
        }
    }

    pub fn as_google_search(&self) -> Option<&Arc<RwLock<GoogleSearchEngine>>> {
        match self {
            MetricEngine::GoogleSearch(engine) => Some(engine),
            MetricEngine::PageView(_) | MetricEngine::PageEdit(_) | MetricEngine::Graph(_) => None,
        }
    }

    pub fn as_graph(&self) -> Option<&Arc<WikiGraph>> {
        match self {
            MetricEngine::Graph(engine) => Some(engine),
            MetricEngine::PageView(_)
            | MetricEngine::PageEdit(_)
            | MetricEngine::GoogleSearch(_) => None,
        }
    }
}

pub struct AppState {
    pub engines: Arc<RwLock<HashMap<(String, MetricType), MetricEngine>>>,
    // Title↔QID resolution: per-wiki parquet title maps (lazily loaded,
    // FIFO-bounded) plus the once-loaded global category-label fallback
    // (None = no canonical snapshot with labels on disk).
    pub title_stores: Arc<RwLock<BoundedCache<String, WikiTitleStore>>>,
    pub category_labels: Arc<OnceLock<Option<Arc<CategoryLabelTable>>>>,
    // Gap-discovery: per-wiki coverage snapshots and per-(reference,target)
    // sorted rankings, both lazily loaded and FIFO-bounded.
    pub coverage_snapshots: Arc<RwLock<BoundedCache<String, CoverageSnapshot>>>,
    pub gap_rankings: Arc<RwLock<BoundedCache<(String, String), GapRanking>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            engines: Arc::new(RwLock::new(HashMap::new())),
            title_stores: Arc::new(RwLock::new(BoundedCache::new(title_cache_capacity()))),
            category_labels: Arc::new(OnceLock::new()),
            coverage_snapshots: Arc::new(RwLock::new(BoundedCache::new(coverage_cache_capacity()))),
            gap_rankings: Arc::new(RwLock::new(BoundedCache::new(ranking_cache_capacity()))),
        }
    }
}

// --- Request DTO ---
#[derive(Deserialize)]
pub struct CategoryTrendParams {
    pub wiki: String,
    pub category: String,
    pub category_qid: Option<u32>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
}

#[derive(Deserialize)]
pub struct CategoriesTrendParams {
    pub wiki: String,
    pub category_query: String,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub match_threshold: Option<f32>,
    pub limit: Option<u64>,
}

#[derive(Deserialize)]
pub struct ArticleTrendParams {
    pub wiki: String,
    pub article: String,
    pub article_qid: Option<u32>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
}

#[derive(Deserialize)]
pub struct TopicTrendParams {
    pub wiki: String,
    pub topic: String,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
}

#[derive(Deserialize)]
pub struct SubCategoryParams {
    pub wiki: String,
    pub category: String,
    pub category_qid: Option<u32>,
}

#[derive(Deserialize)]
pub struct TopCategoriesParams {
    pub wiki: String,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub top_n: Option<u32>,
}

#[derive(Deserialize)]
pub struct PageViewCategoryDeltaParams {
    pub wiki: String,
    pub baseline_start_date: NaiveDate,
    pub baseline_end_date: NaiveDate,
    pub impact_start_date: NaiveDate,
    pub impact_end_date: NaiveDate,
    pub limit: Option<u32>,
}

#[derive(Deserialize)]
pub struct PageViewArticleDeltaParams {
    pub wiki: String,
    pub category_qid: u32,
    pub baseline_start_date: NaiveDate,
    pub baseline_end_date: NaiveDate,
    pub impact_start_date: NaiveDate,
    pub impact_end_date: NaiveDate,
    pub limit: Option<u32>,
}

#[derive(Deserialize)]
pub struct PageEditCategoryDeltaParams {
    pub wiki: String,
    pub baseline_start_date: NaiveDate,
    pub baseline_end_date: NaiveDate,
    pub impact_start_date: NaiveDate,
    pub impact_end_date: NaiveDate,
    pub limit: Option<u32>,
}

#[derive(Deserialize)]
pub struct PageEditArticleDeltaParams {
    pub wiki: String,
    pub category_qid: u32,
    pub baseline_start_date: NaiveDate,
    pub baseline_end_date: NaiveDate,
    pub impact_start_date: NaiveDate,
    pub impact_end_date: NaiveDate,
    pub limit: Option<u32>,
}

#[derive(Deserialize)]
pub struct GoogleSearchCategoryDeltaParams {
    pub wiki: String,
    pub baseline_start_date: NaiveDate,
    pub baseline_end_date: NaiveDate,
    pub impact_start_date: NaiveDate,
    pub impact_end_date: NaiveDate,
    pub limit: Option<u32>,
}

#[derive(Deserialize)]
pub struct GoogleSearchArticleDeltaParams {
    pub wiki: String,
    pub category_qid: u32,
    pub baseline_start_date: NaiveDate,
    pub baseline_end_date: NaiveDate,
    pub impact_start_date: NaiveDate,
    pub impact_end_date: NaiveDate,
    pub limit: Option<u32>,
}

// --- Response DTO ---
#[derive(Serialize, JsonSchema)]
pub struct DailyViews {
    pub date: NaiveDate,
    pub views: u64,
}

#[derive(Serialize, JsonSchema)]
pub struct DailyEdits {
    pub date: NaiveDate,
    pub edits: u64,
}

#[derive(Serialize, JsonSchema)]
pub struct DailyGoogleSearch {
    pub date: NaiveDate,
    pub clicks: u64,
    pub impressions: u64,
    pub ctr: f64,
    pub position: f64,
}

#[derive(Serialize, JsonSchema)]
pub struct ArticleTrendResponse {
    pub qid: u32,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_en: Option<String>,
    pub views: Vec<DailyViews>,
}

#[derive(Serialize, JsonSchema)]
pub struct CategoryTrendResponse {
    pub qid: u32,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_en: Option<String>,
    pub views: Vec<DailyViews>,
    pub top_articles: Vec<TopArticle>,
}

#[derive(Serialize, JsonSchema)]
pub struct CategoriesTrendResponse {
    pub categories: Vec<CategoryInfo>,
    pub cumulative_views: Vec<DailyViews>,
    pub top_articles: Vec<TopArticle>,
}

#[derive(Serialize, JsonSchema)]
pub struct ArticleEditTrendResponse {
    pub qid: u32,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_en: Option<String>,
    pub edits: Vec<DailyEdits>,
}

#[derive(Serialize, JsonSchema)]
pub struct CategoryEditTrendResponse {
    pub qid: u32,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_en: Option<String>,
    pub edits: Vec<DailyEdits>,
    pub top_articles: Vec<TopArticleEdits>,
}

#[derive(Serialize, JsonSchema)]
pub struct CategoryInfo {
    pub qid: u32,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_en: Option<String>,
}

#[derive(Serialize, JsonSchema)]
pub struct TopArticle {
    pub qid: u32,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_en: Option<String>,
    pub views: u32,
    pub source_categories: Vec<TopArticleCategory>,
}

#[derive(Serialize, JsonSchema)]
pub struct TopArticleEdits {
    pub qid: u32,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_en: Option<String>,
    pub edits: u64,
    pub source_categories: Vec<TopArticleCategory>,
}

#[derive(Serialize, JsonSchema)]
pub struct TopCategory {
    pub qid: u32,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_en: Option<String>,
    pub views: u32,
    pub top_articles: Vec<TopArticle>,
}

#[derive(Serialize, JsonSchema)]
pub struct CategoryRankResponse {
    pub categories: Vec<TopCategory>,
}

#[derive(Serialize, JsonSchema)]
pub struct TopArticleCategory {
    pub qid: u32,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_en: Option<String>,
}

#[derive(Serialize, JsonSchema)]
pub struct PageViewTopArticle {
    pub qid: u32,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_en: Option<String>,
    pub views: u32,
    pub categories: Vec<TopArticleCategory>,
}

#[derive(Serialize, JsonSchema)]
pub struct PageViewTopArticlesResponse {
    pub articles: Vec<PageViewTopArticle>,
}

#[derive(Serialize, JsonSchema)]
pub struct PageEditTopArticle {
    pub qid: u32,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_en: Option<String>,
    pub edits: u64,
    pub categories: Vec<TopArticleCategory>,
}

#[derive(Serialize, JsonSchema)]
pub struct PageEditTopArticlesResponse {
    pub articles: Vec<PageEditTopArticle>,
}

#[derive(Serialize, JsonSchema)]
pub struct TopArticleByEdits {
    pub qid: u32,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_en: Option<String>,
    pub edits: u64,
}

#[derive(Serialize, JsonSchema)]
pub struct TopCategoryByEdits {
    pub qid: u32,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_en: Option<String>,
    pub edits: u64,
    pub top_articles: Vec<TopArticleByEdits>,
}

#[derive(Serialize, JsonSchema)]
pub struct CategoryEditRankResponse {
    pub categories: Vec<TopCategoryByEdits>,
}

#[derive(Serialize, JsonSchema)]
pub struct TopArticleBySearch {
    pub qid: u32,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_en: Option<String>,
    pub clicks: u64,
    pub impressions: u64,
    pub ctr: f64,
}

#[derive(Serialize, JsonSchema)]
pub struct TopCategoryBySearch {
    pub qid: u32,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_en: Option<String>,
    pub clicks: u64,
    pub impressions: u64,
    pub ctr: f64,
    pub top_articles: Vec<TopArticleBySearch>,
}

#[derive(Serialize, JsonSchema)]
pub struct CategorySearchRankResponse {
    pub categories: Vec<TopCategoryBySearch>,
}

#[derive(Serialize, JsonSchema)]
pub struct GoogleSearchTopArticle {
    pub qid: u32,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_en: Option<String>,
    pub clicks: u64,
    pub impressions: u64,
    pub ctr: f64,
    pub categories: Vec<TopArticleCategory>,
}

#[derive(Serialize, JsonSchema)]
pub struct GoogleSearchTopArticlesResponse {
    pub articles: Vec<GoogleSearchTopArticle>,
}

#[derive(Serialize, JsonSchema)]
pub struct PageViewCategoryDeltaItemResponse {
    pub category_qid: u32,
    pub category_title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_title_en: Option<String>,
    pub baseline_views: u64,
    pub impact_views: u64,
    pub delta_percentage: f64,
    pub absolute_delta: i64,
}

#[derive(Serialize, JsonSchema)]
pub struct PageViewCategoryDeltaResponse {
    pub categories: Vec<PageViewCategoryDeltaItemResponse>,
    pub baseline_period: String,
    pub impact_period: String,
}

#[derive(Serialize, JsonSchema)]
pub struct PageViewArticleDeltaItemResponse {
    pub article_qid: u32,
    pub article_title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub article_title_en: Option<String>,
    pub baseline_views: u64,
    pub impact_views: u64,
    pub delta_percentage: f64,
    pub absolute_delta: i64,
}

#[derive(Serialize, JsonSchema)]
pub struct PageViewArticleDeltaResponse {
    pub articles: Vec<PageViewArticleDeltaItemResponse>,
    pub category_qid: u32,
    pub category_title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_title_en: Option<String>,
    pub baseline_period: String,
    pub impact_period: String,
}

#[derive(Serialize, JsonSchema)]
pub struct PageEditCategoryDeltaItemResponse {
    pub category_qid: u32,
    pub category_title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_title_en: Option<String>,
    pub baseline_edits: u64,
    pub impact_edits: u64,
    pub delta_percentage: f64,
    pub absolute_delta: i64,
}

#[derive(Serialize, JsonSchema)]
pub struct PageEditCategoryDeltaResponse {
    pub categories: Vec<PageEditCategoryDeltaItemResponse>,
    pub baseline_period: String,
    pub impact_period: String,
}

#[derive(Serialize, JsonSchema)]
pub struct PageEditArticleDeltaItemResponse {
    pub article_qid: u32,
    pub article_title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub article_title_en: Option<String>,
    pub baseline_edits: u64,
    pub impact_edits: u64,
    pub delta_percentage: f64,
    pub absolute_delta: i64,
}

#[derive(Serialize, JsonSchema)]
pub struct PageEditArticleDeltaResponse {
    pub articles: Vec<PageEditArticleDeltaItemResponse>,
    pub category_qid: u32,
    pub category_title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_title_en: Option<String>,
    pub baseline_period: String,
    pub impact_period: String,
}

#[derive(Serialize, JsonSchema)]
pub struct TopArticleGoogleSearch {
    pub qid: u32,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_en: Option<String>,
    pub clicks: u64,
    pub impressions: u64,
    pub ctr: f64,
    pub source_categories: Vec<TopArticleCategory>,
}

#[derive(Serialize, JsonSchema)]
pub struct GoogleSearchCategoryTrendResponse {
    pub qid: u32,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_en: Option<String>,
    pub search: Vec<DailyGoogleSearch>,
    pub top_articles: Vec<TopArticleGoogleSearch>,
}

#[derive(Serialize, JsonSchema)]
pub struct GoogleSearchArticleTrendResponse {
    pub qid: u32,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_en: Option<String>,
    pub search: Vec<DailyGoogleSearch>,
}

#[derive(Serialize, JsonSchema)]
pub struct GoogleSearchCategoryDeltaItemResponse {
    pub category_qid: u32,
    pub category_title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_title_en: Option<String>,
    pub baseline_clicks: u64,
    pub impact_clicks: u64,
    pub baseline_impressions: u64,
    pub impact_impressions: u64,
    pub delta_percentage: f64,
    pub absolute_delta: i64,
}

#[derive(Serialize, JsonSchema)]
pub struct GoogleSearchCategoryDeltaResponse {
    pub categories: Vec<GoogleSearchCategoryDeltaItemResponse>,
    pub baseline_period: String,
    pub impact_period: String,
}

#[derive(Serialize, JsonSchema)]
pub struct GoogleSearchArticleDeltaItemResponse {
    pub article_qid: u32,
    pub article_title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub article_title_en: Option<String>,
    pub baseline_clicks: u64,
    pub impact_clicks: u64,
    pub baseline_impressions: u64,
    pub impact_impressions: u64,
    pub delta_percentage: f64,
    pub absolute_delta: i64,
}

#[derive(Serialize, JsonSchema)]
pub struct GoogleSearchArticleDeltaResponse {
    pub articles: Vec<GoogleSearchArticleDeltaItemResponse>,
    pub category_qid: u32,
    pub category_title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_title_en: Option<String>,
    pub baseline_period: String,
    pub impact_period: String,
}

#[derive(Deserialize)]
pub struct CategorySearchParams {
    pub query: String,
    pub wiki: String,
    pub match_threshold: Option<f32>,
    pub limit: Option<u64>,
}

#[derive(Serialize, JsonSchema)]
pub struct CategorySearchItemResponse {
    pub category_qid: u32,
    pub category_title_en: String,
    pub category_title: String,
    pub match_score: f32,
}

#[derive(Serialize, JsonSchema)]
pub struct CategorySearchResponse {
    pub categories: Vec<CategorySearchItemResponse>,
}

#[derive(Deserialize)]
pub struct ListArticleCategoriesParams {
    pub wiki: String,
    pub article: Option<String>,
    pub article_qid: Option<u32>,
}

/// One category of an article, ranked by cross-wiki agreement. `wiki_count`
/// is the number of editions asserting the assignment (1 everywhere when the
/// server runs on local topology); ties at the same count are common for
/// articles existing in few editions — consumers should read the count, not
/// trust the order alone.
#[derive(Serialize, JsonSchema)]
pub struct RankedCategoryInfo {
    pub qid: u32,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_en: Option<String>,
    pub wiki_count: u16,
}

#[derive(Serialize, JsonSchema)]
pub struct ArticleCategoriesResponse {
    pub categories: Vec<RankedCategoryInfo>,
}

#[derive(Deserialize)]
pub struct RelatedArticlesParams {
    pub wiki: String,
    pub article: Option<String>,
    pub article_qid: Option<u32>,
    pub limit: Option<usize>,
}

#[derive(Serialize, JsonSchema)]
pub struct RelatedArticleItem {
    pub qid: u32,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_en: Option<String>,
    pub url: String,
    pub score: u32,
}

#[derive(Serialize, JsonSchema)]
pub struct RelatedArticlesResponse {
    pub articles: Vec<RelatedArticleItem>,
}

#[derive(Deserialize)]
pub struct ListArticlesInCategoryParams {
    pub wiki: String,
    pub category: Option<String>,
    pub category_qid: Option<u32>,
    /// Keep only members at least this many wikis agree on (default 1).
    pub min_agreement: Option<u16>,
}

#[derive(Deserialize)]
pub struct ContentGapParams {
    pub category: Option<String>,
    pub category_qid: Option<u32>,
    pub wikis: String,
}

#[derive(Deserialize)]
pub struct ContentGapTopicParams {
    pub topic: String,
    pub wikis: String,
}

#[derive(Deserialize)]
pub struct GapDiscoveryParams {
    pub reference: String,
    pub target: String,
    #[serde(alias = "skip")]
    pub offset: Option<usize>,
    pub limit: Option<usize>,
    pub min_ref: Option<u32>,
    pub has_category: Option<bool>,
}

#[derive(Serialize, JsonSchema)]
pub struct GapDiscoveryItemResponse {
    pub category_qid: u32,
    pub category_title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category_title_en: Option<String>,
    pub direct_coverage_target: u32,
    pub overlap_target: u32,
    pub overlap_reference: u32,
    pub gap: i64,
    pub coverage_pct: f64,
    pub has_category: bool,
}

#[derive(Serialize, JsonSchema)]
pub struct GapDiscoveryResponse {
    pub reference: String,
    pub target: String,
    pub reference_date: String,
    pub target_date: String,
    pub total: usize,
    pub with_category: usize,
    pub without_category: usize,
    pub offset: usize,
    pub limit: usize,
    pub categories: Vec<GapDiscoveryItemResponse>,
}

#[derive(Serialize, JsonSchema)]
pub struct ArticlesInCategoryResponse {
    pub articles: Vec<ArticleItem>,
}

#[derive(Serialize, JsonSchema)]
pub struct SubCategoriesResponse {
    pub categories: Vec<CategoryInfo>,
}

#[derive(Serialize, JsonSchema)]
pub struct ArticleItem {
    pub qid: u32,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_en: Option<String>,
}

#[derive(Serialize, JsonSchema)]
pub struct ContentGapWikiResult {
    pub wiki: String,
    pub article_count: usize,
}

#[derive(Serialize, JsonSchema)]
pub struct ContentGapResult {
    pub category: String,
    pub category_qid: u32,
    pub wikis: Vec<ContentGapWikiResult>,
}
