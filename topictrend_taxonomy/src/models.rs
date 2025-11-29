use ahash::HashMap;
use qdrant_client::qdrant::Value;
use serde::Deserialize;
use std::fmt;

#[derive(Deserialize)]
pub struct EmbeddingResponse {
    pub embeddings: Vec<Vec<f32>>,
}

pub struct SearchResult {
    pub score: f32,
    pub payload: HashMap<String, Value>,
}

impl fmt::Display for SearchResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Score: {:.4}", self.score)?;

        // Extract and display page_id
        if let Some(page_id) = self.payload.get("page_id") {
            if let Some(kind) = &page_id.kind {
                match kind {
                    qdrant_client::qdrant::value::Kind::IntegerValue(val) => {
                        writeln!(f, "  Page ID: {}", val)?;
                    }
                    _ => writeln!(f, "  Page ID: {:?}", page_id)?,
                }
            }
        }

        // Extract and display page_title
        if let Some(page_title) = self.payload.get("page_title") {
            if let Some(kind) = &page_title.kind {
                match kind {
                    qdrant_client::qdrant::value::Kind::StringValue(val) => {
                        writeln!(f, "  Title: {}", val)?;
                    }
                    _ => writeln!(f, "  Title: {:?}", page_title)?,
                }
            }
        }

        // Display any other fields
        for (key, value) in self.payload.iter() {
            if key != "page_id" && key != "page_title" {
                if let Some(kind) = &value.kind {
                    match kind {
                        qdrant_client::qdrant::value::Kind::StringValue(val) => {
                            writeln!(f, "  {}: {}", key, val)?;
                        }
                        qdrant_client::qdrant::value::Kind::IntegerValue(val) => {
                            writeln!(f, "  {}: {}", key, val)?;
                        }
                        qdrant_client::qdrant::value::Kind::DoubleValue(val) => {
                            writeln!(f, "  {}: {}", key, val)?;
                        }
                        qdrant_client::qdrant::value::Kind::BoolValue(val) => {
                            writeln!(f, "  {}: {}", key, val)?;
                        }
                        _ => writeln!(f, "  {}: {:?}", key, value)?,
                    }
                }
            }
        }

        Ok(())
    }
}
