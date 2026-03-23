use crate::{error::Result, InkdexError};
use anyhow::anyhow;
use model2vec_rs::model::StaticModel;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmbeddingBackend {
    Model2Vec { model: String, batch_size: usize },
    Hashing { dimensions: usize },
}

impl Default for EmbeddingBackend {
    fn default() -> Self {
        Self::Model2Vec {
            model: "minishlab/potion-base-2M".to_string(),
            batch_size: 256,
        }
    }
}

pub struct Embedder {
    backend: EmbeddingBackend,
    model2vec: Option<StaticModel>,
}

impl Embedder {
    pub fn new(backend: EmbeddingBackend) -> Result<Self> {
        let model2vec = match &backend {
            EmbeddingBackend::Model2Vec { model, .. } => Some(
                StaticModel::from_pretrained(model, None, None, None)
                    .map_err(|error| InkdexError::Embedding(anyhow!(error.to_string())))?,
            ),
            EmbeddingBackend::Hashing { .. } => None,
        };
        Ok(Self { backend, model2vec })
    }

    pub fn backend(&self) -> &EmbeddingBackend {
        &self.backend
    }

    pub fn embed_texts(&mut self, inputs: &[String]) -> Result<Vec<Vec<f32>>> {
        self.embed(inputs)
    }

    fn embed(&mut self, inputs: &[String]) -> Result<Vec<Vec<f32>>> {
        match (&self.backend, self.model2vec.as_ref()) {
            (EmbeddingBackend::Model2Vec { batch_size, .. }, Some(model)) => {
                Ok(model.encode_with_args(inputs, Some(512), *batch_size))
            }
            (EmbeddingBackend::Hashing { dimensions }, _) => Ok(inputs
                .iter()
                .map(|value| hashing_embedding(value, *dimensions))
                .collect()),
            _ => Err(InkdexError::Embedding(anyhow!(
                "embedding backend was not initialized"
            ))),
        }
    }
}

pub fn format_query_for_embedding(query: &str) -> String {
    format!("query: {}", normalize_text(query))
}

pub fn format_chunk_for_embedding(
    relative_path: &str,
    title: Option<&str>,
    heading_path: &[String],
    chunk_text: &str,
) -> String {
    let mut lines = Vec::new();
    lines.push(format!("path: {}", normalize_text(relative_path)));
    if let Some(title) = title.filter(|value| !value.trim().is_empty()) {
        lines.push(format!("title: {}", normalize_text(title)));
    }
    if !heading_path.is_empty() {
        lines.push(format!(
            "headings: {}",
            normalize_text(&heading_path.join(" > "))
        ));
    }
    lines.push(format!("text: {}", normalize_text(chunk_text)));
    lines.join("\n")
}

pub fn format_document_for_reranking(
    relative_path: &str,
    title: Option<&str>,
    heading_path: &[String],
    excerpt: &str,
    metadata: &BTreeMap<String, String>,
) -> String {
    let mut lines = Vec::new();
    lines.push(format!("path: {}", normalize_text(relative_path)));
    if let Some(title) = title.filter(|value| !value.trim().is_empty()) {
        lines.push(format!("title: {}", normalize_text(title)));
    }
    if !heading_path.is_empty() {
        lines.push(format!(
            "headings: {}",
            normalize_text(&heading_path.join(" > "))
        ));
    }
    if !metadata.is_empty() {
        let metadata_line = metadata
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!("metadata: {}", normalize_text(&metadata_line)));
    }
    lines.push(format!("excerpt: {}", normalize_text(excerpt)));
    lines.join("\n")
}

pub fn vector_to_bytes(vector: &[f32]) -> Vec<u8> {
    vector
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect::<Vec<_>>()
}

pub fn bytes_to_vector(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

pub fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    if left.is_empty() || right.is_empty() || left.len() != right.len() {
        return 0.0;
    }
    let (mut dot, mut left_norm, mut right_norm) = (0.0f32, 0.0f32, 0.0f32);
    for (l, r) in left.iter().zip(right.iter()) {
        dot += l * r;
        left_norm += l * l;
        right_norm += r * r;
    }
    if left_norm == 0.0 || right_norm == 0.0 {
        return 0.0;
    }
    dot / (left_norm.sqrt() * right_norm.sqrt())
}

fn hashing_embedding(input: &str, dimensions: usize) -> Vec<f32> {
    let mut vector = vec![0.0f32; dimensions];
    for token in input.split_whitespace() {
        let hash = blake3::hash(token.as_bytes());
        let bytes = hash.as_bytes();
        let bucket = usize::from(bytes[0]) % dimensions;
        let sign = if bytes[1] % 2 == 0 { 1.0 } else { -1.0 };
        vector[bucket] += sign;
    }
    normalize(&mut vector);
    vector
}

fn normalize(vector: &mut [f32]) {
    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm == 0.0 {
        return;
    }
    for value in vector {
        *value /= norm;
    }
}

fn normalize_text(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::{
        format_chunk_for_embedding, format_document_for_reranking, format_query_for_embedding,
    };
    use std::collections::BTreeMap;

    #[test]
    fn formats_query_for_embedding() {
        assert_eq!(
            format_query_for_embedding("  rust retrieval  "),
            "query: rust retrieval"
        );
    }

    #[test]
    fn formats_chunk_with_document_context() {
        let formatted = format_chunk_for_embedding(
            "docs/guide.md",
            Some("Guide"),
            &["Intro".to_string(), "Usage".to_string()],
            "hello   world",
        );
        assert!(formatted.contains("path: docs/guide.md"));
        assert!(formatted.contains("title: Guide"));
        assert!(formatted.contains("headings: Intro > Usage"));
        assert!(formatted.contains("text: hello world"));
    }

    #[test]
    fn formats_document_for_reranking() {
        let mut metadata = BTreeMap::new();
        metadata.insert("lang".to_string(), "rust".to_string());
        let formatted = format_document_for_reranking(
            "docs/guide.md",
            Some("Guide"),
            &["Intro".to_string()],
            "hello   world",
            &metadata,
        );
        assert!(formatted.contains("path: docs/guide.md"));
        assert!(formatted.contains("title: Guide"));
        assert!(formatted.contains("headings: Intro"));
        assert!(formatted.contains("metadata: lang=rust"));
        assert!(formatted.contains("excerpt: hello world"));
    }
}
