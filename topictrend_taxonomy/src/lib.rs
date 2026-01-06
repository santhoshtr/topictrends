use std::error::Error;
use std::path::Path;

use ahash::HashMap;
use anyhow::Result;
use polars::prelude::PlPath;
use qdrant_client::qdrant::HnswConfigDiffBuilder;
use qdrant_client::qdrant::PointsOperationResponse;
use qdrant_client::qdrant::SearchParamsBuilder;
use qdrant_client::qdrant::SearchPointsBuilder;
use qdrant_client::qdrant::Value;
use qdrant_client::{
    Payload, Qdrant,
    qdrant::{
        CreateCollectionBuilder, Distance, PointStruct, ScalarQuantizationBuilder,
        UpsertPointsBuilder, VectorParamsBuilder,
    },
};

use std::sync::Arc;

pub use crate::models::SearchResult;
use crate::sentence_embedder::SentenceEmbedder;
mod models;
mod sentence_embedder;

pub async fn get_connection() -> Result<Qdrant, Box<dyn Error>> {
    let quadrant_server =
        std::env::var("QUADRANT_SERVER").unwrap_or_else(|_| "http://localhost:6334".to_string());
    let client = Qdrant::from_url(&quadrant_server)
        .skip_compatibility_check()
        .build()?;
    Ok(client)
}

pub async fn injest(db: &Qdrant, wiki: String) -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = std::env::var("DATA_DIR").unwrap_or_else(|_| "data".to_string());
    let parquet_path = format!("{}/{}/categories.parquet", data_dir, wiki);

    // Read parquet file in a blocking context to avoid runtime conflicts
    let (page_ids_vec, page_titles_vec) = tokio::task::spawn_blocking(move || {
        let parquet_path: PlPath = PlPath::Local(Arc::from(Path::new(&parquet_path)));
        let df = polars::prelude::LazyFrame::scan_parquet(parquet_path, Default::default())?
            .select([
                polars::prelude::col("qid"),
                polars::prelude::col("page_title"),
            ])
            .collect()?;

        let page_qids = df.column("qid")?.u32()?;
        let page_titles = df.column("page_title")?.str()?;

        // Convert to Vec to move out of the blocking task
        let qids: Vec<Option<u32>> = page_qids.into_iter().collect();
        let titles: Vec<Option<&str>> = page_titles.into_iter().collect();
        let titles_owned: Vec<Option<String>> = titles
            .into_iter()
            .map(|opt| opt.map(|s| s.to_string()))
            .collect();
        Ok::<_, polars::error::PolarsError>((qids, titles_owned))
    })
    .await
    .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?
    .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    let total_records = page_ids_vec.len();
    println!("Found {} records to process", total_records);
    let mut processed = 0;
    let mut batch = Vec::new();
    let mut encoder = SentenceEmbedder::new().await?;
    for (page_qid, page_title) in page_ids_vec.into_iter().zip(page_titles_vec.into_iter()) {
        if let (Some(qid), Some(title)) = (page_qid, page_title) {
            batch.push((qid, title));

            if batch.len() == 100 {
                let embeddings = fetch_embeddings(&mut encoder, &batch).await?;
                insert_to_qdrant(db, &wiki, &batch, &embeddings).await?;
                batch.clear();
            }
            processed += 1;

            print!(
                "\rProgress: {}/{} records processed ({:.1}%)",
                processed,
                total_records,
                (processed as f64 / total_records as f64) * 100.0
            );
        }
    }

    if !batch.is_empty() {
        let embeddings = fetch_embeddings(&mut encoder, &batch).await?;
        insert_to_qdrant(db, &wiki, &batch, &embeddings).await?;
        processed += batch.len();
        print!(
            "\rProgress: {}/{} records processed ({:.1}%)",
            processed,
            total_records,
            (processed as f64 / total_records as f64) * 100.0
        );
    }
    println!();
    println!("✓ Completed! Total records processed: {}", processed);
    Ok(())
}

async fn insert_to_qdrant(
    client: &Qdrant,
    wiki: &str,
    batch: &[(u32, String)],
    embeddings: &Vec<Vec<f32>>,
) -> Result<PointsOperationResponse> {
    let collection_name = format!("{}-categories", wiki);

    // Try to create collection, ignore error if it already exists
    let _ = client
        .create_collection(
            CreateCollectionBuilder::new(&collection_name)
                .vectors_config(VectorParamsBuilder::new(384, Distance::Cosine).on_disk(true))
                .quantization_config(ScalarQuantizationBuilder::default().always_ram(true))
                .hnsw_config(
                    HnswConfigDiffBuilder::default()
                        .on_disk(true)
                        .inline_storage(true),
                ),
        )
        .await;

    // Use page_qid as the point ID to avoid duplicates
    // Qdrant will automatically overwrite points with the same ID
    let points: Vec<PointStruct> = batch
        .iter()
        .zip(embeddings.iter())
        .map(|((page_qid, title), embedding)| {
            let mut payload = Payload::new();
            payload.insert("qid", Value::from(*page_qid as i64));
            payload.insert("page_title", Value::from(title.clone()));

            // Use page_id as the point QID instead of sequential index
            PointStruct::new(*page_qid as u64, embedding.clone(), payload)
        })
        .collect();
    Ok(client
        .upsert_points(UpsertPointsBuilder::new(&collection_name, points))
        .await?)
}

async fn fetch_embeddings(
    encoder: &mut SentenceEmbedder,
    batch: &[(u32, String)],
) -> Result<Vec<Vec<f32>>, Box<dyn Error>> {
    let titles: Vec<&str> = batch.iter().map(|(_, title)| title.as_str()).collect();
    encoder.encode_batch(&titles).await
}

pub async fn search(
    query: String,
    wiki: String,
    limit: u64,
) -> Result<Vec<SearchResult>, Box<dyn std::error::Error>> {
    let client = get_connection().await?;
    let collection_name = format!("{}-categories", wiki);

    let mut encoder = SentenceEmbedder::new().await?;
    let query_embedding = encoder.encode(&query).await?;

    let search_result = client
        .search_points(
            SearchPointsBuilder::new(collection_name, query_embedding, limit)
                .with_payload(true)
                .params(SearchParamsBuilder::default().exact(true)),
        )
        .await?;

    let results: Vec<SearchResult> = search_result
        .result
        .into_iter()
        .filter_map(|point| {
            let payload = point
                .payload
                .into_iter()
                .collect::<HashMap<String, Value>>();
            SearchResult::from_qdrant_result(point.score, payload)
        })
        .collect();

    Ok(results)
}

// Integration test helper - only runs when Qdrant is available
#[cfg(test)]
mod integration_tests {
    use super::*;

    async fn is_qdrant_available() -> bool {
        match get_connection().await {
            Ok(client) => client.health_check().await.is_ok(),
            Err(_) => false,
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_full_insert_flow() {
        if !is_qdrant_available().await {
            println!("Qdrant not available, skipping integration test");
            return;
        }

        let qdrant_client = get_connection().await.expect("Failed to connect to Qdrant");
        let wiki = "enwiki";
        let batch = vec![
            (1u32, "Machine Learning".to_string()),
            (2u32, "Artificial Intelligence".to_string()),
            (3u32, "Deep Learning".to_string()),
            (4u32, "Deep blue sea".to_string()),
            (5u32, "ഡീപ് ലേങിങ്ങ്".to_string()),
        ];

        let mut encoder = SentenceEmbedder::new()
            .await
            .expect("Failed to create encoder");
        let embeddings_result = fetch_embeddings(&mut encoder, &batch).await;

        match embeddings_result {
            Ok(embeddings) => {
                println!("Successfully fetched {} embeddings", embeddings.len());
                println!("Embedding dimension: {}", embeddings[0].len());

                let result = insert_to_qdrant(&qdrant_client, wiki, &batch, &embeddings).await;
                assert!(
                    result.is_ok(),
                    "Failed to insert to Qdrant: {:?}",
                    result.err()
                );

                println!("Successfully inserted embeddings to Qdrant");

                // Now test search functionality
                println!("\nTesting search functionality...");

                // Search for a query similar to our inserted data
                let search_query = "Neural Networks".to_string();
                println!("Searching for: {}", search_query);

                // First get embedding for the search query
                let query_batch = vec![(0u32, search_query.clone())];
                let query_embedding_result = fetch_embeddings(&mut encoder, &query_batch).await;

                match query_embedding_result {
                    Ok(query_embeddings) => {
                        println!("Got search query embedding");

                        // Perform the search
                        let collection_name = format!("{}-categories", wiki);
                        let search_result = qdrant_client
                            .search_points(
                                SearchPointsBuilder::new(
                                    collection_name.clone(),
                                    query_embeddings[0].clone(),
                                    3,
                                )
                                .with_payload(true)
                                .params(SearchParamsBuilder::default().exact(false)),
                            )
                            .await;

                        match search_result {
                            Ok(results) => {
                                println!("Search returned {} results", results.result.len());

                                for (idx, point) in results.result.iter().enumerate() {
                                    println!("\nResult {}:", idx + 1);
                                    println!("  Score: {}", point.score);
                                    println!("  ID: {:?}", point.id);

                                    if let Some(title) = point.payload.get("page_title") {
                                        println!("  Title: {:?}", title);
                                    }
                                    if let Some(page_id) = point.payload.get("page_id") {
                                        println!("  Page ID: {:?}", page_id);
                                    }
                                }

                                assert!(!results.result.is_empty(), "Search should return results");
                                assert!(
                                    results.result.len() <= 3,
                                    "Should return at most 3 results"
                                );

                                // Verify that results have the expected payload fields
                                for point in &results.result {
                                    assert!(
                                        point.payload.contains_key("page_title"),
                                        "Result should have page_title"
                                    );
                                    assert!(
                                        point.payload.contains_key("page_id"),
                                        "Result should have page_id"
                                    );
                                }

                                println!("\nSearch test passed!");
                            }
                            Err(e) => {
                                panic!("Search failed: {:?}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("Failed to fetch query embedding: {:?}", e);
                        println!("Skipping search test");
                    }
                }
            }
            Err(e) => {
                println!("Failed to fetch embeddings from API: {:?}", e);
                println!("Skipping Qdrant insertion test");
                return;
            }
        }

        // Cleanup
        let collection_name = format!("{}-categories", wiki);
        let delete_result = qdrant_client.delete_collection(&collection_name).await;
        println!("\nCleanup result: {:?}", delete_result);
    }
}
