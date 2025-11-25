use std::{collections::HashMap, sync::Arc};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use topictrend::pageview_engine::PageViewEngine;

// --- Application State ---
// Shared across all web threads
pub struct AppState {
    pub engines: HashMap<String, PageViewEngine>,
}

// --- Request DTO ---
#[derive(Deserialize)]
pub struct TrendParams {
    pub depth: Option<u8>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
}

// --- Response DTO ---
#[derive(Serialize)]
pub struct TrendResponse {
    pub date: NaiveDate,
    pub views: u64,
}
