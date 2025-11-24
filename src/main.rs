use crate::wikigraph::GraphBuilder;
use clap::{Arg, ArgMatches, Command};
use std::error::Error;

mod graphbuilder;
mod pageviews;
mod wikigraph;

fn main() -> Result<(), Box<dyn Error>> {
    // Define the CLI structure
    let matches = Command::new("WikiGraph CLI")
        .version("0.1.0")
        .author("Santhosh Thottingal <santhosh.thottingal@gmail.com>")
        .about("Command-line interface for WikiGraph operations")
        .arg(
            Arg::new("data-dir")
                .long("data-dir")
                .short('d')
                .default_value("data")
                .help("Path to the directory containing Parquet files"),
        )
        .subcommand(
            Command::new("list-articles")
                .about("Retrieve all articles in a category")
                .arg(
                    Arg::new("category-id")
                        .long("category-id")
                        .short('c')
                        .required(true)
                        .value_parser(clap::value_parser!(u32))
                        .help("The Wiki ID of the category"),
                )
                .arg(
                    Arg::new("depth")
                        .long("depth")
                        .short('n')
                        .default_value("1")
                        .value_parser(clap::value_parser!(u8))
                        .help("Depth for recursive queries"),
                ),
        )
        .subcommand(
            Command::new("list-child-categories")
                .about("Retrieve immediate subcategories of a category")
                .arg(
                    Arg::new("category-id")
                        .long("category-id")
                        .short('c')
                        .required(true)
                        .value_parser(clap::value_parser!(u32))
                        .help("The Wiki ID of the category"),
                ),
        )
        .subcommand(
            Command::new("list-descendant-categories")
                .about("Retrieve all subcategories up to a specific depth")
                .arg(
                    Arg::new("category-id")
                        .long("category-id")
                        .short('c')
                        .required(true)
                        .value_parser(clap::value_parser!(u32))
                        .help("The Wiki ID of the category"),
                )
                .arg(
                    Arg::new("depth")
                        .long("depth")
                        .short('n')
                        .default_value("1")
                        .help("Depth for recursive queries"),
                ),
        )
        .subcommand(
            Command::new("list-parent-categories")
                .about("Retrieve parent categories of a category")
                .arg(
                    Arg::new("category-id")
                        .long("category-id")
                        .short('c')
                        .required(true)
                        .value_parser(clap::value_parser!(u32))
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
        .get_matches();

    // Load the graph
    let data_dir = matches.get_one::<String>("data-dir").unwrap();
    let graph = GraphBuilder::build(data_dir)?;

    // Dispatch subcommands
    match matches.subcommand() {
        Some(("list-articles", sub_m)) => handle_get_articles(&graph, sub_m),
        Some(("list-child-categories", sub_m)) => handle_get_child_categories(&graph, sub_m),
        Some(("list-descendant-categories", sub_m)) => {
            handle_get_descendant_categories(&graph, sub_m)
        }
        Some(("list-parent-categories", sub_m)) => handle_get_parent_categories(&graph, sub_m),
        Some(("list-article-categories", sub_m)) => handle_get_article_categories(&graph, sub_m),
        _ => println!("No valid subcommand provided. Use --help for usage."),
    }

    Ok(())
}

fn handle_get_articles(graph: &wikigraph::WikiGraph, matches: &ArgMatches) {
    let category_id: &u32 = matches.get_one::<u32>("category-id").unwrap();
    let depth: &u8 = matches.get_one::<u8>("depth").unwrap();

    let articles = graph.get_articles_in_category(*category_id, *depth);
    println!(
        "Found {} articles in category {} (depth {}).",
        articles.len(),
        category_id,
        depth
    );

    for article_id in articles.iter().take(10) {
        if let Some(name) = graph.get_article_name(article_id) {
            println!(" - {}", name);
        }
    }
}

fn handle_get_child_categories(graph: &wikigraph::WikiGraph, matches: &ArgMatches) {
    let category_id: &u32 = matches.get_one::<u32>("category-id").unwrap();

    let children = graph.get_child_categories(*category_id);
    println!(
        "Found {} child categories for category {}.",
        children.len(),
        category_id
    );

    for (id, name) in children {
        println!(" - {}: {}", id, name);
    }
}

fn handle_get_descendant_categories(graph: &wikigraph::WikiGraph, matches: &ArgMatches) {
    let category_id: &u32 = matches.get_one::<u32>("category-id").unwrap();
    let depth: &u8 = matches.get_one::<u8>("depth").unwrap();

    let descendants = graph.get_descendant_categories(*category_id, *depth);
    println!(
        "Found {} descendant categories for category {} (depth {}).",
        descendants.len(),
        category_id,
        depth
    );

    for (id, name, d) in descendants {
        println!(" - {}: {} (depth {})", id, name, d);
    }
}

fn handle_get_parent_categories(graph: &wikigraph::WikiGraph, matches: &ArgMatches) {
    let category_id: &u32 = matches.get_one::<u32>("category-id").unwrap();

    let parents = graph.get_parent_categories(*category_id);
    println!(
        "Found {} parent categories for category {}.",
        parents.len(),
        category_id
    );

    for id in parents {
        println!(" - {}", id);
    }
}

fn handle_get_article_categories(graph: &wikigraph::WikiGraph, matches: &ArgMatches) {
    let article_id: &u32 = matches.get_one::<u32>("article-id").unwrap();

    let categories = graph.get_categories_for_article(*article_id);
    println!(
        "Found {} categories for article {}.",
        categories.len(),
        article_id
    );

    for (id, name) in categories {
        println!(" - {}: {}", id, name);
    }
}
