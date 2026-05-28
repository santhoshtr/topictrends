use polars::prelude::*;
use std::fs::{self, File};
use std::sync::Mutex;
use topictrend::{pageview_bin, pageview_engine::PageViewEngine};

// Tests share process-wide state (DATA_DIR env var) and must run serially.
static TEST_LOCK: Mutex<()> = Mutex::new(());

fn set_data_dir(path: &str) {
    // SAFETY: we hold TEST_LOCK for the entire duration of any test that
    // mutates DATA_DIR; no other thread reads or writes the env during the
    // critical section.
    unsafe {
        std::env::set_var("DATA_DIR", path);
    }
}

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
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let base = "/tmp/topictrend_test_traversal";
    let _ = fs::remove_dir_all(base);
    set_data_dir(base);

    setup_test_data(base);

    let pairs = pageview_bin::get_daily_pageviews("testwiki", 2032, 10, 12);
    pageview_bin::write_pageview_parquet(
        &pairs,
        &format!("{}/testwiki/pageviews/2032/10/12.parquet", base),
    )
    .expect("write_pageview_parquet failed");

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

    // Top categories — direct article→category scatter, no graph propagation.
    // Per article_category.parquet: Q1+Q2 → Cat1; Q3 → Cat2; Q4 → Cat3.
    // Views: Q1=100, Q2=700, Q3=300, Q4=600.
    let top = engine
        .get_top_categories(
            "2032-10-12".parse().unwrap(),
            "2032-10-12".parse().unwrap(),
            10,
        )
        .unwrap();
    assert_eq!(top[0].category_qid, 1);
    assert_eq!(top[0].total_views, 800);
    assert_eq!(top[1].category_qid, 3);
    assert_eq!(top[1].total_views, 600);
    assert_eq!(top[2].category_qid, 2);
    assert_eq!(top[2].total_views, 300);
}

/// Round-trip test: pairs written to a Parquet must read back identically via
/// `PageViewEngine`, regardless of how the underlying dense IDs map.
#[test]
fn test_pageview_parquet_round_trip() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let base = "/tmp/topictrend_test_round_trip";
    let _ = fs::remove_dir_all(base);
    set_data_dir(base);

    fs::create_dir_all(format!("{}/testwiki/pageviews/2030/01", base)).unwrap();

    // Minimal graph: 3 articles (Q10, Q20, Q30), one category each.
    create_parquet(
        df![
            "page_id"    => &[1_u32, 2_u32, 3_u32],
            "qid"        => &[10_u32, 20_u32, 30_u32],
            "page_title" => &["A", "B", "C"]
        ]
        .unwrap(),
        &format!("{}/testwiki/articles.parquet", base),
    );
    create_parquet(
        df![
            "page_id"    => &[100_u32],
            "qid"        => &[100_u32],
            "page_title" => &["Cat"]
        ]
        .unwrap(),
        &format!("{}/testwiki/categories.parquet", base),
    );
    create_parquet(
        df![
            "article_qid"  => &[10_u32, 20_u32, 30_u32],
            "category_qid" => &[100_u32, 100_u32, 100_u32]
        ]
        .unwrap(),
        &format!("{}/testwiki/article_category.parquet", base),
    );
    create_parquet(
        df![
            "parent_qid" => Vec::<u32>::new(),
            "child_qid"  => Vec::<u32>::new()
        ]
        .unwrap(),
        &format!("{}/testwiki/category_graph.parquet", base),
    );

    // Write a pageview Parquet directly via the public writer.
    pageview_bin::write_pageview_parquet(
        &[(10_u32, 5_u32), (20_u32, 7_u32), (30_u32, 9_u32)],
        &format!("{}/testwiki/pageviews/2030/01/15.parquet", base),
    )
    .expect("write_pageview_parquet failed");

    let engine = PageViewEngine::new("testwiki");
    let date = "2030-01-15".parse().unwrap();

    assert_eq!(engine.get_article_trend(10, date, date)[0].1, 5);
    assert_eq!(engine.get_article_trend(20, date, date)[0].1, 7);
    assert_eq!(engine.get_article_trend(30, date, date)[0].1, 9);
}

/// Refresh-stability: a pageview Parquet written against one article set must
/// still produce correct values when read against a refreshed article set
/// (deletions, additions). This is the exact failure mode of the old
/// dense-indexed `.bin` format.
#[test]
fn test_pageview_parquet_survives_article_refresh() {
    let _guard = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let base = "/tmp/topictrend_test_refresh";
    let _ = fs::remove_dir_all(base);
    set_data_dir(base);

    fs::create_dir_all(format!("{}/testwiki/pageviews/2030/01", base)).unwrap();

    // First snapshot: articles Q10, Q20, Q30. Write a pageview file against it.
    create_parquet(
        df![
            "page_id"    => &[1_u32, 2_u32, 3_u32],
            "qid"        => &[10_u32, 20_u32, 30_u32],
            "page_title" => &["A", "B", "C"]
        ]
        .unwrap(),
        &format!("{}/testwiki/articles.parquet", base),
    );
    create_parquet(
        df![
            "page_id"    => &[100_u32],
            "qid"        => &[100_u32],
            "page_title" => &["Cat"]
        ]
        .unwrap(),
        &format!("{}/testwiki/categories.parquet", base),
    );
    create_parquet(
        df![
            "article_qid"  => &[10_u32, 20_u32, 30_u32],
            "category_qid" => &[100_u32, 100_u32, 100_u32]
        ]
        .unwrap(),
        &format!("{}/testwiki/article_category.parquet", base),
    );
    create_parquet(
        df![
            "parent_qid" => Vec::<u32>::new(),
            "child_qid"  => Vec::<u32>::new()
        ]
        .unwrap(),
        &format!("{}/testwiki/category_graph.parquet", base),
    );

    pageview_bin::write_pageview_parquet(
        &[(10_u32, 5_u32), (20_u32, 7_u32), (30_u32, 9_u32)],
        &format!("{}/testwiki/pageviews/2030/01/15.parquet", base),
    )
    .expect("write_pageview_parquet failed");

    // Second snapshot: Q20 deleted, Q40 added. Q10 stays at row 0 but Q30's
    // row position shifts up (was index 2, now index 1). Under the old dense
    // format this would silently misalign every entry; here we verify the
    // QID-keyed Parquet is correctly resolved against the new article set.
    create_parquet(
        df![
            "page_id"    => &[1_u32, 3_u32, 4_u32],
            "qid"        => &[10_u32, 30_u32, 40_u32],
            "page_title" => &["A", "C", "D"]
        ]
        .unwrap(),
        &format!("{}/testwiki/articles.parquet", base),
    );
    create_parquet(
        df![
            "article_qid"  => &[10_u32, 30_u32, 40_u32],
            "category_qid" => &[100_u32, 100_u32, 100_u32]
        ]
        .unwrap(),
        &format!("{}/testwiki/article_category.parquet", base),
    );

    let engine = PageViewEngine::new("testwiki");
    let date = "2030-01-15".parse().unwrap();

    // Q10 and Q30 are still in the active set — their counts must be intact.
    assert_eq!(engine.get_article_trend(10, date, date)[0].1, 5);
    assert_eq!(engine.get_article_trend(30, date, date)[0].1, 9);

    // Q40 is in the new set but had no entry in the bin written before its
    // existence — must read as zero, not borrow another article's value.
    assert_eq!(engine.get_article_trend(40, date, date)[0].1, 0);

    // Q20 is no longer in the active set; querying it returns empty (engine
    // can't translate the QID to a dense ID). Its historical value must not
    // bleed into another article — covered by the Q10/Q30 assertions above.
    assert!(engine.get_article_trend(20, date, date).is_empty());

    // Category aggregation must equal the sum of only currently-active
    // articles' views: 5 (Q10) + 9 (Q30) = 14. Q20's old count of 7 must
    // not contribute since Q20 has been removed.
    let trend = engine.get_category_trend(100, 0, date, date);
    assert_eq!(trend.len(), 1);
    assert_eq!(
        trend[0].1, 14,
        "Category total must exclude deleted-article views"
    );
}
