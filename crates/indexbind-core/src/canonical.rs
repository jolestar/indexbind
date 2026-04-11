use crate::build::{build_chunk_id, build_doc_id, BuildArtifactOptions};
use crate::chunking::chunk_document;
use crate::embedding::{format_chunk_for_embedding, vector_to_bytes, Embedder, EmbeddingBackend};
use crate::lexical::{tokenize, LEXICAL_TOKENIZER_VERSION};
use crate::types::{MetadataMap, NormalizedDocument};
use crate::{IndexbindError, Result};
#[cfg(not(target_arch = "wasm32"))]
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
#[cfg(not(target_arch = "wasm32"))]
use hf_hub::api::sync::Api;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalArtifactManifest {
    pub schema_version: String,
    pub artifact_format: String,
    pub built_at: String,
    pub embedding_backend: EmbeddingBackend,
    pub document_count: usize,
    pub chunk_count: usize,
    pub vector_dimensions: usize,
    pub chunking: CanonicalChunkingConfig,
    pub files: CanonicalArtifactFiles,
    pub features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalChunkingConfig {
    pub target_tokens: usize,
    pub overlap_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalArtifactFiles {
    pub documents: String,
    pub chunks: String,
    pub vectors: String,
    pub postings: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<CanonicalModelFiles>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalModelFiles {
    pub tokenizer: String,
    pub config: String,
    pub weights: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalDocumentRecord {
    pub doc_id: String,
    pub relative_path: String,
    pub canonical_url: Option<String>,
    pub title: Option<String>,
    pub summary: Option<String>,
    #[serde(default)]
    pub metadata: MetadataMap,
    pub first_chunk_index: usize,
    pub chunk_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalChunkRecord {
    pub chunk_id: i64,
    pub doc_id: String,
    pub ordinal: usize,
    pub heading_path: Vec<String>,
    pub char_start: usize,
    pub char_end: usize,
    pub token_count: usize,
    pub excerpt: String,
    pub chunk_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalPostings {
    pub tokenizer: String,
    pub avg_chunk_length: f32,
    pub document_frequency: BTreeMap<String, usize>,
    pub postings: BTreeMap<String, Vec<CanonicalPosting>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalPosting {
    pub chunk_index: usize,
    pub term_frequency: usize,
}

#[derive(Debug, Clone)]
pub struct CanonicalBuildStats {
    pub document_count: usize,
    pub chunk_count: usize,
    pub vector_dimensions: usize,
}

pub fn build_canonical_artifact(
    output_dir: &Path,
    documents: &[NormalizedDocument],
    options: &BuildArtifactOptions,
) -> Result<CanonicalBuildStats> {
    fs::create_dir_all(output_dir)?;

    let mut embedder = Embedder::new(options.embedding_backend.clone())?;
    let built_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| IndexbindError::Embedding(error.into()))?
        .as_secs()
        .to_string();

    let mut canonical_documents = Vec::with_capacity(documents.len());
    let mut canonical_chunks = Vec::new();
    let mut vectors = Vec::new();

    for document in documents {
        let doc_id = document
            .doc_id
            .clone()
            .unwrap_or_else(|| build_doc_id(&options.source_root.id, &document.relative_path));
        let mut chunks = chunk_document(&doc_id, &document.content, &options.chunking);
        for chunk in &mut chunks {
            chunk.chunk_id = build_chunk_id(&doc_id, chunk.ordinal);
        }

        let embedding_inputs = chunks
            .iter()
            .map(|chunk| {
                format_chunk_for_embedding(
                    &document.relative_path,
                    document.title.as_deref(),
                    &chunk.heading_path,
                    &chunk.chunk_text,
                )
            })
            .collect::<Vec<_>>();
        let embeddings = embedder.embed_texts(&embedding_inputs)?;

        let first_chunk_index = canonical_chunks.len();
        canonical_documents.push(CanonicalDocumentRecord {
            doc_id: doc_id.clone(),
            relative_path: document.relative_path.clone(),
            canonical_url: document.canonical_url.clone(),
            title: document.title.clone(),
            summary: document.summary.clone(),
            metadata: document.metadata.clone(),
            first_chunk_index,
            chunk_count: chunks.len(),
        });

        for (chunk, embedding) in chunks.into_iter().zip(embeddings.into_iter()) {
            canonical_chunks.push(CanonicalChunkRecord {
                chunk_id: chunk.chunk_id,
                doc_id: chunk.doc_id,
                ordinal: chunk.ordinal,
                heading_path: chunk.heading_path,
                char_start: chunk.char_start,
                char_end: chunk.char_end,
                token_count: chunk.token_count,
                excerpt: chunk.excerpt,
                chunk_text: chunk.chunk_text,
            });
            vectors.extend_from_slice(&vector_to_bytes(&embedding));
        }
    }

    let postings = build_postings(&canonical_chunks);
    let vector_dimensions = if canonical_chunks.is_empty() {
        0
    } else {
        vectors.len() / 4 / canonical_chunks.len()
    };
    let model_files = maybe_write_model_assets(output_dir, &options.embedding_backend)?;
    let mut features = vec![
        "vector-search".to_string(),
        "lexical-postings".to_string(),
        "retrieval-only-results".to_string(),
    ];
    if model_files.is_some() {
        features.push("model2vec-query".to_string());
    }

    let manifest = CanonicalArtifactManifest {
        schema_version: "1".to_string(),
        artifact_format: "file-bundle-v1".to_string(),
        built_at,
        embedding_backend: options.embedding_backend.clone(),
        document_count: canonical_documents.len(),
        chunk_count: canonical_chunks.len(),
        vector_dimensions,
        chunking: CanonicalChunkingConfig {
            target_tokens: options.chunking.target_tokens,
            overlap_tokens: options.chunking.overlap_tokens,
        },
        files: CanonicalArtifactFiles {
            documents: "documents.json".to_string(),
            chunks: "chunks.json".to_string(),
            vectors: "vectors.bin".to_string(),
            postings: "postings.json".to_string(),
            model: model_files,
        },
        features,
    };

    fs::write(
        output_dir.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    fs::write(
        output_dir.join("documents.json"),
        serde_json::to_vec_pretty(&canonical_documents)?,
    )?;
    fs::write(
        output_dir.join("chunks.json"),
        serde_json::to_vec_pretty(&canonical_chunks)?,
    )?;
    fs::write(output_dir.join("vectors.bin"), vectors)?;
    fs::write(
        output_dir.join("postings.json"),
        serde_json::to_vec_pretty(&postings)?,
    )?;

    Ok(CanonicalBuildStats {
        document_count: manifest.document_count,
        chunk_count: manifest.chunk_count,
        vector_dimensions,
    })
}

pub(crate) fn maybe_write_model_assets(
    output_dir: &Path,
    embedding_backend: &EmbeddingBackend,
) -> Result<Option<CanonicalModelFiles>> {
    let EmbeddingBackend::Model2Vec { model, .. } = embedding_backend else {
        return Ok(None);
    };

    #[cfg(target_arch = "wasm32")]
    {
        let _ = output_dir;
        let _ = model;
        Err(IndexbindError::Embedding(anyhow::anyhow!(
            "model asset bundling is unavailable on wasm targets"
        )))
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let files = resolve_model_files_for_bundle(model, None)
            .map_err(|error| IndexbindError::Embedding(error.into()))?;
        let model_dir = output_dir.join("model");
        fs::create_dir_all(&model_dir)?;
        fs::copy(&files.tokenizer, model_dir.join("tokenizer.json"))?;
        fs::copy(&files.config, model_dir.join("config.json"))?;
        fs::copy(&files.model, model_dir.join("model.safetensors"))?;
        Ok(Some(CanonicalModelFiles {
            tokenizer: "model/tokenizer.json".to_string(),
            config: "model/config.json".to_string(),
            weights: "model/model.safetensors".to_string(),
        }))
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone)]
struct BundleModelFiles {
    tokenizer: std::path::PathBuf,
    config: std::path::PathBuf,
    model: std::path::PathBuf,
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_model_files_for_bundle(model: &str, token: Option<&str>) -> anyhow::Result<BundleModelFiles> {
    let base = Path::new(model);
    if base.exists() {
        return resolve_local_model_files(base);
    }

    let api = Api::new().context("hf-hub API init failed")?;
    let repo = api.model(model.to_owned());
    let _ = token;
    Ok(BundleModelFiles {
        tokenizer: repo.get("tokenizer.json")?,
        config: repo.get("config.json")?,
        model: repo.get("model.safetensors")?,
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_local_model_files(base: &Path) -> anyhow::Result<BundleModelFiles> {
    let tokenizer = base.join("tokenizer.json");
    let config = base.join("config.json");
    let model = base.join("model.safetensors");
    if !tokenizer.exists() || !config.exists() || !model.exists() {
        anyhow::bail!("local path {base:?} missing tokenizer / model / config");
    }
    Ok(BundleModelFiles {
        tokenizer,
        config,
        model,
    })
}

pub(crate) fn build_postings(chunks: &[CanonicalChunkRecord]) -> CanonicalPostings {
    let mut postings: BTreeMap<String, Vec<CanonicalPosting>> = BTreeMap::new();
    let mut document_frequency: BTreeMap<String, usize> = BTreeMap::new();
    let mut total_chunk_length = 0usize;

    for (chunk_index, chunk) in chunks.iter().enumerate() {
        let tokens = tokenize(&chunk.chunk_text);
        total_chunk_length += tokens.len();
        let mut frequencies: BTreeMap<String, usize> = BTreeMap::new();
        for token in tokens {
            *frequencies.entry(token).or_default() += 1;
        }

        for (token, term_frequency) in frequencies {
            postings
                .entry(token.clone())
                .or_default()
                .push(CanonicalPosting {
                    chunk_index,
                    term_frequency,
                });
            *document_frequency.entry(token).or_default() += 1;
        }
    }

    let avg_chunk_length = if chunks.is_empty() {
        0.0
    } else {
        total_chunk_length as f32 / chunks.len() as f32
    };

    CanonicalPostings {
        tokenizer: LEXICAL_TOKENIZER_VERSION.to_string(),
        avg_chunk_length,
        document_frequency,
        postings,
    }
}
#[cfg(test)]
mod tests {
    use super::{
        build_canonical_artifact, CanonicalArtifactManifest, CanonicalChunkRecord,
        CanonicalDocumentRecord, CanonicalPostings,
    };
    use crate::build::BuildArtifactOptions;
    use crate::embedding::EmbeddingBackend;
    use crate::types::{NormalizedDocument, SourceRoot};
    use serde_json::Value;
    use std::collections::BTreeMap;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn writes_bundle_files_with_expected_records() {
        let dir = tempdir().unwrap();
        let output = dir.path().join("bundle");
        let mut metadata = BTreeMap::new();
        metadata.insert("lang".to_string(), Value::String("rust".to_string()));
        let stats = build_canonical_artifact(
            &output,
            &[NormalizedDocument {
                doc_id: Some("guide-rust".to_string()),
                source_path: None,
                relative_path: "guides/rust.md".to_string(),
                canonical_url: Some("/guides/rust".to_string()),
                title: Some("Rust Guide".to_string()),
                summary: Some("Guide summary".to_string()),
                content: "# Intro\nRust retrieval guide.".to_string(),
                metadata,
            }],
            &BuildArtifactOptions {
                source_root: SourceRoot {
                    id: "root".to_string(),
                    original_path: ".".to_string(),
                },
                embedding_backend: EmbeddingBackend::Hashing { dimensions: 128 },
                chunking: Default::default(),
            },
        )
        .unwrap();

        assert_eq!(stats.document_count, 1);
        assert_eq!(stats.chunk_count, 1);
        assert_eq!(stats.vector_dimensions, 128);

        let manifest: CanonicalArtifactManifest =
            serde_json::from_slice(&fs::read(output.join("manifest.json")).unwrap()).unwrap();
        assert_eq!(manifest.artifact_format, "file-bundle-v1");
        assert_eq!(manifest.document_count, 1);

        let documents: Vec<CanonicalDocumentRecord> =
            serde_json::from_slice(&fs::read(output.join("documents.json")).unwrap()).unwrap();
        assert_eq!(documents[0].doc_id, "guide-rust");
        assert_eq!(documents[0].canonical_url.as_deref(), Some("/guides/rust"));

        let chunks: Vec<CanonicalChunkRecord> =
            serde_json::from_slice(&fs::read(output.join("chunks.json")).unwrap()).unwrap();
        assert_eq!(chunks[0].doc_id, "guide-rust");
        assert!(chunks[0].chunk_text.contains("Rust retrieval"));

        let postings: CanonicalPostings =
            serde_json::from_slice(&fs::read(output.join("postings.json")).unwrap()).unwrap();
        assert!(postings.postings.contains_key("rust"));
        assert_eq!(postings.tokenizer, crate::LEXICAL_TOKENIZER_VERSION);
        assert_eq!(fs::read(output.join("vectors.bin")).unwrap().len(), 128 * 4);
    }
}
