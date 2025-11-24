use chrono::NaiveDate;
use dotenv::dotenv;
use polars::{
    frame::DataFrame,
    prelude::{LazyFrame, PlPath},
};
use roaring::RoaringBitmap;
use std::{collections::HashMap, error::Error, fs::File, path::Path, sync::Arc};

use crate::wikigraph::WikiGraph;

pub struct PageViewEngine {
    // Map Date -> Vector of pageviews (Index is Dense Article ID)
    // We use Arc to make it cheap to clone/share across web threads
    daily_views: HashMap<NaiveDate, Vec<u32>>,
}

pub fn load_bin_file(path: &str, expected_size: usize) -> Result<Vec<u32>> {
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
    /// Calculate the total pageviews for a set of articles over time.
    /// This is the function your Web API calls.
    pub fn get_category_trend(
        &self,
        article_mask: &RoaringBitmap,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> Vec<(NaiveDate, u64)> {
        let mut results = Vec::new();
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

    pub fn load_history(
        graph: &WikiGraph,
        dates: Vec<NaiveDate>,
    ) -> Result<PageViewEngine, Box<dyn Error>> {
        let mut daily_views = HashMap::new();
        let num_articles = graph.art_dense_to_original.len();
        dotenv().ok();

        let data_dir = std::env::var("DATA_DIR").expect("DATA_DIR not set in .env");
        for date in dates {
            let filename = format!("{}/views_{}.parquet", data_dir, date);
            if !std::path::Path::new(&filename).exists() {
                eprintln!("Could not find page view data for {}", date);
                continue;
            }

            // 1. Initialize Zero Vector for this day
            let mut day_vec = vec![0u32; num_articles];

            // 2. Load Parquet: [page_id, views]
            let path: PlPath = PlPath::Local(Arc::from(Path::new(filename.as_str())));
            let df: DataFrame = LazyFrame::scan_parquet(path, Default::default())?.collect()?;

            let ids = df.column("page_id")?.u32()?;
            let views = df.column("views")?.u32()?;

            // 3. Map WikiID -> DenseID and fill Vector
            for (opt_id, opt_view) in ids.into_iter().zip(views.into_iter()) {
                if let (Some(wiki_id), Some(count)) = (opt_id, opt_view) {
                    // Crucial: Use the Graph to find where this article lives in the dense array
                    if let Some(&dense_id) = graph.art_original_to_dense.get(&wiki_id) {
                        day_vec[dense_id as usize] = count;
                    }
                }
            }

            daily_views.insert(date, day_vec);
            println!("Loaded views for {}", date);
        }

        Ok(PageViewEngine { daily_views })
    }
}
