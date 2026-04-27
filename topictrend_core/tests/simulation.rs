use polars::prelude::*;
use std::fs::{self, File};
use topictrend::{pageview_bin, pageview_engine::PageViewEngine};

fn create_parquet(mut df: DataFrame, path: &str) {
    let mut file = File::create(path).unwrap_or_else(|_| panic!("Failed to create {}", path));
    ParquetWriter::new(&mut file)
        .with_compression(ParquetCompression::Snappy)
        .finish(&mut df)
        .expect("Failed to write parquet");
}

fn setup_test_data(base: &str) {
    fs::create_dir_all(format!("{}/testwiki/pageviews/2032/10", base)).unwrap();
    fs::create_dir_all(format!("{}/pageviews/2032/10", base)).unwrap();

    create_parquet(
        df![
            "page_id" => &[1_u32, 2_u32, 3_u32, 4_u32],
            "qid"     => &[1_u32, 2_u32, 3_u32, 4_u32],
            "page_title" => &["Article 1", "Article 2", "Article 3", "Article 4"]
        ]
        .unwrap(),
        &format!("{}/testwiki/articles.parquet", base),
    );

    create_parquet(
        df![
            "page_id" => &[1_u32, 2_u32, 3_u32],
            "qid"     => &[1_u32, 2_u32, 3_u32],
            "page_title" => &["Category 1", "Category 2", "Category 3"]
        ]
        .unwrap(),
        &format!("{}/testwiki/categories.parquet", base),
    );

    create_parquet(
        df![
            "article_qid"  => &[1_u32, 2_u32, 3_u32, 4_u32],
            "category_qid" => &[1_u32, 1_u32, 2_u32, 3_u32]
        ]
        .unwrap(),
        &format!("{}/testwiki/article_category.parquet", base),
    );

    create_parquet(
        df![
            "parent_qid" => &[1_u32, 2_u32, 3_u32],
            "child_qid"  => &[2_u32, 3_u32, 1_u32]
        ]
        .unwrap(),
        &format!("{}/testwiki/category_graph.parquet", base),
    );

    create_parquet(
        df![
            "wiki"          => &["testwiki", "testwiki", "testwiki", "testwiki", "testwiki"],
            "page_id"       => &[1_u32, 2_u32, 3_u32, 2_u32, 4_u32],
            "access_method" => &["desktop", "desktop", "desktop", "mobile-web", "desktop"],
            "daily_views"   => &[100_u32, 200_u32, 300_u32, 500_u32, 600_u32]
        ]
        .unwrap(),
        &format!("{}/pageviews/2032/10/12.parquet", base),
    );
}

#[test]
fn test_graph_traversal_and_category_trends() {
    let base = "/tmp/topictrend_test";
    std::env::set_var("DATA_DIR", base);

    setup_test_data(base);

    let views = pageview_bin::get_daily_pageviews("testwiki", 2032, 10, 12);
    pageview_bin::generate_bin_dump(
        views,
        &format!("{}/testwiki/pageviews/2032/10/12.bin", base),
    )
    .expect("generate_bin_dump failed");

    let engine = PageViewEngine::new("testwiki");
    let graph = engine.get_wikigraph();

    // Depth 0: only direct articles of category 1 (articles 1 and 2)
    let articles = graph.get_articles_in_category(1, 0).unwrap();
    assert_eq!(articles.len(), 2);

    // Depth 1: adds category 2's articles (article 3)
    let articles = graph.get_articles_in_category(1, 1).unwrap();
    assert_eq!(articles.len(), 3);

    // Unbounded: all 4 articles reachable
    let articles = graph.get_articles_in_category(1, u32::MAX).unwrap();
    assert_eq!(articles.len(), 4);

    // Category trend for category 1, depth 0, single day
    let trend = engine.get_category_trend(
        1,
        0,
        "2032-10-12".parse().unwrap(),
        "2032-10-12".parse().unwrap(),
    );
    assert_eq!(trend.len(), 1);
    assert_eq!(
        trend[0].1, 800,
        "Expected 800 total views for category 1 (articles 1+2)"
    );

    // Top categories: category 2 leads (inherits from 3 and itself)
    let top = engine
        .get_top_categories(
            "2032-10-12".parse().unwrap(),
            "2032-10-12".parse().unwrap(),
            10,
        )
        .unwrap();
    assert_eq!(top[0].category_qid, 2);
    assert_eq!(top[0].total_views, 1700);
    assert_eq!(top[1].category_qid, 3);
    assert_eq!(top[1].total_views, 1400);
    assert_eq!(top[2].category_qid, 1);
    assert_eq!(top[2].total_views, 1100);
}
