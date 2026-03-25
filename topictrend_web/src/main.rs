mod grpc_service;
mod handlers;
mod models;
mod routes;
mod services;
mod templates;
mod wiki;

use crate::grpc_service::{
    TopicTrendGrpcService, topictrend_proto::topic_trend_service_server::TopicTrendServiceServer,
};
use crate::models::AppState;
use crate::routes::app_router;
use std::{net::SocketAddr, sync::Arc};
use tonic::transport::Server;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

async fn run_http_server(
    state: Arc<AppState>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8765".to_string())
        .parse::<u16>()
        .unwrap_or(8765);

    let app = app_router(state);

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
