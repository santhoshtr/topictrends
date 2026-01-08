use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use sqlx::{MySql, Pool};
use topictrend::pageview_engine::PageViewEngine;

pub struct AppState {
    pub engines: Arc<RwLock<HashMap<String, Arc<RwLock<PageViewEngine>>>>>,
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
pub struct CategoryDeltaParams {
    pub wiki: String,
    pub baseline_start_date: NaiveDate,
    pub baseline_end_date: NaiveDate,
    pub impact_start_date: NaiveDate,
    pub impact_end_date: NaiveDate,
    pub limit: Option<u32>,
    pub depth: Option<u32>,
}

#[derive(Deserialize)]
pub struct ArticleDeltaParams {
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
pub struct CategoryDeltaItemResponse {
    pub category_qid: u32,
    pub category_title: String,
    pub baseline_views: u64,
    pub impact_views: u64,
    pub delta_percentage: f64,
    pub absolute_delta: i64,
}

#[derive(Serialize)]
pub struct CategoryDeltaResponse {
    pub categories: Vec<CategoryDeltaItemResponse>,
    pub baseline_period: String,
    pub impact_period: String,
}

#[derive(Serialize)]
pub struct ArticleDeltaItemResponse {
    pub article_qid: u32,
    pub article_title: String,
    pub baseline_views: u64,
    pub impact_views: u64,
    pub delta_percentage: f64,
    pub absolute_delta: i64,
}

#[derive(Serialize)]
pub struct ArticleDeltaResponse {
    pub articles: Vec<ArticleDeltaItemResponse>,
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

#[derive(Serialize)]
pub struct ArticlesInCategoryResponse {
    pub articles: Vec<ArticleItem>,
}

#[derive(Serialize)]
pub struct ArticleItem {
    pub qid: u32,
    pub title: String,
}
