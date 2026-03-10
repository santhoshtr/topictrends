use crate::{graphbuilder::GraphBuilder, wikigraph::WikiGraph};
use chrono::NaiveDate;
use polars::prelude::*;
use std::fmt;
use std::path::Path;
use std::sync::RwLock;
use std::time::{Duration, Instant};
use std::{collections::HashMap, error::Error};

/// Sparse storage for edit counts on a single date
/// Optimized for read-heavy workloads with binary search lookups
#[derive(Debug, Clone)]
pub struct DailyEditData {
    article_ids: Vec<u32>, // Sorted dense article IDs with edits
    edit_counts: Vec<u32>, // Corresponding edit counts
}

impl DailyEditData {
    /// O(log n) lookup via binary search
    /// Returns 0 if article has no edits on this date
    #[inline]
    pub fn get(&self, article_dense_id: u32) -> u32 {
        self.article_ids
            .binary_search(&article_dense_id)
            .map(|idx| self.edit_counts[idx])
            .unwrap_or(0)
    }

    /// Efficient iteration over all (article_id, count) pairs
    pub fn iter(&self) -> impl Iterator<Item = (u32, u32)> + '_ {
        self.article_ids
            .iter()
            .copied()
            .zip(self.edit_counts.iter().copied())
    }

    /// Build from unsorted data
    pub fn from_pairs(mut pairs: Vec<(u32, u32)>) -> Self {
        // Sort by article_id for binary search
        pairs.sort_unstable_by_key(|(id, _)| *id);

        let (article_ids, edit_counts): (Vec<u32>, Vec<u32>) = pairs.into_iter().unzip();

        Self {
            article_ids,
            edit_counts,
        }
    }

    /// Returns true if no edits on this date
    pub fn is_empty(&self) -> bool {
        self.article_ids.is_empty()
    }

    /// Number of articles with edits on this date
    pub fn len(&self) -> usize {
        self.article_ids.len()
    }
}

#[derive(Debug, Clone)]
pub struct ArticleRank {
    pub article_qid: u32,
    pub total_edits: u64,
}

impl fmt::Display for ArticleRank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Article: Q{} - Edits: {}",
            self.article_qid, self.total_edits
        )
    }
}

#[derive(Debug, Clone)]
pub struct CategoryRank {
    pub category_qid: u32,
    pub total_edits: u64,
    pub top_articles: Vec<ArticleRank>,
}

impl fmt::Display for CategoryRank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Category: Q{}", self.category_qid)?;
        writeln!(f, "Total Edits: {}", self.total_edits)?;
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

        // Recent data changes frequently, cache for shorter time
        if days_ago <= 1 {
            Duration::from_secs(15 * 60) // 15 minutes
        } else if days_ago <= 7 {
            Duration::from_secs(60 * 60) // 1 hour
        } else if days_ago <= 30 {
            Duration::from_secs(6 * 60 * 60) // 6 hours
        } else {
            Duration::from_secs(24 * 60 * 60) // 24 hours for historical data
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

        // Cleanup expired entries every 10 minutes
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

#[derive(Debug)]
pub struct PageEditsEngine {
    // Map Date -> Sparse edit data (article_dense_id -> edit_count)
    daily_edits: HashMap<NaiveDate, DailyEditData>,
    wiki: String,
    wikigraph: WikiGraph,
    top_categories_cache: RwLock<TopCategoriesCache>,
}

impl PageEditsEngine {
    pub fn new(wiki: &str) -> Self {
        let graph_builder = GraphBuilder::new(wiki);
        let graph: WikiGraph = graph_builder.build().expect("Error while building graph");

        // Load pageedits data from parquet
        let daily_edits =
            Self::load_pageedits_from_parquet(wiki, &graph).expect("Error loading pageedits data");

        println!(
            "Loaded pageedits for {} with {} dates",
            wiki,
            daily_edits.len()
        );

        Self {
            wiki: wiki.to_string(),
            daily_edits,
            wikigraph: graph,
            top_categories_cache: RwLock::new(TopCategoriesCache::new()),
        }
    }

    pub fn get_wikigraph(&self) -> &WikiGraph {
        &self.wikigraph
    }

    /// Load pageedits data from parquet file
    fn load_pageedits_from_parquet(
        wiki: &str,
        wikigraph: &WikiGraph,
    ) -> Result<HashMap<NaiveDate, DailyEditData>, Box<dyn Error>> {
        let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string());
        let parquet_path = format!("{}/{}/pageedits/pageedits.parquet", data_dir, wiki);

        if !std::path::Path::new(&parquet_path).exists() {
            eprintln!("Pageedits file not found: {}", parquet_path);
            return Ok(HashMap::new());
        }

        println!("Loading pageedits from: {}", parquet_path);

        let path = PlRefPath::try_from_path(Path::new(&parquet_path))?;
        let df = LazyFrame::scan_parquet(path, Default::default())?.collect()?;

        let article_qids = df.column("article_qid")?.u32()?;
        let dates = df.column("date")?.str()?;
        let edit_counts = df.column("edit_count")?.u32()?;

        // Group data by date
        let mut date_groups: HashMap<NaiveDate, Vec<(u32, u32)>> = HashMap::new();
        let mut skipped = 0;
        let mut loaded = 0;

        for i in 0..df.height() {
            if let (Some(article_qid), Some(date_str), Some(edit_count)) =
                (article_qids.get(i), dates.get(i), edit_counts.get(i))
            {
                // Translate QID to dense_id
                if let Some(dense_id) = wikigraph.art_original_to_dense.get(article_qid) {
                    // Parse date
                    if let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                        date_groups
                            .entry(date)
                            .or_insert_with(Vec::new)
                            .push((dense_id, edit_count));
                        loaded += 1;
                    }
                } else {
                    skipped += 1;
                }
            }
        }

        println!(
            "Loaded {} edit records, skipped {} unknown articles",
            loaded, skipped
        );

        // Build DailyEditData for each date
        let daily_edits: HashMap<NaiveDate, DailyEditData> = date_groups
            .into_iter()
            .map(|(date, pairs)| (date, DailyEditData::from_pairs(pairs)))
            .collect();

        println!("Built edit data for {} unique dates", daily_edits.len());

        Ok(daily_edits)
    }

    /// Get edit trend for a single article over a date range
    pub fn get_article_trend(
        &self,
        article_qid: u32,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Vec<(NaiveDate, u64)> {
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
            let edit_count = self
                .daily_edits
                .get(&curr)
                .map(|day_data| day_data.get(article_dense_id) as u64)
                .unwrap_or(0);

            results.push((curr, edit_count));
            curr = curr.succ_opt().unwrap();
        }

        results
    }

    /// Get edit trend for a category (all articles in category) over a date range
    pub fn get_category_trend(
        &self,
        category_qid: u32,
        depth: u32,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Vec<(NaiveDate, u64)> {
        let mut results = Vec::new();

        let article_mask = match self
            .wikigraph
            .get_articles_in_category_as_dense(category_qid, depth)
        {
            Ok(mask) => mask,
            Err(err) => {
                eprintln!("Error: {}", err);
                return vec![];
            }
        };

        // Optimization: If mask is empty, return early
        if article_mask.is_empty() {
            eprintln!(
                "Could not find articles in category: {}/{}",
                self.wiki, category_qid
            );
            return vec![];
        }

        println!(
            "Found {} articles in category {}/{} at depth {}",
            article_mask.len(),
            self.wiki,
            category_qid,
            depth
        );

        let mut curr = start_date;
        while curr <= end_date {
            let daily_total = if let Some(day_data) = self.daily_edits.get(&curr) {
                // Iterate over articles with edits and check if they're in the category
                day_data
                    .iter()
                    .filter(|(article_dense_id, _)| article_mask.contains(*article_dense_id))
                    .map(|(_, count)| count as u64)
                    .sum()
            } else {
                0
            };

            results.push((curr, daily_total));
            curr = curr.succ_opt().unwrap();
        }

        results
    }

    /// Clear the top categories cache
    pub fn clear_top_categories_cache(&self) {
        self.top_categories_cache
            .write()
            .expect("top_categories_cache lock poisoned")
            .clear();
    }

    /// Returns top N categories by edit count for a date range
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

        // Read lock: cache hit check — dropped immediately after.
        if let Some(cached_result) = self
            .top_categories_cache
            .read()
            .expect("top_categories_cache lock poisoned")
            .get(&cache_key)
        {
            println!("Cache hit for top_categories query: {:?}", cache_key);
            return Ok(cached_result);
        }

        println!("Cache miss for top_categories query: {:?}", cache_key);

        let num_articles = self.wikigraph.art_dense_to_original.len();
        let num_cats = self.wikigraph.cat_dense_to_original.len();

        // Phase 1: Aggregate edits per article across date range
        let mut article_edits = vec![0u32; num_articles];

        let mut curr = start_date;
        while curr <= end_date {
            if let Some(day_data) = self.daily_edits.get(&curr) {
                for (article_dense_id, count) in day_data.iter() {
                    article_edits[article_dense_id as usize] += count;
                }
            }
            curr = curr.succ_opt().unwrap();
        }

        // Phase 2: Scatter article edits to categories
        let mut cat_scores = vec![0u64; num_cats];
        let mut cat_articles: Vec<Vec<(u32, u32)>> = vec![Vec::new(); num_cats];

        for (art_dense_id, &edits) in article_edits.iter().enumerate() {
            if edits == 0 {
                continue;
            }

            // Get categories for this article
            let article_categories = self.wikigraph.article_cats.get(art_dense_id as u32);

            for &cat_dense_id in article_categories {
                unsafe {
                    *cat_scores.get_unchecked_mut(cat_dense_id as usize) += edits as u64;
                }
                cat_articles[cat_dense_id as usize].push((art_dense_id as u32, edits));
            }
        }

        // Phase 3: Sort & Top N
        let mut ranked: Vec<usize> = (0..num_cats).collect();
        ranked.sort_by(|&a, &b| cat_scores[b].cmp(&cat_scores[a]));

        // Transform to output
        let results: Vec<CategoryRank> = ranked
            .into_iter()
            .take(top_n)
            .filter(|&idx| cat_scores[idx] > 0)
            .map(|cat_dense_id| {
                // Sort articles for this category by edits
                let mut articles = cat_articles[cat_dense_id].clone();
                articles.sort_unstable_by(|a, b| b.1.cmp(&a.1));

                let top_articles: Vec<ArticleRank> = articles
                    .into_iter()
                    .take(top_n)
                    .map(|(art_dense_id, edits)| ArticleRank {
                        article_qid: self.wikigraph.art_dense_to_original[art_dense_id as usize],
                        total_edits: edits as u64,
                    })
                    .collect();

                CategoryRank {
                    category_qid: self.wikigraph.cat_dense_to_original[cat_dense_id],
                    total_edits: cat_scores[cat_dense_id],
                    top_articles,
                }
            })
            .collect();

        // Write lock only to insert the computed result.
        self.top_categories_cache
            .write()
            .expect("top_categories_cache lock poisoned")
            .insert(cache_key, results.clone());

        Ok(results)
    }

    /// Get top articles in a category by edit count
    pub fn get_top_articles_in_category(
        &self,
        category_qid: u32,
        start_date: NaiveDate,
        end_date: NaiveDate,
        depth: u32,
        top_n: usize,
    ) -> Result<CategoryRank, Box<dyn Error>> {
        // Get all articles in this category
        let article_mask = self
            .wikigraph
            .get_articles_in_category_as_dense(category_qid, depth)?;

        if article_mask.is_empty() {
            return Ok(CategoryRank {
                category_qid,
                total_edits: 0,
                top_articles: vec![],
            });
        }

        // Aggregate edits for each article
        let mut article_edits: Vec<(u32, u64)> = Vec::new();

        for article_dense_id in article_mask.iter() {
            let mut total_edits = 0u64;

            let mut curr = start_date;
            while curr <= end_date {
                if let Some(day_data) = self.daily_edits.get(&curr) {
                    total_edits += day_data.get(article_dense_id) as u64;
                }
                curr = curr.succ_opt().unwrap();
            }

            if total_edits > 0 {
                let article_qid = self.wikigraph.art_dense_to_original[article_dense_id as usize];
                article_edits.push((article_qid, total_edits));
            }
        }

        // Sort by edits descending
        article_edits.sort_unstable_by(|a, b| b.1.cmp(&a.1));

        // Take top N and convert to ArticleRank
        let top_articles: Vec<ArticleRank> = article_edits
            .into_iter()
            .take(top_n)
            .map(|(article_qid, total_edits)| ArticleRank {
                article_qid,
                total_edits,
            })
            .collect();

        // Calculate total edits for the category
        let total_edits: u64 = top_articles.iter().map(|a| a.total_edits).sum();

        Ok(CategoryRank {
            category_qid,
            total_edits,
            top_articles,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daily_edit_data_empty() {
        let data = DailyEditData::from_pairs(vec![]);
        assert_eq!(data.len(), 0);
        assert!(data.is_empty());
        assert_eq!(data.get(0), 0);
        assert_eq!(data.get(100), 0);
    }

    #[test]
    fn test_daily_edit_data_single() {
        let data = DailyEditData::from_pairs(vec![(42, 5)]);
        assert_eq!(data.len(), 1);
        assert!(!data.is_empty());
        assert_eq!(data.get(42), 5);
        assert_eq!(data.get(0), 0);
        assert_eq!(data.get(100), 0);
    }

    #[test]
    fn test_daily_edit_data_multiple() {
        let data = DailyEditData::from_pairs(vec![(10, 3), (5, 7), (20, 1), (5, 2)]);
        // Note: duplicate keys (5) will be preserved, last one wins after sort
        assert_eq!(data.get(5), 2); // or 7, depends on sort stability
        assert_eq!(data.get(10), 3);
        assert_eq!(data.get(20), 1);
        assert_eq!(data.get(15), 0);
    }

    #[test]
    fn test_daily_edit_data_iteration() {
        let data = DailyEditData::from_pairs(vec![(10, 3), (5, 7), (20, 1)]);
        let items: Vec<(u32, u32)> = data.iter().collect();

        // Should be sorted by article_id
        assert_eq!(items.len(), 3);
        assert!(items[0].0 < items[1].0);
        assert!(items[1].0 < items[2].0);
    }

    #[test]
    fn test_daily_edit_data_binary_search() {
        // Test with many articles to ensure binary search works
        let mut pairs = vec![];
        for i in (0..1000).step_by(2) {
            pairs.push((i, i * 10));
        }

        let data = DailyEditData::from_pairs(pairs);

        // Test existing articles
        assert_eq!(data.get(0), 0);
        assert_eq!(data.get(100), 1000);
        assert_eq!(data.get(500), 5000);

        // Test non-existing articles (odd numbers)
        assert_eq!(data.get(1), 0);
        assert_eq!(data.get(101), 0);
        assert_eq!(data.get(999), 0);
    }

    #[test]
    #[ignore] // Run with: cargo test -- --ignored
    fn test_load_real_mlwiki_data() {
        // This test requires real data: data/mlwiki/pageedits/pageedits.parquet
        // Run with: cargo test -p topictrend_core test_load_real_mlwiki_data -- --ignored --nocapture

        let engine = PageEditsEngine::new("mlwiki");

        // Check basic stats
        println!("Loaded {} dates", engine.daily_edits.len());
        assert!(
            engine.daily_edits.len() > 0,
            "Should have loaded some dates"
        );

        // Test getting a date range
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();

        // Try to get any article - we don't know specific QIDs, just verify it doesn't crash
        let trend = engine.get_article_trend(1, start, end);
        println!("Article 1 trend has {} data points", trend.len());

        println!("Successfully loaded and queried mlwiki pageedits!");
    }
}
