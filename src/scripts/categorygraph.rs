use dotenv::dotenv;

use mysql::prelude::*;
use mysql::*;
use parquet::file::writer::SerializedFileWriter;
use parquet::{file::properties::WriterProperties, record::RecordWriter as _};
use parquet_derive::ParquetRecordWriter;
use std::fs::File;
use std::sync::Arc;

#[derive(Debug, ParquetRecordWriter)]
struct GraphRelation {
    parent: u32,
    child: u32,
}
#[derive(Debug, ParquetRecordWriter)]
struct CategoryRelation {
    category: i32,
    parent_category: i32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();

    let db_user = std::env::var("DB_USER").expect("DB_USER not set in .env");
    let db_password = std::env::var("DB_PASSWORD").expect("DB_PASSWORD not set in .env");
    let db_host = std::env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_string());
    let db_port = std::env::var("DB_PORT").unwrap_or_else(|_| "3306".to_string());
    let db_name = std::env::var("DB_NAME").unwrap_or_else(|_| "enwiki".to_string());
    let opts = OptsBuilder::new()
        .user(Some(db_user))
        .pass(Some(db_password))
        .ip_or_hostname(Some(db_host))
        .tcp_port(db_port.parse().expect("Failed to parse string to i32"))
        .db_name(Some(db_name));

    let pool = Pool::new(opts)?;
    let mut conn = pool.get_conn().expect("Connection failed");

    let query = "
        SELECT cl_from AS category, page_id AS parent_category
        FROM categorylinks
        JOIN page ON page_namespace = 14 AND page_title = cl_to
        WHERE cl_type = 'subcat'
     ";
    let results: Vec<CategoryRelation> =
        conn.query_map(query, |(category, parent_category)| CategoryRelation {
            category,
            parent_category,
        })?;

    println!("Retrieved {} records", results.len());

    // Forward graph: Category ID -> List of Child Category IDs
    let mut cat_children: Vec<Vec<u32>> = Vec::new();

    // Reverse graph: Category ID -> List of Parent Category IDs
    let mut cat_parents: Vec<Vec<u32>> = Vec::new();

    for record in &results {
        let category = record.category as usize;
        let parent_category = record.parent_category as usize;

        // Ensure the vectors are large enough to hold the indices
        if category >= cat_parents.len() {
            cat_parents.resize(category + 1, Vec::new());
        }
        if parent_category >= cat_children.len() {
            cat_children.resize(parent_category + 1, Vec::new());
        }

        // Populate the forward graph
        cat_children[parent_category].push(category as u32);

        // Populate the reverse graph
        cat_parents[category].push(parent_category as u32);
    }

    println!("Forward graph (cat_children) and reverse graph (cat_parents) prepared.");

    // Flatten cat_children into a list of GraphRelation records
    let mut forward_relations = Vec::new();
    for (parent, children) in cat_children.iter().enumerate() {
        for &child in children {
            forward_relations.push(GraphRelation {
                parent: parent as u32,
                child,
            });
        }
    }

    // Flatten cat_parents into a list of GraphRelation records
    let mut reverse_relations = Vec::new();
    for (child, parents) in cat_parents.iter().enumerate() {
        for &parent in parents {
            reverse_relations.push(GraphRelation {
                parent,
                child: child as u32,
            });
        }
    }

    // Write forward_relations to a Parquet file
    let forward_file = File::create("data/cat_children.parquet")?;
    let forward_props = Arc::new(WriterProperties::builder().build());
    let mut forward_writer = SerializedFileWriter::new(
        forward_file,
        forward_relations.as_slice().schema()?,
        forward_props,
    )?;
    let mut forward_row_group = forward_writer.next_row_group()?;
    forward_relations
        .as_slice()
        .write_to_row_group(&mut forward_row_group)?;
    forward_row_group.close()?;
    forward_writer.close()?;

    // Write reverse_relations to a Parquet file
    let reverse_file = File::create("data/cat_parents.parquet")?;
    let reverse_props = Arc::new(WriterProperties::builder().build());
    let mut reverse_writer = SerializedFileWriter::new(
        reverse_file,
        reverse_relations.as_slice().schema()?,
        reverse_props,
    )?;
    let mut reverse_row_group = reverse_writer.next_row_group()?;
    reverse_relations
        .as_slice()
        .write_to_row_group(&mut reverse_row_group)?;
    reverse_row_group.close()?;
    reverse_writer.close()?;

    println!("Successfully wrote cat_children and cat_parents to Parquet files.");

    Ok(())
}
