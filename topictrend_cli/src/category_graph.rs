use parquet::file::writer::SerializedFileWriter;
use parquet::{file::properties::WriterProperties, record::RecordWriter as _};
use parquet_derive::ParquetRecordWriter;
use std::fs::File;
use std::io::{self, BufRead};
use std::sync::Arc;

#[derive(Debug, ParquetRecordWriter)]
struct GraphRelation {
    parent: u32,
    child: u32,
}
#[derive(Debug, ParquetRecordWriter)]
struct CategoryRelation {
    category: u32,
    parent_category: u32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <output_file>", args[0]);
        std::process::exit(1);
    }
    let output_file = &args[1];
    let stdin = io::stdin();
    let results: Vec<CategoryRelation> = stdin
        .lock()
        .lines()
        .filter_map(|line| {
            let line = line.ok()?;
            let mut parts = line.split('\t');
            let category = parts.next()?.parse::<u32>().ok()?;
            let parent_category = parts.next()?.parse::<u32>().ok()?;
            Some(CategoryRelation {
                category,
                parent_category,
            })
        })
        .collect();

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

    // Write forward_relations to a Parquet file
    let forward_file = File::create(output_file)?;
    let forward_props = Arc::new(
        WriterProperties::builder()
            .set_compression(parquet::basic::Compression::SNAPPY)
            .build(),
    );

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

    println!("Successfully wrote data to {}", output_file);
    Ok(())
}
