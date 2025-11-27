use clap::{Arg, ArgMatches, Command};
use std::error::Error;
use topictrend::{graphbuilder::GraphBuilder, pageview_engine::PageViewEngine, wikigraph};

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
                        .value_parser(clap::value_parser!(String))
                        .help("The Wiki ID of the category"),
                )
                .arg(
                    Arg::new("depth")
                        .long("depth")
                        .short('d')
                        .default_value("1")
                        .value_parser(clap::value_parser!(u8))
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
                        .value_parser(clap::value_parser!(String))
                        .help("The Wiki ID of the category"),
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
                        .value_parser(clap::value_parser!(String))
                        .help("The Wiki ID of the category"),
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
                        .value_parser(clap::value_parser!(String))
                        .help("The Wiki ID of the category"),
                ),
        )
        .subcommand(
            Command::new("list-article-categories")
                .about("Retrieve all categories for a specific article")
                .arg(
                    Arg::new("article-id")
                        .long("article-id")
                        .short('a')
                        .required(true)
                        .value_parser(clap::value_parser!(u32))
                        .help("The Wiki ID of the article"),
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
                        .value_parser(clap::value_parser!(String))
                        .help("The ID of the category in the wiki"),
                )
                .arg(
                    Arg::new("depth")
                        .long("depth")
                        .short('d')
                        .default_value("0")
                        .value_parser(clap::value_parser!(u8))
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
    let graph_builder = GraphBuilder::new(wiki_id);
    let graph = graph_builder.build().expect("Error while building graph");

    // Dispatch subcommands
    match matches.subcommand() {
        Some(("list-articles", sub_m)) => handle_get_articles(&graph, sub_m),
        Some(("list-child-categories", sub_m)) => handle_get_child_categories(&graph, sub_m),
        Some(("list-descendant-categories", sub_m)) => {
            handle_get_descendant_categories(&graph, sub_m)
        }
        Some(("list-parent-categories", sub_m)) => handle_get_parent_categories(&graph, sub_m),
        Some(("list-article-categories", sub_m)) => handle_get_article_categories(&graph, sub_m),
        Some(("category-trend", sub_m)) => handle_category_trend(wiki_id, sub_m),
        _ => println!("No valid subcommand provided. Use --help for usage."),
    }

    Ok(())
}

fn handle_get_articles(graph: &wikigraph::WikiGraph, matches: &ArgMatches) {
    let category_title: &String = matches.get_one::<String>("category").unwrap();
    let depth: &u8 = matches.get_one::<u8>("depth").unwrap();

    let articles = match graph.get_articles_in_category(category_title, *depth) {
        Ok(articles) => articles,
        Err(err) => {
            eprintln!("Error: {}", err);
            std::process::exit(1);
        }
    };
    println!(
        "Found {} articles in category {} (depth {}).",
        articles.len(),
        category_title,
        depth
    );

    for article_id in articles.iter().take(10) {
        if let Some(name) = graph.get_article_name(article_id) {
            println!(" - {}", name);
        }
    }
}

fn handle_get_child_categories(graph: &wikigraph::WikiGraph, matches: &ArgMatches) {
    let category_title: &String = matches.get_one::<String>("category").unwrap();

    let children = match graph.get_child_categories(category_title) {
        Ok(children) => children,
        Err(err) => {
            eprintln!("Error: {}", err);
            std::process::exit(1);
        }
    };
    println!(
        "Found {} child categories for category {}.",
        children.len(),
        category_title
    );

    for (id, name) in children {
        println!(" - {}: {}", id, name);
    }
}

fn handle_get_descendant_categories(graph: &wikigraph::WikiGraph, matches: &ArgMatches) {
    let category_title: &String = matches.get_one::<String>("category").unwrap();
    let depth: &u8 = matches.get_one::<u8>("depth").unwrap();

    let descendants = match graph.get_descendant_categories(category_title, *depth) {
        Ok(descendants) => descendants,
        Err(err) => {
            eprintln!("Error: {}", err);
            std::process::exit(1);
        }
    };
    println!(
        "Found {} descendant categories for category {} (depth {}).",
        descendants.len(),
        category_title,
        depth
    );

    for (id, name, d) in descendants {
        println!(" - {}: {} (depth {})", id, name, d);
    }
}

fn handle_get_parent_categories(graph: &wikigraph::WikiGraph, matches: &ArgMatches) {
    let category_title: &String = matches.get_one::<String>("category").unwrap();

    let parents = match graph.get_parent_categories(category_title) {
        Ok(parents) => parents,
        Err(err) => {
            eprintln!("Error: {}", err);
            std::process::exit(1);
        }
    };
    println!(
        "Found {} parent categories for category {}.",
        parents.len(),
        category_title
    );

    for id in parents {
        println!(" - {}", id);
    }
}

fn handle_get_article_categories(graph: &wikigraph::WikiGraph, matches: &ArgMatches) {
    let article_id: &u32 = matches.get_one::<u32>("article-id").unwrap();

    let categories = match graph.get_categories_for_article(*article_id) {
        Ok(categories) => categories,
        Err(err) => {
            eprintln!("Error: {}", err);
            std::process::exit(1);
        }
    };
    println!(
        "Found {} categories for article {}.",
        categories.len(),
        article_id
    );

    for (id, name) in categories {
        println!(" - {}: {}", id, name);
    }
}

fn handle_category_trend(wiki_id: &str, matches: &ArgMatches) {
    let category: &String = matches.get_one::<String>("category").unwrap();
    let depth: &u8 = matches.get_one::<u8>("depth").unwrap();
    let start_date = matches
        .get_one::<String>("start-date")
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| chrono::Local::now().date_naive() - chrono::Duration::days(30));
    let end_date = matches
        .get_one::<String>("end-date")
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .unwrap_or_else(|| chrono::Local::now().date_naive());

    let mut engine = PageViewEngine::new(wiki_id);
    let raw_data = engine.get_category_trend(category, *depth, start_date, end_date);

    println!(
        "Category trend for category {} (depth {}, start: {}, end: {}):",
        category, depth, start_date, end_date
    );

    for trend in raw_data {
        println!(" - {}: {} views", trend.0, trend.1);
    }
}
