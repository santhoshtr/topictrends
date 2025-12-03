use std::time::Instant;

use topictrend::pageview_engine::PageViewEngine;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = PageViewEngine::new("enwiki");

    let start = Instant::now();
    let top_n = 10;

    let top_cats = engine
        .get_top_categories(
            "2025-11-01".parse().unwrap(),
            "2025-12-01".parse().unwrap(),
            top_n,
        )
        .unwrap();
    for category_rank in top_cats {
        println!("{}", category_rank);
    }
    println!("Time Taken: {:.2?}", start.elapsed());
    Ok(())
}
