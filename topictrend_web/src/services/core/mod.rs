pub mod article_service;
pub mod category_service;
pub mod coverage_service;
pub mod engine_service;
pub mod google_search_service;
pub mod pageedit_service;
pub mod pageview_service;
pub mod qid_service;
pub mod related_service;
pub mod title_store;

pub use article_service::ArticleService;
pub use category_service::CategoryService;
pub use coverage_service::CoverageService;
pub use engine_service::EngineService;
pub use google_search_service::GoogleSearchService;
pub use pageedit_service::PageEditService;
pub use pageview_service::PageViewService;
pub use qid_service::QidService;
pub use related_service::RelatedService;

#[derive(Debug)]
pub enum CoreServiceError {
    EngineError(String),
    NotFound,
    InternalError(String),
}
