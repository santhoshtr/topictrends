use crate::models::AppState;
use crate::services::composite::taxonomy_search_category_qids;
use crate::services::core::{CoreServiceError, PageEditService, QidService};
use chrono::NaiveDate;
use std::collections::HashMap;
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

        let categories =
            PageEditService::get_top_categories(Arc::clone(&state), wiki, start, end, top_n as usize)
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
            match QidService::get_qid_by_title(Arc::clone(&state), wiki, category, 14).await {
                Ok(qid) => qid,
                Err(_) => {
                    let qids = taxonomy_search_category_qids(category).await?;
                    *qids.first().ok_or(CoreServiceError::NotFound)?
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
