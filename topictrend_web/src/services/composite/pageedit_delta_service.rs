use crate::{
    models::AppState,
    services::core::{CoreServiceError, PageEditService, QidService},
};
use chrono::NaiveDate;
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Debug)]
pub struct PageEditCategoryDeltaItem {
    pub category_qid: u32,
    pub category_title: String,
    pub baseline_edits: u64,
    pub impact_edits: u64,
    pub delta_percentage: f64,
    pub absolute_delta: i64,
}

#[derive(Clone, Debug)]
pub struct PageEditArticleDeltaItem {
    pub article_qid: u32,
    pub article_title: String,
    pub baseline_edits: u64,
    pub impact_edits: u64,
    pub delta_percentage: f64,
    pub absolute_delta: i64,
}

pub struct PageEditDeltaService;

impl PageEditDeltaService {
    pub async fn get_category_delta(
        state: Arc<AppState>,
        wiki: &str,
        baseline_start: NaiveDate,
        baseline_end: NaiveDate,
        impact_start: NaiveDate,
        impact_end: NaiveDate,
        limit: usize,
        depth: u32,
    ) -> Result<Vec<PageEditCategoryDeltaItem>, CoreServiceError> {
        // STEP 1: Get top categories from BOTH periods
        let baseline_categories = PageEditService::get_top_categories(
            Arc::clone(&state),
            wiki,
            baseline_start,
            baseline_end,
            limit,
        )
        .await?;

        let impact_categories = PageEditService::get_top_categories(
            Arc::clone(&state),
            wiki,
            impact_start,
            impact_end,
            limit,
        )
        .await?;

        // STEP 2: Create union of QIDs from both periods
        let mut all_qids = std::collections::HashSet::new();

        // Add baseline top categories
        for cat in &baseline_categories {
            all_qids.insert(cat.category_qid);
        }

        // Add impact top categories
        for cat in &impact_categories {
            all_qids.insert(cat.category_qid);
        }

        let all_qids: Vec<u32> = all_qids.into_iter().collect();

        // STEP 3: Create maps for quick lookup
        let baseline_map: HashMap<u32, u64> = baseline_categories
            .into_iter()
            .map(|cat| (cat.category_qid, cat.total_edits))
            .collect();

        let impact_map: HashMap<u32, u64> = impact_categories
            .into_iter()
            .map(|cat| (cat.category_qid, cat.total_edits))
            .collect();

        // STEP 4: For QIDs missing from either period, fetch their data
        let mut final_baseline_map = baseline_map.clone();
        let mut final_impact_map = impact_map.clone();

        for qid in &all_qids {
            // Fetch missing baseline data
            if !final_baseline_map.contains_key(qid)
                && let Ok(edits) = PageEditService::get_category_edits(
                    Arc::clone(&state),
                    wiki,
                    *qid,
                    baseline_start,
                    baseline_end,
                    depth,
                )
                .await
            {
                let total: u64 = edits.iter().map(|(_, v)| v).sum();
                final_baseline_map.insert(*qid, total);
            }

            // Fetch missing impact data
            if !final_impact_map.contains_key(qid)
                && let Ok(edits) = PageEditService::get_category_edits(
                    Arc::clone(&state),
                    wiki,
                    *qid,
                    impact_start,
                    impact_end,
                    depth,
                )
                .await
            {
                let total: u64 = edits.iter().map(|(_, v)| v).sum();
                final_impact_map.insert(*qid, total);
            }
        }

        // Get titles for all categories
        let titles_map =
            QidService::get_titles_by_qids(Arc::clone(&state), wiki, &all_qids).await?;

        // STEP 5: Calculate deltas for all categories in the union
        let mut delta_items: Vec<PageEditCategoryDeltaItem> = Vec::new();

        for qid in &all_qids {
            let baseline_edits = final_baseline_map.get(qid).unwrap_or(&0);
            let impact_edits = final_impact_map.get(qid).unwrap_or(&0);

            let delta_percentage = if *baseline_edits == 0 {
                if *impact_edits > 0 { 100.0 } else { 0.0 }
            } else {
                ((*impact_edits as f64 - *baseline_edits as f64) / *baseline_edits as f64) * 100.0
            };

            let absolute_delta = *impact_edits as i64 - *baseline_edits as i64;

            let category_title = titles_map
                .get(qid)
                .cloned()
                .unwrap_or_else(|| format!("Q{}", qid));

            delta_items.push(PageEditCategoryDeltaItem {
                category_qid: *qid,
                category_title,
                baseline_edits: *baseline_edits,
                impact_edits: *impact_edits,
                delta_percentage,
                absolute_delta,
            });
        }

        // STEP 6: Sort by absolute delta descending (biggest absolute changes first)
        // This captures both big increases and big decreases
        delta_items.sort_by_key(|b| std::cmp::Reverse(b.absolute_delta.abs()));

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
    ) -> Result<Vec<PageEditArticleDeltaItem>, CoreServiceError> {
        // STEP 1: Get top articles from BOTH periods
        let baseline_articles = PageEditService::get_top_articles(
            Arc::clone(&state),
            wiki,
            category_qid,
            baseline_start,
            baseline_end,
            depth,
            limit,
        )
        .await?;

        let impact_articles = PageEditService::get_top_articles(
            Arc::clone(&state),
            wiki,
            category_qid,
            impact_start,
            impact_end,
            depth,
            limit,
        )
        .await?;

        // STEP 2: Create union of QIDs from both periods
        let mut all_qids = std::collections::HashSet::new();

        // Add baseline top articles
        for art in &baseline_articles {
            all_qids.insert(art.article_qid);
        }

        // Add impact top articles
        for art in &impact_articles {
            all_qids.insert(art.article_qid);
        }

        let all_qids: Vec<u32> = all_qids.into_iter().collect();

        // STEP 3: Create maps for quick lookup
        let baseline_map: HashMap<u32, u64> = baseline_articles
            .into_iter()
            .map(|art| (art.article_qid, art.total_edits))
            .collect();

        let impact_map: HashMap<u32, u64> = impact_articles
            .into_iter()
            .map(|art| (art.article_qid, art.total_edits))
            .collect();

        // STEP 4: For QIDs missing from either period, fetch their data
        let mut final_baseline_map = baseline_map.clone();
        let mut final_impact_map = impact_map.clone();

        for qid in &all_qids {
            // Fetch missing baseline data
            if !final_baseline_map.contains_key(qid)
                && let Ok(edits) = PageEditService::get_article_edits(
                    Arc::clone(&state),
                    wiki,
                    *qid,
                    baseline_start,
                    baseline_end,
                )
                .await
            {
                let total: u64 = edits.iter().map(|(_, v)| v).sum();
                final_baseline_map.insert(*qid, total);
            }

            // Fetch missing impact data
            if !final_impact_map.contains_key(qid)
                && let Ok(edits) = PageEditService::get_article_edits(
                    Arc::clone(&state),
                    wiki,
                    *qid,
                    impact_start,
                    impact_end,
                )
                .await
            {
                let total: u64 = edits.iter().map(|(_, v)| v).sum();
                final_impact_map.insert(*qid, total);
            }
        }

        // Get titles for all articles
        let titles_map =
            QidService::get_titles_by_qids(Arc::clone(&state), wiki, &all_qids).await?;

        // STEP 5: Calculate deltas for all articles in the union
        let mut delta_items: Vec<PageEditArticleDeltaItem> = Vec::new();

        for qid in &all_qids {
            let baseline_edits = final_baseline_map.get(qid).unwrap_or(&0);
            let impact_edits = final_impact_map.get(qid).unwrap_or(&0);

            let delta_percentage = if *baseline_edits == 0 {
                if *impact_edits > 0 { 100.0 } else { 0.0 }
            } else {
                ((*impact_edits as f64 - *baseline_edits as f64) / *baseline_edits as f64) * 100.0
            };

            let absolute_delta = *impact_edits as i64 - *baseline_edits as i64;

            let article_title = titles_map
                .get(qid)
                .cloned()
                .unwrap_or_else(|| format!("Q{}", qid));

            delta_items.push(PageEditArticleDeltaItem {
                article_qid: *qid,
                article_title,
                baseline_edits: *baseline_edits,
                impact_edits: *impact_edits,
                delta_percentage,
                absolute_delta,
            });
        }

        // STEP 6: Sort by absolute delta descending (biggest absolute changes first)
        // This captures both big increases and big decreases
        delta_items.sort_by_key(|b| std::cmp::Reverse(b.absolute_delta.abs()));

        Ok(delta_items)
    }
}
