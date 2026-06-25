use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

// Custom error type for API handlers
#[derive(Debug)]
pub enum ApiError {
    ServiceError(crate::services::ServiceError),
    DeltaError(crate::services::core::CoreServiceError),
}

impl From<crate::services::ServiceError> for ApiError {
    fn from(err: crate::services::ServiceError) -> Self {
        ApiError::ServiceError(err)
    }
}

impl From<crate::services::core::CoreServiceError> for ApiError {
    fn from(err: crate::services::core::CoreServiceError) -> Self {
        ApiError::DeltaError(err)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ApiError::ServiceError(e) => match e {
                crate::services::ServiceError::CoreError(core_err) => match core_err {
                    crate::services::core::CoreServiceError::EngineError(e) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Engine error: {}", e),
                    ),
                    crate::services::core::CoreServiceError::NotFound => {
                        (StatusCode::NOT_FOUND, "Resource not found".to_string())
                    }
                    crate::services::core::CoreServiceError::InternalError(e) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Internal server error: {}", e),
                    ),
                },
            },
            ApiError::DeltaError(core_err) => match core_err {
                crate::services::core::CoreServiceError::EngineError(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Engine error: {}", e),
                ),
                crate::services::core::CoreServiceError::NotFound => {
                    (StatusCode::NOT_FOUND, "Resource not found".to_string())
                }
                crate::services::core::CoreServiceError::InternalError(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Internal server error: {}", e),
                ),
            },
        };

        (status, Json(serde_json::json!({ "error": error_message }))).into_response()
    }
}

pub mod pageviews;
pub mod pageedits;
pub mod googlesearch;
pub mod delta;
pub mod gap_discovery;
pub mod search;
pub mod related;
pub mod cluster;

pub use pageviews::*;
pub use pageedits::*;
pub use googlesearch::*;
pub use delta::*;
pub use gap_discovery::*;
pub use search::*;
pub use related::*;
pub use cluster::*;
