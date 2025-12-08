use crate::models::AppState;
use crate::services::core::{CoreServiceError, PageViewService, QidService};
use chrono::NaiveDate;
use std::collections::HashMap;
use std::sync::Arc;

pub struct PageViewsService;

#[derive(Debug)]
pub enum ServiceError {
    CoreError(CoreServiceError),
}

impl From<CoreServiceError> for ServiceError {
    fn from(err: CoreServiceError) -> Self {
        ServiceError::CoreError(err)
    }
}

pub struct CategoryTrendResult {
    pub qid: u32,
    pub title: String,
    pub views: Vec<(NaiveDate, u64)>,
    pub top_articles: Vec<ArticleRank>,
}

pub struct ArticleTrendResult {
    pub qid: u32,
    pub title: String,
    pub views: Vec<(NaiveDate, u64)>,
}

pub struct ArticleRank {
    pub qid: u32,
    pub title: String,
    pub views: u32,
}

pub struct CategoryRank {
    pub qid: u32,
    pub title: String,
    pub views: u32,
    pub top_articles: Vec<ArticleRank>,
}

impl PageViewsService {
    pub async fn get_category_trend(
        state: Arc<AppState>,
        wiki: &str,
        category: &str,
        category_qid: Option<u32>,
        depth: Option<u32>,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> Result<CategoryTrendResult, ServiceError> {
        let depth = depth.unwrap_or(0);
        let start = start_date
            .unwrap_or_else(|| chrono::Local::now().date_naive() - chrono::Duration::days(30));
        let end = end_date.unwrap_or_else(|| chrono::Local::now().date_naive());

        let category_qid = if let Some(qid) = category_qid {
            qid
        } else {
            QidService::get_qid_by_title(Arc::clone(&state), wiki, category, 14).await?
        };

        // Get raw pageview data
        let data = PageViewService::get_category_views(
            Arc::clone(&state),
            wiki,
            category_qid,
            start,
            end,
            depth,
        )
        .await?;

        // Get top articles
        let top_articles = PageViewService::get_top_articles(
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
            QidService::get_titles_by_qids(Arc::clone(&state), wiki, article_qids).await?;

        let top_articles: Vec<ArticleRank> = top_articles
            .into_iter()
            .map(|art| {
                let article_title = titles_map
                    .get(&art.article_qid)
                    .cloned()
                    .unwrap_or_else(|| format!("Q{}", art.article_qid));

                ArticleRank {
                    qid: art.article_qid,
                    title: article_title,
                    views: art.total_views as u32,
                }
            })
            .collect();

        Ok(CategoryTrendResult {
            qid: category_qid,
            title: category.to_string(),
            views: data,
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
    ) -> Result<ArticleTrendResult, ServiceError> {
        let start = start_date
            .unwrap_or_else(|| chrono::Local::now().date_naive() - chrono::Duration::days(30));
        let end = end_date.unwrap_or_else(|| chrono::Local::now().date_naive());

        let article_qid = if let Some(qid) = article_qid {
            qid
        } else {
            QidService::get_qid_by_title(Arc::clone(&state), wiki, article, 0).await?
        };

        let data = PageViewService::get_article_views(state, wiki, article_qid, start, end).await?;

        Ok(ArticleTrendResult {
            qid: article_qid,
            title: article.to_string(),
            views: data,
        })
    }

    pub async fn get_top_categories(
        state: Arc<AppState>,
        wiki: &str,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        top_n: Option<u32>,
    ) -> Result<Vec<CategoryRank>, ServiceError> {
        let top_n = top_n.unwrap_or(10);
        let start = start_date
            .unwrap_or_else(|| chrono::Local::now().date_naive() - chrono::Duration::days(30));
        let end = end_date.unwrap_or_else(|| chrono::Local::now().date_naive());

        let categories: Vec<crate::services::core::pageview_service::CategoryViews> =
            PageViewService::get_top_categories(
                Arc::clone(&state),
                wiki,
                start,
                end,
                top_n as usize,
            )
            .await?;

        let mut all_qids = Vec::new();
        for category in &categories {
            all_qids.push(category.category_qid);
            for article in &category.top_articles {
                all_qids.push(article.article_qid);
            }
        }

        let titles_map: HashMap<u32, String> =
            QidService::get_titles_by_qids(Arc::clone(&state), wiki, all_qids).await?;

        let top_categories_with_titles: Vec<CategoryRank> = categories
            .into_iter()
            .map(|cat| {
                let category_title = titles_map
                    .get(&cat.category_qid)
                    .cloned()
                    .unwrap_or_else(|| format!("Q{}", cat.category_qid));

                let top_articles: Vec<ArticleRank> = cat
                    .top_articles
                    .into_iter()
                    .map(|art| {
                        let article_title = titles_map
                            .get(&art.article_qid)
                            .cloned()
                            .unwrap_or_else(|| format!("Q{}", art.article_qid));

                        ArticleRank {
                            qid: art.article_qid,
                            title: article_title,
                            views: art.total_views as u32,
                        }
                    })
                    .collect();

                CategoryRank {
                    qid: cat.category_qid,
                    title: category_title,
                    views: cat.total_views as u32,
                    top_articles,
                }
            })
            .collect();

        Ok(top_categories_with_titles)
    }

    pub async fn get_sub_categories(
        state: Arc<AppState>,
        wiki: &str,
        category: &str,
        category_qid: Option<u32>,
    ) -> Result<HashMap<u32, String>, ServiceError> {
        use crate::services::core::CategoryService;

        let category_qid = if let Some(qid) = category_qid {
            qid
        } else {
            QidService::get_qid_by_title(Arc::clone(&state), wiki, category, 14).await?
        };

        let category_qids =
            CategoryService::get_child_categories(Arc::clone(&state), wiki, category_qid).await?;
        let titles_map = QidService::get_titles_by_qids(state, wiki, category_qids).await?;

        Ok(titles_map)
    }
}
