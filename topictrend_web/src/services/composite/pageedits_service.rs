use crate::models::AppState;
use crate::services::core::{CoreServiceError, PageEditService, QidService};
use chrono::NaiveDate;
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
}

impl PageEditsService {
    async fn resolve_category_qid_or_search(
        state: Arc<AppState>,
        wiki: &str,
        category: &str,
        depth: Option<u32>,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> Result<CategoryEditTrendResult, CoreServiceError> {
        let limit = 1000u64;
        let match_threshold = 0.6;

        let search_results =
            topictrend_taxonomy::search(category.to_string(), "enwiki".to_string(), limit)
                .await
                .map_err(|e| {
                    CoreServiceError::InternalError(format!("Taxonomy search failed: {}", e))
                })?;

        let category_qid = search_results
            .into_iter()
            .filter(|result| result.score >= match_threshold)
            .map(|result| result.qid)
            .next()
            .ok_or(CoreServiceError::NotFound)?;

        Box::pin(Self::get_category_edit_trend(
            state,
            wiki,
            category,
            Some(category_qid),
            depth,
            start_date,
            end_date,
        ))
        .await
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
            match QidService::get_qid_by_title(Arc::clone(&state), wiki, category, 14).await {
                Ok(qid) => qid,
                Err(_) => {
                    return Self::resolve_category_qid_or_search(
                        Arc::clone(&state),
                        wiki,
                        category,
                        Some(depth),
                        start_date,
                        end_date,
                    )
                    .await;
                }
            }
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
}
