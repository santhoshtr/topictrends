use std::{env, time::Instant};

use topictrend::pageview_engine::PageViewEngine;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let engine = PageViewEngine::new("enwiki");

    let graph = engine.get_wikigraph();

    // Get category ID from command-line argument
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <category_id>", args[0]);
        std::process::exit(1);
    }
    let category_qid: u32 = args[1]
        .parse()
        .expect("Please provide a valid u32 category QID");

    let start = Instant::now();
    let (max, avg, hist, unreachable) = graph.analyze_depth_from_root(category_qid);

    println!("--- Depth Analysis  ---");
    println!("Time Taken: {:.2?}", start.elapsed());
    println!("Max Depth: {}", max);
    println!("Avg Depth: {:.2}", avg);
    println!("Unreachable Categories: {}", unreachable);

    println!("\nDistribution:");
    // Print sorted by depth
    let mut keys: Vec<&u32> = hist.keys().collect();
    keys.sort();
    for key in keys {
        println!("Depth {:>2}: {:>7} categories", key, hist[key]);
    }

    Ok(())
}
