use parquet::file::writer::SerializedFileWriter;
use parquet::{file::properties::WriterProperties, record::RecordWriter as _};
use parquet_derive::ParquetRecordWriter;
use polars::prelude::{LazyFrame, PlPath};
use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::sync::Arc;
use topictrend::direct_map::DirectMap;

#[derive(Debug, ParquetRecordWriter)]
struct ArticleCategory {
    article_qid: u32,
    category_qid: u32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <articles_parquet> <output_file>", args[0]);
        std::process::exit(1);
    }
    let articles_parquet = &args[1];
    let categories_parquet = &args[2];
    let output_file = &args[3];
    let stdin = io::stdin();

    // Before we write this to parquet file, we want to do a filtering
    // Check if article_qid is present in articles.parquet (column is  page_id)
    // This is because the article category mapping can contain articles in any namespace
    // but we are interested in 0 (main) namespace. Filtering out in sql query is very slow
    // for English wikipedia due to multiple joins.
    // Load articles.parquet to get valid article IDs
    let articles_parquet_path: PlPath = PlPath::Local(Arc::from(Path::new(&articles_parquet)));
    let articles_df =
        LazyFrame::scan_parquet(articles_parquet_path, Default::default())?.collect()?;
    let article_qids = articles_df.column("page_id")?.u32()?;
    let article_qids = articles_df.column("qid")?.u32()?;

    let article_qid_to_qid: DirectMap = article_qids
        .into_iter()
        .zip(article_qids.into_iter())
        .filter_map(|(id, qid)| Some((id?, qid?)))
        .collect();

    let valid_article_qids_set: HashSet<u32> = article_qid_to_qid.keys().into_iter().collect();

    let categories_parquet_path: PlPath = PlPath::Local(Arc::from(Path::new(&categories_parquet)));
    let categories_df =
        LazyFrame::scan_parquet(categories_parquet_path, Default::default())?.collect()?;

    let category_qids = categories_df.column("page_id")?.u32()?;
    let category_qids = categories_df.column("qid")?.u32()?;

    let category_qid_to_qid: DirectMap = category_qids
        .into_iter()
        .zip(category_qids.into_iter())
        .filter_map(|(id, qid)| Some((id?, qid?)))
        .collect();

    let valid_category_qids_set: std::collections::HashSet<u32> =
        category_qid_to_qid.keys().into_iter().collect();

    let mut record_count = 0;
    let mut lines_count = 0;
    let results: Vec<ArticleCategory> = stdin
        .lock()
        .lines()
        .filter_map(|line| {
            let line = line.ok()?;
            lines_count += 1;
            let mut parts = line.split('\t');
            let article_qid = parts.next()?.parse::<u32>().ok()?;
            let category_qid = parts.next()?.parse::<u32>().ok()?;

            let article_qid = article_qid_to_qid.get(article_qid)?.clone();
            let category_qid = category_qid_to_qid.get(category_qid)?.clone();
            if lines_count % 1000 == 0 {
                print!(
                    "\rRetrieved {} records from {} query results",
                    record_count, lines_count
                );
            }

            if valid_article_qids_set.contains(&article_qid)
                && valid_category_qids_set.contains(&category_qid)
            {
                record_count += 1;
                Some(ArticleCategory {
                    article_qid,
                    category_qid,
                })
            } else {
                None
            }
        })
        .collect();

    println!(
        "\nRetrieved {} records from {} query results",
        results.len(),
        lines_count
    );

    let schema = results.as_slice().schema().unwrap();
    let props = Arc::new(
        WriterProperties::builder()
            .set_compression(parquet::basic::Compression::SNAPPY)
            .build(),
    );

    let file = File::create(output_file).unwrap();
    let mut writer = SerializedFileWriter::new(file, schema, props).unwrap();
    let mut row_group = writer.next_row_group().unwrap();
    results
        .as_slice()
        .write_to_row_group(&mut row_group)
        .unwrap();
    row_group.close().unwrap();
    writer.close().unwrap();

    println!("Successfully wrote data to {}", args[1]);
    Ok(())
}
