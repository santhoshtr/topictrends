use axum::{
    Json,
    extract::{Query, State},
};
use std::sync::Arc;

use crate::services::{
    composite::{
        GoogleSearchDeltaService, PageEditDeltaService, PageViewDeltaService,
    },
    core::QidService,
};
use crate::models::{
    AppState, PageViewCategoryDeltaParams, PageViewCategoryDeltaResponse,
    PageViewArticleDeltaParams, PageViewArticleDeltaResponse, PageEditCategoryDeltaParams,
    PageEditCategoryDeltaResponse, PageEditArticleDeltaParams, PageEditArticleDeltaResponse,
    GoogleSearchCategoryDeltaParams, GoogleSearchCategoryDeltaResponse,
    GoogleSearchArticleDeltaParams, GoogleSearchArticleDeltaResponse,
};

use super::ApiError;

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
