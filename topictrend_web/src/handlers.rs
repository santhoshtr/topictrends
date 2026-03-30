use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use axum_macros::debug_handler;
use std::sync::Arc;

use crate::services::{ContentGapService, PageEditsService, PageViewsService};
use crate::{
    models::{
        AppState, ArticleEditTrendResponse, ArticleItem, ArticleTrendParams, ArticleTrendResponse,
        ArticlesInCategoryResponse, CategoryEditRankResponse, CategoryEditTrendResponse,
        CategoryRankResponse, CategorySearchItemResponse, CategorySearchParams,
        CategorySearchRankResponse, CategorySearchResponse, CategoryTrendParams,
        CategoryTrendResponse, ContentGapParams, ContentGapResult, ContentGapTopicParams, DailyEdits, DailyGoogleSearch,
        DailyViews, GoogleSearchArticleDeltaParams, GoogleSearchArticleDeltaResponse,
        GoogleSearchArticleTrendResponse, GoogleSearchCategoryDeltaParams,
        GoogleSearchCategoryDeltaResponse, GoogleSearchCategoryTrendResponse,
        GoogleSearchTopArticle, GoogleSearchTopArticlesResponse, ListArticlesInCategoryParams,
        PageEditArticleDeltaParams, PageEditArticleDeltaResponse, PageEditCategoryDeltaParams,
        PageEditCategoryDeltaResponse, PageEditTopArticle, PageEditTopArticlesResponse,
        PageViewArticleDeltaParams, PageViewArticleDeltaResponse, PageViewCategoryDeltaParams,
        PageViewCategoryDeltaResponse, PageViewTopArticle, PageViewTopArticlesResponse,
        SubCategoryParams, TopArticle, TopArticleByEdits, TopArticleBySearch, TopArticleCategory,
        TopArticleEdits, TopArticleGoogleSearch, TopCategoriesParams, TopCategory,
        TopCategoryByEdits, TopCategoryBySearch, TopicTrendParams,
    },
    services::core::CategoryService,
};
use crate::{
    models::{CategoriesTrendParams, CategoriesTrendResponse},
    services::{
        composite::{
            GoogleSearchDeltaService, GoogleSearchTrendsService, PageEditDeltaService,
            PageViewDeltaService, taxonomy_search_category_qids,
        },
        core::{CoreServiceError, QidService},
    },
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
                    crate::services::core::CoreServiceError::DatabaseError(e) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Database error: {}", e),
                    ),
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
                crate::services::core::CoreServiceError::DatabaseError(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Database error: {}", e),
                ),
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

pub async fn get_category_trend_handler(
    Query(params): Query<CategoryTrendParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<CategoryTrendResponse>, ApiError> {
    let result = PageViewsService::get_category_trend(
        state,
        &params.wiki,
        &params.category,
        params.category_qid,
        params.depth,
        params.start_date,
        params.end_date,
    )
    .await?;

    let daily_views: Vec<DailyViews> = result
        .views
        .into_iter()
        .map(|(date, views)| DailyViews { date, views })
        .collect();

    let top_articles: Vec<TopArticle> = result
        .top_articles
        .into_iter()
        .map(|art| TopArticle {
            qid: art.qid,
            title: art.title,
            views: art.views,
            source_category_qid: art.source_category_qid,
            source_category_title: art.source_category_title,
            source_category_origin: art.source_category_origin,
        })
        .collect();

    Ok(Json(CategoryTrendResponse {
        qid: result.qid,
        title: result.title,
        views: daily_views,
        top_articles,
    }))
}

pub async fn get_article_trend_handler(
    Query(params): Query<ArticleTrendParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<ArticleTrendResponse>, ApiError> {
    let result = PageViewsService::get_article_trend(
        state,
        &params.wiki,
        &params.article,
        params.article_qid,
        params.start_date,
        params.end_date,
    )
    .await?;

    let daily_views = result
        .views
        .into_iter()
        .map(|(date, views)| DailyViews { date, views })
        .collect();

    Ok(Json(ArticleTrendResponse {
        qid: result.qid,
        title: result.title,
        views: daily_views,
    }))
}

pub async fn get_category_edit_trend_handler(
    Query(params): Query<CategoryTrendParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<CategoryEditTrendResponse>, ApiError> {
    let result = PageEditsService::get_category_edit_trend(
        state,
        &params.wiki,
        &params.category,
        params.category_qid,
        params.depth,
        params.start_date,
        params.end_date,
    )
    .await?;

    let daily_edits: Vec<DailyEdits> = result
        .edits
        .into_iter()
        .map(|(date, edits)| DailyEdits { date, edits })
        .collect();

    let top_articles: Vec<TopArticleEdits> = result
        .top_articles
        .into_iter()
        .map(|art| TopArticleEdits {
            qid: art.qid,
            title: art.title,
            edits: art.edits,
            source_category_qid: art.source_category_qid,
            source_category_title: art.source_category_title,
            source_category_origin: art.source_category_origin,
        })
        .collect();

    Ok(Json(CategoryEditTrendResponse {
        qid: result.qid,
        title: result.title,
        edits: daily_edits,
        top_articles,
    }))
}

pub async fn get_article_edit_trend_handler(
    Query(params): Query<ArticleTrendParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<ArticleEditTrendResponse>, ApiError> {
    let result = PageEditsService::get_article_edit_trend(
        state,
        &params.wiki,
        &params.article,
        params.article_qid,
        params.start_date,
        params.end_date,
    )
    .await?;

    let daily_edits = result
        .edits
        .into_iter()
        .map(|(date, edits)| DailyEdits { date, edits })
        .collect();

    Ok(Json(ArticleEditTrendResponse {
        qid: result.qid,
        title: result.title,
        edits: daily_edits,
    }))
}

pub async fn get_category_google_search_trend_handler(
    Query(params): Query<CategoryTrendParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<GoogleSearchCategoryTrendResponse>, ApiError> {
    let result = GoogleSearchTrendsService::get_category_trend(
        state,
        &params.wiki,
        &params.category,
        params.category_qid,
        params.depth,
        params.start_date,
        params.end_date,
    )
    .await?;

    let daily_search: Vec<DailyGoogleSearch> = result
        .search
        .into_iter()
        .map(|item| DailyGoogleSearch {
            date: item.date,
            clicks: item.clicks,
            impressions: item.impressions,
            ctr: item.ctr,
            position: item.position,
        })
        .collect();

    let top_articles: Vec<TopArticleGoogleSearch> = result
        .top_articles
        .into_iter()
        .map(|article| TopArticleGoogleSearch {
            qid: article.qid,
            title: article.title,
            clicks: article.clicks,
            impressions: article.impressions,
            ctr: article.ctr,
            source_category_qid: article.source_category_qid,
            source_category_title: article.source_category_title,
            source_category_origin: article.source_category_origin,
        })
        .collect();

    Ok(Json(GoogleSearchCategoryTrendResponse {
        qid: result.qid,
        title: result.title,
        search: daily_search,
        top_articles,
    }))
}

pub async fn get_article_google_search_trend_handler(
    Query(params): Query<ArticleTrendParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<GoogleSearchArticleTrendResponse>, ApiError> {
    let result = GoogleSearchTrendsService::get_article_trend(
        state,
        &params.wiki,
        &params.article,
        params.article_qid,
        params.start_date,
        params.end_date,
    )
    .await?;

    let daily_search: Vec<DailyGoogleSearch> = result
        .search
        .into_iter()
        .map(|item| DailyGoogleSearch {
            date: item.date,
            clicks: item.clicks,
            impressions: item.impressions,
            ctr: item.ctr,
            position: item.position,
        })
        .collect();

    Ok(Json(GoogleSearchArticleTrendResponse {
        qid: result.qid,
        title: result.title,
        search: daily_search,
    }))
}

pub async fn get_topic_pageview_trend_handler(
    Query(params): Query<TopicTrendParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<CategoryTrendResponse>, ApiError> {
    let result = PageViewsService::get_topic_trend(
        state,
        &params.wiki,
        &params.topic,
        params.depth,
        params.start_date,
        params.end_date,
    )
    .await?;

    let daily_views: Vec<DailyViews> = result
        .views
        .into_iter()
        .map(|(date, views)| DailyViews { date, views })
        .collect();

    let top_articles: Vec<TopArticle> = result
        .top_articles
        .into_iter()
        .map(|art| TopArticle {
            qid: art.qid,
            title: art.title,
            views: art.views,
            source_category_qid: art.source_category_qid,
            source_category_title: art.source_category_title,
            source_category_origin: art.source_category_origin,
        })
        .collect();

    Ok(Json(CategoryTrendResponse {
        qid: result.qid,
        title: result.title,
        views: daily_views,
        top_articles,
    }))
}

pub async fn get_topic_edit_trend_handler(
    Query(params): Query<TopicTrendParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<CategoryEditTrendResponse>, ApiError> {
    let result = PageEditsService::get_topic_edit_trend(
        state,
        &params.wiki,
        &params.topic,
        params.depth,
        params.start_date,
        params.end_date,
    )
    .await?;

    let daily_edits: Vec<DailyEdits> = result
        .edits
        .into_iter()
        .map(|(date, edits)| DailyEdits { date, edits })
        .collect();

    let top_articles: Vec<TopArticleEdits> = result
        .top_articles
        .into_iter()
        .map(|art| TopArticleEdits {
            qid: art.qid,
            title: art.title,
            edits: art.edits,
            source_category_qid: art.source_category_qid,
            source_category_title: art.source_category_title,
            source_category_origin: art.source_category_origin,
        })
        .collect();

    Ok(Json(CategoryEditTrendResponse {
        qid: result.qid,
        title: result.title,
        edits: daily_edits,
        top_articles,
    }))
}

pub async fn get_topic_google_search_trend_handler(
    Query(params): Query<TopicTrendParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<GoogleSearchCategoryTrendResponse>, ApiError> {
    let result = GoogleSearchTrendsService::get_topic_google_search_trend(
        state,
        &params.wiki,
        &params.topic,
        params.depth,
        params.start_date,
        params.end_date,
    )
    .await?;

    let daily_search: Vec<DailyGoogleSearch> = result
        .search
        .into_iter()
        .map(|item| DailyGoogleSearch {
            date: item.date,
            clicks: item.clicks,
            impressions: item.impressions,
            ctr: item.ctr,
            position: item.position,
        })
        .collect();

    let top_articles: Vec<TopArticleGoogleSearch> = result
        .top_articles
        .into_iter()
        .map(|article| TopArticleGoogleSearch {
            qid: article.qid,
            title: article.title,
            clicks: article.clicks,
            impressions: article.impressions,
            ctr: article.ctr,
            source_category_qid: article.source_category_qid,
            source_category_title: article.source_category_title,
            source_category_origin: article.source_category_origin,
        })
        .collect();

    Ok(Json(GoogleSearchCategoryTrendResponse {
        qid: result.qid,
        title: result.title,
        search: daily_search,
        top_articles,
    }))
}

pub async fn get_sub_categories(
    Query(params): Query<SubCategoryParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<std::collections::HashMap<u32, String>>, ApiError> {
    let titles_map = PageViewsService::get_sub_categories(
        state,
        &params.wiki,
        &params.category,
        params.category_qid,
    )
    .await?;

    Ok(Json(titles_map))
}

#[debug_handler]
pub async fn get_pageviews_top_categories_handler(
    Query(params): Query<TopCategoriesParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<CategoryRankResponse>, ApiError> {
    let categories = PageViewsService::get_top_categories(
        state,
        &params.wiki,
        params.start_date,
        params.end_date,
        params.top_n,
    )
    .await?;

    let top_categories_with_titles: Vec<TopCategory> = categories
        .into_iter()
        .map(|cat| {
            let top_articles: Vec<TopArticle> = cat
                .top_articles
                .into_iter()
                .map(|art| TopArticle {
                    qid: art.qid,
                    title: art.title,
                    views: art.views,
                    source_category_qid: art.source_category_qid,
                    source_category_title: art.source_category_title,
                    source_category_origin: art.source_category_origin,
                })
                .collect();

            TopCategory {
                qid: cat.qid,
                title: cat.title,
                views: cat.views,
                top_articles,
            }
        })
        .collect();

    let response = CategoryRankResponse {
        categories: top_categories_with_titles,
    };

    Ok(Json(response))
}

#[debug_handler]
pub async fn get_pageviews_top_articles_handler(
    Query(params): Query<TopCategoriesParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<PageViewTopArticlesResponse>, ApiError> {
    let articles = PageViewsService::get_top_articles_global(
        state,
        &params.wiki,
        params.start_date,
        params.end_date,
        params.top_n,
    )
    .await?;

    let response = PageViewTopArticlesResponse {
        articles: articles
            .into_iter()
            .map(|article| PageViewTopArticle {
                qid: article.qid,
                title: article.title,
                views: article.views,
                categories: article
                    .categories
                    .into_iter()
                    .map(|category| TopArticleCategory {
                        qid: category.qid,
                        title: category.title,
                    })
                    .collect(),
            })
            .collect(),
    };

    Ok(Json(response))
}

#[debug_handler]
pub async fn get_pageedits_top_categories_handler(
    Query(params): Query<TopCategoriesParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<CategoryEditRankResponse>, ApiError> {
    let categories = PageEditsService::get_top_categories(
        state,
        &params.wiki,
        params.start_date,
        params.end_date,
        params.top_n,
    )
    .await?;

    let response = CategoryEditRankResponse {
        categories: categories
            .into_iter()
            .map(|cat| TopCategoryByEdits {
                qid: cat.qid,
                title: cat.title,
                edits: cat.edits,
                top_articles: cat
                    .top_articles
                    .into_iter()
                    .map(|art| TopArticleByEdits {
                        qid: art.qid,
                        title: art.title,
                        edits: art.edits,
                    })
                    .collect(),
            })
            .collect(),
    };

    Ok(Json(response))
}

#[debug_handler]
pub async fn get_pageedits_top_articles_handler(
    Query(params): Query<TopCategoriesParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<PageEditTopArticlesResponse>, ApiError> {
    let articles = PageEditsService::get_top_articles_global(
        state,
        &params.wiki,
        params.start_date,
        params.end_date,
        params.top_n,
    )
    .await?;

    let response = PageEditTopArticlesResponse {
        articles: articles
            .into_iter()
            .map(|article| PageEditTopArticle {
                qid: article.qid,
                title: article.title,
                edits: article.edits,
                categories: article
                    .categories
                    .into_iter()
                    .map(|category| TopArticleCategory {
                        qid: category.qid,
                        title: category.title,
                    })
                    .collect(),
            })
            .collect(),
    };

    Ok(Json(response))
}

#[debug_handler]
pub async fn get_googlesearch_top_categories_handler(
    Query(params): Query<TopCategoriesParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<CategorySearchRankResponse>, ApiError> {
    let categories = GoogleSearchTrendsService::get_top_categories(
        state,
        &params.wiki,
        params.start_date,
        params.end_date,
        params.top_n,
    )
    .await?;

    let response = CategorySearchRankResponse {
        categories: categories
            .into_iter()
            .map(|cat| TopCategoryBySearch {
                qid: cat.qid,
                title: cat.title,
                clicks: cat.clicks,
                impressions: cat.impressions,
                ctr: cat.ctr,
                top_articles: cat
                    .top_articles
                    .into_iter()
                    .map(|art| TopArticleBySearch {
                        qid: art.qid,
                        title: art.title,
                        clicks: art.clicks,
                        impressions: art.impressions,
                        ctr: art.ctr,
                    })
                    .collect(),
            })
            .collect(),
    };

    Ok(Json(response))
}

#[debug_handler]
pub async fn get_googlesearch_top_articles_handler(
    Query(params): Query<TopCategoriesParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<GoogleSearchTopArticlesResponse>, ApiError> {
    let articles = GoogleSearchTrendsService::get_top_articles_global(
        state,
        &params.wiki,
        params.start_date,
        params.end_date,
        params.top_n,
    )
    .await?;

    let response = GoogleSearchTopArticlesResponse {
        articles: articles
            .into_iter()
            .map(|article| GoogleSearchTopArticle {
                qid: article.qid,
                title: article.title,
                clicks: article.clicks,
                impressions: article.impressions,
                ctr: article.ctr,
                categories: article
                    .categories
                    .into_iter()
                    .map(|category| TopArticleCategory {
                        qid: category.qid,
                        title: category.title,
                    })
                    .collect(),
            })
            .collect(),
    };

    Ok(Json(response))
}

pub async fn get_category_pageview_delta_handler(
    Query(params): Query<PageViewCategoryDeltaParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<PageViewCategoryDeltaResponse>, ApiError> {
    let limit = params.limit.unwrap_or(100) as usize;
    let depth = params.depth.unwrap_or(0);

    let delta_items = PageViewDeltaService::get_category_delta(
        Arc::clone(&state),
        &params.wiki,
        params.baseline_start_date,
        params.baseline_end_date,
        params.impact_start_date,
        params.impact_end_date,
        limit,
        depth,
    )
    .await?;

    let categories: Vec<crate::models::PageViewCategoryDeltaItemResponse> = delta_items
        .into_iter()
        .map(|item| crate::models::PageViewCategoryDeltaItemResponse {
            category_qid: item.category_qid,
            category_title: item.category_title,
            baseline_views: item.baseline_views,
            impact_views: item.impact_views,
            delta_percentage: item.delta_percentage,
            absolute_delta: item.absolute_delta,
        })
        .collect();

    let baseline_period = format!(
        "{} to {}",
        params.baseline_start_date, params.baseline_end_date
    );
    let impact_period = format!("{} to {}", params.impact_start_date, params.impact_end_date);

    Ok(Json(PageViewCategoryDeltaResponse {
        categories,
        baseline_period,
        impact_period,
    }))
}

pub async fn get_article_pageview_delta_handler(
    Query(params): Query<PageViewArticleDeltaParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<PageViewArticleDeltaResponse>, ApiError> {
    use crate::services::core::QidService;

    let limit = params.limit.unwrap_or(100) as usize;
    let depth = params.depth.unwrap_or(0);

    let delta_items = PageViewDeltaService::get_article_delta(
        Arc::clone(&state),
        &params.wiki,
        params.category_qid,
        params.baseline_start_date,
        params.baseline_end_date,
        params.impact_start_date,
        params.impact_end_date,
        limit,
        depth,
    )
    .await?;

    let articles: Vec<crate::models::PageViewArticleDeltaItemResponse> = delta_items
        .into_iter()
        .map(|item| crate::models::PageViewArticleDeltaItemResponse {
            article_qid: item.article_qid,
            article_title: item.article_title,
            baseline_views: item.baseline_views,
            impact_views: item.impact_views,
            delta_percentage: item.delta_percentage,
            absolute_delta: item.absolute_delta,
        })
        .collect();

    // Get category title
    let category_title =
        QidService::get_title_by_qid(Arc::clone(&state), &params.wiki, params.category_qid)
            .await
            .unwrap_or_else(|_| format!("Q{}", params.category_qid));

    let baseline_period = format!(
        "{} to {}",
        params.baseline_start_date, params.baseline_end_date
    );
    let impact_period = format!("{} to {}", params.impact_start_date, params.impact_end_date);

    Ok(Json(PageViewArticleDeltaResponse {
        articles,
        category_qid: params.category_qid,
        category_title,
        baseline_period,
        impact_period,
    }))
}

pub async fn get_category_pageedit_delta_handler(
    Query(params): Query<PageEditCategoryDeltaParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<PageEditCategoryDeltaResponse>, ApiError> {
    let limit = params.limit.unwrap_or(100) as usize;
    let depth = params.depth.unwrap_or(0);

    let delta_items = PageEditDeltaService::get_category_delta(
        Arc::clone(&state),
        &params.wiki,
        params.baseline_start_date,
        params.baseline_end_date,
        params.impact_start_date,
        params.impact_end_date,
        limit,
        depth,
    )
    .await?;

    let categories: Vec<crate::models::PageEditCategoryDeltaItemResponse> = delta_items
        .into_iter()
        .map(|item| crate::models::PageEditCategoryDeltaItemResponse {
            category_qid: item.category_qid,
            category_title: item.category_title,
            baseline_edits: item.baseline_edits,
            impact_edits: item.impact_edits,
            delta_percentage: item.delta_percentage,
            absolute_delta: item.absolute_delta,
        })
        .collect();

    let baseline_period = format!(
        "{} to {}",
        params.baseline_start_date, params.baseline_end_date
    );
    let impact_period = format!("{} to {}", params.impact_start_date, params.impact_end_date);

    Ok(Json(PageEditCategoryDeltaResponse {
        categories,
        baseline_period,
        impact_period,
    }))
}

pub async fn get_article_pageedit_delta_handler(
    Query(params): Query<PageEditArticleDeltaParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<PageEditArticleDeltaResponse>, ApiError> {
    use crate::services::core::QidService;

    let limit = params.limit.unwrap_or(100) as usize;
    let depth = params.depth.unwrap_or(0);

    let delta_items = PageEditDeltaService::get_article_delta(
        Arc::clone(&state),
        &params.wiki,
        params.category_qid,
        params.baseline_start_date,
        params.baseline_end_date,
        params.impact_start_date,
        params.impact_end_date,
        limit,
        depth,
    )
    .await?;

    let articles: Vec<crate::models::PageEditArticleDeltaItemResponse> = delta_items
        .into_iter()
        .map(|item| crate::models::PageEditArticleDeltaItemResponse {
            article_qid: item.article_qid,
            article_title: item.article_title,
            baseline_edits: item.baseline_edits,
            impact_edits: item.impact_edits,
            delta_percentage: item.delta_percentage,
            absolute_delta: item.absolute_delta,
        })
        .collect();

    // Get category title
    let category_title =
        QidService::get_title_by_qid(Arc::clone(&state), &params.wiki, params.category_qid)
            .await
            .unwrap_or_else(|_| format!("Q{}", params.category_qid));

    let baseline_period = format!(
        "{} to {}",
        params.baseline_start_date, params.baseline_end_date
    );
    let impact_period = format!("{} to {}", params.impact_start_date, params.impact_end_date);

    Ok(Json(PageEditArticleDeltaResponse {
        articles,
        category_qid: params.category_qid,
        category_title,
        baseline_period,
        impact_period,
    }))
}

pub async fn get_category_google_search_delta_handler(
    Query(params): Query<GoogleSearchCategoryDeltaParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<GoogleSearchCategoryDeltaResponse>, ApiError> {
    let limit = params.limit.unwrap_or(100) as usize;
    let depth = params.depth.unwrap_or(0);

    let delta_items = GoogleSearchDeltaService::get_category_delta(
        Arc::clone(&state),
        &params.wiki,
        params.baseline_start_date,
        params.baseline_end_date,
        params.impact_start_date,
        params.impact_end_date,
        limit,
        depth,
    )
    .await?;

    let categories: Vec<crate::models::GoogleSearchCategoryDeltaItemResponse> = delta_items
        .into_iter()
        .map(
            |item| crate::models::GoogleSearchCategoryDeltaItemResponse {
                category_qid: item.category_qid,
                category_title: item.category_title,
                baseline_clicks: item.baseline_clicks,
                impact_clicks: item.impact_clicks,
                baseline_impressions: item.baseline_impressions,
                impact_impressions: item.impact_impressions,
                delta_percentage: item.delta_percentage,
                absolute_delta: item.absolute_delta,
            },
        )
        .collect();

    let baseline_period = format!(
        "{} to {}",
        params.baseline_start_date, params.baseline_end_date
    );
    let impact_period = format!("{} to {}", params.impact_start_date, params.impact_end_date);

    Ok(Json(GoogleSearchCategoryDeltaResponse {
        categories,
        baseline_period,
        impact_period,
    }))
}

pub async fn get_article_google_search_delta_handler(
    Query(params): Query<GoogleSearchArticleDeltaParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<GoogleSearchArticleDeltaResponse>, ApiError> {
    let limit = params.limit.unwrap_or(100) as usize;
    let depth = params.depth.unwrap_or(0);

    let delta_items = GoogleSearchDeltaService::get_article_delta(
        Arc::clone(&state),
        &params.wiki,
        params.category_qid,
        params.baseline_start_date,
        params.baseline_end_date,
        params.impact_start_date,
        params.impact_end_date,
        limit,
        depth,
    )
    .await?;

    let articles: Vec<crate::models::GoogleSearchArticleDeltaItemResponse> = delta_items
        .into_iter()
        .map(|item| crate::models::GoogleSearchArticleDeltaItemResponse {
            article_qid: item.article_qid,
            article_title: item.article_title,
            baseline_clicks: item.baseline_clicks,
            impact_clicks: item.impact_clicks,
            baseline_impressions: item.baseline_impressions,
            impact_impressions: item.impact_impressions,
            delta_percentage: item.delta_percentage,
            absolute_delta: item.absolute_delta,
        })
        .collect();

    let category_title =
        QidService::get_title_by_qid(Arc::clone(&state), &params.wiki, params.category_qid)
            .await
            .unwrap_or_else(|_| format!("Q{}", params.category_qid));

    let baseline_period = format!(
        "{} to {}",
        params.baseline_start_date, params.baseline_end_date
    );
    let impact_period = format!("{} to {}", params.impact_start_date, params.impact_end_date);

    Ok(Json(GoogleSearchArticleDeltaResponse {
        articles,
        category_qid: params.category_qid,
        category_title,
        baseline_period,
        impact_period,
    }))
}

pub async fn search_categories(
    Query(params): Query<CategorySearchParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<CategorySearchResponse>, ApiError> {
    let limit: u64 = params.limit.unwrap_or(1000u64);
    let match_threshold = params.match_threshold.unwrap_or(0.6);

    let search_results: Vec<topictrend_taxonomy::SearchResult> = topictrend_taxonomy::search(
        params.query.clone(),
        "enwiki".to_string(),
        limit,
        match_threshold,
    )
    .await
    .map_err(|e| {
        ApiError::ServiceError(crate::services::ServiceError::CoreError(
            crate::services::core::CoreServiceError::InternalError(e.to_string()),
        ))
    })?;

    let mut categories: Vec<CategorySearchItemResponse> = search_results
        .into_iter()
        .map(|result| CategorySearchItemResponse {
            category_qid: result.qid,
            category_title_en: result.page_title,
            category_title: "".to_string(),
            match_score: result.score,
        })
        .collect();

    if params.wiki != "enwiki" {
        let qids: Vec<u32> = categories.iter().map(|cat| cat.category_qid).collect();

        let titles_in_target_wiki =
            QidService::get_titles_by_qids(Arc::clone(&state), &params.wiki, &qids)
                .await
                .unwrap_or_default();

        categories.retain_mut(|category| {
            if let Some(title) = titles_in_target_wiki.get(&category.category_qid) {
                category.category_title = title.clone();
                true
            } else {
                false
            }
        });
    } else {
        for category in &mut categories {
            category.category_title = category.category_title_en.clone();
        }
    }

    Ok(Json(CategorySearchResponse { categories }))
}

pub async fn get_categories_trend_by_search_handler(
    Query(params): Query<CategoriesTrendParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<CategoriesTrendResponse>, ApiError> {
    let limit: u64 = params.limit.unwrap_or(1000u64);
    let match_threshold = params.match_threshold.unwrap_or(0.6);
    let search_results: Vec<topictrend_taxonomy::SearchResult> = topictrend_taxonomy::search(
        params.category_query.clone(),
        "enwiki".to_string(),
        limit,
        match_threshold,
    )
    .await
    .map_err(|e| {
        ApiError::ServiceError(crate::services::ServiceError::CoreError(
            crate::services::core::CoreServiceError::InternalError(e.to_string()),
        ))
    })?;

    let category_qids: Vec<u32> = search_results
        .into_iter()
        .map(|result| result.qid)
        .collect();

    let result = PageViewsService::get_categories_trend(
        state,
        &params.wiki,
        category_qids,
        Some(1u32),
        params.start_date,
        params.end_date,
    )
    .await?;

    let cumulative_views: Vec<DailyViews> = result
        .cumulative_views
        .into_iter()
        .map(|(date, views)| DailyViews { date, views })
        .collect();

    let top_articles: Vec<TopArticle> = result
        .top_articles
        .into_iter()
        .map(|art| TopArticle {
            qid: art.qid,
            title: art.title,
            views: art.views,
            source_category_qid: art.source_category_qid,
            source_category_title: art.source_category_title,
            source_category_origin: art.source_category_origin,
        })
        .collect();

    let categories: Vec<crate::models::CategoryInfo> = result
        .categories
        .into_iter()
        .map(|cat| crate::models::CategoryInfo {
            qid: cat.qid,
            title: cat.title,
        })
        .collect();

    Ok(Json(CategoriesTrendResponse {
        categories,
        cumulative_views,
        top_articles,
    }))
}

pub async fn get_articles_in_category(
    Query(params): Query<ListArticlesInCategoryParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<ArticlesInCategoryResponse>, ApiError> {
    let category_qid = if let Some(qid) = params.category_qid {
        qid
    } else {
        let category = params.category.ok_or_else(|| {
            CoreServiceError::InternalError(
                "Either category or category_qid must be provided".to_string(),
            )
        })?;
        QidService::get_qid_by_title(Arc::clone(&state), params.wiki.as_str(), &category, 14)
            .await?
    };

    // Get all articles in the category (depth 0 = direct members only)
    let article_qids = CategoryService::get_category_articles(
        Arc::clone(&state),
        params.wiki.as_str(),
        category_qid,
        0,
    )
    .await?;

    // Get titles for all articles
    let titles_map =
        QidService::get_titles_by_qids(Arc::clone(&state), params.wiki.as_str(), &article_qids)
            .await?;

    // Get view data for each article
    let mut articles_in_category = Vec::new();

    for article_qid in article_qids {
        let title = titles_map
            .get(&article_qid)
            .cloned()
            .unwrap_or_else(|| format!("Q{}", article_qid));

        articles_in_category.push(ArticleItem {
            qid: article_qid,
            title,
        });
    }
    Ok(Json(ArticlesInCategoryResponse {
        articles: articles_in_category,
    }))
}

pub async fn get_content_gap_handler(
    Query(params): Query<ContentGapParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<ContentGapResult>, ApiError> {
    let depth = params.depth.unwrap_or(0);

    let wikis: Vec<String> = params
        .wikis
        .split(',')
        .map(|wiki| wiki.trim())
        .filter(|wiki| !wiki.is_empty())
        .map(|wiki| wiki.to_string())
        .collect();

    let category_qid = if let Some(qid) = params.category_qid {
        qid
    } else {
        let category = params.category.as_ref().ok_or_else(|| {
            CoreServiceError::InternalError(
                "Either category or category_qid must be provided".to_string(),
            )
        })?;
        QidService::get_qid_by_title(Arc::clone(&state), "enwiki", category, 14).await?
    };

    let category_label = params
        .category
        .clone()
        .unwrap_or_else(|| format!("Q{}", category_qid));

    let result =
        ContentGapService::get_content_gap(state, category_qid, &category_label, wikis, depth)
            .await?;

    Ok(Json(result))
}

pub async fn get_content_gap_topic_handler(
    Query(params): Query<ContentGapTopicParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<ContentGapResult>, ApiError> {
    let wikis: Vec<String> = params
        .wikis
        .split(',')
        .map(|wiki| wiki.trim())
        .filter(|wiki| !wiki.is_empty())
        .map(|wiki| wiki.to_string())
        .collect();

    let result =
        ContentGapService::get_topic_content_gap(state, &params.topic, wikis, params.depth)
            .await?;

    Ok(Json(result))
}
