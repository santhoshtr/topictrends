use crate::{graphbuilder::GraphBuilder, wikigraph::WikiGraph};
use chrono::{Datelike, NaiveDate};
use roaring::RoaringBitmap;
use std::fmt;
use std::io::Read;
use std::time::{Duration, Instant};
use std::{collections::HashMap, error::Error, fs::File};

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

    fn get_ttl(start_date: NaiveDate, end_date: NaiveDate) -> Duration {
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
pub struct PageViewEngine {
    // Map Date -> Vector of pageviews (Index is Dense Article ID)
    // We use Arc to make it cheap to clone/share across web threads
    daily_views: HashMap<NaiveDate, Vec<u32>>,
    wiki: String,
    wikigraph: WikiGraph,
    top_categories_cache: TopCategoriesCache,
}

pub fn load_bin_file(path: &str, expected_size: usize) -> Result<Vec<u32>, Box<dyn Error>> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    // Simple Header Check
    if &buffer[0..4] != b"VIEW" {
        panic!("Invalid Magic");
    }

    // Cast raw bytes to u32 slice (unsafe/fast or using bytemuck)
    // This skips parsing entirely.
    let (_head, body, _tail) = unsafe { buffer[16..].align_to::<u32>() };

    if body.len() != expected_size {
        eprintln!(
            "Graph/View Mismatch! Re-run the pipeline.Expected {} Got:{}",
            expected_size,
            body.len()
        );
    }

    Ok(body.to_vec())
}

impl PageViewEngine {
    pub fn new(wiki: &str) -> Self {
        let graph_builder = GraphBuilder::new(wiki);
        let graph: WikiGraph = graph_builder.build().expect("Error while building graph");
        Self {
            wiki: wiki.to_string(),
            daily_views: HashMap::new(),
            wikigraph: graph,
            top_categories_cache: TopCategoriesCache::new(),
        }
    }

    pub fn get_wikigraph(&self) -> &WikiGraph {
        &self.wikigraph
    }

    pub fn get_category_trend(
        &mut self,
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

        self.load_history_for_date_range(start_date, end_date)
            .expect("Error in loading pageview history");

        let mut curr = start_date;
        while curr <= end_date {
            if let Some(day_data) = self.daily_views.get(&curr) {
                // High Performance Loop
                // Summing values only for articles in the category
                let mut daily_total: u64 = 0;

                // RoaringBitmap iter is sorted, which is cache-friendly
                for article_dense_id in article_mask.iter() {
                    // distinct get is O(1)
                    // We use get unchecked for max speed if we are sure indices are valid
                    if let Some(&views) = day_data.get(article_dense_id as usize) {
                        daily_total += views as u64;
                    }
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
        &mut self,
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
        // println!(
        //     "Found {} articles in category {}/{}",
        //     article_mask.len(),
        //     self.wiki,
        //     &article
        // );
        let mut curr: NaiveDate = start_date;

        self.load_history_for_date_range(start_date, end_date)
            .expect("Error in loading pageview history");

        while curr <= end_date {
            match self.daily_views.get(&curr) {
                Some(day_data) => {
                    let mut daily_total: u64 = 0;
                    for article_dense_id in article_mask.iter() {
                        // distinct get is O(1)
                        // We use get unchecked for max speed if we are sure indices are valid
                        if let Some(&views) = day_data.get(article_dense_id as usize) {
                            daily_total += views as u64;
                        }
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

    pub fn load_history_for_date_range(
        &mut self,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Result<(), Box<dyn Error>> {
        let mut curr_date = start_date;

        while curr_date <= end_date {
            if !self.daily_views.contains_key(&curr_date) {
                // Attempt to load the data for the date if not in cache
                if let Some(day_vec) = self.load_daily_view(curr_date)? {
                    self.daily_views.insert(curr_date, day_vec);
                }
            }
            curr_date = curr_date.succ_opt().unwrap();
        }

        Ok(())
    }

    fn load_daily_view(&self, date: NaiveDate) -> Result<Option<Vec<u32>>, Box<dyn Error>> {
        let num_articles = self.wikigraph.art_dense_to_original.len();

        let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string());
        let bin_filename = format!(
            "{}/{}/pageviews/{}/{:02}/{:02}.bin",
            data_dir,
            self.wiki,
            date.year(),
            date.month(),
            date.day()
        );

        if !std::path::Path::new(&bin_filename).exists() {
            // eprintln!(
            //     "Could not find page view data for {} at {}",
            //     date, bin_filename
            // );
            return Ok(None);
        }

        let day_vec = load_bin_file(&bin_filename, num_articles)
            .expect("Error reading the pageview bin file");
        println!(
            "Loaded page views for {} on {}, found {} articles",
            self.wiki,
            date,
            day_vec.len()
        );

        Ok(Some(day_vec))
    }

    /// Clear the top categories cache
    pub fn clear_top_categories_cache(&mut self) {
        self.top_categories_cache.clear();
    }

    /// Returns top N categories by DIRECT article views for a date range.
    pub fn get_top_categories(
        &mut self,
        start_date: NaiveDate,
        end_date: NaiveDate,
        top_n: usize,
    ) -> Result<Vec<CategoryRank>, Box<dyn Error>> {
        // Check cache first
        let cache_key = TopCategoriesCacheKey {
            start: start_date,
            end: end_date,
            top_n,
        };

        if let Some(cached_result) = self.top_categories_cache.get(&cache_key) {
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

        self.load_history_for_date_range(start_date, end_date)
            .expect("Error in loading pageview history");

        let mut curr = start_date;
        while curr <= end_date {
            if let Some(day_vec) = self.daily_views.get(&curr) {
                // Vectorized addition (compiler auto-vectorizes this loop)
                for (article_dense_id, &views) in day_vec.iter().enumerate() {
                    article_views[article_dense_id] += views;
                }
            }
            curr = curr.succ_opt().unwrap();
        }

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
                articles.sort_unstable_by(|a, b| b.1.cmp(&a.1));

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

        // Cache the result
        self.top_categories_cache.insert(cache_key, results.clone());

        Ok(results)
    }

    pub fn get_top_articles_in_category(
        &mut self,
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
        self.load_history_for_date_range(start_date, end_date)?;

        // Aggregate views for each article
        let mut article_views: Vec<(u32, u64)> = Vec::new();

        for article_dense_id in article_mask.iter() {
            let mut total_views = 0u64;

            let mut curr = start_date;
            while curr <= end_date {
                if let Some(day_data) = self.daily_views.get(&curr) {
                    if let Some(&views) = day_data.get(article_dense_id as usize) {
                        total_views += views as u64;
                    }
                }
                curr = curr.succ_opt().unwrap();
            }

            if total_views > 0 {
                let article_qid = self.wikigraph.art_dense_to_original[article_dense_id as usize];
                article_views.push((article_qid, total_views));
            }
        }

        // Sort by views descending
        article_views.sort_unstable_by(|a, b| b.1.cmp(&a.1));

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
