use crate::models::AppState;
use crate::wiki::{get_qid_by_title, get_titles_by_qids};
use chrono::NaiveDate;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Instant;
use topictrend::pageview_engine::PageViewEngine;

pub struct PageViewsService;

#[derive(Debug)]
pub enum ServiceError {
    DatabaseError(sqlx::Error),
    EngineError(String),
    NotFound,
    InternalError(String),
}

impl From<sqlx::Error> for ServiceError {
    fn from(err: sqlx::Error) -> Self {
        ServiceError::DatabaseError(err)
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
            get_qid_by_title(Arc::clone(&state), wiki, category, &14_i8).await?
        };

        let now = Instant::now();
        let engine = get_or_build_engine(Arc::clone(&state), wiki)
            .await
            .map_err(|e| ServiceError::InternalError(format!("Failed to build engine: {}", e)))?;

        println!("Engine build completed in {:.2?}s", now.elapsed());

        let (raw_data, category_rank) = {
            let mut engine_lock = engine.write().map_err(|e| {
                ServiceError::InternalError(format!("Failed to acquire write lock: {}", e))
            })?;

            let trend_data = engine_lock.get_category_trend(category_qid, depth, start, end);
            let top_articles = engine_lock
                .get_top_articles_in_category(category_qid, start, end, depth, 10)
                .map_err(|e| {
                    ServiceError::EngineError(format!("Failed to get top articles: {}", e))
                })?;

            (trend_data, top_articles)
        };

        let article_qids: Vec<u32> = category_rank
            .top_articles
            .iter()
            .map(|a| a.article_qid)
            .collect();

        let titles_map = get_titles_by_qids(Arc::clone(&state), wiki, article_qids).await?;

        let top_articles: Vec<ArticleRank> = category_rank
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

        Ok(CategoryTrendResult {
            qid: category_qid,
            title: category.to_string(),
            views: raw_data,
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
            get_qid_by_title(Arc::clone(&state), wiki, article, &0_i8).await?
        };

        let engine = get_or_build_engine(state, wiki)
            .await
            .map_err(|e| ServiceError::InternalError(format!("Failed to build engine: {}", e)))?;

        let raw_data = {
            let mut engine_lock = engine.write().map_err(|e| {
                ServiceError::InternalError(format!("Failed to acquire write lock: {}", e))
            })?;
            engine_lock.get_article_trend(article_qid, start, end)
        };

        Ok(ArticleTrendResult {
            qid: article_qid,
            title: article.to_string(),
            views: raw_data,
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

        let engine = get_or_build_engine(Arc::clone(&state), wiki)
            .await
            .map_err(|e| ServiceError::InternalError(format!("Failed to build engine: {}", e)))?;

        let categories = {
            let mut engine_lock = engine.write().map_err(|e| {
                ServiceError::InternalError(format!("Failed to acquire write lock: {}", e))
            })?;

            engine_lock
                .get_top_categories(start, end, top_n as usize)
                .map_err(|e| {
                    ServiceError::EngineError(format!("Failed to get top categories: {}", e))
                })?
        };

        let mut all_qids = Vec::new();
        for category in &categories {
            all_qids.push(category.category_qid);
            for article in &category.top_articles {
                all_qids.push(article.article_qid);
            }
        }

        let titles_map = get_titles_by_qids(Arc::clone(&state), wiki, all_qids).await?;

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
        let category_qid = if let Some(qid) = category_qid {
            qid
        } else {
            get_qid_by_title(Arc::clone(&state), wiki, category, &14_i8).await?
        };

        let engine = get_or_build_engine(Arc::clone(&state), wiki)
            .await
            .map_err(|e| ServiceError::InternalError(format!("Failed to build engine: {}", e)))?;

        let category_qids: Result<Vec<u32>, String> = {
            let engine_lock = engine.read().map_err(|e| {
                ServiceError::InternalError(format!("Failed to acquire read lock: {}", e))
            })?;

            engine_lock
                .get_wikigraph()
                .get_child_categories(category_qid)
        };

        match category_qids {
            Ok(categories) => {
                let titles_map = get_titles_by_qids(state, wiki, categories).await?;
                Ok(titles_map)
            }
            Err(e) => Err(ServiceError::EngineError(format!(
                "Failed to get child categories: {}",
                e
            ))),
        }
    }
}

async fn get_or_build_engine(
    state: Arc<AppState>,
    wiki: &str,
) -> Result<Arc<RwLock<PageViewEngine>>, Box<dyn std::error::Error + Send + Sync>> {
    let wiki = wiki.to_string();

    tokio::task::spawn_blocking(move || {
        let mut engines = state
            .engines
            .write()
            .map_err(|_| "Failed to acquire engines lock")?;
        if let Some(engine) = engines.get(&wiki) {
            Ok(Arc::clone(engine))
        } else {
            let new_engine = Arc::new(RwLock::new(PageViewEngine::new(&wiki)));
            engines.insert(wiki.clone(), Arc::clone(&new_engine));
            Ok(new_engine)
        }
    })
    .await
    .map_err(|_| "Failed to spawn blocking task")?
}
