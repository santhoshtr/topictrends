pub mod article_service;
pub mod category_service;
pub mod engine_service;
pub mod pageview_service;
pub mod qid_service;

pub use article_service::ArticleService;
pub use category_service::CategoryService;
pub use engine_service::EngineService;
pub use pageview_service::PageViewService;
pub use qid_service::QidService;

#[derive(Debug)]
pub enum CoreServiceError {
    DatabaseError(sqlx::Error),
    EngineError(String),
    NotFound,
    InternalError(String),
}

impl From<sqlx::Error> for CoreServiceError {
    fn from(err: sqlx::Error) -> Self {
        CoreServiceError::DatabaseError(err)
    }
}
