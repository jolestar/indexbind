use crate::build::{build_chunk_id, build_doc_id, BuildArtifactOptions, BuildStats};
use crate::chunking::chunk_document;
use crate::embedding::{format_chunk_for_embedding, vector_to_bytes, Embedder};
use crate::types::NormalizedDocument;
use crate::{IndexbindError, Result};
use rusqlite::{params, Connection};
use serde_json::json;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn build_artifact(
    output_path: &Path,
    documents: &[NormalizedDocument],
    options: &BuildArtifactOptions,
) -> Result<BuildStats> {
    let mut embedder = Embedder::new(options.embedding_backend.clone())?;
    let connection = Connection::open(output_path)?;
    initialize_schema(&connection)?;

    let mut document_count = 0usize;
    let mut chunk_count = 0usize;
    let built_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| IndexbindError::Embedding(error.into()))?
        .as_secs()
        .to_string();

    connection.execute(
        "INSERT INTO artifact_meta (key, value) VALUES (?1, ?2)",
        params!["schema_version", "2"],
    )?;
    connection.execute(
        "INSERT INTO artifact_meta (key, value) VALUES (?1, ?2)",
        params!["built_at", built_at],
    )?;
    connection.execute(
        "INSERT INTO artifact_meta (key, value) VALUES (?1, ?2)",
        params!["source_root", serde_json::to_string(&options.source_root)?],
    )?;
    connection.execute(
        "INSERT INTO artifact_meta (key, value) VALUES (?1, ?2)",
        params![
            "embedding_backend",
            serde_json::to_string(embedder.backend())?
        ],
    )?;
    connection.execute(
        "INSERT INTO artifact_meta (key, value) VALUES (?1, ?2)",
        params![
            "chunking",
            serde_json::to_string(&json!({
                "target_tokens": options.chunking.target_tokens,
                "overlap_tokens": options.chunking.overlap_tokens,
            }))?
        ],
    )?;

    let transaction = connection.unchecked_transaction()?;
    for document in documents {
        let doc_id = document
            .doc_id
            .clone()
            .unwrap_or_else(|| build_doc_id(&options.source_root.id, &document.relative_path));
        let content_hash = blake3::hash(document.content.as_bytes())
            .to_hex()
            .to_string();
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

        transaction.execute(
            "INSERT INTO documents (
                doc_id, source_root_id, source_path, relative_path, canonical_url, title,
                summary, content_hash, modified_at, chunk_count, metadata_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, ?9, ?10)",
            params![
                doc_id,
                options.source_root.id,
                document.source_path,
                document.relative_path,
                document.canonical_url,
                document.title,
                document.summary,
                content_hash,
                chunks.len() as i64,
                serde_json::to_string(&document.metadata)?,
            ],
        )?;

        for (chunk, embedding) in chunks.iter().zip(embeddings.iter()) {
            transaction.execute(
                "INSERT INTO chunks (
                    chunk_id, doc_id, ordinal, heading_path_json, char_start, char_end,
                    token_count, chunk_text, excerpt
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    chunk.chunk_id,
                    chunk.doc_id,
                    chunk.ordinal as i64,
                    serde_json::to_string(&chunk.heading_path)?,
                    chunk.char_start as i64,
                    chunk.char_end as i64,
                    chunk.token_count as i64,
                    chunk.chunk_text,
                    chunk.excerpt,
                ],
            )?;
            transaction.execute(
                "INSERT INTO chunk_vectors (chunk_id, dimensions, vector_blob) VALUES (?1, ?2, ?3)",
                params![
                    chunk.chunk_id,
                    embedding.len() as i64,
                    vector_to_bytes(embedding),
                ],
            )?;
            transaction.execute(
                "INSERT INTO fts_chunks (chunk_id, doc_id, chunk_text, excerpt) VALUES (?1, ?2, ?3, ?4)",
                params![chunk.chunk_id, chunk.doc_id, chunk.chunk_text, chunk.excerpt],
            )?;
        }

        document_count += 1;
        chunk_count += chunks.len();
    }
    transaction.commit()?;

    Ok(BuildStats {
        document_count,
        chunk_count,
    })
}

fn initialize_schema(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        CREATE TABLE artifact_meta (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE documents (
            doc_id TEXT PRIMARY KEY,
            source_root_id TEXT NOT NULL,
            source_path TEXT,
            relative_path TEXT NOT NULL,
            canonical_url TEXT,
            title TEXT,
            summary TEXT,
            content_hash TEXT NOT NULL,
            modified_at INTEGER,
            chunk_count INTEGER NOT NULL,
            metadata_json TEXT NOT NULL
        );
        CREATE TABLE chunks (
            chunk_id INTEGER PRIMARY KEY,
            doc_id TEXT NOT NULL,
            ordinal INTEGER NOT NULL,
            heading_path_json TEXT NOT NULL,
            char_start INTEGER NOT NULL,
            char_end INTEGER NOT NULL,
            token_count INTEGER NOT NULL,
            chunk_text TEXT NOT NULL,
            excerpt TEXT NOT NULL
        );
        CREATE TABLE chunk_vectors (
            chunk_id INTEGER PRIMARY KEY,
            dimensions INTEGER NOT NULL,
            vector_blob BLOB NOT NULL
        );
        CREATE VIRTUAL TABLE fts_chunks USING fts5(
            chunk_id UNINDEXED,
            doc_id UNINDEXED,
            chunk_text,
            excerpt
        );
        ",
    )?;
    Ok(())
}
