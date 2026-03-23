use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedDocument {
    pub original_path: String,
    pub relative_path: String,
    pub title: Option<String>,
    pub content: String,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRoot {
    pub id: String,
    pub original_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredDocument {
    pub doc_id: String,
    pub source_root_id: String,
    pub original_path: String,
    pub relative_path: String,
    pub title: Option<String>,
    pub content_hash: String,
    pub modified_at: Option<i64>,
    pub chunk_count: usize,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredChunk {
    pub chunk_id: i64,
    pub doc_id: String,
    pub ordinal: usize,
    pub heading_path: Vec<String>,
    pub char_start: usize,
    pub char_end: usize,
    pub token_count: usize,
    pub chunk_text: String,
    pub excerpt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BestMatch {
    pub chunk_id: i64,
    pub excerpt: String,
    pub heading_path: Vec<String>,
    pub char_start: usize,
    pub char_end: usize,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentHit {
    pub doc_id: String,
    pub original_path: String,
    pub relative_path: String,
    pub title: Option<String>,
    pub score: f32,
    pub best_match: BestMatch,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadedDocument {
    pub original_path: String,
    pub relative_path: String,
    pub content: String,
}
