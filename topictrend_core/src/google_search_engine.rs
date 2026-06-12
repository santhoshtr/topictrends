use crate::{graphbuilder::GraphBuilder, wikigraph::WikiGraph};
use chrono::NaiveDate;
use polars::prelude::*;
use std::collections::HashMap;
use std::fmt;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use std::{error::Error, fs};

#[derive(Debug, Clone, Copy, Default)]
pub struct SearchMetrics {
    pub clicks: u64,
    pub impressions: u64,
    pub ctr: f64,
    pub position: f64,
}

impl SearchMetrics {
    #[inline]
    fn from_parts(clicks: u64, impressions: u64, position_weighted_sum: f64) -> Self {
        let ctr = if impressions > 0 {
            clicks as f64 / impressions as f64
        } else {
            0.0
        };
        let position = if impressions > 0 {
            position_weighted_sum / impressions as f64
        } else {
            0.0
        };

        Self {
            clicks,
            impressions,
            ctr,
            position,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ArticleRank {
    pub article_qid: u32,
    pub total_clicks: u64,
    pub total_impressions: u64,
    pub ctr: f64,
}

impl fmt::Display for ArticleRank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Article: Q{} - Clicks: {}, Impressions: {}, CTR: {:.4}",
            self.article_qid, self.total_clicks, self.total_impressions, self.ctr
        )
    }
}

#[derive(Debug, Clone)]
pub struct CategoryRank {
    pub category_qid: u32,
    pub total_clicks: u64,
    pub total_impressions: u64,
    pub ctr: f64,
    pub top_articles: Vec<ArticleRank>,
}

impl fmt::Display for CategoryRank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Category: Q{}", self.category_qid)?;
        writeln!(f, "Total Clicks: {}", self.total_clicks)?;
        writeln!(f, "Total Impressions: {}", self.total_impressions)?;
        writeln!(f, "CTR: {:.4}", self.ctr)?;
        writeln!(f, "Top Articles:")?;
        for (i, article) in self.top_articles.iter().enumerate() {
            writeln!(f, "{:>2}. {}", i + 1, article)?;
        }
        Ok(())
    }
}

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
struct TopCategoriesCacheKey {
    start: NaiveDate,
    end: NaiveDate,
    top_n: usize,
}

#[derive(Debug)]
struct TopCategoriesCacheEntry {
    data: Vec<CategoryRank>,
    created_at: Instant,
    ttl: Duration,
}

impl TopCategoriesCacheEntry {
    fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }
}

#[derive(Debug)]
pub struct TopCategoriesCache {
    cache: HashMap<TopCategoriesCacheKey, TopCategoriesCacheEntry>,
    last_cleanup: Instant,
}

impl TopCategoriesCache {
    fn new() -> Self {
        Self {
            cache: HashMap::new(),
            last_cleanup: Instant::now(),
        }
    }

    fn get_ttl(_start_date: NaiveDate, end_date: NaiveDate) -> Duration {
        let today = chrono::Local::now().date_naive();
        let days_ago = (today - end_date).num_days();

        if days_ago <= 1 {
            Duration::from_secs(15 * 60)
        } else if days_ago <= 7 {
            Duration::from_secs(60 * 60)
        } else if days_ago <= 30 {
            Duration::from_secs(6 * 60 * 60)
        } else {
            Duration::from_secs(24 * 60 * 60)
        }
    }

    fn get(&self, key: &TopCategoriesCacheKey) -> Option<Vec<CategoryRank>> {
        self.cache.get(key).and_then(|entry| {
            if entry.is_expired() {
                None
            } else {
                Some(entry.data.clone())
            }
        })
    }

    fn insert(&mut self, key: TopCategoriesCacheKey, data: Vec<CategoryRank>) {
        let ttl = Self::get_ttl(key.start, key.end);
        let entry = TopCategoriesCacheEntry {
            data,
            created_at: Instant::now(),
            ttl,
        };
        self.cache.insert(key, entry);

        if self.last_cleanup.elapsed() > Duration::from_secs(10 * 60) {
            self.cleanup_expired();
            self.last_cleanup = Instant::now();
        }
    }

    fn cleanup_expired(&mut self) {
        self.cache.retain(|_, entry| !entry.is_expired());
    }

    fn clear(&mut self) {
        self.cache.clear();
    }
}

#[derive(Debug, Clone)]
pub struct DailySearchData {
    article_ids: Vec<u32>,
    clicks: Vec<u64>,
    impressions: Vec<u64>,
    position_weighted_sum: Vec<f64>,
}

impl DailySearchData {
    pub fn from_pairs(pairs: Vec<(u32, u64, u64, f64)>) -> Self {
        let mut by_article: HashMap<u32, (u64, u64, f64)> = HashMap::new();

        for (article_id, clicks, impressions, position_weighted_sum) in pairs {
            let entry = by_article.entry(article_id).or_insert((0, 0, 0.0));
            entry.0 += clicks;
            entry.1 += impressions;
            entry.2 += position_weighted_sum;
        }

        let mut rows: Vec<(u32, (u64, u64, f64))> = by_article.into_iter().collect();
        rows.sort_unstable_by_key(|(article_id, _)| *article_id);

        let mut article_ids = Vec::with_capacity(rows.len());
        let mut clicks = Vec::with_capacity(rows.len());
        let mut impressions = Vec::with_capacity(rows.len());
        let mut position_weighted_sum = Vec::with_capacity(rows.len());

        for (article_id, (article_clicks, article_impressions, article_pos_sum)) in rows {
            article_ids.push(article_id);
            clicks.push(article_clicks);
            impressions.push(article_impressions);
            position_weighted_sum.push(article_pos_sum);
        }

        Self {
            article_ids,
            clicks,
            impressions,
            position_weighted_sum,
        }
    }

    #[inline]
    pub fn get(&self, article_dense_id: u32) -> SearchMetrics {
        self.article_ids
            .binary_search(&article_dense_id)
            .map(|idx| {
                SearchMetrics::from_parts(
                    self.clicks[idx],
                    self.impressions[idx],
                    self.position_weighted_sum[idx],
                )
            })
            .unwrap_or_default()
    }

    pub fn iter(&self) -> impl Iterator<Item = (u32, u64, u64, f64)> + '_ {
        self.article_ids
            .iter()
            .copied()
            .zip(self.clicks.iter().copied())
            .zip(self.impressions.iter().copied())
            .zip(self.position_weighted_sum.iter().copied())
            .map(
                |(((article_id, clicks), impressions), position_weighted_sum)| {
                    (article_id, clicks, impressions, position_weighted_sum)
                },
            )
    }
}

#[derive(Debug)]
pub struct GoogleSearchEngine {
    daily_search: HashMap<NaiveDate, DailySearchData>,
    wiki: String,
    // `Arc<WikiGraph>` so the graph can be shared with PageViewEngine and
    // PageEditsEngine for the same wiki — see EngineService.
    wikigraph: Arc<WikiGraph>,
    top_categories_cache: RwLock<TopCategoriesCache>,
}

impl GoogleSearchEngine {
    /// Build a `GoogleSearchEngine` that owns its own `WikiGraph`.
    /// Convenient for CLI tools and tests; the web server should use
    /// [`GoogleSearchEngine::with_graph`].
    pub fn new(wiki: &str) -> Self {
        let graph: WikiGraph = GraphBuilder::new(wiki)
            .build()
            .expect("Error while building graph");
        Self::with_graph(wiki, Arc::new(graph))
    }

    /// Build a `GoogleSearchEngine` against a pre-built `WikiGraph` shared
    /// with other metric engines for the same wiki.
    pub fn with_graph(wiki: &str, wikigraph: Arc<WikiGraph>) -> Self {
        let daily_search = Self::load_google_search_from_parquet(wiki, &wikigraph)
            .expect("Error loading Google Search data");

        println!(
            "Loaded Google Search data for {} with {} dates",
            wiki,
            daily_search.len()
        );

        Self {
            daily_search,
            wiki: wiki.to_string(),
            wikigraph,
            top_categories_cache: RwLock::new(TopCategoriesCache::new()),
        }
    }

    fn load_google_search_from_parquet(
        wiki: &str,
        wikigraph: &WikiGraph,
    ) -> Result<HashMap<NaiveDate, DailySearchData>, Box<dyn Error>> {
        let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string());
        let gsc_root = format!("{}/{}/gsc", data_dir, wiki);

        if !Path::new(&gsc_root).exists() {
            eprintln!("Google Search data directory not found: {}", gsc_root);
            return Ok(HashMap::new());
        }

        let mut date_groups: HashMap<NaiveDate, Vec<(u32, u64, u64, f64)>> = HashMap::new();
        let mut loaded_rows = 0usize;
        let mut skipped_rows = 0usize;

        for year_entry in fs::read_dir(&gsc_root)? {
            let year_entry = year_entry?;
            if !year_entry.file_type()?.is_dir() {
                continue;
            }

            let year_name = year_entry.file_name();
            let year_name = year_name.to_string_lossy();
            let year: i32 = match year_name.parse() {
                Ok(value) => value,
                Err(_) => continue,
            };

            for month_entry in fs::read_dir(year_entry.path())? {
                let month_entry = month_entry?;
                if !month_entry.file_type()?.is_dir() {
                    continue;
                }

                let month_name = month_entry.file_name();
                let month_name = month_name.to_string_lossy();
                let month: u32 = match month_name.parse() {
                    Ok(value) => value,
                    Err(_) => continue,
                };

                for day_entry in fs::read_dir(month_entry.path())? {
                    let day_entry = day_entry?;
                    if !day_entry.file_type()?.is_file() {
                        continue;
                    }

                    let path = day_entry.path();
                    if path.extension().and_then(|ext| ext.to_str()) != Some("parquet") {
                        continue;
                    }

                    let day_stem = match path.file_stem().and_then(|stem| stem.to_str()) {
                        Some(stem) => stem,
                        None => continue,
                    };

                    let day: u32 = match day_stem.parse() {
                        Ok(value) => value,
                        Err(_) => continue,
                    };

                    let date = match NaiveDate::from_ymd_opt(year, month, day) {
                        Some(value) => value,
                        None => continue,
                    };

                    let path = PlRefPath::try_from_path(Path::new(&path))?;
                    let df = LazyFrame::scan_parquet(path, Default::default())?.collect()?;

                    let qids = df.column("qid")?.u32()?;
                    let clicks = df.column("clicks")?.i64()?;
                    let impressions = df.column("impressions")?.i64()?;
                    let positions = df.column("position")?.f64()?;

                    for i in 0..df.height() {
                        if let (
                            Some(article_qid),
                            Some(article_clicks),
                            Some(article_impressions),
                        ) = (qids.get(i), clicks.get(i), impressions.get(i))
                        {
                            if let Some(article_dense_id) =
                                wikigraph.art_original_to_dense.get(article_qid)
                            {
                                let safe_clicks = if article_clicks < 0 {
                                    0
                                } else {
                                    article_clicks as u64
                                };
                                let safe_impressions = if article_impressions < 0 {
                                    0
                                } else {
                                    article_impressions as u64
                                };
                                let position = positions.get(i).unwrap_or(0.0);
                                let position_weighted_sum = position * safe_impressions as f64;

                                date_groups.entry(date).or_default().push((
                                    article_dense_id,
                                    safe_clicks,
                                    safe_impressions,
                                    position_weighted_sum,
                                ));
                                loaded_rows += 1;
                            } else {
                                skipped_rows += 1;
                            }
                        }
                    }
                }
            }
        }

        println!(
            "Loaded {} Google Search rows, skipped {} unknown articles",
            loaded_rows, skipped_rows
        );

        let daily_search = date_groups
            .into_iter()
            .map(|(date, pairs)| (date, DailySearchData::from_pairs(pairs)))
            .collect();

        Ok(daily_search)
    }

    pub fn get_article_trend(
        &self,
        article_qid: u32,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Vec<(NaiveDate, SearchMetrics)> {
        let mut results = Vec::new();

        let article_dense_id = match self.wikigraph.art_original_to_dense.get(article_qid) {
            Some(dense_id) => dense_id,
            None => {
                eprintln!(
                    "Could not find dense id for article: {}/{}",
                    self.wiki, article_qid
                );
                return vec![];
            }
        };

        let mut curr = start_date;
        while curr <= end_date {
            let metrics = self
                .daily_search
                .get(&curr)
                .map(|day_data| day_data.get(article_dense_id))
                .unwrap_or_default();

            results.push((curr, metrics));
            curr = curr.succ_opt().expect("Invalid date progression");
        }

        results
    }

    pub fn get_category_trend(
        &self,
        category_qid: u32,
        depth: u32,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Vec<(NaiveDate, SearchMetrics)> {
        self.get_categories_trend(&[category_qid], depth, start_date, end_date)
    }

    /// Combined daily search metrics over the union of several categories'
    /// article sets. An article in more than one category is counted once
    /// per day.
    pub fn get_categories_trend(
        &self,
        category_qids: &[u32],
        depth: u32,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Vec<(NaiveDate, SearchMetrics)> {
        let mut results = Vec::new();

        let article_mask = self
            .wikigraph
            .get_articles_in_categories_as_dense(category_qids, depth);

        if article_mask.is_empty() {
            eprintln!(
                "Could not find articles in categories: {}/{:?}",
                self.wiki, category_qids
            );
            return vec![];
        }

        let mut curr = start_date;
        while curr <= end_date {
            let metrics = if let Some(day_data) = self.daily_search.get(&curr) {
                let mut clicks_total = 0u64;
                let mut impressions_total = 0u64;
                let mut position_weighted_sum_total = 0.0f64;

                for (article_dense_id, clicks, impressions, position_weighted_sum) in
                    day_data.iter()
                {
                    if article_mask.contains(article_dense_id) {
                        clicks_total += clicks;
                        impressions_total += impressions;
                        position_weighted_sum_total += position_weighted_sum;
                    }
                }

                SearchMetrics::from_parts(
                    clicks_total,
                    impressions_total,
                    position_weighted_sum_total,
                )
            } else {
                SearchMetrics::default()
            };

            results.push((curr, metrics));
            curr = curr.succ_opt().expect("Invalid date progression");
        }

        results
    }

    pub fn clear_top_categories_cache(&self) {
        self.top_categories_cache
            .write()
            .expect("top_categories_cache lock poisoned")
            .clear();
    }

    pub fn get_top_categories(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
        top_n: usize,
    ) -> Result<Vec<CategoryRank>, Box<dyn Error>> {
        let cache_key = TopCategoriesCacheKey {
            start: start_date,
            end: end_date,
            top_n,
        };

        if let Some(cached_result) = self
            .top_categories_cache
            .read()
            .expect("top_categories_cache lock poisoned")
            .get(&cache_key)
        {
            return Ok(cached_result);
        }

        let num_articles = self.wikigraph.art_dense_to_original.len();
        let num_cats = self.wikigraph.cat_dense_to_original.len();

        let mut article_clicks = vec![0u64; num_articles];
        let mut article_impressions = vec![0u64; num_articles];

        let mut curr = start_date;
        while curr <= end_date {
            if let Some(day_data) = self.daily_search.get(&curr) {
                for (article_dense_id, clicks, impressions, _) in day_data.iter() {
                    article_clicks[article_dense_id as usize] += clicks;
                    article_impressions[article_dense_id as usize] += impressions;
                }
            }
            curr = curr.succ_opt().expect("Invalid date progression");
        }

        let mut cat_clicks = vec![0u64; num_cats];
        let mut cat_impressions = vec![0u64; num_cats];
        let mut cat_articles: Vec<Vec<(u32, u64, u64)>> = vec![Vec::new(); num_cats];

        for (article_dense_id, &clicks) in article_clicks.iter().enumerate() {
            if clicks == 0 {
                continue;
            }

            let impressions = article_impressions[article_dense_id];
            let article_categories = self.wikigraph.article_cats.get(article_dense_id as u32);

            for &cat_dense_id in article_categories {
                unsafe {
                    *cat_clicks.get_unchecked_mut(cat_dense_id as usize) += clicks;
                    *cat_impressions.get_unchecked_mut(cat_dense_id as usize) += impressions;
                }
                cat_articles[cat_dense_id as usize].push((
                    article_dense_id as u32,
                    clicks,
                    impressions,
                ));
            }
        }

        let mut ranked: Vec<usize> = (0..num_cats).collect();
        ranked.sort_by(|&a, &b| cat_clicks[b].cmp(&cat_clicks[a]));

        let results: Vec<CategoryRank> = ranked
            .into_iter()
            .take(top_n)
            .filter(|&idx| cat_clicks[idx] > 0)
            .map(|cat_dense_id| {
                let mut articles = cat_articles[cat_dense_id].clone();
                articles.sort_unstable_by_key(|b| std::cmp::Reverse(b.1));

                let top_articles: Vec<ArticleRank> = articles
                    .into_iter()
                    .take(top_n)
                    .map(|(article_dense_id, clicks, impressions)| {
                        let article_qid =
                            self.wikigraph.art_dense_to_original[article_dense_id as usize];
                        let ctr = if impressions > 0 {
                            clicks as f64 / impressions as f64
                        } else {
                            0.0
                        };

                        ArticleRank {
                            article_qid,
                            total_clicks: clicks,
                            total_impressions: impressions,
                            ctr,
                        }
                    })
                    .collect();

                let total_clicks = cat_clicks[cat_dense_id];
                let total_impressions = cat_impressions[cat_dense_id];
                let ctr = if total_impressions > 0 {
                    total_clicks as f64 / total_impressions as f64
                } else {
                    0.0
                };

                CategoryRank {
                    category_qid: self.wikigraph.cat_dense_to_original[cat_dense_id],
                    total_clicks,
                    total_impressions,
                    ctr,
                    top_articles,
                }
            })
            .collect();

        self.top_categories_cache
            .write()
            .expect("top_categories_cache lock poisoned")
            .insert(cache_key, results.clone());

        Ok(results)
    }

    pub fn get_top_articles(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
        top_n: usize,
    ) -> Result<Vec<ArticleRank>, Box<dyn Error>> {
        let num_articles = self.wikigraph.art_dense_to_original.len();

        let mut article_clicks = vec![0u64; num_articles];
        let mut article_impressions = vec![0u64; num_articles];

        let mut curr = start_date;
        while curr <= end_date {
            if let Some(day_data) = self.daily_search.get(&curr) {
                for (article_dense_id, clicks, impressions, _) in day_data.iter() {
                    article_clicks[article_dense_id as usize] += clicks;
                    article_impressions[article_dense_id as usize] += impressions;
                }
            }
            curr = curr.succ_opt().expect("Invalid date progression");
        }

        let mut ranked: Vec<(usize, u64, u64)> = article_clicks
            .into_iter()
            .zip(article_impressions)
            .enumerate()
            .map(|(article_dense_id, (clicks, impressions))| {
                (article_dense_id, clicks, impressions)
            })
            .collect();

        ranked.sort_unstable_by_key(|b| std::cmp::Reverse(b.1));

        let results = ranked
            .into_iter()
            .take(top_n)
            .filter(|(_, clicks, _)| *clicks > 0)
            .map(|(article_dense_id, total_clicks, total_impressions)| {
                let ctr = if total_impressions > 0 {
                    total_clicks as f64 / total_impressions as f64
                } else {
                    0.0
                };

                ArticleRank {
                    article_qid: self.wikigraph.art_dense_to_original[article_dense_id],
                    total_clicks,
                    total_impressions,
                    ctr,
                }
            })
            .collect();

        Ok(results)
    }

    pub fn get_top_articles_in_category(
        &self,
        category_qid: u32,
        start_date: NaiveDate,
        end_date: NaiveDate,
        depth: u32,
        top_n: usize,
    ) -> Result<CategoryRank, Box<dyn Error>> {
        let article_mask = self
            .wikigraph
            .get_articles_in_category_as_dense(category_qid, depth)?;

        if article_mask.is_empty() {
            return Ok(CategoryRank {
                category_qid,
                total_clicks: 0,
                total_impressions: 0,
                ctr: 0.0,
                top_articles: vec![],
            });
        }

        let mut article_totals: Vec<(u32, u64, u64)> = Vec::new();

        for article_dense_id in article_mask.iter() {
            let mut total_clicks = 0u64;
            let mut total_impressions = 0u64;

            let mut curr = start_date;
            while curr <= end_date {
                if let Some(day_data) = self.daily_search.get(&curr) {
                    let metrics = day_data.get(article_dense_id);
                    total_clicks += metrics.clicks;
                    total_impressions += metrics.impressions;
                }
                curr = curr.succ_opt().expect("Invalid date progression");
            }

            if total_clicks > 0 {
                let article_qid = self.wikigraph.art_dense_to_original[article_dense_id as usize];
                article_totals.push((article_qid, total_clicks, total_impressions));
            }
        }

        article_totals.sort_unstable_by_key(|b| std::cmp::Reverse(b.1));

        let top_articles: Vec<ArticleRank> = article_totals
            .into_iter()
            .take(top_n)
            .map(|(article_qid, total_clicks, total_impressions)| {
                let ctr = if total_impressions > 0 {
                    total_clicks as f64 / total_impressions as f64
                } else {
                    0.0
                };
                ArticleRank {
                    article_qid,
                    total_clicks,
                    total_impressions,
                    ctr,
                }
            })
            .collect();

        let total_clicks: u64 = top_articles.iter().map(|a| a.total_clicks).sum();
        let total_impressions: u64 = top_articles.iter().map(|a| a.total_impressions).sum();
        let ctr = if total_impressions > 0 {
            total_clicks as f64 / total_impressions as f64
        } else {
            0.0
        };

        Ok(CategoryRank {
            category_qid,
            total_clicks,
            total_impressions,
            ctr,
            top_articles,
        })
    }
}
