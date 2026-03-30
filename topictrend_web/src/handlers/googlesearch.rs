use axum::{
    Json,
    extract::{Query, State},
};
use std::sync::Arc;

use crate::services::composite::GoogleSearchTrendsService;
use crate::models::{
    AppState, ArticleTrendParams, GoogleSearchArticleTrendResponse, CategoryTrendParams,
    GoogleSearchCategoryTrendResponse, DailyGoogleSearch, TopArticleGoogleSearch, TopCategoriesParams,
    CategorySearchRankResponse, TopCategoryBySearch, TopArticleBySearch, GoogleSearchTopArticlesResponse,
    GoogleSearchTopArticle, TopArticleCategory, TopicTrendParams,
};

use super::ApiError;

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
            source_categories: article.source_categories.into_iter()
                .map(|(qid, title)| TopArticleCategory { qid, title })
                .collect(),
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
            source_categories: article.source_categories.into_iter()
                .map(|(qid, title)| TopArticleCategory { qid, title })
                .collect(),
        })
        .collect();

    Ok(Json(GoogleSearchCategoryTrendResponse {
        qid: result.qid,
        title: result.title,
        search: daily_search,
        top_articles,
    }))
}

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
