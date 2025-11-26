mod handlers;
mod models;

use crate::models::AppState;
use axum::{
    Router,
    http::{Method, StatusCode, header::*},
    response::Html,
    routing::{get, get_service},
};
use std::{net::SocketAddr, sync::Arc};
use tower_http::trace::TraceLayer;
use tower_http::{cors::CorsLayer, services::ServeDir};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=trace", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8765".to_string())
        .parse::<u16>()
        .unwrap_or(8765);

    let static_files = get_service(ServeDir::new("topictrend_web/static"))
        .handle_error(|_| async { (StatusCode::INTERNAL_SERVER_ERROR, "Static file error") });

    let cors = CorsLayer::new()
        .allow_origin("*".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers([AUTHORIZATION, ACCEPT, CONTENT_TYPE]);

    let state: Arc<AppState> = Arc::new(AppState::new());
    let app = Router::new()
        .route(
            "/",
            get(|| async { Html(include_str!("../static/index.html")) }),
        )
        .nest_service("/static", static_files)
        .route(
            "/api/pageviews/category",
            get(handlers::get_category_trend_handler),
        )
        .route(
            "/api/pageviews/article",
            get(handlers::get_article_trend_handler),
        )
        .with_state(state)
        .layer(cors);

    println!("🚀 Server started successfully on port {}", port);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap_or_else(|e| {
            eprintln!("Failed to bind to address {}: {}", addr, e);
            panic!("Server failed to start");
        });
    axum::serve(listener, app).await.unwrap()
}
