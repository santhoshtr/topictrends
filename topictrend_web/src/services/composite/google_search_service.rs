use crate::models::AppState;
use crate::services::composite::taxonomy_search_category_qids;
use crate::services::core::{CoreServiceError, GoogleSearchService, QidService};
use chrono::NaiveDate;
use std::sync::Arc;

pub struct GoogleSearchTrendsService;

pub struct CategoryGoogleSearchTrendResult {
    pub qid: u32,
    pub title: String,
    pub search: Vec<GoogleSearchDailyResult>,
    pub top_articles: Vec<ArticleGoogleSearchRank>,
}

pub struct ArticleGoogleSearchTrendResult {
    pub qid: u32,
    pub title: String,
    pub search: Vec<GoogleSearchDailyResult>,
}

pub struct GoogleSearchDailyResult {
    pub date: NaiveDate,
    pub clicks: u64,
    pub impressions: u64,
    pub ctr: f64,
    pub position: f64,
}

pub struct ArticleGoogleSearchRank {
    pub qid: u32,
    pub title: String,
    pub clicks: u64,
    pub impressions: u64,
    pub ctr: f64,
}

impl GoogleSearchTrendsService {
    pub async fn get_category_trend(
        state: Arc<AppState>,
        wiki: &str,
        category: &str,
        category_qid: Option<u32>,
        depth: Option<u32>,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> Result<CategoryGoogleSearchTrendResult, CoreServiceError> {
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
                    let qids = taxonomy_search_category_qids(category).await?;
                    *qids.first().ok_or(CoreServiceError::NotFound)?
                }
            }
        };

        let data = GoogleSearchService::get_category_search_trend(
            Arc::clone(&state),
            wiki,
            category_qid,
            start,
            end,
            depth,
        )
        .await?;

        let top_articles = GoogleSearchService::get_top_articles(
            Arc::clone(&state),
            wiki,
            category_qid,
            start,
            end,
            depth,
            10,
        )
        .await?;

        let article_qids: Vec<u32> = top_articles.iter().map(|a| a.article_qid).collect();
        let titles_map =
            QidService::get_titles_by_qids(Arc::clone(&state), wiki, &article_qids).await?;

        let top_articles: Vec<ArticleGoogleSearchRank> = top_articles
            .into_iter()
            .map(|article| {
                let article_title = titles_map
                    .get(&article.article_qid)
                    .cloned()
                    .unwrap_or_else(|| format!("Q{}", article.article_qid));

                ArticleGoogleSearchRank {
                    qid: article.article_qid,
                    title: article_title,
                    clicks: article.total_clicks,
                    impressions: article.total_impressions,
                    ctr: article.ctr,
                }
            })
            .collect();

        let category_title = QidService::get_title_by_qid(Arc::clone(&state), wiki, category_qid)
            .await
            .unwrap_or_else(|_| category.to_string());

        Ok(CategoryGoogleSearchTrendResult {
            qid: category_qid,
            title: category_title,
            search: data
                .into_iter()
                .map(|item| GoogleSearchDailyResult {
                    date: item.date,
                    clicks: item.clicks,
                    impressions: item.impressions,
                    ctr: item.ctr,
                    position: item.position,
                })
                .collect(),
            top_articles,
        })
    }

    pub async fn get_article_trend(
        state: Arc<AppState>,
        wiki: &str,
        article: &str,
        article_qid: Option<u32>,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> Result<ArticleGoogleSearchTrendResult, CoreServiceError> {
        let start = start_date
            .unwrap_or_else(|| chrono::Local::now().date_naive() - chrono::Duration::days(30));
        let end = end_date.unwrap_or_else(|| chrono::Local::now().date_naive());

        let article_qid = if let Some(qid) = article_qid {
            qid
        } else {
            QidService::get_qid_by_title(Arc::clone(&state), wiki, article, 0).await?
        };

        let data = GoogleSearchService::get_article_search_trend(
            Arc::clone(&state),
            wiki,
            article_qid,
            start,
            end,
        )
        .await?;

        let article_title = QidService::get_title_by_qid(Arc::clone(&state), wiki, article_qid)
            .await
            .unwrap_or_else(|_| article.to_string());

        Ok(ArticleGoogleSearchTrendResult {
            qid: article_qid,
            title: article_title,
            search: data
                .into_iter()
                .map(|item| GoogleSearchDailyResult {
                    date: item.date,
                    clicks: item.clicks,
                    impressions: item.impressions,
                    ctr: item.ctr,
                    position: item.position,
                })
                .collect(),
        })
    }
}
