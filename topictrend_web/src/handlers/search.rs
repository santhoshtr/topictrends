use axum::{
    Json,
    extract::{Query, State},
};
use std::sync::Arc;

use crate::models::{
    AppState, ArticleCategoriesResponse, ArticleItem, ArticlesInCategoryResponse,
    CategorySearchItemResponse, CategorySearchParams, CategorySearchResponse, ContentGapParams,
    ContentGapResult, ListArticleCategoriesParams, ListArticlesInCategoryParams,
};
use crate::services::{
    ContentGapService,
    core::{ArticleService, CategoryService, CoreServiceError, CoverageService, QidService},
};

use super::ApiError;

pub async fn search_categories(
    Query(params): Query<CategorySearchParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<CategorySearchResponse>, ApiError> {
    let limit: u64 = params.limit.unwrap_or(100u64);
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
        // Keep only categories actually populated in the target wiki's
        // canonical graph (qid_overlap > 0 in its coverage snapshot). A
        // local category *page* is not required — the canonical projection
        // populates categories the wiki never created. If no snapshot
        // exists, serve the matches unfiltered rather than failing.
        match CoverageService::get_or_load_snapshot(Arc::clone(&state), &params.wiki).await {
            Ok(snapshot) => {
                categories.retain(|c| snapshot.matrix.get(c.category_qid).1 > 0);
            }
            Err(e) => {
                tracing::warn!(
                    "no coverage snapshot for {}: {:?}; serving unfiltered category matches",
                    params.wiki,
                    e
                );
            }
        }

        let qids: Vec<u32> = categories.iter().map(|cat| cat.category_qid).collect();
        let titles_in_target_wiki =
            QidService::get_titles_by_qids(Arc::clone(&state), &params.wiki, &qids)
                .await
                .unwrap_or_default();

        for category in &mut categories {
            category.category_title = titles_in_target_wiki
                .get(&category.category_qid)
                .cloned()
                .unwrap_or_else(|| category.category_title_en.clone());
        }
    } else {
        for category in &mut categories {
            category.category_title = category.category_title_en.clone();
        }
    }

    Ok(Json(CategorySearchResponse { categories }))
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
        params.min_agreement.unwrap_or(1),
    )
    .await?;

    // Get titles for all articles
    let titles_map =
        QidService::get_titles_by_qids(Arc::clone(&state), params.wiki.as_str(), &article_qids)
            .await?;
    let en = QidService::get_english_titles(Arc::clone(&state), &params.wiki, &article_qids).await;

    let mut articles_in_category = Vec::new();

    for article_qid in article_qids {
        let title = titles_map
            .get(&article_qid)
            .cloned()
            .unwrap_or_else(|| format!("Q{}", article_qid));

        articles_in_category.push(ArticleItem {
            qid: article_qid,
            title,
            title_en: en.get(&article_qid).cloned(),
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
        ContentGapService::get_content_gap(state, category_qid, &category_label, wikis).await?;

    Ok(Json(result))
}

pub async fn get_article_categories(
    Query(params): Query<ListArticleCategoriesParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<ArticleCategoriesResponse>, ApiError> {
    let article_qid = if let Some(qid) = params.article_qid {
        qid
    } else {
        let article = params.article.ok_or_else(|| {
            CoreServiceError::InternalError(
                "Either article or article_qid must be provided".to_string(),
            )
        })?;
        QidService::get_qid_by_title(Arc::clone(&state), params.wiki.as_str(), &article, 0).await?
    };

    let ranked = ArticleService::get_article_categories(
        Arc::clone(&state),
        params.wiki.as_str(),
        article_qid,
    )
    .await?;

    let category_qids: Vec<u32> = ranked.iter().map(|(qid, _)| *qid).collect();
    let titles_map =
        QidService::get_titles_by_qids(Arc::clone(&state), params.wiki.as_str(), &category_qids)
            .await?;
    let en = QidService::get_english_titles(Arc::clone(&state), &params.wiki, &category_qids).await;

    let categories: Vec<crate::models::RankedCategoryInfo> = ranked
        .into_iter()
        .map(|(qid, wiki_count)| crate::models::RankedCategoryInfo {
            qid,
            title: titles_map
                .get(&qid)
                .cloned()
                .unwrap_or_else(|| format!("Q{}", qid)),
            title_en: en.get(&qid).cloned(),
            wiki_count,
        })
        .collect();

    Ok(Json(ArticleCategoriesResponse { categories }))
}
