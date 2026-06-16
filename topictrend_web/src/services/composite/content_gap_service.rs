use crate::models::{AppState, ContentGapResult, ContentGapWikiResult};
use crate::services::core::{CoreServiceError, CoverageService};
use std::sync::Arc;

pub struct ContentGapService;

impl ContentGapService {
    /// Compare one category's coverage across wikis. Each wiki's count is
    /// `qid_overlap_coverage` from its latest coverage snapshot — how many of
    /// the category's globally-known articles exist in that wiki — the same
    /// measure gap discovery ranks, served from the same artifact instead of
    /// building a WikiGraph per compared wiki.
    pub async fn get_content_gap(
        state: Arc<AppState>,
        category_qid: u32,
        category_label: &str,
        wikis: Vec<String>,
    ) -> Result<ContentGapResult, CoreServiceError> {
        let mut results: Vec<ContentGapWikiResult> = Vec::new();

        for wiki in &wikis {
            let snapshot =
                CoverageService::get_or_load_snapshot(Arc::clone(&state), wiki).await?;
            let (_direct, overlap, _pageviews) = snapshot.matrix.get(category_qid);

            results.push(ContentGapWikiResult {
                wiki: wiki.clone(),
                article_count: overlap as usize,
            });
        }

        Ok(ContentGapResult {
            category: category_label.to_string(),
            category_qid,
            wikis: results,
        })
    }

}
