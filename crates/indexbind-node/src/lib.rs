use indexbind_core::{DocumentHit, LoadedDocument, Retriever, SearchOptions};
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
    pub original_path: String,
    pub relative_path: String,
    pub title: Option<String>,
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

#[napi(object)]
pub struct NodeLoadedDocument {
    pub original_path: String,
    pub relative_path: String,
    pub content: String,
}

#[napi]
pub struct NativeIndex {
    inner: Mutex<Retriever>,
}

#[napi]
impl NativeIndex {
    #[napi(factory)]
    pub fn open(artifact_path: String, source_root_override: Option<String>) -> napi::Result<Self> {
        let override_path = source_root_override.map(PathBuf::from);
        let artifact_path = PathBuf::from(artifact_path);
        let retriever = Retriever::open(&artifact_path, override_path).map_err(map_error)?;
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
                .collect(),
            ..SearchOptions::default()
        };
        let hits = retriever.search(&query, options).map_err(map_error)?;
        Ok(hits.into_iter().map(map_hit).collect())
    }

    #[napi]
    pub fn read_document(
        &self,
        doc_id: String,
        original_path: String,
        relative_path: String,
        title: Option<String>,
        score: f64,
        best_match: NodeBestMatch,
    ) -> napi::Result<NodeLoadedDocument> {
        let retriever = self
            .inner
            .lock()
            .map_err(|error| Error::from_reason(error.to_string()))?;
        let loaded = retriever
            .read_document(&DocumentHit {
                doc_id,
                original_path,
                relative_path,
                title,
                score: score as f32,
                best_match: indexbind_core::BestMatch {
                    chunk_id: best_match.chunk_id,
                    excerpt: best_match.excerpt,
                    heading_path: best_match.heading_path,
                    char_start: best_match.char_start as usize,
                    char_end: best_match.char_end as usize,
                    score: best_match.score as f32,
                },
                metadata: Default::default(),
            })
            .map_err(map_error)?;
        Ok(map_loaded(loaded))
    }
}

fn map_hit(hit: DocumentHit) -> NodeDocumentHit {
    NodeDocumentHit {
        doc_id: hit.doc_id,
        original_path: hit.original_path,
        relative_path: hit.relative_path,
        title: hit.title,
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

fn map_loaded(loaded: LoadedDocument) -> NodeLoadedDocument {
    NodeLoadedDocument {
        original_path: loaded.original_path,
        relative_path: loaded.relative_path,
        content: loaded.content,
    }
}

fn map_error(error: impl std::fmt::Display) -> Error {
    Error::from_reason(error.to_string())
}

fn map_serde_error(error: serde_json::Error) -> Error {
    Error::from_reason(error.to_string())
}
