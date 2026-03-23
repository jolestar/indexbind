use indexbind_core::{DocumentHit, Retriever, SearchOptions};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

#[napi(object)]
pub struct NodeSearchOptions {
    pub top_k: Option<u32>,
    pub hybrid: Option<bool>,
    pub reranker: Option<NodeRerankerOptions>,
    pub relative_path_prefix: Option<String>,
    pub metadata: Option<HashMap<String, String>>,
}

#[napi(object)]
pub struct NodeRerankerOptions {
    pub kind: Option<String>,
    pub candidate_pool_size: Option<u32>,
}

#[napi(object)]
pub struct NodeBestMatch {
    pub chunk_id: i64,
    pub excerpt: String,
    pub heading_path: Vec<String>,
    pub char_start: u32,
    pub char_end: u32,
    pub score: f64,
}

#[napi(object)]
pub struct NodeDocumentHit {
    pub doc_id: String,
    pub relative_path: String,
    pub canonical_url: Option<String>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub metadata: String,
    pub score: f64,
    pub best_match: NodeBestMatch,
}

#[napi(object)]
pub struct NodeArtifactInfo {
    pub schema_version: String,
    pub built_at: String,
    pub embedding_backend: String,
    pub source_root: String,
    pub document_count: u32,
    pub chunk_count: u32,
}

#[napi]
pub struct NativeIndex {
    inner: Mutex<Retriever>,
}

#[napi]
impl NativeIndex {
    #[napi(factory)]
    pub fn open(artifact_path: String) -> napi::Result<Self> {
        let artifact_path = PathBuf::from(artifact_path);
        let retriever = Retriever::open(&artifact_path).map_err(map_error)?;
        Ok(Self {
            inner: Mutex::new(retriever),
        })
    }

    #[napi]
    pub fn info(&self) -> napi::Result<NodeArtifactInfo> {
        let retriever = self
            .inner
            .lock()
            .map_err(|error| Error::from_reason(error.to_string()))?;
        let info = retriever.info();
        let embedding_backend =
            serde_json::to_string(&info.embedding_backend).map_err(map_serde_error)?;
        let source_root = serde_json::to_string(&info.source_root).map_err(map_serde_error)?;
        Ok(NodeArtifactInfo {
            schema_version: info.schema_version.clone(),
            built_at: info.built_at.clone(),
            embedding_backend,
            source_root,
            document_count: info.document_count as u32,
            chunk_count: info.chunk_count as u32,
        })
    }

    #[napi]
    pub fn search(
        &self,
        query: String,
        options: Option<NodeSearchOptions>,
    ) -> napi::Result<Vec<NodeDocumentHit>> {
        let mut retriever = self
            .inner
            .lock()
            .map_err(|error| Error::from_reason(error.to_string()))?;
        let options = SearchOptions {
            top_k: options.as_ref().and_then(|value| value.top_k).unwrap_or(10) as usize,
            hybrid: options
                .as_ref()
                .and_then(|value| value.hybrid)
                .unwrap_or(true),
            reranker: options
                .as_ref()
                .and_then(|value| value.reranker.as_ref())
                .map(|value| {
                    Ok(indexbind_core::RerankerOptions {
                        kind: match value.kind.as_deref() {
                            Some("embedding-v1") | None => indexbind_core::RerankerKind::EmbeddingV1,
                            Some("heuristic-v1") => indexbind_core::RerankerKind::HeuristicV1,
                            Some(other) => {
                                return Err(Error::from_reason(format!(
                                    "unsupported reranker kind: {other}"
                                )))
                            }
                        },
                        candidate_pool_size: value.candidate_pool_size.unwrap_or(50) as usize,
                    })
                })
                .transpose()?,
            relative_path_prefix: options
                .as_ref()
                .and_then(|value| value.relative_path_prefix.clone()),
            metadata: options
                .and_then(|value| value.metadata)
                .unwrap_or_default()
                .into_iter()
                .map(|(key, value)| (key, serde_json::Value::String(value)))
                .collect(),
            ..SearchOptions::default()
        };
        let hits = retriever.search(&query, options).map_err(map_error)?;
        Ok(hits.into_iter().map(map_hit).collect())
    }
}

fn map_hit(hit: DocumentHit) -> NodeDocumentHit {
    NodeDocumentHit {
        doc_id: hit.doc_id,
        relative_path: hit.relative_path,
        canonical_url: hit.canonical_url,
        title: hit.title,
        summary: hit.summary,
        metadata: serde_json::to_string(&hit.metadata).unwrap_or_else(|_| "{}".to_string()),
        score: hit.score as f64,
        best_match: NodeBestMatch {
            chunk_id: hit.best_match.chunk_id,
            excerpt: hit.best_match.excerpt,
            heading_path: hit.best_match.heading_path,
            char_start: hit.best_match.char_start as u32,
            char_end: hit.best_match.char_end as u32,
            score: hit.best_match.score as f64,
        },
    }
}

fn map_error(error: impl std::fmt::Display) -> Error {
    Error::from_reason(error.to_string())
}

fn map_serde_error(error: serde_json::Error) -> Error {
    Error::from_reason(error.to_string())
}
