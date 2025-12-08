use crate::{
    models::AppState,
    services::core::{CoreServiceError, PageViewService, QidService},
};
use chrono::NaiveDate;
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Debug)]
pub struct CategoryDeltaItem {
    pub category_qid: u32,
    pub category_title: String,
    pub baseline_views: u64,
    pub impact_views: u64,
    pub delta_percentage: f64,
    pub absolute_delta: i64,
}

#[derive(Clone, Debug)]
pub struct ArticleDeltaItem {
    pub article_qid: u32,
    pub article_title: String,
    pub baseline_views: u64,
    pub impact_views: u64,
    pub delta_percentage: f64,
    pub absolute_delta: i64,
}

pub struct DeltaService;

impl DeltaService {
    pub async fn get_category_delta(
        state: Arc<AppState>,
        wiki: &str,
        baseline_start: NaiveDate,
        baseline_end: NaiveDate,
        impact_start: NaiveDate,
        impact_end: NaiveDate,
        limit: usize,
        depth: u32,
    ) -> Result<Vec<CategoryDeltaItem>, CoreServiceError> {
        // Get top categories for both periods
        let baseline_categories = PageViewService::get_top_categories(
            Arc::clone(&state),
            wiki,
            baseline_start,
            baseline_end,
            limit,
        )
        .await?;

        let impact_categories = PageViewService::get_top_categories(
            Arc::clone(&state),
            wiki,
            impact_start,
            impact_end,
            limit,
        )
        .await?;

        // Create maps for quick lookup
        let baseline_map: HashMap<u32, u64> = baseline_categories
            .into_iter()
            .map(|cat| (cat.category_qid, cat.total_views))
            .collect();

        let impact_map: HashMap<u32, u64> = impact_categories
            .into_iter()
            .map(|cat| (cat.category_qid, cat.total_views))
            .collect();

        // Get all unique category QIDs
        let mut all_qids: Vec<u32> = baseline_map.keys().cloned().collect();
        all_qids.extend(impact_map.keys().cloned());
        all_qids.sort();
        all_qids.dedup();

        // Fetch missing data for categories that appear in one period but not the other
        let mut final_baseline_map = baseline_map.clone();
        let mut final_impact_map = impact_map.clone();

        for qid in &all_qids {
            if !final_baseline_map.contains_key(qid) {
                // Get baseline views for this category
                if let Ok(views) = PageViewService::get_category_views(
                    Arc::clone(&state),
                    wiki,
                    *qid,
                    baseline_start,
                    baseline_end,
                    depth,
                )
                .await
                {
                    let total: u64 = views.iter().map(|(_, v)| v).sum();
                    final_baseline_map.insert(*qid, total);
                }
            }

            if !final_impact_map.contains_key(qid) {
                // Get impact views for this category
                if let Ok(views) = PageViewService::get_category_views(
                    Arc::clone(&state),
                    wiki,
                    *qid,
                    impact_start,
                    impact_end,
                    depth,
                )
                .await
                {
                    let total: u64 = views.iter().map(|(_, v)| v).sum();
                    final_impact_map.insert(*qid, total);
                }
            }
        }

        // Get titles for all categories
        let titles_map =
            QidService::get_titles_by_qids(Arc::clone(&state), wiki, &all_qids).await?;

        // Calculate deltas
        let mut delta_items: Vec<CategoryDeltaItem> = Vec::new();

        for qid in &all_qids {
            let baseline_views = final_baseline_map.get(qid).unwrap_or(&0);
            let impact_views = final_impact_map.get(qid).unwrap_or(&0);

            let delta_percentage = if *baseline_views == 0 {
                if *impact_views > 0 { 100.0 } else { 0.0 }
            } else {
                ((*impact_views as f64 - *baseline_views as f64) / *baseline_views as f64) * 100.0
            };

            let absolute_delta = *impact_views as i64 - *baseline_views as i64;

            let category_title = titles_map
                .get(qid)
                .cloned()
                .unwrap_or_else(|| format!("Q{}", qid));

            delta_items.push(CategoryDeltaItem {
                category_qid: *qid,
                category_title,
                baseline_views: *baseline_views,
                impact_views: *impact_views,
                delta_percentage,
                absolute_delta,
            });
        }

        // Sort by delta percentage descending (most increased first)
        delta_items.sort_by(|a, b| b.delta_percentage.partial_cmp(&a.delta_percentage).unwrap());

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
    ) -> Result<Vec<ArticleDeltaItem>, CoreServiceError> {
        // Get top articles for both periods
        let baseline_articles = PageViewService::get_top_articles(
            Arc::clone(&state),
            wiki,
            category_qid,
            baseline_start,
            baseline_end,
            depth,
            limit,
        )
        .await?;

        let impact_articles = PageViewService::get_top_articles(
            Arc::clone(&state),
            wiki,
            category_qid,
            impact_start,
            impact_end,
            depth,
            limit,
        )
        .await?;

        // Create maps for quick lookup
        let baseline_map: HashMap<u32, u64> = baseline_articles
            .into_iter()
            .map(|art| (art.article_qid, art.total_views))
            .collect();

        let impact_map: HashMap<u32, u64> = impact_articles
            .into_iter()
            .map(|art| (art.article_qid, art.total_views))
            .collect();

        // Get all unique article QIDs
        let mut all_qids: Vec<u32> = baseline_map.keys().cloned().collect();
        all_qids.extend(impact_map.keys().cloned());
        all_qids.sort();
        all_qids.dedup();

        // Fetch missing data for articles that appear in one period but not the other
        let mut final_baseline_map = baseline_map.clone();
        let mut final_impact_map = impact_map.clone();

        for qid in &all_qids {
            if !final_baseline_map.contains_key(qid) {
                // Get baseline views for this article
                if let Ok(views) = PageViewService::get_article_views(
                    Arc::clone(&state),
                    wiki,
                    *qid,
                    baseline_start,
                    baseline_end,
                )
                .await
                {
                    let total: u64 = views.iter().map(|(_, v)| v).sum();
                    final_baseline_map.insert(*qid, total);
                }
            }

            if !final_impact_map.contains_key(qid) {
                // Get impact views for this article
                if let Ok(views) = PageViewService::get_article_views(
                    Arc::clone(&state),
                    wiki,
                    *qid,
                    impact_start,
                    impact_end,
                )
                .await
                {
                    let total: u64 = views.iter().map(|(_, v)| v).sum();
                    final_impact_map.insert(*qid, total);
                }
            }
        }

        // Get titles for all articles
        let titles_map =
            QidService::get_titles_by_qids(Arc::clone(&state), wiki, &all_qids).await?;

        // Calculate deltas
        let mut delta_items: Vec<ArticleDeltaItem> = Vec::new();

        for qid in &all_qids {
            let baseline_views = final_baseline_map.get(qid).unwrap_or(&0);
            let impact_views = final_impact_map.get(qid).unwrap_or(&0);

            let delta_percentage = if *baseline_views == 0 {
                if *impact_views > 0 { 100.0 } else { 0.0 }
            } else {
                ((*impact_views as f64 - *baseline_views as f64) / *baseline_views as f64) * 100.0
            };

            let absolute_delta = *impact_views as i64 - *baseline_views as i64;

            let article_title = titles_map
                .get(qid)
                .cloned()
                .unwrap_or_else(|| format!("Q{}", qid));

            delta_items.push(ArticleDeltaItem {
                article_qid: *qid,
                article_title,
                baseline_views: *baseline_views,
                impact_views: *impact_views,
                delta_percentage,
                absolute_delta,
            });
        }

        // Sort by delta percentage descending (most increased first)
        delta_items.sort_by(|a, b| b.delta_percentage.partial_cmp(&a.delta_percentage).unwrap());

        Ok(delta_items)
    }
}
