use crate::models::AppState;
use crate::services::composite::source_attribution::resolve_source_categories;
use crate::services::composite::taxonomy_search_category_qids;
use crate::services::core::{
    ArticleService, CoreServiceError, EngineService, GoogleSearchService, QidService,
};
use chrono::NaiveDate;
use std::collections::{HashMap, HashSet};
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
    pub source_categories: Vec<(u32, String)>,
}

pub struct ArticleSearchRankResult {
    pub qid: u32,
    pub title: String,
    pub clicks: u64,
    pub impressions: u64,
    pub ctr: f64,
}

pub struct CategorySearchRankResult {
    pub qid: u32,
    pub title: String,
    pub clicks: u64,
    pub impressions: u64,
    pub ctr: f64,
    pub top_articles: Vec<ArticleSearchRankResult>,
}

pub struct ArticleCategoryRank {
    pub qid: u32,
    pub title: String,
}

pub struct TopArticleSearchRank {
    pub qid: u32,
    pub title: String,
    pub clicks: u64,
    pub impressions: u64,
    pub ctr: f64,
    pub categories: Vec<ArticleCategoryRank>,
}

impl GoogleSearchTrendsService {
    pub async fn get_top_categories(
        state: Arc<AppState>,
        wiki: &str,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        top_n: Option<u32>,
    ) -> Result<Vec<CategorySearchRankResult>, CoreServiceError> {
        let top_n = top_n.unwrap_or(10);
        let start = start_date
            .unwrap_or_else(|| chrono::Local::now().date_naive() - chrono::Duration::days(30));
        let end = end_date.unwrap_or_else(|| chrono::Local::now().date_naive());

        let categories = GoogleSearchService::get_top_categories(
            Arc::clone(&state),
            wiki,
            start,
            end,
            top_n as usize,
        )
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
                        ArticleSearchRankResult {
                            qid: art.article_qid,
                            title: art_title,
                            clicks: art.total_clicks,
                            impressions: art.total_impressions,
                            ctr: art.ctr,
                        }
                    })
                    .collect();

                CategorySearchRankResult {
                    qid: cat.category_qid,
                    title,
                    clicks: cat.total_clicks,
                    impressions: cat.total_impressions,
                    ctr: cat.ctr,
                    top_articles,
                }
            })
            .collect();

        Ok(result)
    }

    pub async fn get_category_trend(
        state: Arc<AppState>,
        wiki: &str,
        category: &str,
        category_qid: Option<u32>,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> Result<CategoryGoogleSearchTrendResult, CoreServiceError> {
        let start = start_date
            .unwrap_or_else(|| chrono::Local::now().date_naive() - chrono::Duration::days(30));
        let end = end_date.unwrap_or_else(|| chrono::Local::now().date_naive());

        let category_qid = if let Some(qid) = category_qid {
            qid
        } else {
            QidService::get_qid_by_title(Arc::clone(&state), wiki, category, 14).await?
        };

        let data = GoogleSearchService::get_category_search_trend(
            Arc::clone(&state),
            wiki,
            category_qid,
            start,
            end,
        )
        .await?;

        let top_articles = GoogleSearchService::get_top_articles(
            Arc::clone(&state),
            wiki,
            category_qid,
            start,
            end,
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
                    source_categories: vec![(category_qid, category.to_string())],
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

    pub async fn get_top_articles_global(
        state: Arc<AppState>,
        wiki: &str,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        top_n: Option<u32>,
    ) -> Result<Vec<TopArticleSearchRank>, CoreServiceError> {
        let top_n = top_n.unwrap_or(50);
        let start = start_date
            .unwrap_or_else(|| chrono::Local::now().date_naive() - chrono::Duration::days(30));
        let end = end_date.unwrap_or_else(|| chrono::Local::now().date_naive());

        let top_articles = GoogleSearchService::get_top_articles_global(
            Arc::clone(&state),
            wiki,
            start,
            end,
            top_n as usize,
        )
        .await?;

        let article_qids: Vec<u32> = top_articles
            .iter()
            .map(|article| article.article_qid)
            .collect();

        let mut article_categories_by_qid: HashMap<u32, Vec<u32>> = HashMap::new();
        let mut all_qids: HashSet<u32> = article_qids.iter().copied().collect();

        for article_qid in &article_qids {
            let category_qids: Vec<u32> =
                ArticleService::get_article_categories(Arc::clone(&state), wiki, *article_qid)
                    .await?
                    .into_iter()
                    .map(|(qid, _)| qid)
                    .collect();
            all_qids.extend(category_qids.iter().copied());
            article_categories_by_qid.insert(*article_qid, category_qids);
        }

        let all_qids_vec: Vec<u32> = all_qids.into_iter().collect();
        let titles_map =
            QidService::get_titles_by_qids(Arc::clone(&state), wiki, &all_qids_vec).await?;

        let mut response_articles = Vec::with_capacity(top_articles.len());

        for article in top_articles {
            let article_title = titles_map
                .get(&article.article_qid)
                .cloned()
                .unwrap_or_else(|| format!("Q{}", article.article_qid));

            let mut categories: Vec<ArticleCategoryRank> = article_categories_by_qid
                .get(&article.article_qid)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|category_qid| ArticleCategoryRank {
                    qid: category_qid,
                    title: titles_map
                        .get(&category_qid)
                        .cloned()
                        .unwrap_or_else(|| format!("Q{}", category_qid)),
                })
                .collect();

            categories.sort_by(|a, b| a.title.cmp(&b.title));

            response_articles.push(TopArticleSearchRank {
                qid: article.article_qid,
                title: article_title,
                clicks: article.total_clicks,
                impressions: article.total_impressions,
                ctr: article.ctr,
                categories,
            });
        }

        Ok(response_articles)
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

    pub async fn get_topic_google_search_trend(
        state: Arc<AppState>,
        wiki: &str,
        topic: &str,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    ) -> Result<CategoryGoogleSearchTrendResult, CoreServiceError> {
        let start = start_date
            .unwrap_or_else(|| chrono::Local::now().date_naive() - chrono::Duration::days(30));
        let end = end_date.unwrap_or_else(|| chrono::Local::now().date_naive());

        let category_qids = taxonomy_search_category_qids(topic).await?;
        let category_qid_set: HashSet<u32> = category_qids.iter().copied().collect();
        let mut all_articles: HashMap<u32, (u64, u64, u64, u32)> = HashMap::new();
        let category_titles =
            QidService::get_titles_by_qids(Arc::clone(&state), wiki, &category_qids).await?;

        let search_data;
        {
            let engine =
                EngineService::get_or_build_google_search_engine(Arc::clone(&state), wiki).await?;
            let engine_lock = engine.read().map_err(|e| {
                CoreServiceError::InternalError(format!("Failed to acquire read lock: {}", e))
            })?;

            // Daily totals over the union of the categories' article sets —
            // an article in several matched categories is counted once per day.
            search_data = engine_lock.get_categories_trend(&category_qids, 0, start, end);

            // An article's totals are article-level metrics (the same values
            // from every category that surfaces it), so assignment is
            // idempotent — never summed.
            for qid in &category_qids {
                let top_articles = engine_lock
                    .get_top_articles_in_category(*qid, start, end, 0, 50)
                    .map_err(|e| {
                        CoreServiceError::EngineError(format!("Failed to get top articles: {}", e))
                    })?
                    .top_articles;

                for article in top_articles {
                    let entry = all_articles
                        .entry(article.article_qid)
                        .or_insert((0, 0, 0, *qid));
                    entry.0 = article.total_clicks;
                    entry.1 = article.total_impressions;
                    if article.total_clicks > entry.2 {
                        entry.2 = article.total_clicks;
                        entry.3 = *qid;
                    }
                }
            }
        }

        let search: Vec<GoogleSearchDailyResult> = search_data
            .into_iter()
            .map(|(date, metrics)| GoogleSearchDailyResult {
                date,
                clicks: metrics.clicks,
                impressions: metrics.impressions,
                ctr: metrics.ctr,
                position: metrics.position,
            })
            .collect();

        let mut article_totals: Vec<(u32, (u64, u64, u64, u32))> =
            all_articles.into_iter().collect();
        article_totals.sort_by_key(|b| std::cmp::Reverse(b.1.0));
        article_totals.truncate(10);

        let article_qids: Vec<u32> = article_totals.iter().map(|(qid, _)| *qid).collect();

        let fallback_source_by_article: HashMap<u32, u32> = article_totals
            .iter()
            .map(|(qid, (_, _, _, fallback_source_category_qid))| {
                (*qid, *fallback_source_category_qid)
            })
            .collect();

        let titles_map = if article_qids.is_empty() {
            HashMap::new()
        } else {
            QidService::get_titles_by_qids(Arc::clone(&state), wiki, &article_qids).await?
        };

        let source_categories_by_article = resolve_source_categories(
            Arc::clone(&state),
            wiki,
            &article_qids,
            &category_qid_set,
            &fallback_source_by_article,
        )
        .await?;

        let top_articles = article_totals
            .into_iter()
            .map(|(qid, (clicks, impressions, _, _))| {
                let source_category_qids = source_categories_by_article
                    .get(&qid)
                    .cloned()
                    .unwrap_or_default();

                let source_categories: Vec<(u32, String)> = source_category_qids
                    .into_iter()
                    .map(|cat_qid| {
                        let title = category_titles
                            .get(&cat_qid)
                            .cloned()
                            .unwrap_or_else(|| format!("Q{}", cat_qid));
                        (cat_qid, title)
                    })
                    .collect();

                let title = titles_map
                    .get(&qid)
                    .cloned()
                    .unwrap_or_else(|| format!("Q{}", qid));

                let ctr = if impressions == 0 {
                    0.0
                } else {
                    clicks as f64 / impressions as f64
                };

                ArticleGoogleSearchRank {
                    qid,
                    title,
                    clicks,
                    impressions,
                    ctr,
                    source_categories,
                }
            })
            .collect();

        Ok(CategoryGoogleSearchTrendResult {
            qid: 0,
            title: topic.to_string(),
            search,
            top_articles,
        })
    }
}
