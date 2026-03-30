use axum::{
    Json,
    extract::{Query, State},
};
use std::sync::Arc;

use crate::services::{
    ContentGapService, PageViewsService,
    core::{QidService, CategoryService, CoreServiceError},
};
use crate::models::{
    AppState, CategorySearchParams, CategorySearchResponse, CategorySearchItemResponse,
    CategoriesTrendParams, CategoriesTrendResponse, ListArticlesInCategoryParams,
    ArticlesInCategoryResponse, ArticleItem, ContentGapParams, ContentGapResult,
    ContentGapTopicParams, DailyViews, TopArticle,
};

use super::ApiError;

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
