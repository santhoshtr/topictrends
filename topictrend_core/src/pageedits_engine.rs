use crate::direct_map::DirectMap;
use crate::{graphbuilder::GraphBuilder, wikigraph::WikiGraph};
use chrono::{Datelike, NaiveDate};
use parquet::file::reader::{FileReader, SerializedFileReader};
use parquet::record::RowAccessor;
use std::collections::VecDeque;
use std::fmt;
use std::fs::File;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use std::{collections::HashMap, error::Error};

/// Environment variable controlling how many distinct dates `PageEditsEngine`
/// keeps in its in-memory edit cache. A value of `0` disables the bound.
/// Default: 120 days (~4 months). Mirrors the pageview cache knob.
const PAGEEDIT_CACHE_DAYS_ENV: &str = "TOPICTREND_PAGEEDIT_CACHE_DAYS";
const DEFAULT_PAGEEDIT_CACHE_DAYS: usize = 120;

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

/// Bounded per-date edit cache, mirroring `PageViewEngine`'s
/// `BoundedDailyViews`. Each entry is a `DailyEditData` (sparse parallel
/// arrays of only the articles edited on that date). FIFO eviction keeps the
/// read path free of access-order bookkeeping so concurrent readers can run
/// under the `RwLock`. Callers hold `Arc` snapshots of the range they need
/// (see [`PageEditsEngine::load_history_for_date_range`]), so mid-request
/// eviction is safe.
#[derive(Debug)]
struct BoundedDailyEdits {
    map: HashMap<NaiveDate, Arc<DailyEditData>>,
    insert_order: VecDeque<NaiveDate>,
    capacity: usize,
}

impl BoundedDailyEdits {
    fn new(capacity: usize) -> Self {
        Self {
            map: HashMap::new(),
            insert_order: VecDeque::new(),
            capacity,
        }
    }

    fn get(&self, date: &NaiveDate) -> Option<Arc<DailyEditData>> {
        self.map.get(date).cloned()
    }

    /// Insert `data` for `date`. If the cache is at capacity, evict the
    /// oldest-inserted entries. A capacity of `0` means "unlimited".
    fn insert(&mut self, date: NaiveDate, data: Arc<DailyEditData>) {
        if self.map.contains_key(&date) {
            return;
        }
        if self.capacity > 0 {
            while self.map.len() >= self.capacity {
                match self.insert_order.pop_front() {
                    Some(oldest) => {
                        self.map.remove(&oldest);
                    }
                    None => break,
                }
            }
        }
        self.map.insert(date, data);
        self.insert_order.push_back(date);
    }

    fn len(&self) -> usize {
        self.map.len()
    }

    fn capacity(&self) -> usize {
        self.capacity
    }
}

/// Read the cache-size cap from the environment, falling back to
/// [`DEFAULT_PAGEEDIT_CACHE_DAYS`]. A value of `0` disables the bound.
fn pageedit_cache_capacity() -> usize {
    match std::env::var(PAGEEDIT_CACHE_DAYS_ENV) {
        Ok(s) => s.parse().unwrap_or(DEFAULT_PAGEEDIT_CACHE_DAYS),
        Err(_) => DEFAULT_PAGEEDIT_CACHE_DAYS,
    }
}

/// Load a per-day pageedit Parquet `(qid, edit_count)` and produce a sparse
/// `DailyEditData` keyed by current dense article ID. QIDs absent from
/// `dense_map` (articles deleted since the file was written) and zero-count
/// rows are dropped. Uses the raw synchronous `parquet` reader (not Polars)
/// so it is safe to call from inside an async runtime — handler code reaches
/// this path without `spawn_blocking`.
fn load_pageedit_parquet(
    path: &str,
    dense_map: &DirectMap,
    num_articles: usize,
) -> Result<DailyEditData, Box<dyn Error>> {
    let file = File::open(path)?;
    let reader = SerializedFileReader::new(file)?;
    let row_iter = reader.get_row_iter(None)?;

    let mut pairs: Vec<(u32, u32)> = Vec::new();
    for row_result in row_iter {
        let row = row_result?;
        let qid = row.get_uint(0)?;
        let edit_count = row.get_uint(1)?;
        if edit_count == 0 {
            continue;
        }
        if let Some(dense_id) = dense_map.get(qid)
            && (dense_id as usize) < num_articles
        {
            pairs.push((dense_id, edit_count));
        }
    }

    Ok(DailyEditData::from_pairs(pairs))
}

#[derive(Debug)]
pub struct PageEditsEngine {
    // Bounded, lazily-populated per-date cache. Per-day Parquet files are
    // loaded on first access (same layout as pageviews).
    daily_edits: RwLock<BoundedDailyEdits>,
    wiki: String,
    // `Arc<WikiGraph>` so the graph can be shared with PageViewEngine and
    // GoogleSearchEngine for the same wiki — see EngineService.
    wikigraph: Arc<WikiGraph>,
    top_categories_cache: RwLock<TopCategoriesCache>,
}

impl PageEditsEngine {
    /// Build a `PageEditsEngine` that owns its own `WikiGraph`. Convenient
    /// for CLI tools and tests; the web server should use
    /// [`PageEditsEngine::with_graph`] so the graph is shared across the
    /// metric engines for the same wiki.
    pub fn new(wiki: &str) -> Self {
        let graph: WikiGraph = GraphBuilder::new(wiki)
            .build()
            .expect("Error while building graph");
        Self::with_graph(wiki, Arc::new(graph))
    }

    /// Build a `PageEditsEngine` against a pre-built `WikiGraph` shared
    /// with other metric engines for the same wiki. Edit data is loaded
    /// lazily per date from per-day Parquet files on first access.
    pub fn with_graph(wiki: &str, wikigraph: Arc<WikiGraph>) -> Self {
        let capacity = pageedit_cache_capacity();
        Self {
            wiki: wiki.to_string(),
            daily_edits: RwLock::new(BoundedDailyEdits::new(capacity)),
            wikigraph,
            top_categories_cache: RwLock::new(TopCategoriesCache::new()),
        }
    }

    pub fn get_wikigraph(&self) -> &WikiGraph {
        &self.wikigraph
    }

    /// Returns the current and configured size of the edit cache as
    /// `(current_entries, capacity)`. A capacity of `0` is unbounded.
    pub fn cache_stats(&self) -> (usize, usize) {
        let cache = self.daily_edits.read().expect("daily_edits lock poisoned");
        (cache.len(), cache.capacity())
    }

    /// Ensure every date in `[start_date, end_date]` that has edit data on
    /// disk is present in the in-memory cache, and return a snapshot of the
    /// range as `Arc` clones. Mirrors
    /// [`PageViewEngine::load_history_for_date_range`]: the returned `Arc`
    /// clones keep the range alive even if a concurrent request evicts these
    /// dates from the bounded cache immediately afterward. Missing dates (no
    /// file on disk) are simply absent — callers treat absence as zero edits.
    pub fn load_history_for_date_range(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<HashMap<NaiveDate, Arc<DailyEditData>>, Box<dyn Error>> {
        // Phase 1: snapshot cached entries, collect missing dates.
        let (mut snapshot, missing): (HashMap<NaiveDate, Arc<DailyEditData>>, Vec<NaiveDate>) = {
            let cache = self.daily_edits.read().expect("daily_edits lock poisoned");
            let mut snap = HashMap::new();
            let mut miss = Vec::new();
            let mut curr = start_date;
            while curr <= end_date {
                match cache.get(&curr) {
                    Some(v) => {
                        snap.insert(curr, v);
                    }
                    None => miss.push(curr),
                }
                curr = curr.succ_opt().unwrap();
            }
            (snap, miss)
        };

        if missing.is_empty() {
            return Ok(snapshot);
        }

        // Phase 2: load missing dates from disk with no lock held.
        let mut loaded: Vec<(NaiveDate, Arc<DailyEditData>)> = Vec::with_capacity(missing.len());
        for date in missing {
            if let Some(day_data) = self.load_daily_edit(date)? {
                let arc = Arc::new(day_data);
                snapshot.insert(date, Arc::clone(&arc));
                loaded.push((date, arc));
            }
        }

        // Phase 3: publish loaded entries under the write lock.
        if !loaded.is_empty() {
            let mut cache = self.daily_edits.write().expect("daily_edits lock poisoned");
            for (date, data) in loaded {
                cache.insert(date, data);
            }
        }

        Ok(snapshot)
    }

    fn load_daily_edit(&self, date: NaiveDate) -> Result<Option<DailyEditData>, Box<dyn Error>> {
        let num_articles = self.wikigraph.art_dense_to_original.len();

        let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string());
        let parquet_filename = format!(
            "{}/{}/pageedits/{}/{:02}/{:02}.parquet",
            data_dir,
            self.wiki,
            date.year(),
            date.month(),
            date.day()
        );

        if !Path::new(&parquet_filename).exists() {
            return Ok(None);
        }

        let day_data = load_pageedit_parquet(
            &parquet_filename,
            &self.wikigraph.art_original_to_dense,
            num_articles,
        )?;

        Ok(Some(day_data))
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

        let snapshot = self
            .load_history_for_date_range(start_date, end_date)
            .expect("Error in loading pageedits history");

        let mut curr = start_date;
        while curr <= end_date {
            let edit_count = snapshot
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
        self.get_categories_trend(&[category_qid], depth, start_date, end_date)
    }

    /// Combined daily edits over the union of several categories' article
    /// sets. An article in more than one category is counted once per day.
    pub fn get_categories_trend(
        &self,
        category_qids: &[u32],
        depth: u32,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Vec<(NaiveDate, u64)> {
        let mut results = Vec::new();

        let article_mask = self
            .wikigraph
            .get_articles_in_categories_as_dense(category_qids, depth);

        // Optimization: If mask is empty, return early
        if article_mask.is_empty() {
            eprintln!(
                "Could not find articles in categories: {}/{:?}",
                self.wiki, category_qids
            );
            return vec![];
        }

        println!(
            "Found {} articles in {} categories ({}) at depth {}",
            article_mask.len(),
            category_qids.len(),
            self.wiki,
            depth
        );

        let snapshot = self
            .load_history_for_date_range(start_date, end_date)
            .expect("Error in loading pageedits history");

        let mut curr = start_date;
        while curr <= end_date {
            let daily_total = if let Some(day_data) = snapshot.get(&curr) {
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

        let snapshot = self
            .load_history_for_date_range(start_date, end_date)
            .expect("Error in loading pageedits history");

        let mut curr = start_date;
        while curr <= end_date {
            if let Some(day_data) = snapshot.get(&curr) {
                for (article_dense_id, count) in day_data.iter() {
                    article_edits[article_dense_id as usize] += count;
                }
            }
            curr = curr.succ_opt().unwrap();
        }
        drop(snapshot);

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
                articles.sort_unstable_by_key(|b| std::cmp::Reverse(b.1));

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

    pub fn get_top_articles(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
        top_n: usize,
    ) -> Result<Vec<ArticleRank>, Box<dyn Error>> {
        let num_articles = self.wikigraph.art_dense_to_original.len();
        let mut article_edits = vec![0u64; num_articles];

        let snapshot = self
            .load_history_for_date_range(start_date, end_date)
            .expect("Error in loading pageedits history");

        let mut curr = start_date;
        while curr <= end_date {
            if let Some(day_data) = snapshot.get(&curr) {
                for (article_dense_id, count) in day_data.iter() {
                    article_edits[article_dense_id as usize] += count as u64;
                }
            }
            curr = curr.succ_opt().unwrap();
        }

        let mut ranked: Vec<(usize, u64)> = article_edits.into_iter().enumerate().collect();
        ranked.sort_unstable_by_key(|b| std::cmp::Reverse(b.1));

        let results = ranked
            .into_iter()
            .take(top_n)
            .filter(|(_, total_edits)| *total_edits > 0)
            .map(|(article_dense_id, total_edits)| ArticleRank {
                article_qid: self.wikigraph.art_dense_to_original[article_dense_id],
                total_edits,
            })
            .collect();

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

        let snapshot = self
            .load_history_for_date_range(start_date, end_date)
            .expect("Error in loading pageedits history");

        // Aggregate edits for each article
        let mut article_edits: Vec<(u32, u64)> = Vec::new();

        for article_dense_id in article_mask.iter() {
            let mut total_edits = 0u64;

            let mut curr = start_date;
            while curr <= end_date {
                if let Some(day_data) = snapshot.get(&curr) {
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
        article_edits.sort_unstable_by_key(|b| std::cmp::Reverse(b.1));

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
        // This test requires real data under data/mlwiki/pageedits/{Y}/{M}/{D}.parquet
        // Run with: cargo test -p topictrend_core test_load_real_mlwiki_data -- --ignored --nocapture

        let engine = PageEditsEngine::new("mlwiki");

        // Test getting a date range — lazily populates the bounded cache.
        let start = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();

        // Try to get any article - we don't know specific QIDs, just verify it doesn't crash
        let trend = engine.get_article_trend(1, start, end);
        println!("Article 1 trend has {} data points", trend.len());

        let (cached, capacity) = engine.cache_stats();
        println!("Cache holds {} dates (capacity {})", cached, capacity);

        println!("Successfully loaded and queried mlwiki pageedits!");
    }
}
