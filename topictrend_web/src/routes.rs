use crate::handlers;
use crate::models::AppState;
use crate::templates::{PageContext, render_template};
use axum::http::header::{CACHE_CONTROL, HeaderValue};
use axum::{
    Router,
    http::{Method, StatusCode, header::*},
    response::{Html, Redirect},
    routing::{get, get_service},
};
use std::sync::Arc;
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::{cors::CorsLayer, services::ServeDir};

const OPENAPI_YAML: &str = include_str!("../openapi.yaml");
const SWAGGER_UI_HTML: &str = include_str!("../static/swagger-ui.html");

pub fn app_router(state: Arc<AppState>) -> Router {
    let static_files = get_service(ServeDir::new("topictrend_web/static"))
        .handle_error(|_| async { (StatusCode::INTERNAL_SERVER_ERROR, "Static file error") });

    let cors = CorsLayer::new()
        .allow_origin("*".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers([AUTHORIZATION, ACCEPT, CONTENT_TYPE]);

    Router::new()
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
            "/pageviews/top",
            get(|| async { render_template("pageviews-top.html", PageContext::pageviews_top()) }),
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
            "/pageedits/top",
            get(|| async { render_template("pageedits-top.html", PageContext::pageedits_top()) }),
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
            "/googlesearch/top",
            get(|| async {
                render_template("googlesearch-top.html", PageContext::googlesearch_top())
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
            "/api/pageviews/top_categories",
            get(handlers::get_pageviews_top_categories_handler),
        )
        .route(
            "/api/pageedits/top_categories",
            get(handlers::get_pageedits_top_categories_handler),
        )
        .route(
            "/api/googlesearch/top_categories",
            get(handlers::get_googlesearch_top_categories_handler),
        )
        .route(
            "/api/list/sub_categories",
            get(handlers::get_sub_categories),
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
        ))
}
