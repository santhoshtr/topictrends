use crate::{direct_map::DirectMap, graphbuilder::GraphBuilder, wikigraph::WikiGraph};
use chrono::{Datelike, NaiveDate};
use parquet::file::reader::{FileReader, SerializedFileReader};
use parquet::record::RowAccessor;
use roaring::RoaringBitmap;
use std::collections::VecDeque;
use std::fmt;
use std::fs::File;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use std::{collections::HashMap, error::Error};

/// Environment variable controlling how many distinct dates `PageViewEngine`
/// keeps in its in-memory pageview cache. A value of `0` disables the bound
/// (preserves pre-bound behavior). Default: 120 days (~4 months).
const PAGEVIEW_CACHE_DAYS_ENV: &str = "TOPICTREND_PAGEVIEW_CACHE_DAYS";
const DEFAULT_PAGEVIEW_CACHE_DAYS: usize = 120;

#[derive(Debug, Clone)]
pub struct ArticleRank {
    pub article_qid: u32,
    pub total_views: u64,
}

impl fmt::Display for ArticleRank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Article: Q{} - Views: {}",
            self.article_qid, self.total_views
        )
    }
}

#[derive(Debug, Clone)]
pub struct CategoryRank {
    pub category_qid: u32,
    pub total_views: u64,
    pub top_articles: Vec<ArticleRank>,
}

impl fmt::Display for CategoryRank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Category: Q{}", self.category_qid)?;
        writeln!(f, "Total Views: {}", self.total_views)?;
        writeln!(f, "Top Articles:")?;
        for (i, article) in self.top_articles.iter().enumerate() {
            writeln!(f, "{:>2}. {}", i + 1, article)?;
        }
        Ok(())
    }
}

/// Sparse storage for pageview counts on a single date.
///
/// A typical enwiki day contains pageviews for ~1.5 M of its ~7 M articles;
/// the previous dense `Vec<u32>` representation indexed by dense article ID
/// allocated 28 MB per cached day and held mostly zeros. This struct stores
/// only the articles that actually have views as two parallel sorted arrays
/// (`article_ids[i]`, `views[i]`), cutting the per-day footprint roughly
/// 3-5× and skipping zeros during aggregation. Lookup is O(log n) via
/// binary search; iteration yields `(dense_id, views)` pairs in dense-id
/// order, which lets aggregation use two-pointer merges or `RoaringBitmap`
/// intersections without first materializing a dense vector.
#[derive(Debug, Clone)]
pub struct DailyPageViewData {
    article_ids: Vec<u32>, // sorted dense article IDs
    views: Vec<u32>,       // corresponding view counts (same length, same order)
}

impl DailyPageViewData {
    /// O(log n) lookup. Returns 0 if the article had no views on this date.
    #[inline]
    pub fn get(&self, article_dense_id: u32) -> u32 {
        self.article_ids
            .binary_search(&article_dense_id)
            .map(|idx| self.views[idx])
            .unwrap_or(0)
    }

    /// Yields `(dense_id, views)` pairs sorted by dense_id.
    pub fn iter(&self) -> impl Iterator<Item = (u32, u32)> + '_ {
        self.article_ids
            .iter()
            .copied()
            .zip(self.views.iter().copied())
    }

    /// Build from unsorted `(dense_id, views)` pairs. Sorts by dense_id so
    /// `get` and `iter` can rely on the ordering invariant.
    pub fn from_pairs(mut pairs: Vec<(u32, u32)>) -> Self {
        pairs.sort_unstable_by_key(|(id, _)| *id);
        let (article_ids, views): (Vec<u32>, Vec<u32>) = pairs.into_iter().unzip();
        Self { article_ids, views }
    }

    pub fn is_empty(&self) -> bool {
        self.article_ids.is_empty()
    }

    pub fn len(&self) -> usize {
        self.article_ids.len()
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

/// Bounded per-date pageview cache.
///
/// Each entry is a `DailyPageViewData` — sparse parallel arrays of only the
/// articles that had views on that date. For enwiki this is ~6-12 MB per
/// cached day (vs ~28 MB for the previous dense representation). The cache
/// still has to be bounded though: at 6-12 MB × N days × M wikis, an
/// unbounded cache still reaches double-digit gigabytes after extended
/// browsing. This wrapper caps the number of cached dates and evicts in
/// FIFO insertion order.
///
/// FIFO (rather than LRU) keeps the read path lock-free of bookkeeping —
/// reads don't need to update access order, so multiple readers can run
/// concurrently under `RwLock`. Callers protect themselves against
/// mid-request eviction by holding an `Arc` snapshot of the range they
/// need (see [`PageViewEngine::load_history_for_date_range`]).
#[derive(Debug)]
struct BoundedDailyViews {
    map: HashMap<NaiveDate, Arc<DailyPageViewData>>,
    insert_order: VecDeque<NaiveDate>,
    capacity: usize,
}

impl BoundedDailyViews {
    fn new(capacity: usize) -> Self {
        Self {
            map: HashMap::new(),
            insert_order: VecDeque::new(),
            capacity,
        }
    }

    fn get(&self, date: &NaiveDate) -> Option<Arc<DailyPageViewData>> {
        self.map.get(date).cloned()
    }

    /// Insert `data` for `date`. If the cache is at capacity, evict the
    /// oldest-inserted entries to make room. A capacity of `0` means
    /// "unlimited" — entries are never evicted.
    fn insert(&mut self, date: NaiveDate, data: Arc<DailyPageViewData>) {
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

#[derive(Debug)]
pub struct PageViewEngine {
    // RwLock for interior mutability: concurrent reads hold read lock,
    // cache misses take write lock only for the missing dates.
    // Eviction (FIFO) happens under the write lock during inserts.
    daily_views: RwLock<BoundedDailyViews>,
    wiki: String,
    wikigraph: WikiGraph,
    top_categories_cache: RwLock<TopCategoriesCache>,
}

/// Read the cache-size cap from the environment, falling back to
/// [`DEFAULT_PAGEVIEW_CACHE_DAYS`]. A value of `0` disables the bound.
fn pageview_cache_capacity() -> usize {
    match std::env::var(PAGEVIEW_CACHE_DAYS_ENV) {
        Ok(s) => s.parse().unwrap_or(DEFAULT_PAGEVIEW_CACHE_DAYS),
        Err(_) => DEFAULT_PAGEVIEW_CACHE_DAYS,
    }
}

/// Load a per-day pageview Parquet and produce a sparse `DailyPageViewData`
/// keyed by current dense article ID. Entries for QIDs not in `dense_map`
/// (articles deleted since the file was written) are silently dropped —
/// the correct behavior for analytics on the active article set. Zero-view
/// rows are also dropped: they carry no information and would only inflate
/// the sparse arrays.
///
/// `num_articles` is the size of the current dense article space — used
/// only as a defensive upper bound on returned dense IDs.
///
/// Uses the raw `parquet` crate (purely synchronous) rather than Polars so
/// this is safe to call from inside an async runtime — handler code reaches
/// this path without `spawn_blocking`, and Polars' lazy reader internally
/// starts a Tokio runtime which panics when one is already active.
fn load_pageview_parquet(
    path: &str,
    dense_map: &DirectMap,
    num_articles: usize,
) -> Result<DailyPageViewData, Box<dyn Error>> {
    let file = File::open(path)?;
    let reader = SerializedFileReader::new(file)?;
    let row_iter = reader.get_row_iter(None)?;

    let mut pairs: Vec<(u32, u32)> = Vec::new();
    for row_result in row_iter {
        let row = row_result?;
        let qid = row.get_uint(0)?;
        let views = row.get_uint(1)?;
        if views == 0 {
            continue;
        }
        if let Some(dense_id) = dense_map.get(qid)
            && (dense_id as usize) < num_articles
        {
            pairs.push((dense_id, views));
        }
    }

    Ok(DailyPageViewData::from_pairs(pairs))
}

impl PageViewEngine {
    pub fn new(wiki: &str) -> Self {
        let graph_builder = GraphBuilder::new(wiki);
        let graph: WikiGraph = graph_builder.build().expect("Error while building graph");
        let capacity = pageview_cache_capacity();
        Self {
            wiki: wiki.to_string(),
            daily_views: RwLock::new(BoundedDailyViews::new(capacity)),
            wikigraph: graph,
            top_categories_cache: RwLock::new(TopCategoriesCache::new()),
        }
    }

    /// Returns the current and configured size of the pageview cache.
    /// Returned as `(current_entries, capacity)`. A capacity of `0`
    /// indicates the cache is unbounded.
    pub fn cache_stats(&self) -> (usize, usize) {
        let cache = self.daily_views.read().expect("daily_views lock poisoned");
        (cache.len(), cache.capacity())
    }

    pub fn get_wikigraph(&self) -> &WikiGraph {
        &self.wikigraph
    }

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
            &category_qid,
            depth
        );

        let snapshot = self
            .load_history_for_date_range(start_date, end_date)
            .expect("Error in loading pageview history");

        let mut curr = start_date;
        while curr <= end_date {
            if let Some(day_data) = snapshot.get(&curr) {
                // Sum views only for articles in the category. `article_mask`
                // and `day_data.article_ids` are both sorted by dense_id, so
                // binary-search lookup is cache-friendly. For small masks
                // this is much cheaper than iterating all non-zero articles
                // in the day; for very large masks the cost converges to a
                // linear scan, which is still fine.
                let mut daily_total: u64 = 0;
                for article_dense_id in article_mask.iter() {
                    daily_total += day_data.get(article_dense_id) as u64;
                }
                results.push((curr, daily_total));
            } else {
                results.push((curr, 0));
            }
            curr = curr.succ_opt().unwrap();
        }

        results
    }

    /// Calculate the total pageviews for a set of articles over time.
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
                    self.wiki, &article_qid
                );
                return vec![];
            }
        };

        let mut article_mask: RoaringBitmap = RoaringBitmap::new();

        article_mask.insert(article_dense_id);

        // Optimization: If mask is empty, return early
        if article_mask.is_empty() {
            eprintln!(
                "Could not find articles in category: {}/{}",
                self.wiki, &article_qid
            );
            return vec![];
        }

        let snapshot = self
            .load_history_for_date_range(start_date, end_date)
            .expect("Error in loading pageview history");

        let mut curr: NaiveDate = start_date;

        while curr <= end_date {
            match snapshot.get(&curr) {
                Some(day_data) => {
                    let mut daily_total: u64 = 0;
                    for article_dense_id in article_mask.iter() {
                        daily_total += day_data.get(article_dense_id) as u64;
                    }
                    results.push((curr, daily_total));
                }
                None => {
                    //eprintln!("Daily views for {} is not available", curr);
                    results.push((curr, 0));
                }
            }
            curr = curr.succ_opt().unwrap();
        }
        results
    }

    /// Ensure every date in `[start_date, end_date]` that has pageview data
    /// on disk is present in the in-memory cache, and return a snapshot of
    /// the range as `Arc` clones.
    ///
    /// Returning a snapshot (rather than asking callers to re-acquire the
    /// cache lock) is what makes the bounded-cache eviction safe under
    /// concurrent requests: even if another thread inserts new dates and
    /// evicts entries from this range immediately after we return, this
    /// caller's `Arc` clones keep the data alive for the duration of the
    /// aggregation. Missing dates (no file on disk) are simply absent from
    /// the snapshot — callers treat absence as zero views.
    pub fn load_history_for_date_range(
        &self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<HashMap<NaiveDate, Arc<DailyPageViewData>>, Box<dyn Error>> {
        // Phase 1: snapshot cached entries in the requested range, identify
        // missing dates. Read lock dropped immediately after.
        let (mut snapshot, missing): (
            HashMap<NaiveDate, Arc<DailyPageViewData>>,
            Vec<NaiveDate>,
        ) = {
            let cache = self.daily_views.read().expect("daily_views lock poisoned");
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

        // Phase 2: load missing dates from disk with no lock held. Pair the
        // loaded data with its date so we can both return it and cache it.
        let mut loaded: Vec<(NaiveDate, Arc<DailyPageViewData>)> =
            Vec::with_capacity(missing.len());
        for date in missing {
            if let Some(day_data) = self.load_daily_view(date)? {
                let arc = Arc::new(day_data);
                snapshot.insert(date, Arc::clone(&arc));
                loaded.push((date, arc));
            }
        }

        // Phase 3: publish loaded entries under the write lock. The bounded
        // cache may evict older entries — but this caller has already taken
        // `Arc` clones for its range, so eviction is safe.
        if !loaded.is_empty() {
            let mut cache = self.daily_views.write().expect("daily_views lock poisoned");
            for (date, data) in loaded {
                cache.insert(date, data);
            }
        }

        Ok(snapshot)
    }

    fn load_daily_view(
        &self,
        date: NaiveDate,
    ) -> Result<Option<DailyPageViewData>, Box<dyn Error>> {
        let num_articles = self.wikigraph.art_dense_to_original.len();

        let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string());
        let parquet_filename = format!(
            "{}/{}/pageviews/{}/{:02}/{:02}.parquet",
            data_dir,
            self.wiki,
            date.year(),
            date.month(),
            date.day()
        );

        if !Path::new(&parquet_filename).exists() {
            return Ok(None);
        }

        let day_data = load_pageview_parquet(
            &parquet_filename,
            &self.wikigraph.art_original_to_dense,
            num_articles,
        )?;
        println!(
            "Loaded page views for {} on {}, found {} articles with views",
            self.wiki,
            date,
            day_data.len()
        );

        Ok(Some(day_data))
    }

    /// Clear the top categories cache
    pub fn clear_top_categories_cache(&self) {
        self.top_categories_cache
            .write()
            .expect("top_categories_cache lock poisoned")
            .clear();
    }

    /// Returns top N categories by DIRECT article views for a date range.
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

        let num_articles = self.wikigraph.art_dense_to_original.len(); // Approx 7M for
        // enwiki
        let num_cats = self.wikigraph.cat_dense_to_original.len(); // Approx 2.5M for enwiki

        // Phase 1: Aggregation (Sum relevant days)
        // We create a temporary view vector for the range.
        // We can parallelize this sum if the range is huge, but usually linear is fine.
        let mut article_views = vec![0u32; num_articles];

        let snapshot = self
            .load_history_for_date_range(start_date, end_date)
            .expect("Error in loading pageview history");

        let mut curr = start_date;
        while curr <= end_date {
            if let Some(day_data) = snapshot.get(&curr) {
                // Iterate only non-zero entries (huge win over the previous
                // dense scan: ~1.5 M iters per day for enwiki instead of 7 M).
                for (article_dense_id, views) in day_data.iter() {
                    article_views[article_dense_id as usize] += views;
                }
            }
            curr = curr.succ_opt().unwrap();
        }
        drop(snapshot);

        // Phase 2: Scatter (Article -> Category)
        // We need an atomic accumulator or thread-local storage for parallel write.
        // For simplicity/speed balance, a single-threaded scatter is often fast enough
        // because it avoids synchronization overhead.
        let mut cat_scores = vec![0u64; num_cats];
        let mut cat_articles: Vec<Vec<(u32, u32)>> = vec![Vec::new(); num_cats];

        for (art_dense_id, &views) in article_views.iter().enumerate() {
            if views == 0 {
                continue;
            }

            // Use the Article->Category CSR
            let article_categories = self.wikigraph.article_cats.get(art_dense_id as u32);

            for &cat_dense_id in article_categories {
                // Safety: cat_dense_id is guaranteed valid by graph construction
                unsafe {
                    *cat_scores.get_unchecked_mut(cat_dense_id as usize) += views as u64;
                }
                cat_articles[cat_dense_id as usize].push((art_dense_id as u32, views));
            }
        }

        // Phase 3: Sort & Top N
        // Create a list of indices to sort
        let mut ranked: Vec<usize> = (0..num_cats).collect();

        // Parallel sort is overkill for 2.5M integers, standard sort is fine.
        // We sort by score descending.
        ranked.sort_by(|&a, &b| cat_scores[b].cmp(&cat_scores[a]));

        //  Transform to Output
        let results: Vec<CategoryRank> = ranked
            .into_iter()
            .take(top_n)
            .filter(|&idx| cat_scores[idx] > 0) // Filter out zero view categories
            .map(|cat_dense_id| {
                // Sort articles for this category by views
                let mut articles = cat_articles[cat_dense_id].clone();
                articles.sort_unstable_by_key(|b| std::cmp::Reverse(b.1));

                let top_articles: Vec<ArticleRank> = articles
                    .into_iter()
                    .take(top_n)
                    .map(|(art_dense_id, views)| ArticleRank {
                        article_qid: self.wikigraph.art_dense_to_original[art_dense_id as usize],
                        total_views: views as u64,
                    })
                    .collect();

                CategoryRank {
                    category_qid: self.wikigraph.cat_dense_to_original[cat_dense_id],
                    total_views: cat_scores[cat_dense_id],
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

        let mut article_views = vec![0u64; num_articles];

        let snapshot = self.load_history_for_date_range(start_date, end_date)?;

        let mut curr = start_date;
        while curr <= end_date {
            if let Some(day_data) = snapshot.get(&curr) {
                for (article_dense_id, views) in day_data.iter() {
                    article_views[article_dense_id as usize] += views as u64;
                }
            }
            curr = curr.succ_opt().unwrap();
        }
        drop(snapshot);

        let mut ranked: Vec<(usize, u64)> = article_views.into_iter().enumerate().collect();
        ranked.sort_unstable_by_key(|b| std::cmp::Reverse(b.1));

        let results = ranked
            .into_iter()
            .take(top_n)
            .filter(|(_, total_views)| *total_views > 0)
            .map(|(article_dense_id, total_views)| ArticleRank {
                article_qid: self.wikigraph.art_dense_to_original[article_dense_id],
                total_views,
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
        // Get all articles in this category (depth 0 for direct children only)
        let article_mask = self
            .wikigraph
            .get_articles_in_category_as_dense(category_qid, depth)?;

        if article_mask.is_empty() {
            return Ok(CategoryRank {
                category_qid,
                total_views: 0,
                top_articles: vec![],
            });
        }

        // Load pageview history for the date range
        let snapshot = self.load_history_for_date_range(start_date, end_date)?;

        // Aggregate views for each article
        let mut article_views: Vec<(u32, u64)> = Vec::new();

        for article_dense_id in article_mask.iter() {
            let mut total_views = 0u64;

            let mut curr = start_date;
            while curr <= end_date {
                if let Some(day_data) = snapshot.get(&curr) {
                    total_views += day_data.get(article_dense_id) as u64;
                }
                curr = curr.succ_opt().unwrap();
            }

            if total_views > 0 {
                let article_qid = self.wikigraph.art_dense_to_original[article_dense_id as usize];
                article_views.push((article_qid, total_views));
            }
        }
        drop(snapshot);

        // Sort by views descending
        article_views.sort_unstable_by_key(|b| std::cmp::Reverse(b.1));

        // Take top N and convert to ArticleRank
        let top_articles: Vec<ArticleRank> = article_views
            .into_iter()
            .take(top_n)
            .map(|(article_qid, total_views)| ArticleRank {
                article_qid,
                total_views,
            })
            .collect();

        // Calculate total views for the category
        let total_views: u64 = top_articles.iter().map(|a| a.total_views).sum();

        Ok(CategoryRank {
            category_qid,
            total_views,
            top_articles,
        })
    }
}

#[cfg(test)]
mod bounded_cache_tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    fn data(pairs: Vec<(u32, u32)>) -> Arc<DailyPageViewData> {
        Arc::new(DailyPageViewData::from_pairs(pairs))
    }

    #[test]
    fn evicts_oldest_when_capacity_reached() {
        let mut cache = BoundedDailyViews::new(2);
        cache.insert(d(2025, 1, 1), data(vec![(0, 1)]));
        cache.insert(d(2025, 1, 2), data(vec![(0, 2)]));
        cache.insert(d(2025, 1, 3), data(vec![(0, 3)]));

        assert_eq!(cache.len(), 2);
        assert!(cache.get(&d(2025, 1, 1)).is_none(), "oldest must be evicted");
        assert!(cache.get(&d(2025, 1, 2)).is_some());
        assert!(cache.get(&d(2025, 1, 3)).is_some());
    }

    #[test]
    fn reinsert_is_noop() {
        let mut cache = BoundedDailyViews::new(2);
        let original = data(vec![(0, 1)]);
        cache.insert(d(2025, 1, 1), Arc::clone(&original));
        cache.insert(d(2025, 1, 1), data(vec![(0, 999)]));

        let stored = cache.get(&d(2025, 1, 1)).unwrap();
        assert!(Arc::ptr_eq(&stored, &original), "first insert wins");
    }

    #[test]
    fn capacity_zero_is_unbounded() {
        let mut cache = BoundedDailyViews::new(0);
        for i in 1..=10 {
            cache.insert(d(2025, 1, i), data(vec![(0, i as u32)]));
        }
        assert_eq!(cache.len(), 10);
        assert!(cache.get(&d(2025, 1, 1)).is_some(), "no eviction at capacity 0");
    }
}

#[cfg(test)]
mod daily_pageview_data_tests {
    use super::*;

    #[test]
    fn get_returns_zero_for_missing_article() {
        let d = DailyPageViewData::from_pairs(vec![(5, 100), (10, 200)]);
        assert_eq!(d.get(5), 100);
        assert_eq!(d.get(10), 200);
        assert_eq!(d.get(7), 0);
        assert_eq!(d.get(0), 0);
        assert_eq!(d.get(u32::MAX), 0);
    }

    #[test]
    fn from_pairs_sorts_by_dense_id() {
        let d = DailyPageViewData::from_pairs(vec![(30, 3), (10, 1), (20, 2)]);
        let items: Vec<(u32, u32)> = d.iter().collect();
        assert_eq!(items, vec![(10, 1), (20, 2), (30, 3)]);
    }

    #[test]
    fn empty_data_returns_zero_for_any_lookup() {
        let d = DailyPageViewData::from_pairs(vec![]);
        assert!(d.is_empty());
        assert_eq!(d.len(), 0);
        assert_eq!(d.get(42), 0);
    }

    #[test]
    fn binary_search_handles_many_entries() {
        // Even-numbered dense IDs only.
        let pairs: Vec<(u32, u32)> = (0..1000).step_by(2).map(|i| (i, i * 10)).collect();
        let d = DailyPageViewData::from_pairs(pairs);

        assert_eq!(d.get(0), 0);
        assert_eq!(d.get(100), 1000);
        assert_eq!(d.get(500), 5000);
        // Odd numbers are not present.
        assert_eq!(d.get(1), 0);
        assert_eq!(d.get(101), 0);
        assert_eq!(d.get(999), 0);
    }
}
