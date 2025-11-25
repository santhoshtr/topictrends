use std::sync::Arc;

use axum::{Router, routing::get};

use crate::{handlers::get_category_trend_handler, models::AppState};

pub fn create_router(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route(
            "/api/trend/:wiki/:category_id",
            get(get_category_trend_handler),
        )
        .with_state(app_state)
}
