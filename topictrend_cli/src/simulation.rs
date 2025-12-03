use polars::prelude::*;
use std::fs::{self, File};
use topictrend::pageview_engine::PageViewEngine;

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

    fs::create_dir_all("data/pageviews/2032/10").expect("Failed to create test data directory");

    // Create articles.parquet
    let articles = df![
        "page_id" => &[1_u32, 2_u32, 3_u32,4_u32],
        "qid" => &[1_u32, 2_u32, 3_u32, 4_u32],
        "page_title" => &["Article 1", "Article 2", "Article 3", "Article_4"]
    ]
    .unwrap();
    create_parquet_file(articles, "data/testwiki/articles.parquet");

    // Create categories.parquet
    let categories = df![
        "page_id" => &[1_u32, 2_u32, 3_u32],
        "qid" => &[1_u32, 2_u32, 3_u32],
        "page_title" => &["Category 1", "Category 2", "Category 3"]
    ]
    .unwrap();
    create_parquet_file(categories, "data/testwiki/categories.parquet");

    // Create article-category mapping
    let article_category = df![
        "article_qid" => &[1_u32, 2_u32, 3_u32, 4_u32],
        "category_qid" => &[1_u32, 1_u32, 2_u32, 3_u32]
    ]
    .unwrap();
    create_parquet_file(article_category, "data/testwiki/article_category.parquet");

    // Create category-graph
    let category_graph = df![
        "parent_qid" => &[1_u32, 2_u32, 3_u32],
        "child_qid" => &[2_u32, 3_u32, 1_u32]
    ]
    .unwrap();
    create_parquet_file(category_graph, "data/testwiki/category_graph.parquet");

    // Create pageviews.parquet
    let pageviews = df![
        "wiki" => &["testwiki", "testwiki", "testwiki", "testwiki", "testwiki"],
        "page_id" => &[1_u32, 2_u32, 3_u32, 2_u32, 4_u32],
        "access_method" => &["desktop", "desktop", "desktop", "mobile-web", "desktop"],
        "daily_views" => &[100_u32, 200_u32, 300_u32, 500_u32, 600_u32]
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

fn verify() {
    let mut engine = PageViewEngine::new("testwiki");
    let articles_in_cat = engine
        .get_wikigraph()
        .get_articles_in_category(1, 0)
        .unwrap();
    assert_eq!(articles_in_cat.len(), 2);

    let articles_in_cat = engine
        .get_wikigraph()
        .get_articles_in_category(1, 1)
        .unwrap();
    assert_eq!(articles_in_cat.len(), 3);

    let articles_in_cat = engine
        .get_wikigraph()
        .get_articles_in_category(1, u8::MAX)
        .unwrap();
    assert_eq!(articles_in_cat.len(), 4);

    let category_views = engine.get_category_trend(
        1,
        0,
        "2032-10-12".parse().unwrap(),
        "2032-10-12".parse().unwrap(),
    );

    assert_eq!(category_views.len(), 1);
    assert_eq!(category_views[0].1, 800); // Total views for Category 1 (Article 1 + Article 2)
    let top_categories = engine.get_top_categories(
        "2032-10-12".parse().unwrap(),
        "2032-10-12".parse().unwrap(),
        10,
    );
    assert_eq!(top_categories[0].category_id, 2);
    assert_eq!(top_categories[0].total_views, 1700);
    assert_eq!(top_categories[1].category_id, 3);
    assert_eq!(top_categories[1].total_views, 1400);
    assert_eq!(top_categories[2].category_id, 1);
    assert_eq!(top_categories[2].total_views, 1100);
}

fn main() {
    setup_test_data();
    generate_pageview_binary();
    verify();
    println!("All tests passed!");
}
