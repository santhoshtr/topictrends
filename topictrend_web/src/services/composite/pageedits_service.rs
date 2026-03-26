use crate::models::AppState;
use crate::services::composite::source_attribution::resolve_source_categories;
use crate::services::composite::taxonomy_search_category_qids;
use crate::services::core::{CoreServiceError, EngineService, PageEditService, QidService};
use chrono::NaiveDate;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub struct PageEditsService;

pub struct CategoryEditTrendResult {
    pub qid: u32,
    pub title: String,
    pub edits: Vec<(NaiveDate, u64)>,
    pub top_articles: Vec<ArticleEditRank>,
}

pub struct ArticleEditTrendResult {
    pub qid: u32,
    pub title: String,
    pub edits: Vec<(NaiveDate, u64)>,
}

pub struct ArticleEditRank {
    pub qid: u32,
    pub title: String,
    pub edits: u64,
    pub source_category_qid: Option<u32>,
    pub source_category_title: Option<String>,
    pub source_category_origin: Option<String>,
}

pub struct CategoryEditRank {
    pub qid: u32,
    pub title: String,
    pub edits: u64,
    pub top_articles: Vec<ArticleEditRank>,
}

impl PageEditsService {
    pub async fn get_top_categories(
        state: Arc<AppState>,
        wiki: &str,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        top_n: Option<u32>,
    ) -> Result<Vec<CategoryEditRank>, CoreServiceError> {
        let top_n = top_n.unwrap_or(10);
        let start = start_date
            .unwrap_or_else(|| chrono::Local::now().date_naive() - chrono::Duration::days(30));
        let end = end_date.unwrap_or_else(|| chrono::Local::now().date_naive());

        let categories = PageEditService::get_top_categories(
            Arc::clone(&state),
            wiki,
            start,
            end,
            top_n as usize,
        )
        .await?;

        let mut all_qids = Vec::new();
        for cat in &categories {
            all_qids.push(cat.category_qid);
            for art in &cat.top_articles {
                all_qids.push(art.article_qid);
            }
        }

        let titles_map: HashMap<u32, String> =
            QidService::get_titles_by_qids(Arc::clone(&state), wiki, &all_qids).await?;

        let result = categories
            .into_iter()
            .map(|cat| {
                let title = titles_map
                    .get(&cat.category_qid)
                    .cloned()
                    .unwrap_or_else(|| format!("Q{}", cat.category_qid));

                let top_articles = cat
                    .top_articles
                    .into_iter()
                    .map(|art| {
                        let art_title = titles_map
                            .get(&art.article_qid)
                            .cloned()
                            .unwrap_or_else(|| format!("Q{}", art.article_qid));
                        ArticleEditRank {
                            qid: art.article_qid,
                            title: art_title,
                            edits: art.total_edits,
                            source_category_qid: Some(cat.category_qid),
                            source_category_title: Some(title.clone()),
                            source_category_origin: None,
                        }
                    })
                    .collect();

                CategoryEditRank {
                    qid: cat.category_qid,
                    title,
                    edits: cat.total_edits,
                    top_articles,
                }
            })
            .collect();

        Ok(result)
    }

    pub async fn get_category_edit_trend(
        state: Arc<AppState>,
        wiki: &str,
        category: &str,
        category_qid: Option<u32>,
        depth: Option<u32>,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> Result<CategoryEditTrendResult, CoreServiceError> {
        let depth = depth.unwrap_or(0);
        let start = start_date
            .unwrap_or_else(|| chrono::Local::now().date_naive() - chrono::Duration::days(30));
        let end = end_date.unwrap_or_else(|| chrono::Local::now().date_naive());

        let category_qid = if let Some(qid) = category_qid {
            qid
        } else {
            QidService::get_qid_by_title(Arc::clone(&state), wiki, category, 14).await?
        };

        // Get raw pageedit data
        let data = PageEditService::get_category_edits(
            Arc::clone(&state),
            wiki,
            category_qid,
            start,
            end,
            depth,
        )
        .await?;

        // Get top articles
        let top_articles = PageEditService::get_top_articles(
            Arc::clone(&state),
            wiki,
            category_qid,
            start,
            end,
            depth,
            10,
        )
        .await?;

        // Get titles for articles
        let article_qids: Vec<u32> = top_articles.iter().map(|a| a.article_qid).collect();

        let titles_map =
            QidService::get_titles_by_qids(Arc::clone(&state), wiki, &article_qids).await?;

        let top_articles: Vec<ArticleEditRank> = top_articles
            .into_iter()
            .map(|art| {
                let article_title = titles_map
                    .get(&art.article_qid)
                    .cloned()
                    .unwrap_or_else(|| format!("Q{}", art.article_qid));

                ArticleEditRank {
                    qid: art.article_qid,
                    title: article_title,
                    edits: art.total_edits,
                    source_category_qid: Some(category_qid),
                    source_category_title: Some(category.to_string()),
                    source_category_origin: None,
                }
            })
            .collect();

        // Get category title
        let category_title = QidService::get_title_by_qid(Arc::clone(&state), wiki, category_qid)
            .await
            .unwrap_or_else(|_| category.to_string());

        Ok(CategoryEditTrendResult {
            qid: category_qid,
            title: category_title,
            edits: data,
            top_articles,
        })
    }

    pub async fn get_article_edit_trend(
        state: Arc<AppState>,
        wiki: &str,
        article: &str,
        article_qid: Option<u32>,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> Result<ArticleEditTrendResult, CoreServiceError> {
        let start = start_date
            .unwrap_or_else(|| chrono::Local::now().date_naive() - chrono::Duration::days(30));
        let end = end_date.unwrap_or_else(|| chrono::Local::now().date_naive());

        let article_qid = if let Some(qid) = article_qid {
            qid
        } else {
            QidService::get_qid_by_title(Arc::clone(&state), wiki, article, 0).await?
        };

        // Get raw pageedit data
        let data =
            PageEditService::get_article_edits(Arc::clone(&state), wiki, article_qid, start, end)
                .await?;

        // Get article title
        let article_title = QidService::get_title_by_qid(Arc::clone(&state), wiki, article_qid)
            .await
            .unwrap_or_else(|_| article.to_string());

        Ok(ArticleEditTrendResult {
            qid: article_qid,
            title: article_title,
            edits: data,
        })
    }

    pub async fn get_topic_edit_trend(
        state: Arc<AppState>,
        wiki: &str,
        topic: &str,
        depth: Option<u32>,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> Result<CategoryEditTrendResult, CoreServiceError> {
        let start = start_date
            .unwrap_or_else(|| chrono::Local::now().date_naive() - chrono::Duration::days(30));
        let end = end_date.unwrap_or_else(|| chrono::Local::now().date_naive());

        let category_qids = taxonomy_search_category_qids(topic).await?;
        let category_qid_set: HashSet<u32> = category_qids.iter().copied().collect();
        let mut all_edits_by_date: HashMap<NaiveDate, u64> = HashMap::new();
        let mut all_articles: HashMap<u32, (u64, u64, u32)> = HashMap::new();
        let category_titles =
            QidService::get_titles_by_qids(Arc::clone(&state), wiki, &category_qids).await?;

        {
            let engine =
                EngineService::get_or_build_pageedit_engine(Arc::clone(&state), wiki).await?;
            let engine_lock = engine.read().map_err(|e| {
                CoreServiceError::InternalError(format!("Failed to acquire read lock: {}", e))
            })?;

            let effective_depth = depth.unwrap_or(1);
            for qid in &category_qids {
                let edits_data = engine_lock.get_category_trend(*qid, effective_depth, start, end);
                for (date, edits) in edits_data {
                    *all_edits_by_date.entry(date).or_insert(0) += edits;
                }

                let top_articles = engine_lock
                    .get_top_articles_in_category(*qid, start, end, effective_depth, 50)
                    .map_err(|e| {
                        CoreServiceError::EngineError(format!("Failed to get top articles: {}", e))
                    })?
                    .top_articles;

                for article in top_articles {
                    let entry = all_articles
                        .entry(article.article_qid)
                        .or_insert((0, 0, *qid));
                    entry.0 += article.total_edits;
                    if article.total_edits > entry.1 {
                        entry.1 = article.total_edits;
                        entry.2 = *qid;
                    }
                }
            }
        }

        let mut edits: Vec<(NaiveDate, u64)> = all_edits_by_date.into_iter().collect();
        edits.sort_by_key(|(date, _)| *date);

        let mut article_totals: Vec<(u32, (u64, u64, u32))> = all_articles.into_iter().collect();
        article_totals.sort_by(|a, b| b.1.0.cmp(&a.1.0));
        article_totals.truncate(10);

        let article_qids: Vec<u32> = article_totals.iter().map(|(qid, _)| *qid).collect();
        let top_article_qid_set: HashSet<u32> = article_qids.iter().copied().collect();

        let mut article_category_edits: HashMap<u32, HashMap<u32, u64>> = HashMap::new();
        if !top_article_qid_set.is_empty() {
            let engine =
                EngineService::get_or_build_pageedit_engine(Arc::clone(&state), wiki).await?;
            let engine_lock = engine.read().map_err(|e| {
                CoreServiceError::InternalError(format!("Failed to acquire read lock: {}", e))
            })?;

            let effective_depth = depth.unwrap_or(1);
            for qid in &category_qids {
                let top_articles = engine_lock
                    .get_top_articles_in_category(*qid, start, end, effective_depth, 50)
                    .map_err(|e| {
                        CoreServiceError::EngineError(format!("Failed to get top articles: {}", e))
                    })?
                    .top_articles;

                for article in top_articles {
                    if !top_article_qid_set.contains(&article.article_qid) {
                        continue;
                    }

                    let per_article_category_edits = article_category_edits
                        .entry(article.article_qid)
                        .or_default();
                    *per_article_category_edits.entry(*qid).or_insert(0) += article.total_edits;
                }
            }
        }

        let fallback_source_by_article: HashMap<u32, u32> = article_totals
            .iter()
            .map(|(qid, (_, _, fallback_source_category_qid))| {
                (*qid, *fallback_source_category_qid)
            })
            .collect();

        let titles_map = if article_qids.is_empty() {
            HashMap::new()
        } else {
            QidService::get_titles_by_qids(Arc::clone(&state), wiki, &article_qids).await?
        };

        let source_by_article = resolve_source_categories(
            Arc::clone(&state),
            wiki,
            &article_qids,
            &category_qid_set,
            &article_category_edits,
            &fallback_source_by_article,
        )
        .await?;

        let top_articles: Vec<ArticleEditRank> = article_totals
            .into_iter()
            .map(|(qid, (edits, _, fallback_source_category_qid))| {
                let resolved_source = source_by_article.get(&qid).copied();
                let source_category_qid = resolved_source
                    .map(|source| source.category_qid)
                    .unwrap_or(fallback_source_category_qid);
                let title = titles_map
                    .get(&qid)
                    .cloned()
                    .unwrap_or_else(|| format!("Q{}", qid));
                let source_category_title = category_titles
                    .get(&source_category_qid)
                    .cloned()
                    .unwrap_or_else(|| format!("Q{}", source_category_qid));
                ArticleEditRank {
                    qid,
                    title,
                    edits,
                    source_category_qid: Some(source_category_qid),
                    source_category_title: Some(source_category_title),
                    source_category_origin: resolved_source
                        .map(|source| source.origin.as_str().to_string()),
                }
            })
            .collect();

        Ok(CategoryEditTrendResult {
            qid: 0,
            title: topic.to_string(),
            edits,
            top_articles,
        })
    }
}
