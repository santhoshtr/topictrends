use clap::{Arg, ArgMatches, Command};
use std::collections::HashMap;
use std::{error::Error, time::Instant};
use topictrend::{
    graphbuilder::GraphBuilder, pageedits_engine::PageEditsEngine, pageview_engine::PageViewEngine,
    wikigraph,
};

mod pageviews;

fn main() -> Result<(), Box<dyn Error>> {
    // Define the CLI structure
    let matches = Command::new("WikiGraph CLI")
        .about("Command-line interface for WikiGraph operations")
        .arg(
            Arg::new("wiki")
                .long("wiki")
                .short('w')
                .default_value("enwiki")
                .help("Wikipedia code. Example enwiki, eswiki, hiwiki etc"),
        )
        .subcommand(
            Command::new("list-articles")
                .about("Retrieve all articles in a category")
                .arg(
                    Arg::new("category")
                        .long("category")
                        .short('c')
                        .required(true)
                        .value_parser(clap::value_parser!(u32))
                        .help("The Wiki QID of the category"),
                )
                .arg(
                    Arg::new("depth")
                        .long("depth")
                        .short('d')
                        .default_value("1")
                        .value_parser(clap::value_parser!(u32))
                        .help("Depth for recursive queries"),
                ),
        )
        .subcommand(
            Command::new("list-child-categories")
                .about("Retrieve immediate subcategories of a category")
                .arg(
                    Arg::new("category")
                        .long("category")
                        .short('c')
                        .required(true)
                        .value_parser(clap::value_parser!(u32))
                        .help("The Wiki QID of the category"),
                ),
        )
        .subcommand(
            Command::new("list-descendant-categories")
                .about("Retrieve all subcategories up to a specific depth")
                .arg(
                    Arg::new("category")
                        .long("category")
                        .short('c')
                        .required(true)
                        .value_parser(clap::value_parser!(u32))
                        .help("The Wiki QID of the category"),
                )
                .arg(
                    Arg::new("depth")
                        .long("depth")
                        .short('d')
                        .default_value("1")
                        .help("Depth for recursive queries"),
                ),
        )
        .subcommand(
            Command::new("list-parent-categories")
                .about("Retrieve parent categories of a category")
                .arg(
                    Arg::new("category")
                        .long("category")
                        .short('c')
                        .required(true)
                        .value_parser(clap::value_parser!(u32))
                        .help("The Wiki QID of the category"),
                ),
        )
        .subcommand(
            Command::new("list-article-categories")
                .about("Retrieve all categories for a specific article")
                .arg(
                    Arg::new("article-qid")
                        .long("article-qid")
                        .short('a')
                        .required(true)
                        .value_parser(clap::value_parser!(u32))
                        .help("The Wiki QID of the article"),
                ),
        )
        .subcommand(
            Command::new("category-trend")
                .about("Retrieve category trends for a specific wiki and category")
                .arg(
                    Arg::new("category")
                        .long("category")
                        .short('c')
                        .required(true)
                        .value_parser(clap::value_parser!(u32))
                        .help("The QID of the category in the wiki"),
                )
                .arg(
                    Arg::new("depth")
                        .long("depth")
                        .short('d')
                        .default_value("0")
                        .value_parser(clap::value_parser!(u32))
                        .help("Depth for recursive queries"),
                )
                .arg(
                    Arg::new("start-date")
                        .long("start-date")
                        .short('s')
                        .required(false)
                        .help("Start date in YYYY-MM-DD format"),
                )
                .arg(
                    Arg::new("end-date")
                        .long("end-date")
                        .short('e')
                        .required(false)
                        .help("End date in YYYY-MM-DD format"),
                ),
        )
        .subcommand(
            Command::new("article-edits")
                .about("Retrieve edit trend for a specific article")
                .arg(
                    Arg::new("article")
                        .long("article")
                        .short('a')
                        .required(true)
                        .value_parser(clap::value_parser!(u32))
                        .help("The QID of the article in the wiki"),
                )
                .arg(
                    Arg::new("start-date")
                        .long("start-date")
                        .short('s')
                        .required(false)
                        .help("Start date in YYYY-MM-DD format"),
                )
                .arg(
                    Arg::new("end-date")
                        .long("end-date")
                        .short('e')
                        .required(false)
                        .help("End date in YYYY-MM-DD format"),
                ),
        )
        .subcommand(
            Command::new("category-edits")
                .about("Retrieve edit trend for a category (all articles in the category)")
                .arg(
                    Arg::new("category")
                        .long("category")
                        .short('c')
                        .required(true)
                        .value_parser(clap::value_parser!(u32))
                        .help("The QID of the category in the wiki"),
                )
                .arg(
                    Arg::new("depth")
                        .long("depth")
                        .short('d')
                        .default_value("0")
                        .value_parser(clap::value_parser!(u32))
                        .help("Depth for recursive queries"),
                )
                .arg(
                    Arg::new("start-date")
                        .long("start-date")
                        .short('s')
                        .required(false)
                        .help("Start date in YYYY-MM-DD format"),
                )
                .arg(
                    Arg::new("end-date")
                        .long("end-date")
                        .short('e')
                        .required(false)
                        .help("End date in YYYY-MM-DD format"),
                ),
        )
        .get_matches();

    let wiki_id: &str = matches.get_one::<String>("wiki").unwrap();
    // Dispatch subcommands
    match matches.subcommand() {
        Some(("list-articles", sub_m)) => {
            let graph_builder = GraphBuilder::new(wiki_id);
            let graph = graph_builder.build().expect("Error while building graph");

            handle_get_articles(&graph, sub_m)
        }
        Some(("list-child-categories", sub_m)) => {
            let graph_builder = GraphBuilder::new(wiki_id);
            let graph = graph_builder.build().expect("Error while building graph");
            handle_get_child_categories(&graph, sub_m)
        }
        Some(("list-descendant-categories", sub_m)) => {
            let graph_builder = GraphBuilder::new(wiki_id);
            let graph = graph_builder.build().expect("Error while building graph");

            handle_get_descendant_categories(&graph, sub_m)
        }
        Some(("list-parent-categories", sub_m)) => {
            let graph_builder = GraphBuilder::new(wiki_id);
            let graph = graph_builder.build().expect("Error while building graph");

            handle_get_parent_categories(&graph, sub_m)
        }
        Some(("list-article-categories", sub_m)) => {
            let graph_builder = GraphBuilder::new(wiki_id);
            let graph = graph_builder.build().expect("Error while building graph");

            handle_get_article_categories(&graph, sub_m, wiki_id)
        }
        Some(("category-trend", sub_m)) => handle_category_trend(wiki_id, sub_m),
        Some(("article-edits", sub_m)) => handle_article_edits(wiki_id, sub_m),
        Some(("category-edits", sub_m)) => handle_category_edits(wiki_id, sub_m),
        _ => println!("No valid subcommand provided. Use --help for usage."),
    }

    Ok(())
}

fn handle_get_articles(graph: &wikigraph::WikiGraph, matches: &ArgMatches) {
    let category_qid: &u32 = matches.get_one::<u32>("category").unwrap();
    let depth: &u32 = matches.get_one::<u32>("depth").unwrap();

    let articles = match graph.get_articles_in_category(*category_qid, *depth) {
        Ok(articles) => articles,
        Err(err) => {
            eprintln!("Error: {}", err);
            std::process::exit(1);
        }
    };
    println!(
        "Found {} articles in category {} (depth {}).",
        articles.len(),
        category_qid,
        depth
    );

    for article_qid in articles.iter().take(10) {
        println!(" - {}", article_qid);
    }
}

fn handle_get_child_categories(graph: &wikigraph::WikiGraph, matches: &ArgMatches) {
    let category_qid: &u32 = matches.get_one::<u32>("category").unwrap();

    let children = match graph.get_child_categories(*category_qid) {
        Ok(children) => children,
        Err(err) => {
            eprintln!("Error: {}", err);
            std::process::exit(1);
        }
    };
    println!(
        "Found {} child categories for category {}.",
        children.len(),
        category_qid
    );

    for qid in children {
        println!(" - {}", qid);
    }
}

fn handle_get_descendant_categories(graph: &wikigraph::WikiGraph, matches: &ArgMatches) {
    let category_qid: &u32 = matches.get_one::<u32>("category").unwrap();
    let depth: &u8 = matches.get_one::<u8>("depth").unwrap();

    let descendants = match graph.get_descendant_categories(*category_qid, *depth) {
        Ok(descendants) => descendants,
        Err(err) => {
            eprintln!("Error: {}", err);
            std::process::exit(1);
        }
    };
    println!(
        "Found {} descendant categories for category {} (depth {}).",
        descendants.len(),
        category_qid,
        depth
    );

    for (id, d) in descendants {
        println!(" - {}: (depth {})", id, d);
    }
}

fn handle_get_parent_categories(graph: &wikigraph::WikiGraph, matches: &ArgMatches) {
    let category_qid: &u32 = matches.get_one::<u32>("category").unwrap();

    let parents = match graph.get_parent_categories(*category_qid) {
        Ok(parents) => parents,
        Err(err) => {
            eprintln!("Error: {}", err);
            std::process::exit(1);
        }
    };
    println!(
        "Found {} parent categories for category {}.",
        parents.len(),
        category_qid
    );

    for id in parents {
        println!(" - {}", id);
    }
}

fn load_qid_title_map(path: &str) -> Result<HashMap<u32, String>, Box<dyn Error>> {
    use polars::prelude::{LazyFrame, PlRefPath};
    let ref_path = PlRefPath::try_from_path(std::path::Path::new(path))?;
    let df = LazyFrame::scan_parquet(ref_path, Default::default())?.collect()?;
    let qids = df.column("qid")?.u32()?;
    let titles = df.column("page_title")?.str()?;
    Ok(qids
        .into_iter()
        .zip(titles)
        .filter_map(|(q, t)| Some((q?, t?.to_string())))
        .collect())
}

fn handle_get_article_categories(
    graph: &wikigraph::WikiGraph,
    matches: &ArgMatches,
    wiki_id: &str,
) {
    let article_qid: &u32 = matches.get_one::<u32>("article-qid").unwrap();

    let ranked = graph.get_categories_for_article_ranked(*article_qid);
    let weighted = !graph.article_cat_weights.is_empty();

    let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string());
    let art_titles = load_qid_title_map(&format!("{}/{}/articles.parquet", data_dir, wiki_id))
        .unwrap_or_default();
    let mut cat_titles = load_qid_title_map(&format!("{}/{}/categories.parquet", data_dir, wiki_id))
        .unwrap_or_default();
    // The canonical relation surfaces categories with no local page; fall
    // back to English labels for those.
    if weighted && wiki_id != "enwiki" {
        for (qid, title) in
            load_qid_title_map(&format!("{}/enwiki/categories.parquet", data_dir)).unwrap_or_default()
        {
            cat_titles.entry(qid).or_insert(title);
        }
    }

    let article_title = art_titles
        .get(article_qid)
        .map(String::as_str)
        .unwrap_or("?");
    println!(
        "Found {} categories for article {} ({}).",
        ranked.len(),
        article_title,
        article_qid
    );

    for (id, weight) in ranked {
        let title = cat_titles.get(&id).map(String::as_str).unwrap_or("?");
        if weighted {
            println!(" - {} ({}) [{} wikis]", title, id, weight);
        } else {
            println!(" - {} ({})", title, id);
        }
    }
}

fn handle_category_trend(wiki_id: &str, matches: &ArgMatches) {
    let start = Instant::now();

    let category: &u32 = matches.get_one::<u32>("category").unwrap();
    let depth: &u32 = matches.get_one::<u32>("depth").unwrap();
    let start_date = matches
        .get_one::<String>("start-date")
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| chrono::Local::now().date_naive() - chrono::Duration::days(30));
    let end_date = matches
        .get_one::<String>("end-date")
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| chrono::Local::now().date_naive());

    let engine = PageViewEngine::new(wiki_id);
    let raw_data = engine.get_category_trend(*category, *depth, start_date, end_date);

    println!(
        "Category trend for category {} (depth {}, start: {}, end: {}):",
        category, depth, start_date, end_date
    );

    for trend in raw_data {
        println!(" - {}: {} views", trend.0, trend.1);
    }
    println!("Trend calculation completed in {:.2?}s", start.elapsed());
}

fn handle_article_edits(wiki_id: &str, matches: &ArgMatches) {
    let start = Instant::now();

    let article_qid: &u32 = matches.get_one::<u32>("article").unwrap();
    let start_date = matches
        .get_one::<String>("start-date")
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| chrono::Local::now().date_naive() - chrono::Duration::days(365));
    let end_date = matches
        .get_one::<String>("end-date")
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| chrono::Local::now().date_naive());

    let engine = PageEditsEngine::new(wiki_id);

    let raw_data = engine.get_article_trend(*article_qid, start_date, end_date);

    println!(
        "Edit trend for article {} (start: {}, end: {}):",
        article_qid, start_date, end_date
    );

    let total_edits: u64 = raw_data.iter().map(|(_, count)| count).sum();
    println!("Total edits: {}", total_edits);
    println!("Days with edits: {}", raw_data.len());

    if !raw_data.is_empty() {
        println!("\nSample (first 10 days):");
        for (date, count) in raw_data.iter().take(10) {
            println!(" - {}: {} edits", date, count);
        }
    }

    println!("\nCompleted in {:.2?}", start.elapsed());
}

fn handle_category_edits(wiki_id: &str, matches: &ArgMatches) {
    let start = Instant::now();

    let category_qid: &u32 = matches.get_one::<u32>("category").unwrap();
    let depth: &u32 = matches.get_one::<u32>("depth").unwrap();
    let start_date = matches
        .get_one::<String>("start-date")
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| chrono::Local::now().date_naive() - chrono::Duration::days(365));
    let end_date = matches
        .get_one::<String>("end-date")
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| chrono::Local::now().date_naive());

    let engine = PageEditsEngine::new(wiki_id);

    let raw_data = engine.get_category_trend(*category_qid, *depth, start_date, end_date);

    println!(
        "Edit trend for category {} (depth: {}, start: {}, end: {}):",
        category_qid, depth, start_date, end_date
    );

    let total_edits: u64 = raw_data.iter().map(|(_, count)| count).sum();
    println!("Total edits: {}", total_edits);
    println!("Days with edits: {}", raw_data.len());

    if !raw_data.is_empty() {
        println!("\nSample (first 10 days):");
        for (date, count) in raw_data.iter().take(10) {
            println!(" - {}: {} edits", date, count);
        }
    }

    println!("\nCompleted in {:.2?}", start.elapsed());
}
