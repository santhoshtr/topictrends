mod grpc_service;
mod handlers;
mod models;
mod services;
mod wiki;

use crate::grpc_service::{
    TopicTrendGrpcService, topictrend_proto::topic_trend_service_server::TopicTrendServiceServer,
};
use crate::models::AppState;
use axum::http::header::{CACHE_CONTROL, HeaderValue};
use axum::{
    Router,
    http::{Method, StatusCode, header::*},
    response::Html,
    routing::{get, get_service},
};
use std::{net::SocketAddr, sync::Arc};
use tonic::transport::Server;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::{cors::CorsLayer, services::ServeDir};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

async fn run_http_server(
    state: Arc<AppState>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
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
        .route(
            "/api/list/sub_categories",
            get(handlers::get_sub_categories),
        )
        .route(
            "/api/list/top_categories",
            get(handlers::get_top_categories_handler),
        )
        .route(
            "/api/delta/categories",
            get(handlers::get_category_delta_handler),
        )
        .route(
            "/api/delta/articles",
            get(handlers::get_article_delta_handler),
        )
        .with_state(state)
        .layer(cors)
        .layer(SetResponseHeaderLayer::if_not_present(
            CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=3600"),
        ));

    println!("🚀 HTTP Server started successfully on port {}", port);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap_or_else(|e| {
            eprintln!("Failed to bind to address {}: {}", addr, e);
            panic!("HTTP Server failed to start");
        });

    axum::serve(listener, app).await.unwrap();
    Ok(())
}

async fn run_grpc_server(
    state: Arc<AppState>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let grpc_port = std::env::var("GRPC_PORT")
        .unwrap_or_else(|_| "50051".to_string())
        .parse::<u16>()
        .unwrap_or(50051);

    let addr = SocketAddr::from(([0, 0, 0, 0], grpc_port));
    let grpc_service = TopicTrendGrpcService::new(state);

    println!("🚀 gRPC Server started successfully on port {}", grpc_port);

    Server::builder()
        .add_service(TopicTrendServiceServer::new(grpc_service))
        .serve(addr)
        .await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=trace", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state: Arc<AppState> = Arc::new(AppState::new());

    let http_server = run_http_server(Arc::clone(&state));
    let grpc_server = run_grpc_server(Arc::clone(&state));

    // Run both servers concurrently
    tokio::try_join!(http_server, grpc_server)?;

    Ok(())
}
