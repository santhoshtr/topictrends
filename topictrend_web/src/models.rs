use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use topictrend::pageview_engine::PageViewEngine;

#[derive(Clone)]
pub struct AppState {
    pub engines: Arc<RwLock<HashMap<String, PageViewEngine>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            engines: Arc::new(RwLock::new(HashMap::new())),
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
pub struct ArticleSearchParams {
    pub wiki: String,
    pub article: String,
}
#[derive(Deserialize)]
pub struct CategorySearchParams {
    pub wiki: String,
    pub category: String,
}

// --- Response DTO ---
#[derive(Serialize)]
pub struct TrendResponse {
    pub date: NaiveDate,
    pub views: u64,
}
