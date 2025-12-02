use parquet::file::writer::SerializedFileWriter;
use parquet::{file::properties::WriterProperties, record::RecordWriter as _};
use parquet_derive::ParquetRecordWriter;
use polars::prelude::{LazyFrame, PlPath};
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::sync::Arc;
use topictrend::direct_map::DirectMap;

#[derive(Debug, ParquetRecordWriter)]
struct GraphRelation {
    parent_qid: u32,
    child_qid: u32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <categories_parquet> <output_file>", args[0]);
        std::process::exit(1);
    }
    let categories_parquet = &args[1];
    let output_file = &args[2];

    // Load categories.parquet to get valid category IDs and their QIDs
    let categories_parquet_path: PlPath = PlPath::Local(Arc::from(Path::new(&categories_parquet)));
    let categories_df =
        LazyFrame::scan_parquet(categories_parquet_path, Default::default())?.collect()?;

    let category_ids = categories_df.column("page_id")?.u32()?;
    let category_qids = categories_df.column("qid")?.u32()?;

    // Build DirectMap for page_id -> qid conversion
    let category_id_to_qid: DirectMap = category_ids
        .into_iter()
        .zip(category_qids.into_iter())
        .filter_map(|(id, qid)| Some((id?, qid?)))
        .collect();

    let stdin = io::stdin();
    let mut lines_count = 0;
    let mut record_count = 0;

    let results: Vec<GraphRelation> = stdin
        .lock()
        .lines()
        .filter_map(|line| {
            let line = line.ok()?;
            lines_count += 1;
            let mut parts = line.split('\t');
            let category_qid = parts.next()?.parse::<u32>().ok()?;
            let parent_category_qid = parts.next()?.parse::<u32>().ok()?;

            // Convert page_ids to qids
            let category_qid = category_id_to_qid.get(category_qid)?;
            let parent_category_qid = category_id_to_qid.get(parent_category_qid)?;

            if lines_count % 1000 == 0 {
                print!(
                    "\rRetrieved {} records from {} query results",
                    record_count, lines_count
                );
            }

            record_count += 1;
            Some(GraphRelation {
                parent_qid: parent_category_qid,
                child_qid: category_qid,
            })
        })
        .collect();

    println!(
        "\nRetrieved {} records from {} query results",
        results.len(),
        lines_count
    );

    // Forward graph: Category QID -> List of Child Category QIDs
    let mut cat_children: Vec<Vec<u32>> = Vec::new();

    // Reverse graph: Category QID -> List of Parent Category QIDs
    let mut cat_parents: Vec<Vec<u32>> = Vec::new();

    for record in &results {
        let category_qid = record.child_qid as usize;
        let parent_category_qid = record.parent_qid as usize;

        // Ensure the vectors are large enough to hold the indices
        if category_qid >= cat_parents.len() {
            cat_parents.resize(category_qid + 1, Vec::new());
        }
        if parent_category_qid >= cat_children.len() {
            cat_children.resize(parent_category_qid + 1, Vec::new());
        }

        // Populate the forward graph
        cat_children[parent_category_qid].push(category_qid as u32);

        // Populate the reverse graph
        cat_parents[category_qid].push(parent_category_qid as u32);
    }

    // Flatten cat_children into a list of GraphRelation records
    let mut forward_relations = Vec::new();
    for (parent_qid, children) in cat_children.iter().enumerate() {
        for &child_qid in children {
            forward_relations.push(GraphRelation {
                parent_qid: parent_qid as u32,
                child_qid,
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
