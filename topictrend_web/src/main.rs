mod handlers;
mod mcp;
mod models;
mod routes;
mod services;
mod templates;
mod wiki;

use crate::models::AppState;
use crate::routes::app_router;
use std::{net::SocketAddr, sync::Arc};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

async fn run_http_server(
    state: Arc<AppState>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8765".to_string())
        .parse::<u16>()
        .unwrap_or(8765);

    let app = app_router(state);

    println!("🚀 Server started successfully on port {}", port);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .unwrap_or_else(|e| {
            eprintln!("Failed to bind to address {}: {}", addr, e);
            panic!("Server failed to start");
        });

    axum::serve(listener, app).await.unwrap();
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

    run_http_server(state).await?;

    Ok(())
}
