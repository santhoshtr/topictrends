use crate::{graphbuilder::GraphBuilder, wikigraph::WikiGraph};
use chrono::{Datelike, NaiveDate};
use roaring::RoaringBitmap;
use std::io::Read;
use std::{collections::HashMap, error::Error, fs::File};

pub struct PageViewEngine {
    // Map Date -> Vector of pageviews (Index is Dense Article ID)
    // We use Arc to make it cheap to clone/share across web threads
    daily_views: HashMap<NaiveDate, Vec<u32>>,
    wiki: String,
    wikigraph: WikiGraph,
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
        panic!("Graph/View Mismatch! Re-run the pipeline.");
    }

    Ok(body.to_vec())
}

impl PageViewEngine {
    pub fn new(wiki: &str) -> Self {
        let graph_builder = GraphBuilder::new(wiki);
        let graph: WikiGraph = graph_builder.build().expect("Error while building graph");
        return Self {
            wiki: wiki.to_string(),
            daily_views: HashMap::new(),
            wikigraph: graph,
        };
    }

    /// Calculate the total pageviews for a set of articles over time.
    pub fn get_category_trend(
        &mut self,
        wiki_cat_id: u32,
        depth: u8,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Vec<(NaiveDate, u64)> {
        let mut results = Vec::new();

        // 2. PREPARE THE MASK
        // This is the key step. We query the topology first.
        // The graph returns the RoaringBitmap of all relevant article IDs.
        let article_mask = self.wikigraph.get_articles_in_category(wiki_cat_id, depth);

        // Optimization: If mask is empty, return early
        if article_mask.is_empty() {
            return vec![];
        }

        let mut curr = start_date;

        self.load_history_for_date_range(start_date, end_date)
            .expect("Error in loading pageview history");

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

        let data_dir = std::env::var("DATA_DIR").expect("DATA_DIR not set in .env");
        let bin_filename = format!(
            "{}/{}/{}/{}/{}.bin",
            data_dir,
            self.wiki,
            date.year(),
            date.month(),
            date.day()
        );

        if !std::path::Path::new(&bin_filename).exists() {
            eprintln!("Could not find page view data for {}", date);
            return Ok(None);
        }

        let day_vec = load_bin_file(&bin_filename, num_articles)
            .expect("Error reading the pageview bin file");
        println!("Loaded views for {}", date);

        Ok(Some(day_vec))
    }
}
