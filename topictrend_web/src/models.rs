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
    pub depth: Option<u8>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
}
#[derive(Deserialize)]
pub struct ArticleTrendParams {
    pub wiki: String,
    pub article: String,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
}

#[derive(Deserialize)]
pub struct SubCategoryParams {
    pub wiki: String,
    pub category: String,
}

#[derive(Deserialize)]
pub struct TopCategoriesParams {
    pub wiki: String,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub top_n: Option<u8>,
}

// --- Response DTO ---
#[derive(Serialize)]
pub struct TrendResponse {
    pub date: NaiveDate,
    pub views: u64,
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
