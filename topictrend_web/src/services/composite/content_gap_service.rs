use crate::models::{AppState, ContentGapResult, ContentGapWikiResult};
use crate::services::core::{CoreServiceError, EngineService};
use crate::services::composite::taxonomy_search_category_qids;
use chrono::NaiveDate;
use std::sync::Arc;

/// Last-month totals shown as subtle subtext under each metric icon.
#[derive(Default)]
struct MetricTotals {
    pageviews: u64,
    edits: u64,
    gsc_clicks: u64,
    gsc_impressions: u64,
}

pub struct ContentGapService;

impl ContentGapService {
    pub async fn get_content_gap(
        state: Arc<AppState>,
        category_qid: u32,
        category_label: &str,
        wikis: Vec<String>,
        depth: u32,
    ) -> Result<ContentGapResult, CoreServiceError> {
        let (start, end) = last_month_range();
        let mut results: Vec<ContentGapWikiResult> = Vec::new();

        for wiki in &wikis {
            let graph = EngineService::get_or_build_graph_engine(Arc::clone(&state), wiki).await?;
            let article_count = graph
                .get_articles_in_category(category_qid, depth)
                .map_err(CoreServiceError::EngineError)?
                .len();

            let totals =
                metric_totals(Arc::clone(&state), wiki, &[category_qid], depth, start, end).await;

            results.push(ContentGapWikiResult {
                wiki: wiki.clone(),
                article_count,
                pageviews_last_month: totals.pageviews,
                edits_last_month: totals.edits,
                gsc_clicks_last_month: totals.gsc_clicks,
                gsc_impressions_last_month: totals.gsc_impressions,
            });
        }

        Ok(ContentGapResult {
            category: category_label.to_string(),
            category_qid,
            depth,
            wikis: results,
        })
    }

    pub async fn get_topic_content_gap(
        state: Arc<AppState>,
        topic: &str,
        wikis: Vec<String>,
        depth: Option<u32>,
    ) -> Result<ContentGapResult, CoreServiceError> {
        let effective_depth = depth.unwrap_or(0);
        let category_qids = taxonomy_search_category_qids(topic).await?;
        let (start, end) = last_month_range();

        let mut results: Vec<ContentGapWikiResult> = Vec::new();

        for wiki in &wikis {
            let graph = EngineService::get_or_build_graph_engine(Arc::clone(&state), wiki).await?;
            let mut total_article_count = 0;

            for qid in &category_qids {
                let articles = graph
                    .get_articles_in_category(*qid, effective_depth)
                    .map_err(CoreServiceError::EngineError)?;
                total_article_count += articles.len();
            }

            // Like article_count above, totals sum across the matched categories;
            // articles shared by sibling categories are double-counted.
            let totals = metric_totals(
                Arc::clone(&state),
                wiki,
                &category_qids,
                effective_depth,
                start,
                end,
            )
            .await;

            results.push(ContentGapWikiResult {
                wiki: wiki.clone(),
                article_count: total_article_count,
                pageviews_last_month: totals.pageviews,
                edits_last_month: totals.edits,
                gsc_clicks_last_month: totals.gsc_clicks,
                gsc_impressions_last_month: totals.gsc_impressions,
            });
        }

        Ok(ContentGapResult {
            category: topic.to_string(),
            category_qid: 0,
            depth: effective_depth,
            wikis: results,
        })
    }
}

/// Trailing 30-day window, matching the default range used by the trends services.
fn last_month_range() -> (NaiveDate, NaiveDate) {
    let end = chrono::Local::now().date_naive();
    (end - chrono::Duration::days(30), end)
}

/// Sum each metric over the given categories and date range for one wiki.
/// A missing engine (e.g. no GSC data for the wiki) contributes 0 rather than
/// failing the whole response — only MariaDB is a hard dependency.
async fn metric_totals(
    state: Arc<AppState>,
    wiki: &str,
    category_qids: &[u32],
    depth: u32,
    start: NaiveDate,
    end: NaiveDate,
) -> MetricTotals {
    let mut totals = MetricTotals::default();

    if let Ok(engine) = EngineService::get_or_build_pageview_engine(Arc::clone(&state), wiki).await {
        if let Ok(lock) = engine.read() {
            for qid in category_qids {
                totals.pageviews += lock
                    .get_category_trend(*qid, depth, start, end)
                    .iter()
                    .map(|(_, views)| views)
                    .sum::<u64>();
            }
        }
    }

    if let Ok(engine) = EngineService::get_or_build_pageedit_engine(Arc::clone(&state), wiki).await {
        if let Ok(lock) = engine.read() {
            for qid in category_qids {
                totals.edits += lock
                    .get_category_trend(*qid, depth, start, end)
                    .iter()
                    .map(|(_, edits)| edits)
                    .sum::<u64>();
            }
        }
    }

    if let Ok(engine) =
        EngineService::get_or_build_google_search_engine(Arc::clone(&state), wiki).await
    {
        if let Ok(lock) = engine.read() {
            for qid in category_qids {
                for (_, metrics) in lock.get_category_trend(*qid, depth, start, end) {
                    totals.gsc_clicks += metrics.clicks;
                    totals.gsc_impressions += metrics.impressions;
                }
            }
        }
    }

    totals
}
