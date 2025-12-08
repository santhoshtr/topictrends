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
        // STEP 1: Get top categories from BASELINE period only (this is the anchor)
        let baseline_categories = PageViewService::get_top_categories(
            Arc::clone(&state),
            wiki,
            baseline_start,
            baseline_end,
            limit,
        )
        .await?;

        // STEP 2: For these baseline top categories, get their impact period data
        let baseline_qids: Vec<u32> = baseline_categories
            .iter()
            .map(|cat| cat.category_qid)
            .collect();

        // Get impact views for the same categories that were top in baseline
        let mut impact_map: HashMap<u32, u64> = HashMap::new();
        for qid in &baseline_qids {
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
                impact_map.insert(*qid, total);
            }
        }

        // Get titles for all categories
        let titles_map =
            QidService::get_titles_by_qids(Arc::clone(&state), wiki, &baseline_qids).await?;

        // STEP 3: Calculate deltas for baseline top categories
        let mut delta_items: Vec<CategoryDeltaItem> = Vec::new();

        for category in baseline_categories {
            let qid = category.category_qid;
            let baseline_views = category.total_views;
            let impact_views = impact_map.get(&qid).unwrap_or(&0);

            let delta_percentage = if baseline_views == 0 {
                if *impact_views > 0 { 100.0 } else { 0.0 }
            } else {
                ((*impact_views as f64 - baseline_views as f64) / baseline_views as f64) * 100.0
            };

            let absolute_delta = *impact_views as i64 - baseline_views as i64;

            let category_title = titles_map
                .get(&qid)
                .cloned()
                .unwrap_or_else(|| format!("Q{}", qid));

            delta_items.push(CategoryDeltaItem {
                category_qid: qid,
                category_title,
                baseline_views,
                impact_views: *impact_views,
                delta_percentage,
                absolute_delta,
            });
        }

        // STEP 4: Sort by delta percentage descending (biggest changes in baseline top categories)
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
        // STEP 1: Get top articles from BASELINE period only (this is the anchor)
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

        // STEP 2: For these baseline top articles, get their impact period data
        let baseline_qids: Vec<u32> = baseline_articles
            .iter()
            .map(|art| art.article_qid)
            .collect();

        // Get impact views for the same articles that were top in baseline
        let mut impact_map: HashMap<u32, u64> = HashMap::new();
        for qid in &baseline_qids {
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
                impact_map.insert(*qid, total);
            }
        }

        // Get titles for all articles
        let titles_map =
            QidService::get_titles_by_qids(Arc::clone(&state), wiki, &baseline_qids).await?;

        // STEP 3: Calculate deltas for baseline top articles
        let mut delta_items: Vec<ArticleDeltaItem> = Vec::new();

        for article in baseline_articles {
            let qid = article.article_qid;
            let baseline_views = article.total_views;
            let impact_views = impact_map.get(&qid).unwrap_or(&0);

            let delta_percentage = if baseline_views == 0 {
                if *impact_views > 0 { 100.0 } else { 0.0 }
            } else {
                ((*impact_views as f64 - baseline_views as f64) / baseline_views as f64) * 100.0
            };

            let absolute_delta = *impact_views as i64 - baseline_views as i64;

            let article_title = titles_map
                .get(&qid)
                .cloned()
                .unwrap_or_else(|| format!("Q{}", qid));

            delta_items.push(ArticleDeltaItem {
                article_qid: qid,
                article_title,
                baseline_views,
                impact_views: *impact_views,
                delta_percentage,
                absolute_delta,
            });
        }

        // STEP 4: Sort by delta percentage descending (biggest changes in baseline top articles)
        delta_items.sort_by(|a, b| b.delta_percentage.partial_cmp(&a.delta_percentage).unwrap());

        Ok(delta_items)
    }
}
