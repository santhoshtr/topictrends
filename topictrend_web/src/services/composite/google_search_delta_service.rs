use crate::{
    models::AppState,
    services::core::{CoreServiceError, GoogleSearchService, QidService},
};
use chrono::NaiveDate;
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Debug)]
pub struct GoogleSearchCategoryDeltaItem {
    pub category_qid: u32,
    pub category_title: String,
    pub baseline_clicks: u64,
    pub impact_clicks: u64,
    pub baseline_impressions: u64,
    pub impact_impressions: u64,
    pub delta_percentage: f64,
    pub absolute_delta: i64,
}

#[derive(Clone, Debug)]
pub struct GoogleSearchArticleDeltaItem {
    pub article_qid: u32,
    pub article_title: String,
    pub baseline_clicks: u64,
    pub impact_clicks: u64,
    pub baseline_impressions: u64,
    pub impact_impressions: u64,
    pub delta_percentage: f64,
    pub absolute_delta: i64,
}

pub struct GoogleSearchDeltaService;

impl GoogleSearchDeltaService {
    pub async fn get_category_delta(
        state: Arc<AppState>,
        wiki: &str,
        baseline_start: NaiveDate,
        baseline_end: NaiveDate,
        impact_start: NaiveDate,
        impact_end: NaiveDate,
        limit: usize,
        depth: u32,
    ) -> Result<Vec<GoogleSearchCategoryDeltaItem>, CoreServiceError> {
        let baseline_categories = GoogleSearchService::get_top_categories(
            Arc::clone(&state),
            wiki,
            baseline_start,
            baseline_end,
            limit,
        )
        .await?;

        let impact_categories = GoogleSearchService::get_top_categories(
            Arc::clone(&state),
            wiki,
            impact_start,
            impact_end,
            limit,
        )
        .await?;

        let mut all_qids = std::collections::HashSet::new();
        for category in &baseline_categories {
            all_qids.insert(category.category_qid);
        }
        for category in &impact_categories {
            all_qids.insert(category.category_qid);
        }
        let all_qids: Vec<u32> = all_qids.into_iter().collect();

        let baseline_map: HashMap<u32, (u64, u64)> = baseline_categories
            .into_iter()
            .map(|category| {
                (
                    category.category_qid,
                    (category.total_clicks, category.total_impressions),
                )
            })
            .collect();

        let impact_map: HashMap<u32, (u64, u64)> = impact_categories
            .into_iter()
            .map(|category| {
                (
                    category.category_qid,
                    (category.total_clicks, category.total_impressions),
                )
            })
            .collect();

        let mut final_baseline_map = baseline_map.clone();
        let mut final_impact_map = impact_map.clone();

        for qid in &all_qids {
            if !final_baseline_map.contains_key(qid)
                && let Ok(search) = GoogleSearchService::get_category_search_trend(
                    Arc::clone(&state),
                    wiki,
                    *qid,
                    baseline_start,
                    baseline_end,
                    depth,
                )
                .await
            {
                let total_clicks: u64 = search.iter().map(|item| item.clicks).sum();
                let total_impressions: u64 = search.iter().map(|item| item.impressions).sum();
                final_baseline_map.insert(*qid, (total_clicks, total_impressions));
            }

            if !final_impact_map.contains_key(qid)
                && let Ok(search) = GoogleSearchService::get_category_search_trend(
                    Arc::clone(&state),
                    wiki,
                    *qid,
                    impact_start,
                    impact_end,
                    depth,
                )
                .await
            {
                let total_clicks: u64 = search.iter().map(|item| item.clicks).sum();
                let total_impressions: u64 = search.iter().map(|item| item.impressions).sum();
                final_impact_map.insert(*qid, (total_clicks, total_impressions));
            }
        }

        let titles_map =
            QidService::get_titles_by_qids(Arc::clone(&state), wiki, &all_qids).await?;

        let mut delta_items: Vec<GoogleSearchCategoryDeltaItem> = Vec::new();
        for qid in &all_qids {
            let (baseline_clicks, baseline_impressions) =
                final_baseline_map.get(qid).copied().unwrap_or((0, 0));
            let (impact_clicks, impact_impressions) =
                final_impact_map.get(qid).copied().unwrap_or((0, 0));

            let delta_percentage = if baseline_clicks == 0 {
                if impact_clicks > 0 { 100.0 } else { 0.0 }
            } else {
                ((impact_clicks as f64 - baseline_clicks as f64) / baseline_clicks as f64) * 100.0
            };

            let absolute_delta = impact_clicks as i64 - baseline_clicks as i64;

            let category_title = titles_map
                .get(qid)
                .cloned()
                .unwrap_or_else(|| format!("Q{}", qid));

            delta_items.push(GoogleSearchCategoryDeltaItem {
                category_qid: *qid,
                category_title,
                baseline_clicks,
                impact_clicks,
                baseline_impressions,
                impact_impressions,
                delta_percentage,
                absolute_delta,
            });
        }

        delta_items.sort_by(|a, b| b.absolute_delta.abs().cmp(&a.absolute_delta.abs()));
        Ok(delta_items)
    }

    pub async fn get_article_delta(
        state: Arc<AppState>,
        wiki: &str,
        category_qid: u32,
        baseline_start: NaiveDate,
        baseline_end: NaiveDate,
        impact_start: NaiveDate,
        impact_end: NaiveDate,
        limit: usize,
        depth: u32,
    ) -> Result<Vec<GoogleSearchArticleDeltaItem>, CoreServiceError> {
        let baseline_articles = GoogleSearchService::get_top_articles(
            Arc::clone(&state),
            wiki,
            category_qid,
            baseline_start,
            baseline_end,
            depth,
            limit,
        )
        .await?;

        let impact_articles = GoogleSearchService::get_top_articles(
            Arc::clone(&state),
            wiki,
            category_qid,
            impact_start,
            impact_end,
            depth,
            limit,
        )
        .await?;

        let mut all_qids = std::collections::HashSet::new();
        for article in &baseline_articles {
            all_qids.insert(article.article_qid);
        }
        for article in &impact_articles {
            all_qids.insert(article.article_qid);
        }
        let all_qids: Vec<u32> = all_qids.into_iter().collect();

        let baseline_map: HashMap<u32, (u64, u64)> = baseline_articles
            .into_iter()
            .map(|article| {
                (
                    article.article_qid,
                    (article.total_clicks, article.total_impressions),
                )
            })
            .collect();

        let impact_map: HashMap<u32, (u64, u64)> = impact_articles
            .into_iter()
            .map(|article| {
                (
                    article.article_qid,
                    (article.total_clicks, article.total_impressions),
                )
            })
            .collect();

        let mut final_baseline_map = baseline_map.clone();
        let mut final_impact_map = impact_map.clone();

        for qid in &all_qids {
            if !final_baseline_map.contains_key(qid)
                && let Ok(search) = GoogleSearchService::get_article_search_trend(
                    Arc::clone(&state),
                    wiki,
                    *qid,
                    baseline_start,
                    baseline_end,
                )
                .await
            {
                let total_clicks: u64 = search.iter().map(|item| item.clicks).sum();
                let total_impressions: u64 = search.iter().map(|item| item.impressions).sum();
                final_baseline_map.insert(*qid, (total_clicks, total_impressions));
            }

            if !final_impact_map.contains_key(qid)
                && let Ok(search) = GoogleSearchService::get_article_search_trend(
                    Arc::clone(&state),
                    wiki,
                    *qid,
                    impact_start,
                    impact_end,
                )
                .await
            {
                let total_clicks: u64 = search.iter().map(|item| item.clicks).sum();
                let total_impressions: u64 = search.iter().map(|item| item.impressions).sum();
                final_impact_map.insert(*qid, (total_clicks, total_impressions));
            }
        }

        let titles_map =
            QidService::get_titles_by_qids(Arc::clone(&state), wiki, &all_qids).await?;

        let mut delta_items: Vec<GoogleSearchArticleDeltaItem> = Vec::new();
        for qid in &all_qids {
            let (baseline_clicks, baseline_impressions) =
                final_baseline_map.get(qid).copied().unwrap_or((0, 0));
            let (impact_clicks, impact_impressions) =
                final_impact_map.get(qid).copied().unwrap_or((0, 0));

            let delta_percentage = if baseline_clicks == 0 {
                if impact_clicks > 0 { 100.0 } else { 0.0 }
            } else {
                ((impact_clicks as f64 - baseline_clicks as f64) / baseline_clicks as f64) * 100.0
            };

            let absolute_delta = impact_clicks as i64 - baseline_clicks as i64;

            let article_title = titles_map
                .get(qid)
                .cloned()
                .unwrap_or_else(|| format!("Q{}", qid));

            delta_items.push(GoogleSearchArticleDeltaItem {
                article_qid: *qid,
                article_title,
                baseline_clicks,
                impact_clicks,
                baseline_impressions,
                impact_impressions,
                delta_percentage,
                absolute_delta,
            });
        }

        delta_items.sort_by(|a, b| b.absolute_delta.abs().cmp(&a.absolute_delta.abs()));
        Ok(delta_items)
    }
}
