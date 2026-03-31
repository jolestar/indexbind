use indexbind_build::{
    build_canonical_from_directory, build_from_directory, update_cache_from_directory_with_mode,
    DirectoryUpdateMode,
};
use indexbind_core::{
    build_canonical_artifact, export_artifact_from_build_cache, export_canonical_from_build_cache,
    update_build_cache, BuildArtifactOptions, BuildCacheUpdate, BuildStats, CanonicalBuildStats,
    ChunkingOptions, DocumentHit, EmbeddingBackend, IncrementalBuildStats, NormalizedDocument,
    Retriever, SearchOptions, SourceRoot,
};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

#[napi(object)]
pub struct NodeSearchOptions {
    pub top_k: Option<u32>,
    pub mode: Option<String>,
    pub min_score: Option<f64>,
    pub reranker: Option<NodeRerankerOptions>,
    pub relative_path_prefix: Option<String>,
    pub metadata: Option<HashMap<String, String>>,
    pub score_adjustment: Option<NodeScoreAdjustmentOptions>,
}

#[napi(object)]
pub struct NodeRerankerOptions {
    pub kind: Option<String>,
    pub candidate_pool_size: Option<u32>,
}

#[napi(object)]
pub struct NodeScoreAdjustmentOptions {
    pub metadata_numeric_multiplier: Option<String>,
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
    pub lexical_tokenizer: String,
    pub source_root: String,
    pub document_count: i64,
    pub chunk_count: i64,
}

#[napi(object)]
pub struct NodeBuildDocument {
    pub doc_id: Option<String>,
    pub source_path: Option<String>,
    pub relative_path: String,
    pub canonical_url: Option<String>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub content: String,
    pub metadata_json: Option<String>,
}

#[napi(object)]
pub struct NodeBuildOptions {
    pub embedding_backend: Option<String>,
    pub hashing_dimensions: Option<u32>,
    pub model: Option<String>,
    pub batch_size: Option<u32>,
    pub source_root_id: Option<String>,
    pub source_root_path: Option<String>,
    pub target_tokens: Option<u32>,
    pub overlap_tokens: Option<u32>,
}

#[napi(object)]
pub struct NodeCanonicalBuildStats {
    pub document_count: i64,
    pub chunk_count: i64,
    pub vector_dimensions: i64,
}

#[napi(object)]
pub struct NodeBuildStats {
    pub document_count: i64,
    pub chunk_count: i64,
}

#[napi(object)]
pub struct NodeIncrementalBuildStats {
    pub scanned_document_count: i64,
    pub new_document_count: i64,
    pub changed_document_count: i64,
    pub unchanged_document_count: i64,
    pub removed_document_count: i64,
    pub active_document_count: i64,
    pub active_chunk_count: i64,
}

#[napi(object)]
#[derive(Clone)]
pub struct NodeDirectoryUpdateMode {
    pub mode: Option<String>,
    pub base_revision: Option<String>,
}

#[napi(object)]
pub struct NodeBenchmarkCaseResult {
    pub name: String,
    pub query: String,
    pub expected_top_hit: String,
    pub actual_top_hit: Option<String>,
    pub passed: bool,
}

#[napi(object)]
pub struct NodeBenchmarkSummary {
    pub fixture: String,
    pub total: i64,
    pub passed: i64,
    pub failed: i64,
    pub results: Vec<NodeBenchmarkCaseResult>,
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
            lexical_tokenizer: info.lexical_tokenizer.clone(),
            source_root,
            document_count: info.document_count as i64,
            chunk_count: info.chunk_count as i64,
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
            mode: match options.as_ref().and_then(|value| value.mode.as_deref()) {
                Some("hybrid") | None => indexbind_core::RetrievalMode::Hybrid,
                Some("vector") => indexbind_core::RetrievalMode::Vector,
                Some("lexical") => indexbind_core::RetrievalMode::Lexical,
                Some(other) => {
                    return Err(Error::from_reason(format!(
                        "unsupported retrieval mode: {other}"
                    )))
                }
            },
            min_score: options
                .as_ref()
                .and_then(|value| value.min_score)
                .map(|value| value as f32),
            reranker: options
                .as_ref()
                .and_then(|value| value.reranker.as_ref())
                .map(|value| {
                    Ok(indexbind_core::RerankerOptions {
                        kind: match value.kind.as_deref() {
                            Some("embedding-v1") | None => {
                                indexbind_core::RerankerKind::EmbeddingV1
                            }
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
                .as_ref()
                .and_then(|value| value.metadata.clone())
                .unwrap_or_default()
                .into_iter()
                .map(|(key, value)| (key, serde_json::Value::String(value)))
                .collect(),
            score_adjustment: options
                .as_ref()
                .and_then(|value| value.score_adjustment.as_ref())
                .map(|value| indexbind_core::ScoreAdjustmentOptions {
                    metadata_numeric_multiplier: value.metadata_numeric_multiplier.clone(),
                }),
            ..SearchOptions::default()
        };
        let hits = retriever.search(&query, options).map_err(map_error)?;
        Ok(hits.into_iter().map(map_hit).collect())
    }
}

#[napi]
pub fn build_canonical_bundle(
    output_dir: String,
    documents: Vec<NodeBuildDocument>,
    options: Option<NodeBuildOptions>,
) -> napi::Result<NodeCanonicalBuildStats> {
    let build_options = map_build_options(options);
    let normalized_documents = documents
        .into_iter()
        .map(map_build_document)
        .collect::<napi::Result<Vec<_>>>()?;
    let stats = build_canonical_artifact(
        &PathBuf::from(output_dir),
        &normalized_documents,
        &build_options,
    )
    .map_err(map_error)?;
    Ok(map_build_stats(stats))
}

#[napi]
pub fn build_artifact_from_directory(
    input_dir: String,
    output_path: String,
    options: Option<NodeBuildOptions>,
) -> napi::Result<NodeBuildStats> {
    let stats = build_from_directory(
        &PathBuf::from(input_dir),
        &PathBuf::from(output_path),
        map_build_options(options),
    )
    .map_err(map_error)?;
    Ok(map_plain_build_stats(stats))
}

#[napi]
pub fn build_canonical_bundle_from_directory(
    input_dir: String,
    output_dir: String,
    options: Option<NodeBuildOptions>,
) -> napi::Result<NodeCanonicalBuildStats> {
    let stats = build_canonical_from_directory(
        &PathBuf::from(input_dir),
        &PathBuf::from(output_dir),
        map_build_options(options),
    )
    .map_err(map_error)?;
    Ok(map_build_stats(stats))
}

#[napi]
pub fn update_build_cache_from_documents(
    cache_path: String,
    documents: Vec<NodeBuildDocument>,
    removed_relative_paths: Option<Vec<String>>,
    options: Option<NodeBuildOptions>,
) -> napi::Result<NodeIncrementalBuildStats> {
    let build_options = map_build_options(options);
    let normalized_documents = documents
        .into_iter()
        .map(map_build_document)
        .collect::<napi::Result<Vec<_>>>()?;
    let stats = update_build_cache(
        &PathBuf::from(cache_path),
        BuildCacheUpdate {
            documents: normalized_documents,
            removed_relative_paths: removed_relative_paths.unwrap_or_default(),
            replace_all: false,
        },
        &build_options,
    )
    .map_err(map_error)?;
    Ok(map_incremental_build_stats(stats))
}

#[napi]
pub fn update_build_cache_from_directory(
    input_dir: String,
    cache_path: String,
    options: Option<NodeBuildOptions>,
    update_mode: Option<NodeDirectoryUpdateMode>,
) -> napi::Result<NodeIncrementalBuildStats> {
    let mode = match update_mode.as_ref().and_then(|value| value.mode.as_deref()) {
        Some(mode) if mode == "git-diff" => DirectoryUpdateMode::GitDiff {
            base_revision: update_mode.and_then(|value| value.base_revision),
        },
        Some(mode) if mode != "full-scan" => {
            return Err(Error::from_reason(format!(
                "unsupported directory update mode: {mode}"
            )))
        }
        _ => DirectoryUpdateMode::FullScan,
    };
    let stats = update_cache_from_directory_with_mode(
        &PathBuf::from(input_dir),
        &PathBuf::from(cache_path),
        map_build_options(options),
        mode,
    )
    .map_err(map_error)?;
    Ok(map_incremental_build_stats(stats))
}

#[napi]
pub fn export_artifact_from_cache(
    cache_path: String,
    output_path: String,
) -> napi::Result<NodeBuildStats> {
    let stats =
        export_artifact_from_build_cache(&PathBuf::from(cache_path), &PathBuf::from(output_path))
            .map_err(map_error)?;
    Ok(map_plain_build_stats(stats))
}

#[napi]
pub fn export_canonical_bundle_from_cache(
    cache_path: String,
    output_dir: String,
) -> napi::Result<NodeCanonicalBuildStats> {
    let stats =
        export_canonical_from_build_cache(&PathBuf::from(cache_path), &PathBuf::from(output_dir))
            .map_err(map_error)?;
    Ok(map_build_stats(stats))
}

#[napi]
pub fn inspect_artifact(artifact_path: String) -> napi::Result<NodeArtifactInfo> {
    let retriever = Retriever::open(&PathBuf::from(artifact_path)).map_err(map_error)?;
    Ok(map_artifact_info(retriever.info()))
}

#[napi]
pub fn benchmark_artifact(
    artifact_path: String,
    queries_json_path: String,
) -> napi::Result<NodeBenchmarkSummary> {
    let payload = fs::read_to_string(queries_json_path).map_err(map_error)?;
    let fixture: BenchmarkFixture = serde_json::from_str(&payload).map_err(map_serde_error)?;
    let mut retriever = Retriever::open(&PathBuf::from(artifact_path)).map_err(map_error)?;

    let mut passed = 0usize;
    let mut results = Vec::new();
    for case in fixture.queries {
        let hits = retriever
            .search(
                &case.query,
                SearchOptions {
                    top_k: case.top_k.unwrap_or(5),
                    ..SearchOptions::default()
                },
            )
            .map_err(map_error)?;
        let actual_top_hit = hits.first().map(|hit| hit.relative_path.clone());
        let success = actual_top_hit.as_deref() == Some(case.expected_top_hit.as_str());
        if success {
            passed += 1;
        }
        results.push(NodeBenchmarkCaseResult {
            name: case.name,
            query: case.query,
            expected_top_hit: case.expected_top_hit,
            actual_top_hit,
            passed: success,
        });
    }

    Ok(NodeBenchmarkSummary {
        fixture: fixture.name,
        total: results.len() as i64,
        passed: passed as i64,
        failed: results.len().saturating_sub(passed) as i64,
        results,
    })
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

fn map_artifact_info(info: &indexbind_core::ArtifactInfo) -> NodeArtifactInfo {
    let embedding_backend =
        serde_json::to_string(&info.embedding_backend).unwrap_or_else(|_| "{}".to_string());
    let source_root = serde_json::to_string(&info.source_root).unwrap_or_else(|_| "{}".to_string());
    NodeArtifactInfo {
        schema_version: info.schema_version.clone(),
        built_at: info.built_at.clone(),
        embedding_backend,
        lexical_tokenizer: info.lexical_tokenizer.clone(),
        source_root,
        document_count: info.document_count as i64,
        chunk_count: info.chunk_count as i64,
    }
}

fn map_build_document(document: NodeBuildDocument) -> napi::Result<NormalizedDocument> {
    let metadata = document
        .metadata_json
        .as_deref()
        .map(serde_json::from_str::<HashMap<String, Value>>)
        .transpose()
        .map_err(map_serde_error)?
        .unwrap_or_default()
        .into_iter()
        .collect();
    Ok(NormalizedDocument {
        doc_id: document.doc_id,
        source_path: document.source_path,
        relative_path: document.relative_path,
        canonical_url: document.canonical_url,
        title: document.title,
        summary: document.summary,
        content: document.content,
        metadata,
    })
}

fn map_build_options(options: Option<NodeBuildOptions>) -> BuildArtifactOptions {
    let options = options.unwrap_or(NodeBuildOptions {
        embedding_backend: None,
        hashing_dimensions: None,
        model: None,
        batch_size: None,
        source_root_id: None,
        source_root_path: None,
        target_tokens: None,
        overlap_tokens: None,
    });
    let embedding_backend = match options.embedding_backend.as_deref() {
        Some("hashing") => EmbeddingBackend::Hashing {
            dimensions: options.hashing_dimensions.unwrap_or(256) as usize,
        },
        Some("model2vec") | None => EmbeddingBackend::Model2Vec {
            model: options
                .model
                .unwrap_or_else(|| "minishlab/potion-base-2M".to_string()),
            batch_size: options.batch_size.unwrap_or(256) as usize,
        },
        Some(other) => EmbeddingBackend::Model2Vec {
            model: other.to_string(),
            batch_size: options.batch_size.unwrap_or(256) as usize,
        },
    };

    BuildArtifactOptions {
        source_root: SourceRoot {
            id: options.source_root_id.unwrap_or_else(|| "root".to_string()),
            original_path: options.source_root_path.unwrap_or_else(|| ".".to_string()),
        },
        embedding_backend,
        chunking: ChunkingOptions {
            target_tokens: options.target_tokens.unwrap_or(512) as usize,
            overlap_tokens: options.overlap_tokens.unwrap_or(64) as usize,
        },
    }
}

fn map_build_stats(stats: CanonicalBuildStats) -> NodeCanonicalBuildStats {
    NodeCanonicalBuildStats {
        document_count: stats.document_count as i64,
        chunk_count: stats.chunk_count as i64,
        vector_dimensions: stats.vector_dimensions as i64,
    }
}

fn map_plain_build_stats(stats: BuildStats) -> NodeBuildStats {
    NodeBuildStats {
        document_count: stats.document_count as i64,
        chunk_count: stats.chunk_count as i64,
    }
}

fn map_incremental_build_stats(stats: IncrementalBuildStats) -> NodeIncrementalBuildStats {
    NodeIncrementalBuildStats {
        scanned_document_count: stats.scanned_document_count as i64,
        new_document_count: stats.new_document_count as i64,
        changed_document_count: stats.changed_document_count as i64,
        unchanged_document_count: stats.unchanged_document_count as i64,
        removed_document_count: stats.removed_document_count as i64,
        active_document_count: stats.active_document_count as i64,
        active_chunk_count: stats.active_chunk_count as i64,
    }
}

fn map_error(error: impl std::fmt::Display) -> Error {
    Error::from_reason(error.to_string())
}

fn map_serde_error(error: serde_json::Error) -> Error {
    Error::from_reason(error.to_string())
}

#[derive(Debug, Deserialize)]
struct BenchmarkFixture {
    name: String,
    queries: Vec<BenchmarkQuery>,
}

#[derive(Debug, Deserialize)]
struct BenchmarkQuery {
    name: String,
    query: String,
    expected_top_hit: String,
    top_k: Option<usize>,
}
