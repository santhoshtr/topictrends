mod handlers;
mod models;
mod routes;

use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use axum::http::{
    HeaderValue, Method,
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
};
use routes::create_router;
use tower_http::cors::CorsLayer;

use crate::models::AppState;

#[tokio::main]
async fn main() {
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8000".to_string())
        .parse::<u16>()
        .unwrap_or(8000);

    let cors = CorsLayer::new()
        .allow_origin("*".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_credentials(true)
        .allow_headers([AUTHORIZATION, ACCEPT, CONTENT_TYPE]);
    let app_state: Arc<AppState> = AppState {
        engines: HashMap::new().into(),
    }
    .into();
    let app = create_router(app_state).layer(cors);

    println!("🚀 Server started successfully");
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
