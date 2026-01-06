use ahash::HashMap;
use std::fmt;

use qdrant_client::qdrant::Value;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
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

// Internal helper to convert from Qdrant payload
impl SearchResult {
    pub(crate) fn from_qdrant_result(score: f32, payload: HashMap<String, Value>) -> Option<Self> {
        let qid = payload.get("qid").and_then(|v| match v {
            Value {
                kind: Some(qdrant_client::qdrant::value::Kind::IntegerValue(i)),
            } => Some(*i as u32),
            _ => None,
        })?;

        let page_title = payload.get("page_title").and_then(|v| match v {
            Value {
                kind: Some(qdrant_client::qdrant::value::Kind::StringValue(s)),
            } => Some(s.clone()),
            _ => None,
        })?;

        Some(SearchResult {
            score,
            qid,
            page_title,
        })
    }
}
