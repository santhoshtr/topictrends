use crate::wikigraph::GraphBuilder;

mod wikigraph;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Assuming parquet files are in "data/" folder
    let graph = GraphBuilder::build("data")?;

    // Example Query 1: Get all articles in a specific category (by Wiki ID)
    // Replace 12345 with a real Category Page ID from your data
    let query_cat_id = 2000;
    let depth = 2;

    println!(
        "Querying Category ID: {} with depth {}",
        query_cat_id, depth
    );

    let article_bitmap = graph.get_articles_in_category(query_cat_id, depth);
    println!("Found {} articles.", article_bitmap.len());

    // Print first 5 articles found
    for dense_id in article_bitmap.iter().take(5) {
        if let Some(name) = graph.get_article_name(dense_id) {
            println!(" - {}", name);
        }
    }

    // Example Query 2: Navigate Up
    let parents = graph.get_parent_categories(query_cat_id);
    println!("Parent Categories: {:?}", parents);

    Ok(())
}
