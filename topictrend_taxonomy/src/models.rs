use std::fmt;

use serde_derive::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    /// Cosine similarity in [0.0, 1.0]; 1.0 = identical, 0.0 = unrelated.
    pub score: f32,
    pub qid: u32,
    pub page_title: String,
}

impl fmt::Display for SearchResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Score: {:.4}", self.score)?;
        writeln!(f, "  QID: {}", self.qid)?;
        writeln!(f, "  Title: {}", self.page_title)
    }
}

impl SearchResult {
    pub fn new(score: f32, qid: u32, page_title: String) -> Self {
        Self {
            score,
            qid,
            page_title,
        }
    }
}
