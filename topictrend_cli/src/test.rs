use polars::prelude::*;
use std::fs::{self, File};
use topictrend::{
    graphbuilder::GraphBuilder, pageview_engine::PageViewEngine, wikigraph::WikiGraph,
};

use crate::per_day_wiki_stats::{generate_bin_dump, get_daily_pageviews};
mod per_day_wiki_stats;

fn create_parquet_file(data: DataFrame, output_path: &str) {
    let mut file = File::create(output_path)
        .unwrap_or_else(|_| panic!("Failed to create Parquet file {}", output_path));
    ParquetWriter::new(&mut file)
        .with_compression(ParquetCompression::Snappy)
        .finish(&mut data.clone())
        .expect("Failed to write Parquet file");
}

fn setup_test_data() {
    fs::create_dir_all("data/testwiki/pageviews/2032/10")
        .expect("Failed to create test data directory");

    // Create articles.parquet
    let articles = df![
        "page_id" => &[1_u32, 2_u32, 3_u32],
        "page_title" => &["Article 1", "Article 2", "Article 3"]
    ]
    .unwrap();
    create_parquet_file(articles, "data/testwiki/articles.parquet");

    // Create categories.parquet
    let categories = df![
        "page_id" => &[1_u32, 2_u32],
        "page_title" => &["Category 1", "Category 2"]
    ]
    .unwrap();
    create_parquet_file(categories, "data/testwiki/categories.parquet");

    // Create article-category mapping
    let article_category = df![
        "article_qid" => &[1_u32, 2_u32, 3_u32],
        "category_qid" => &[1_u32, 1_u32, 2_u32]
    ]
    .unwrap();
    create_parquet_file(article_category, "data/testwiki/article_category.parquet");

    // Create category-graph
    let category_graph = df![
        "parent" => &[1_u32],
        "child" => &[2_u32]
    ]
    .unwrap();
    create_parquet_file(category_graph, "data/testwiki/category_graph.parquet");

    // Create pageviews.parquet
    let pageviews = df![
        "project" => &["testwiki", "testwiki", "testwiki", "testwiki"],
        "page_id" => &[1_u32, 2_u32, 3_u32, 2_u32],
        "access_method" => &["desktop", "desktop", "desktop", "mobile-web"],
        "daily_views" => &[100_u32, 200_u32, 300_u32, 500_u32]
    ]
    .unwrap();
    create_parquet_file(pageviews, "data/pageviews/2032/10/12.parquet");
}

fn generate_pageview_binary() {
    let wiki = "testwiki";
    let year = 2032;
    let month = 10;
    let day = 12;
    let page_views_dense_vector =
        get_daily_pageviews(wiki, &(year as i16), &(month as i8), &(day as i8));

    generate_bin_dump(
        page_views_dense_vector,
        &String::from("data/testwiki/pageviews/2032/10/12.bin"),
    )
    .expect("Fail")
}

fn verify_pageviews() {
    let graph_builder = GraphBuilder::new("testwiki");
    let graph: WikiGraph = graph_builder.build().expect("Failed to build graph");

    let mut engine = PageViewEngine::new("testwiki");
    let category_views = engine.get_category_trend(
        &("Category 1".to_string()),
        0,
        "2032-10-12".parse().unwrap(),
        "2032-10-12".parse().unwrap(),
    );

    assert_eq!(category_views.len(), 1);
    assert_eq!(category_views[0].1, 800); // Total views for Category 1 (Article 1 + Article 2)
}

fn main() {
    setup_test_data();
    generate_pageview_binary();
    verify_pageviews();
    println!("All tests passed!");
}
