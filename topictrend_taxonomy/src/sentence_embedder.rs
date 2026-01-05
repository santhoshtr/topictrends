use tonic::Request;

// Include the generated protobuf code
pub mod embedding {
    tonic::include_proto!("embedding");
}

use embedding::embedding_service_client::EmbeddingServiceClient;
use embedding::{Embedding, EncodeRequest, HealthCheckRequest, SimilarityRequest};

pub struct SentenceEmbedder {
    client: EmbeddingServiceClient<tonic::transport::Channel>,
}

impl SentenceEmbedder {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let embedding_server = std::env::var("EMBEDDING_SERVER")
            .unwrap_or_else(|_| "http://localhost:50051".to_string());
        let client = EmbeddingServiceClient::connect(embedding_server).await?;

        Ok(Self { client })
    }

    pub async fn encode(&mut self, text: &str) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        let embeddings = self.encode_batch(&[text]).await?;
        Ok(embeddings.into_iter().next().unwrap())
    }

    pub async fn encode_batch(
        &mut self,
        texts: &[&str],
    ) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error>> {
        let texts_owned: Vec<String> = texts.iter().map(|s| s.to_string()).collect();

        let request = EncodeRequest {
            texts: texts_owned,
            prompt_name: None,
        };

        let response = self.client.encode(Request::new(request)).await?;
        let embeddings = response.into_inner().embeddings;

        Ok(embeddings.into_iter().map(|e| e.values).collect())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to the server
    let embedding_server =
        std::env::var("EMBEDDING_SERVER").unwrap_or_else(|_| "http://localhost:50051".to_string());
    let mut client = EmbeddingServiceClient::connect(embedding_server).await?;

    // Health check
    println!("=== Health Check ===");
    let health_response = client
        .health_check(Request::new(HealthCheckRequest {}))
        .await?
        .into_inner();
    println!(
        "Healthy: {}, Model: {}",
        health_response.healthy, health_response.model_name
    );

    // Encode queries
    println!("\n=== Encoding Queries ===");
    let queries = vec![
        "What is the capital of China?".to_string(),
        "Explain gravity".to_string(),
    ];

    let query_request = EncodeRequest {
        texts: queries.clone(),
        prompt_name: Some("query".to_string()),
    };

    let query_response = client.encode(Request::new(query_request)).await?;
    let query_embeddings = query_response.into_inner().embeddings;

    println!("Encoded {} queries", query_embeddings.len());
    println!(
        "Query embedding dimensions: {}",
        query_embeddings[0].values.len()
    );

    // Encode documents
    println!("\n=== Encoding Documents ===");
    let documents = vec![
        "The capital of China is Beijing.".to_string(),
        "Gravity is a force that attracts two bodies towards each other. It gives weight to physical objects and is responsible for the movement of planets around the sun.".to_string(),
    ];

    let doc_request = EncodeRequest {
        texts: documents.clone(),
        prompt_name: None,
    };

    let doc_response = client.encode(Request::new(doc_request)).await?;
    let document_embeddings = doc_response.into_inner().embeddings;

    println!("Encoded {} documents", document_embeddings.len());

    // Compute similarity
    println!("\n=== Computing Similarity ===");
    let similarity_request = SimilarityRequest {
        query_embeddings: query_embeddings.clone(),
        document_embeddings: document_embeddings.clone(),
    };

    let similarity_response = client
        .compute_similarity(Request::new(similarity_request))
        .await?
        .into_inner();

    // Reshape flat similarity scores into matrix
    let num_queries = similarity_response.num_queries as usize;
    let num_docs = similarity_response.num_documents as usize;

    println!("Similarity Matrix ({} x {}):", num_queries, num_docs);
    for i in 0..num_queries {
        print!("Query {}: ", i);
        for j in 0..num_docs {
            let idx = i * num_docs + j;
            print!("{:.4} ", similarity_response.similarities[idx]);
        }
        println!();
    }

    // Find best matches
    println!("\n=== Best Matches ===");
    for (i, query) in queries.iter().enumerate() {
        let mut scores: Vec<(usize, f32)> = (0..num_docs)
            .map(|j| {
                let idx = i * num_docs + j;
                (j, similarity_response.similarities[idx])
            })
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        println!("Query: \"{}\"", query);
        println!(
            "  Best match: \"{}\" (score: {:.4})",
            documents[scores[0].0], scores[0].1
        );
    }

    Ok(())
}
