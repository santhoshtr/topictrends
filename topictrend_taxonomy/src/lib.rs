use std::error::Error;

use tonic::Request;

pub use crate::models::SearchResult;
mod models;

pub mod embedding {
    tonic::include_proto!("embedding");
}

use embedding::embedding_service_client::EmbeddingServiceClient;
use embedding::{InjestRequest, SearchRequest};

pub async fn injest(wiki: String) -> Result<(), Box<dyn Error>> {
    let embedding_server = std::env::var("EMBEDDING_SERVER")
        .unwrap_or_else(|_| "http://localhost:50051".to_string());

    let mut client = EmbeddingServiceClient::connect(embedding_server).await?;

    let request = InjestRequest { wiki };

    let response = client.injest(Request::new(request)).await?;

    println!("Ingested {} records", response.into_inner().records_processed);

    Ok(())
}

pub async fn search(
    query: String,
    wiki: String,
    limit: u64,
) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    let embedding_server = std::env::var("EMBEDDING_SERVER")
        .unwrap_or_else(|_| "http://localhost:50051".to_string());

    let mut client = EmbeddingServiceClient::connect(embedding_server).await?;

    let request = SearchRequest {
        query,
        wiki,
        limit: limit as i64,
    };

    let response = client.search(Request::new(request)).await?;

    let results: Vec<SearchResult> = response
        .into_inner()
        .results
        .into_iter()
        .map(|r| SearchResult::new(r.score, r.qid, r.page_title))
        .collect();

    Ok(results)
}
