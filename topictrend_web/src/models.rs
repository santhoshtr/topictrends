use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::{MySql, Pool};
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
    Graph(Arc<RwLock<WikiGraph>>),
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

    pub fn as_graph(&self) -> Option<&Arc<RwLock<WikiGraph>>> {
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
    pub db_pools: Arc<RwLock<HashMap<String, Pool<MySql>>>>,
    pub db_username: String,
    pub db_password: String,
}

impl AppState {
    pub fn new() -> Self {
        let db_username = std::env::var("DB_USERNAME").expect("DB_USERNAME must be set");
        let db_password = std::env::var("DB_PASSWORD").expect("DB_PASSWORD must be set");

        Self {
            engines: Arc::new(RwLock::new(HashMap::new())),
            db_pools: Arc::new(RwLock::new(HashMap::new())),
            db_username,
            db_password,
        }
    }
}

// --- Request DTO ---
#[derive(Deserialize)]
pub struct CategoryTrendParams {
    pub wiki: String,
    pub category: String,
    pub depth: Option<u32>,
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
    pub depth: Option<u32>,
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
    pub depth: Option<u32>,
}

#[derive(Deserialize)]
pub struct PageEditCategoryDeltaParams {
    pub wiki: String,
    pub baseline_start_date: NaiveDate,
    pub baseline_end_date: NaiveDate,
    pub impact_start_date: NaiveDate,
    pub impact_end_date: NaiveDate,
    pub limit: Option<u32>,
    pub depth: Option<u32>,
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
    pub depth: Option<u32>,
}

#[derive(Deserialize)]
pub struct GoogleSearchCategoryDeltaParams {
    pub wiki: String,
    pub baseline_start_date: NaiveDate,
    pub baseline_end_date: NaiveDate,
    pub impact_start_date: NaiveDate,
    pub impact_end_date: NaiveDate,
    pub limit: Option<u32>,
    pub depth: Option<u32>,
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
    pub depth: Option<u32>,
}

// --- Response DTO ---
#[derive(Serialize)]
pub struct DailyViews {
    pub date: NaiveDate,
    pub views: u64,
}

#[derive(Serialize)]
pub struct DailyEdits {
    pub date: NaiveDate,
    pub edits: u64,
}

#[derive(Serialize)]
pub struct DailyGoogleSearch {
    pub date: NaiveDate,
    pub clicks: u64,
    pub impressions: u64,
    pub ctr: f64,
    pub position: f64,
}

#[derive(Serialize)]
pub struct ArticleTrendResponse {
    pub qid: u32,
    pub title: String,
    pub views: Vec<DailyViews>,
}

#[derive(Serialize)]
pub struct CategoryTrendResponse {
    pub qid: u32,
    pub title: String,
    pub views: Vec<DailyViews>,
    pub top_articles: Vec<TopArticle>,
}

#[derive(Serialize)]
pub struct CategoriesTrendResponse {
    pub categories: Vec<CategoryInfo>,
    pub cumulative_views: Vec<DailyViews>,
    pub top_articles: Vec<TopArticle>,
}

#[derive(Serialize)]
pub struct ArticleEditTrendResponse {
    pub qid: u32,
    pub title: String,
    pub edits: Vec<DailyEdits>,
}

#[derive(Serialize)]
pub struct CategoryEditTrendResponse {
    pub qid: u32,
    pub title: String,
    pub edits: Vec<DailyEdits>,
    pub top_articles: Vec<TopArticleEdits>,
}

#[derive(Serialize)]
pub struct CategoryInfo {
    pub qid: u32,
    pub title: String,
}

#[derive(Serialize)]
pub struct TopArticle {
    pub qid: u32,
    pub title: String,
    pub views: u32,
}

#[derive(Serialize)]
pub struct TopArticleEdits {
    pub qid: u32,
    pub title: String,
    pub edits: u64,
}

#[derive(Serialize)]
pub struct TopCategory {
    pub qid: u32,
    pub title: String,
    pub views: u32,
    pub top_articles: Vec<TopArticle>,
}

#[derive(Serialize)]
pub struct CategoryRankResponse {
    pub categories: Vec<TopCategory>,
}

#[derive(Serialize)]
pub struct PageViewCategoryDeltaItemResponse {
    pub category_qid: u32,
    pub category_title: String,
    pub baseline_views: u64,
    pub impact_views: u64,
    pub delta_percentage: f64,
    pub absolute_delta: i64,
}

#[derive(Serialize)]
pub struct PageViewCategoryDeltaResponse {
    pub categories: Vec<PageViewCategoryDeltaItemResponse>,
    pub baseline_period: String,
    pub impact_period: String,
}

#[derive(Serialize)]
pub struct PageViewArticleDeltaItemResponse {
    pub article_qid: u32,
    pub article_title: String,
    pub baseline_views: u64,
    pub impact_views: u64,
    pub delta_percentage: f64,
    pub absolute_delta: i64,
}

#[derive(Serialize)]
pub struct PageViewArticleDeltaResponse {
    pub articles: Vec<PageViewArticleDeltaItemResponse>,
    pub category_qid: u32,
    pub category_title: String,
    pub baseline_period: String,
    pub impact_period: String,
}

#[derive(Serialize)]
pub struct PageEditCategoryDeltaItemResponse {
    pub category_qid: u32,
    pub category_title: String,
    pub baseline_edits: u64,
    pub impact_edits: u64,
    pub delta_percentage: f64,
    pub absolute_delta: i64,
}

#[derive(Serialize)]
pub struct PageEditCategoryDeltaResponse {
    pub categories: Vec<PageEditCategoryDeltaItemResponse>,
    pub baseline_period: String,
    pub impact_period: String,
}

#[derive(Serialize)]
pub struct PageEditArticleDeltaItemResponse {
    pub article_qid: u32,
    pub article_title: String,
    pub baseline_edits: u64,
    pub impact_edits: u64,
    pub delta_percentage: f64,
    pub absolute_delta: i64,
}

#[derive(Serialize)]
pub struct PageEditArticleDeltaResponse {
    pub articles: Vec<PageEditArticleDeltaItemResponse>,
    pub category_qid: u32,
    pub category_title: String,
    pub baseline_period: String,
    pub impact_period: String,
}

#[derive(Serialize)]
pub struct TopArticleGoogleSearch {
    pub qid: u32,
    pub title: String,
    pub clicks: u64,
    pub impressions: u64,
    pub ctr: f64,
}

#[derive(Serialize)]
pub struct GoogleSearchCategoryTrendResponse {
    pub qid: u32,
    pub title: String,
    pub search: Vec<DailyGoogleSearch>,
    pub top_articles: Vec<TopArticleGoogleSearch>,
}

#[derive(Serialize)]
pub struct GoogleSearchArticleTrendResponse {
    pub qid: u32,
    pub title: String,
    pub search: Vec<DailyGoogleSearch>,
}

#[derive(Serialize)]
pub struct GoogleSearchCategoryDeltaItemResponse {
    pub category_qid: u32,
    pub category_title: String,
    pub baseline_clicks: u64,
    pub impact_clicks: u64,
    pub baseline_impressions: u64,
    pub impact_impressions: u64,
    pub delta_percentage: f64,
    pub absolute_delta: i64,
}

#[derive(Serialize)]
pub struct GoogleSearchCategoryDeltaResponse {
    pub categories: Vec<GoogleSearchCategoryDeltaItemResponse>,
    pub baseline_period: String,
    pub impact_period: String,
}

#[derive(Serialize)]
pub struct GoogleSearchArticleDeltaItemResponse {
    pub article_qid: u32,
    pub article_title: String,
    pub baseline_clicks: u64,
    pub impact_clicks: u64,
    pub baseline_impressions: u64,
    pub impact_impressions: u64,
    pub delta_percentage: f64,
    pub absolute_delta: i64,
}

#[derive(Serialize)]
pub struct GoogleSearchArticleDeltaResponse {
    pub articles: Vec<GoogleSearchArticleDeltaItemResponse>,
    pub category_qid: u32,
    pub category_title: String,
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

#[derive(Serialize)]
pub struct CategorySearchItemResponse {
    pub category_qid: u32,
    pub category_title_en: String,
    pub category_title: String,
    pub match_score: f32,
}

#[derive(Serialize)]
pub struct CategorySearchResponse {
    pub categories: Vec<CategorySearchItemResponse>,
}

#[derive(Deserialize)]
pub struct ListArticlesInCategoryParams {
    pub wiki: String,
    pub category: Option<String>,
    pub category_qid: Option<u32>,
}

#[derive(Deserialize)]
pub struct ContentGapParams {
    pub category: Option<String>,
    pub category_qid: Option<u32>,
    pub wikis: String,
    pub depth: Option<u32>,
}

#[derive(Serialize)]
pub struct ArticlesInCategoryResponse {
    pub articles: Vec<ArticleItem>,
}

#[derive(Serialize)]
pub struct ArticleItem {
    pub qid: u32,
    pub title: String,
}

#[derive(Serialize)]
pub struct ContentGapWikiResult {
    pub wiki: String,
    pub article_count: usize,
}

#[derive(Serialize)]
pub struct ContentGapResult {
    pub category: String,
    pub category_qid: u32,
    pub depth: u32,
    pub wikis: Vec<ContentGapWikiResult>,
}
