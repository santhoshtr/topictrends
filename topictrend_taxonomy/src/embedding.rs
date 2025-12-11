use anyhow::Result;
use ndarray::{Axis, Ix2};
use ort::{
    execution_providers::OpenVINOExecutionProvider,
    session::{Session, builder::GraphOptimizationLevel},
    value::TensorRef,
};
use tokenizers::Tokenizer;
use std::time::Instant;

pub struct SentenceEmbedder {
    session: Session,
    tokenizer: Tokenizer,
}

impl SentenceEmbedder {
    /// Load the ONNX model and tokenizer
    pub fn new() -> Result<Self> {
        let model_path = "topictrend_taxonomy/models/all-MiniLM-L6-v2.onnx";
        let tokenizer_path = "topictrend_taxonomy/models/tokenizer.json";

        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_execution_providers([OpenVINOExecutionProvider::default().build()])?
            .with_intra_threads(4)?
            .with_inter_threads(8)?
            .commit_from_file(model_path)?;

        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e))?;

        Ok(Self { session, tokenizer })
    }

    /// Encode multiple sentences to embeddings
    pub fn encode_batch(&mut self, sentences: &[&str]) -> Result<Vec<Vec<f32>>> {
        // Encode all sentences with padding
        let encodings = self
            .tokenizer
            .encode_batch(sentences.to_vec(), true)
            .map_err(|e| anyhow::anyhow!("Tokenization failed: {}", e))?;

        let batch_size = sentences.len();
        let seq_len = encodings[0].len();

        // Pre-allocate with exact capacity
        let mut ids = Vec::with_capacity(batch_size * seq_len);
        let mut mask = Vec::with_capacity(batch_size * seq_len);

        // Get token IDs and attention mask as flattened arrays
        for encoding in &encodings {
            ids.extend(encoding.get_ids().iter().map(|&i| i as i64));
            mask.extend(encoding.get_attention_mask().iter().map(|&i| i as i64));
        }
        let padded_token_length = encodings[0].len();

        // Create 2D tensor views [batch_size, sequence_length]
        let a_ids = TensorRef::from_array_view(([sentences.len(), padded_token_length], &*ids))?;
        let a_mask = TensorRef::from_array_view(([sentences.len(), padded_token_length], &*mask))?;

        // Run inference
        let outputs = self.session.run(ort::inputs![a_ids, a_mask])?;

        // Extract embeddings (output at index 1 is the sentence embeddings)
        let embeddings = outputs[1]
            .try_extract_array::<f32>()?
            .into_dimensionality::<Ix2>()?;

        // Convert to Vec of Vec
        let result: Vec<Vec<f32>> = embeddings
            .axis_iter(Axis(0))
            .map(|row| Self::normalize(&row.to_vec()))
            .collect();

        Ok(result)
    }

    /// Encode a single sentence
    pub fn encode(&mut self, sentence: &str) -> Result<Vec<f32>> {
        let embeddings = self.encode_batch(&[sentence])?;
        Ok(embeddings.into_iter().next().unwrap())
    }

    /// L2 normalize a vector to unit length
    fn normalize(vec: &[f32]) -> Vec<f32> {
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 1e-12 {
            vec.iter().map(|x| x / norm).collect()
        } else {
            vec.to_vec()
        }
    }
}

/// Calculate cosine similarity between two embedding vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    dot_product
}

fn main() -> Result<()> {
    // Load the model - you can use from_url for testing or new() for local files
    let mut embedder = SentenceEmbedder::new()?;

    // Example phrases (similar to Wikipedia category names)
    let phrases = vec![
        "Machine Learning",
        "Artificial Intelligence",
        "Medieval History",
        "Deep Learning",
        "Computer Science",
    ];

    println!("Generating embeddings for {} phrases...\n", phrases.len());

    // Generate embeddings
    let start_time = Instant::now();
    let embeddings = embedder.encode_batch(&phrases)?;
    let elapsed = start_time.elapsed();
    println!("Embedding generation took: {:.2?}\n", elapsed);

    // Print embedding dimensions
    println!("Embedding dimension: {}\n", embeddings[0].len());

    // Calculate and display similarity matrix
    println!("Similarity Matrix:");
    println!("{:<30} | {}", "Phrase Pair", "Similarity");
    println!("{:-<30}-+-{:-<10}", "", "");

    for (i, phrase1) in phrases.iter().enumerate() {
        for (j, phrase2) in phrases.iter().enumerate() {
            if j > i {
                let similarity = cosine_similarity(&embeddings[i], &embeddings[j]);
                println!(
                    "{:<30} | {:.4} ({:.1}%)",
                    format!("{} <-> {}", phrase1, phrase2),
                    similarity,
                    similarity * 100.0
                );
            }
        }
    }

    println!("\n--- Semantic Search Example ---");
    let query = "Deep Neural Networks";
    println!("Query: '{}'\n", query);

    let query_embedding = embedder.encode(query)?;

    let mut similarities: Vec<(usize, f32)> = embeddings
        .iter()
        .enumerate()
        .map(|(i, emb)| (i, cosine_similarity(&query_embedding, emb)))
        .collect();

    // Sort by similarity (descending)
    similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    println!("Top matches:");
    for (i, sim) in similarities.iter().take(3) {
        println!("  {}: {:.1}% - '{}'", i + 1, sim * 100.0, phrases[*i]);
    }

    Ok(())
}
