use std::error::Error;

use tokio::sync::OnceCell;
use tonic::Request;
use tonic::transport::Channel;

pub use crate::models::SearchResult;
mod models;

pub mod embedding {
    tonic::include_proto!("embedding");
}

use embedding::embedding_service_client::EmbeddingServiceClient;
use embedding::{InjestRequest, SearchRequest};

/// Connected client, established once and cloned per call — tonic channels
/// are multiplexed and reconnect on their own. A failed connect leaves the
/// cell empty, so the next call retries instead of caching the failure.
static CLIENT: OnceCell<EmbeddingServiceClient<Channel>> = OnceCell::const_new();

async fn client() -> Result<EmbeddingServiceClient<Channel>, Box<dyn Error>> {
    let client = CLIENT
        .get_or_try_init(|| async {
            let embedding_server = std::env::var("EMBEDDING_SERVER")
                .unwrap_or_else(|_| "http://localhost:50051".to_string());
            EmbeddingServiceClient::connect(embedding_server).await
        })
        .await?;
    Ok(client.clone())
}

pub async fn injest(wiki: String) -> Result<(), Box<dyn Error>> {
    let mut client = client().await?;

    let request = InjestRequest { wiki };

    let response = client.injest(Request::new(request)).await?;

    println!(
        "Ingested {} records",
        response.into_inner().records_processed
    );

    Ok(())
}

pub async fn search(
    query: String,
    wiki: String,
    limit: u64,
    match_threshold: f32,
) -> Result<Vec<SearchResult>, Box<dyn Error>> {
    let mut client = client().await?;

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
        .filter_map(|r| {
            let similarity = 1.0 - r.score;
            (similarity >= match_threshold)
                .then(|| SearchResult::new(similarity, r.qid, r.page_title))
        })
        .collect();

    Ok(results)
}
