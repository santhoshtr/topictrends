use axum::{
    Json,
    extract::{Query, State},
};
use std::sync::Arc;

use crate::services::PageEditsService;
use crate::services::core::QidService;
use crate::models::{
    AppState, ArticleTrendParams, ArticleEditTrendResponse, CategoryTrendParams,
    CategoryEditTrendResponse, DailyEdits, TopArticleEdits, TopCategoriesParams,
    CategoryEditRankResponse, TopCategoryByEdits, TopArticleByEdits, PageEditTopArticlesResponse,
    PageEditTopArticle, TopArticleCategory,
};

use super::ApiError;

pub async fn get_category_edit_trend_handler(
    Query(params): Query<CategoryTrendParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<CategoryEditTrendResponse>, ApiError> {
    let result = PageEditsService::get_category_edit_trend(
        Arc::clone(&state),
        &params.wiki,
        &params.category,
        params.category_qid,
        params.start_date,
        params.end_date,
    )
    .await?;

    let mut en_qids: Vec<u32> = vec![result.qid];
    for art in &result.top_articles {
        en_qids.push(art.qid);
        en_qids.extend(art.source_categories.iter().map(|(qid, _)| *qid));
    }
    let en = QidService::get_english_titles(state, &params.wiki, &en_qids).await;

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
            title_en: en.get(&art.qid).cloned(),
            title: art.title,
            edits: art.edits,
            source_categories: art.source_categories.into_iter()
                .map(|(qid, title)| TopArticleCategory {
                    qid,
                    title,
                    title_en: en.get(&qid).cloned(),
                })
                .collect(),
        })
        .collect();

    Ok(Json(CategoryEditTrendResponse {
        qid: result.qid,
        title_en: en.get(&result.qid).cloned(),
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
        Arc::clone(&state),
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

    let en = QidService::get_english_titles(state, &params.wiki, &[result.qid]).await;

    Ok(Json(ArticleEditTrendResponse {
        qid: result.qid,
        title_en: en.get(&result.qid).cloned(),
        title: result.title,
        edits: daily_edits,
    }))
}

pub async fn get_pageedits_top_categories_handler(
    Query(params): Query<TopCategoriesParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<CategoryEditRankResponse>, ApiError> {
    let categories = PageEditsService::get_top_categories(
        Arc::clone(&state),
        &params.wiki,
        params.start_date,
        params.end_date,
        params.top_n,
    )
    .await?;

    let mut en_qids: Vec<u32> = Vec::new();
    for cat in &categories {
        en_qids.push(cat.qid);
        en_qids.extend(cat.top_articles.iter().map(|a| a.qid));
    }
    let en = QidService::get_english_titles(state, &params.wiki, &en_qids).await;

    let response = CategoryEditRankResponse {
        categories: categories
            .into_iter()
            .map(|cat| TopCategoryByEdits {
                qid: cat.qid,
                title_en: en.get(&cat.qid).cloned(),
                title: cat.title,
                edits: cat.edits,
                top_articles: cat
                    .top_articles
                    .into_iter()
                    .map(|art| TopArticleByEdits {
                        qid: art.qid,
                        title_en: en.get(&art.qid).cloned(),
                        title: art.title,
                        edits: art.edits,
                    })
                    .collect(),
            })
            .collect(),
    };

    Ok(Json(response))
}

pub async fn get_pageedits_top_articles_handler(
    Query(params): Query<TopCategoriesParams>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<PageEditTopArticlesResponse>, ApiError> {
    let articles = PageEditsService::get_top_articles_global(
        Arc::clone(&state),
        &params.wiki,
        params.start_date,
        params.end_date,
        params.top_n,
    )
    .await?;

    let mut en_qids: Vec<u32> = Vec::new();
    for art in &articles {
        en_qids.push(art.qid);
        en_qids.extend(art.categories.iter().map(|c| c.qid));
    }
    let en = QidService::get_english_titles(state, &params.wiki, &en_qids).await;

    let response = PageEditTopArticlesResponse {
        articles: articles
            .into_iter()
            .map(|article| PageEditTopArticle {
                qid: article.qid,
                title_en: en.get(&article.qid).cloned(),
                title: article.title,
                edits: article.edits,
                categories: article
                    .categories
                    .into_iter()
                    .map(|category| TopArticleCategory {
                        qid: category.qid,
                        title_en: en.get(&category.qid).cloned(),
                        title: category.title,
                    })
                    .collect(),
            })
            .collect(),
    };

    Ok(Json(response))
}
