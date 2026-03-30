use axum::{
    Json,
    extract::{Query, State},
};
use std::sync::Arc;

use crate::services::PageViewsService;
use crate::models::{
    AppState, ArticleTrendParams, ArticleTrendResponse, CategoryTrendParams,
    CategoryTrendResponse, DailyViews, SubCategoryParams, TopArticle, TopCategoriesParams,
    CategoryRankResponse, TopCategory, PageViewTopArticlesResponse, PageViewTopArticle,
    TopArticleCategory, TopicTrendParams,
};

use super::ApiError;

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
