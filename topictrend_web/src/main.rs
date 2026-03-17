mod grpc_service;
mod handlers;
mod models;
mod services;
mod templates;
mod wiki;

use crate::grpc_service::{
    TopicTrendGrpcService, topictrend_proto::topic_trend_service_server::TopicTrendServiceServer,
};
use crate::models::AppState;
use crate::templates::{PageContext, render_template};
use axum::http::header::{CACHE_CONTROL, HeaderValue};
use axum::{
    Router,
    http::{Method, StatusCode, header::*},
    response::Html,
    response::Redirect,
    routing::{get, get_service},
};
use std::{net::SocketAddr, sync::Arc};
use tonic::transport::Server;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::{cors::CorsLayer, services::ServeDir};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

const OPENAPI_YAML: &str = include_str!("../openapi.yaml");
const SWAGGER_UI_HTML: &str = include_str!("../static/swagger-ui.html");

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
            get(|| async { render_template("index.html", PageContext::home()) }),
        )
        .route(
            "/pageviews/trends",
            get(|| async {
                render_template("pageview-trends.html", PageContext::pageview_trends())
            }),
        )
        .route(
            "/pageviews/delta",
            get(|| async { render_template("pageview-delta.html", PageContext::pageview_delta()) }),
        )
        .route(
            "/pageedits/trends",
            get(|| async {
                render_template("pageedit-trends.html", PageContext::pageedit_trends())
            }),
        )
        .route(
            "/pageedits/delta",
            get(|| async { render_template("pageedit-delta.html", PageContext::pageedit_delta()) }),
        )
        .route(
            "/googlesearch/trends",
            get(|| async {
                render_template(
                    "google-search-trends.html",
                    PageContext::google_search_trends(),
                )
            }),
        )
        .route(
            "/googlesearch/delta",
            get(|| async {
                render_template(
                    "google-search-delta.html",
                    PageContext::google_search_delta(),
                )
            }),
        )
        .route(
            "/delta",
            get(|| async { Redirect::permanent("/pageviews/delta") }),
        )
        .route(
            "/search",
            get(|| async { render_template("search.html", PageContext::search()) }),
        )
        .route(
            "/content-gap",
            get(|| async { render_template("content-gap.html", PageContext::content_gap()) }),
        )
        .route(
            "/openapi.yaml",
            get(|| async {
                (
                    [(CONTENT_TYPE, HeaderValue::from_static("application/yaml"))],
                    OPENAPI_YAML,
                )
            }),
        )
        .route("/docs", get(|| async { Html(SWAGGER_UI_HTML) }))
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
            "/api/pageedits/category",
            get(handlers::get_category_edit_trend_handler),
        )
        .route(
            "/api/pageedits/article",
            get(handlers::get_article_edit_trend_handler),
        )
        .route(
            "/api/googlesearch/category",
            get(handlers::get_category_google_search_trend_handler),
        )
        .route(
            "/api/googlesearch/article",
            get(handlers::get_article_google_search_trend_handler),
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
            "/api/pageviews/delta/categories",
            get(handlers::get_category_pageview_delta_handler),
        )
        .route(
            "/api/pageviews/delta/articles",
            get(handlers::get_article_pageview_delta_handler),
        )
        .route(
            "/api/pageedits/delta/categories",
            get(handlers::get_category_pageedit_delta_handler),
        )
        .route(
            "/api/pageedits/delta/articles",
            get(handlers::get_article_pageedit_delta_handler),
        )
        .route(
            "/api/googlesearch/delta/categories",
            get(handlers::get_category_google_search_delta_handler),
        )
        .route(
            "/api/googlesearch/delta/articles",
            get(handlers::get_article_google_search_delta_handler),
        )
        .route("/api/search/categories", get(handlers::search_categories))
        .route(
            "/api/list/articles",
            get(handlers::get_articles_in_category),
        )
        .route(
            "/api/pageviews/categories",
            get(handlers::get_categories_trend_by_search_handler),
        )
        .route(
            "/api/content_gap/categories",
            get(handlers::get_content_gap_handler),
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
