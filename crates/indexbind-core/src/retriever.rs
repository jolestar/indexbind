use crate::embedding::{
    bytes_to_vector, cosine_similarity, format_document_for_reranking, format_query_for_embedding,
    Embedder, EmbeddingBackend,
};
use crate::types::{
    BestMatch, DocumentHit, LoadedDocument, SourceRoot, StoredChunk, StoredDocument,
};
use crate::{IndexbindError, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactInfo {
    pub schema_version: String,
    pub built_at: String,
    pub embedding_backend: EmbeddingBackend,
    pub source_root: SourceRoot,
    pub document_count: usize,
    pub chunk_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOptions {
    pub top_k: usize,
    pub hybrid: bool,
    pub candidate_multiplier: usize,
    #[serde(default)]
    pub reranker: Option<RerankerOptions>,
    #[serde(default)]
    pub relative_path_prefix: Option<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankerOptions {
    #[serde(default = "default_reranker_kind")]
    pub kind: RerankerKind,
    pub candidate_pool_size: usize,
}

impl Default for RerankerOptions {
    fn default() -> Self {
        Self {
            kind: RerankerKind::EmbeddingV1,
            candidate_pool_size: 50,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum RerankerKind {
    #[default]
    EmbeddingV1,
    HeuristicV1,
}

fn default_reranker_kind() -> RerankerKind {
    RerankerKind::EmbeddingV1
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            top_k: 10,
            hybrid: true,
            candidate_multiplier: 8,
            reranker: None,
            relative_path_prefix: None,
            metadata: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct IndexedChunk {
    chunk: StoredChunk,
    embedding: Vec<f32>,
}

pub struct Retriever {
    connection: Connection,
    info: ArtifactInfo,
    documents: HashMap<String, StoredDocument>,
    chunks: Vec<IndexedChunk>,
    chunks_by_id: HashMap<i64, StoredChunk>,
    source_root_override: Option<PathBuf>,
    embedder: Embedder,
}

impl Retriever {
    pub fn open(path: &Path, source_root_override: Option<PathBuf>) -> Result<Self> {
        let connection = Connection::open(path)?;
        let info = load_info(&connection)?;
        let documents = load_documents(&connection)?;
        let chunks = load_chunks(&connection)?;
        let chunks_by_id = chunks
            .iter()
            .map(|entry| (entry.chunk.chunk_id, entry.chunk.clone()))
            .collect::<HashMap<_, _>>();
        let embedder = Embedder::new(info.embedding_backend.clone())?;

        Ok(Self {
            connection,
            info,
            documents,
            chunks,
            chunks_by_id,
            source_root_override,
            embedder,
        })
    }

    pub fn info(&self) -> &ArtifactInfo {
        &self.info
    }

    pub fn search(&mut self, query: &str, options: SearchOptions) -> Result<Vec<DocumentHit>> {
        let allowed_doc_ids = self.allowed_doc_ids(&options);
        if allowed_doc_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rerank_candidate_limit = options
            .reranker
            .as_ref()
            .map(|config| config.candidate_pool_size.max(options.top_k))
            .unwrap_or(options.top_k);
        let limit = (options.top_k * options.candidate_multiplier.max(1))
            .max(rerank_candidate_limit)
            .max(options.top_k);
        let formatted_query = format_query_for_embedding(query);
        let query_embedding = self
            .embedder
            .embed_texts(&[formatted_query])?
            .into_iter()
            .next()
            .unwrap_or_default();
        let vector_docs = self.rank_documents_by_vector(&query_embedding, limit, &allowed_doc_ids);
        let lexical_docs = if options.hybrid {
            self.rank_documents_by_lexical(query, limit, &allowed_doc_ids)?
        } else {
            Vec::new()
        };
        let fused_hits = fuse_documents(&self.documents, &vector_docs, &lexical_docs, limit);
        let hits =
            self.rerank_documents(query, &fused_hits, options.reranker.as_ref(), options.top_k)?;
        Ok(hits)
    }

    pub fn read_document(&self, hit: &DocumentHit) -> Result<LoadedDocument> {
        let path = if let Some(source_root) = &self.source_root_override {
            source_root.join(&hit.relative_path)
        } else {
            PathBuf::from(&hit.original_path)
        };
        let content = fs::read_to_string(&path)
            .map_err(|_| IndexbindError::DocumentNotFound(path.display().to_string()))?;
        Ok(LoadedDocument {
            original_path: hit.original_path.clone(),
            relative_path: hit.relative_path.clone(),
            content,
        })
    }
}

impl Retriever {
    fn allowed_doc_ids(&self, options: &SearchOptions) -> HashSet<String> {
        self.documents
            .values()
            .filter(|document| document_matches(document, options))
            .map(|document| document.doc_id.clone())
            .collect()
    }

    fn rank_documents_by_vector(
        &self,
        query_embedding: &[f32],
        limit: usize,
        allowed_doc_ids: &HashSet<String>,
    ) -> Vec<RankedDocument> {
        let mut chunk_scores = self
            .chunks
            .iter()
            .filter(|indexed_chunk| allowed_doc_ids.contains(&indexed_chunk.chunk.doc_id))
            .map(|indexed_chunk| {
                (
                    cosine_similarity(query_embedding, &indexed_chunk.embedding),
                    &indexed_chunk.chunk,
                )
            })
            .filter(|(score, _)| *score > 0.0)
            .collect::<Vec<_>>();
        chunk_scores.sort_by(|left, right| right.0.partial_cmp(&left.0).unwrap_or(Ordering::Equal));

        aggregate_ranked_documents(chunk_scores.into_iter().take(limit * 2), limit)
    }

    fn rank_documents_by_lexical(
        &self,
        query: &str,
        limit: usize,
        allowed_doc_ids: &HashSet<String>,
    ) -> Result<Vec<RankedDocument>> {
        let Some(fts_query) = build_fts_query(query) else {
            return Ok(Vec::new());
        };

        let mut statement = self.connection.prepare(
            "
            SELECT chunk_id, doc_id, bm25(fts_chunks) AS rank
            FROM fts_chunks
            WHERE fts_chunks MATCH ?1
            ORDER BY rank
            LIMIT ?2
            ",
        )?;
        let rows = statement.query_map(params![fts_query, limit as i64], |row| {
            let chunk_id: i64 = row.get(0)?;
            let doc_id: String = row.get(1)?;
            let bm25: f64 = row.get(2)?;
            Ok((chunk_id, doc_id, bm25))
        })?;

        let mut chunk_scores = Vec::new();
        for row in rows {
            let (chunk_id, _doc_id, bm25) = row?;
            if let Some(chunk) = self.chunks_by_id.get(&chunk_id) {
                if !allowed_doc_ids.contains(&chunk.doc_id) {
                    continue;
                }
                let lexical = 1.0 / (1.0 + bm25.abs() as f32);
                chunk_scores.push((lexical, chunk));
            }
        }

        Ok(aggregate_ranked_documents(chunk_scores.into_iter(), limit))
    }

    fn rerank_documents(
        &mut self,
        query: &str,
        hits: &[DocumentHit],
        config: Option<&RerankerOptions>,
        top_k: usize,
    ) -> Result<Vec<DocumentHit>> {
        let Some(config) = config else {
            return Ok(hits.iter().take(top_k).cloned().collect());
        };

        match config.kind {
            RerankerKind::EmbeddingV1 => {
                rerank_documents_with_embeddings(&mut self.embedder, query, hits, config, top_k)
            }
            RerankerKind::HeuristicV1 => {
                Ok(rerank_documents_with_heuristic(query, hits, config, top_k))
            }
        }
    }
}

fn document_matches(document: &StoredDocument, options: &SearchOptions) -> bool {
    if let Some(prefix) = &options.relative_path_prefix {
        if !document.relative_path.starts_with(prefix) {
            return false;
        }
    }

    options
        .metadata
        .iter()
        .all(|(key, value)| document.metadata.get(key) == Some(value))
}

#[derive(Debug, Clone)]
struct RankedDocument {
    doc_id: String,
    score: f32,
    best_match: BestMatch,
}

fn aggregate_ranked_documents<'a, I>(chunk_scores: I, limit: usize) -> Vec<RankedDocument>
where
    I: Iterator<Item = (f32, &'a StoredChunk)>,
{
    let mut by_document: HashMap<String, Vec<(f32, &StoredChunk)>> = HashMap::new();
    for (score, chunk) in chunk_scores {
        by_document
            .entry(chunk.doc_id.clone())
            .or_default()
            .push((score, chunk));
    }

    let mut documents = by_document
        .into_iter()
        .filter_map(|(doc_id, mut scores)| {
            scores.sort_by(|left, right| right.0.partial_cmp(&left.0).unwrap_or(Ordering::Equal));
            let best = scores.first()?;
            let aggregate = best.0
                + scores
                    .iter()
                    .skip(1)
                    .take(2)
                    .map(|entry| entry.0)
                    .sum::<f32>()
                    * 0.1;
            Some(RankedDocument {
                doc_id,
                score: aggregate,
                best_match: BestMatch {
                    chunk_id: best.1.chunk_id,
                    excerpt: best.1.excerpt.clone(),
                    heading_path: best.1.heading_path.clone(),
                    char_start: best.1.char_start,
                    char_end: best.1.char_end,
                    score: best.0,
                },
            })
        })
        .collect::<Vec<_>>();

    documents.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(Ordering::Equal)
    });
    documents.truncate(limit);
    documents
}

fn fuse_documents(
    documents: &HashMap<String, StoredDocument>,
    vector_docs: &[RankedDocument],
    lexical_docs: &[RankedDocument],
    top_k: usize,
) -> Vec<DocumentHit> {
    const RRF_K: f32 = 60.0;

    #[derive(Default)]
    struct FusedScore {
        score: f32,
        vector_best: Option<BestMatch>,
        lexical_best: Option<BestMatch>,
    }

    let mut fused: HashMap<String, FusedScore> = HashMap::new();

    for (rank, entry) in vector_docs.iter().enumerate() {
        let score = 1.0 / (RRF_K + rank as f32 + 1.0);
        let fused_entry = fused.entry(entry.doc_id.clone()).or_default();
        fused_entry.score += score;
        fused_entry.vector_best = Some(entry.best_match.clone());
    }

    for (rank, entry) in lexical_docs.iter().enumerate() {
        let score = 1.0 / (RRF_K + rank as f32 + 1.0);
        let fused_entry = fused.entry(entry.doc_id.clone()).or_default();
        fused_entry.score += score;
        fused_entry.lexical_best = Some(entry.best_match.clone());
    }

    let mut hits = fused
        .into_iter()
        .filter_map(|(doc_id, fused_score)| {
            let document = documents.get(&doc_id)?;
            let best_match = fused_score
                .vector_best
                .or(fused_score.lexical_best)
                .unwrap_or(BestMatch {
                    chunk_id: 0,
                    excerpt: String::new(),
                    heading_path: Vec::new(),
                    char_start: 0,
                    char_end: 0,
                    score: 0.0,
                });
            Some(DocumentHit {
                doc_id: document.doc_id.clone(),
                original_path: document.original_path.clone(),
                relative_path: document.relative_path.clone(),
                title: document.title.clone(),
                score: fused_score.score,
                best_match,
                metadata: document.metadata.clone(),
            })
        })
        .collect::<Vec<_>>();

    hits.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(Ordering::Equal)
    });
    hits.truncate(top_k);
    hits
}

fn rerank_documents_with_heuristic(
    query: &str,
    hits: &[DocumentHit],
    config: &RerankerOptions,
    top_k: usize,
) -> Vec<DocumentHit> {
    let candidate_limit = config.candidate_pool_size.max(top_k);
    let query_tokens = tokenize(query);
    let normalized_query = normalize_text(query);
    let mut reranked = hits
        .iter()
        .take(candidate_limit)
        .cloned()
        .map(|mut hit| {
            let rerank_score = score_document_heuristic(&hit, &query_tokens, &normalized_query);
            hit.score = hit.score * 0.35 + rerank_score * 0.65;
            hit
        })
        .collect::<Vec<_>>();

    reranked.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(Ordering::Equal)
    });
    reranked.truncate(top_k);
    reranked
}

fn rerank_documents_with_embeddings(
    embedder: &mut Embedder,
    query: &str,
    hits: &[DocumentHit],
    config: &RerankerOptions,
    top_k: usize,
) -> Result<Vec<DocumentHit>> {
    let candidate_limit = config.candidate_pool_size.max(top_k);
    let query_tokens = tokenize(query);
    let normalized_query = normalize_text(query);
    let mut inputs = Vec::with_capacity(candidate_limit + 1);
    inputs.push(format_query_for_embedding(query));
    inputs.extend(hits.iter().take(candidate_limit).map(|hit| {
        format_document_for_reranking(
            &hit.relative_path,
            hit.title.as_deref(),
            &hit.best_match.heading_path,
            &hit.best_match.excerpt,
            &hit.metadata,
        )
    }));

    let mut embeddings = embedder.embed_texts(&inputs)?;
    if embeddings.len() <= 1 {
        return Ok(hits.iter().take(top_k).cloned().collect());
    }

    let query_embedding = embeddings.remove(0);
    let mut reranked = hits
        .iter()
        .take(candidate_limit)
        .cloned()
        .zip(embeddings.into_iter())
        .map(|(mut hit, document_embedding)| {
            let embedding_score = cosine_similarity(&query_embedding, &document_embedding).max(0.0);
            let heuristic_score = score_document_heuristic(&hit, &query_tokens, &normalized_query);
            let rerank_score = embedding_score * 0.8 + heuristic_score * 0.2;
            hit.score = hit.score * 0.2 + rerank_score * 0.8;
            hit
        })
        .collect::<Vec<_>>();

    reranked.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(Ordering::Equal)
    });
    reranked.truncate(top_k);
    Ok(reranked)
}

fn score_document_heuristic(
    hit: &DocumentHit,
    query_tokens: &[String],
    normalized_query: &str,
) -> f32 {
    let title = hit.title.as_deref().unwrap_or_default();
    let heading = hit.best_match.heading_path.join(" ");
    let title_norm = normalize_text(title);
    let path_norm = normalize_text(&hit.relative_path);
    let heading_norm = normalize_text(&heading);
    let excerpt_norm = normalize_text(&hit.best_match.excerpt);

    let title_coverage = score_token_coverage(query_tokens, &title_norm);
    let heading_coverage = score_token_coverage(query_tokens, &heading_norm);
    let excerpt_coverage = score_token_coverage(query_tokens, &excerpt_norm);
    let path_coverage = score_token_coverage(query_tokens, &path_norm);

    let phrase_bonus = [
        contains_phrase(&title_norm, normalized_query, 0.30),
        contains_phrase(&heading_norm, normalized_query, 0.20),
        contains_phrase(&excerpt_norm, normalized_query, 0.15),
        contains_phrase(&path_norm, normalized_query, 0.05),
    ]
    .into_iter()
    .sum::<f32>();

    title_coverage * 0.45
        + heading_coverage * 0.20
        + excerpt_coverage * 0.25
        + path_coverage * 0.10
        + phrase_bonus
}

fn score_token_coverage(query_tokens: &[String], haystack: &str) -> f32 {
    if query_tokens.is_empty() {
        return 0.0;
    }

    let matched = query_tokens
        .iter()
        .filter(|token| haystack.contains(token.as_str()))
        .count();
    matched as f32 / query_tokens.len() as f32
}

fn contains_phrase(haystack: &str, needle: &str, weight: f32) -> f32 {
    if needle.is_empty() || !haystack.contains(needle) {
        return 0.0;
    }
    weight
}

fn normalize_text(input: &str) -> String {
    input
        .chars()
        .map(|ch| if ch.is_alphanumeric() { ch } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn load_info(connection: &Connection) -> Result<ArtifactInfo> {
    let mut statement = connection.prepare("SELECT key, value FROM artifact_meta")?;
    let mut rows = statement.query([])?;
    let mut values = HashMap::new();
    while let Some(row) = rows.next()? {
        let key: String = row.get(0)?;
        let value: String = row.get(1)?;
        values.insert(key, value);
    }

    let schema_version = values
        .remove("schema_version")
        .ok_or(IndexbindError::MissingMetadata("schema_version"))?;
    let built_at = values
        .remove("built_at")
        .ok_or(IndexbindError::MissingMetadata("built_at"))?;
    let embedding_backend = serde_json::from_str(
        values
            .get("embedding_backend")
            .ok_or(IndexbindError::MissingMetadata("embedding_backend"))?,
    )?;
    let source_root = serde_json::from_str(
        values
            .get("source_root")
            .ok_or(IndexbindError::MissingMetadata("source_root"))?,
    )?;

    let document_count =
        connection.query_row("SELECT COUNT(*) FROM documents", [], |row| row.get(0))?;
    let chunk_count = connection.query_row("SELECT COUNT(*) FROM chunks", [], |row| row.get(0))?;

    Ok(ArtifactInfo {
        schema_version,
        built_at,
        embedding_backend,
        source_root,
        document_count,
        chunk_count,
    })
}

fn load_documents(connection: &Connection) -> Result<HashMap<String, StoredDocument>> {
    let mut statement = connection.prepare(
        "SELECT doc_id, source_root_id, original_path, relative_path, title, content_hash, modified_at, chunk_count, metadata_json FROM documents",
    )?;
    let documents = statement
        .query_map([], |row| {
            let metadata_json: String = row.get(8)?;
            Ok(StoredDocument {
                doc_id: row.get(0)?,
                source_root_id: row.get(1)?,
                original_path: row.get(2)?,
                relative_path: row.get(3)?,
                title: row.get(4)?,
                content_hash: row.get(5)?,
                modified_at: row.get(6)?,
                chunk_count: row.get::<_, i64>(7)? as usize,
                metadata: serde_json::from_str(&metadata_json).unwrap_or_default(),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(documents
        .into_iter()
        .map(|document| (document.doc_id.clone(), document))
        .collect())
}

fn load_chunks(connection: &Connection) -> Result<Vec<IndexedChunk>> {
    let mut statement = connection.prepare(
        "
        SELECT
            c.chunk_id,
            c.doc_id,
            c.ordinal,
            c.heading_path_json,
            c.char_start,
            c.char_end,
            c.token_count,
            c.chunk_text,
            c.excerpt,
            v.vector_blob
        FROM chunks c
        INNER JOIN chunk_vectors v ON v.chunk_id = c.chunk_id
        ",
    )?;
    let chunks = statement
        .query_map([], |row| {
            let heading_path_json: String = row.get(3)?;
            let vector_blob: Vec<u8> = row.get(9)?;
            Ok(IndexedChunk {
                chunk: StoredChunk {
                    chunk_id: row.get(0)?,
                    doc_id: row.get(1)?,
                    ordinal: row.get::<_, i64>(2)? as usize,
                    heading_path: serde_json::from_str(&heading_path_json).unwrap_or_default(),
                    char_start: row.get::<_, i64>(4)? as usize,
                    char_end: row.get::<_, i64>(5)? as usize,
                    token_count: row.get::<_, i64>(6)? as usize,
                    chunk_text: row.get(7)?,
                    excerpt: row.get(8)?,
                },
                embedding: bytes_to_vector(&vector_blob),
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(chunks)
}

fn tokenize(input: &str) -> Vec<String> {
    input
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|segment| !segment.is_empty())
        .map(|segment| segment.to_lowercase())
        .collect()
}

fn build_fts_query(input: &str) -> Option<String> {
    let tokens = tokenize(input);
    if tokens.is_empty() {
        return None;
    }
    Some(tokens.join(" OR "))
}

#[cfg(test)]
mod tests {
    use super::{
        rerank_documents_with_embeddings, rerank_documents_with_heuristic, BestMatch, DocumentHit,
        RerankerKind, RerankerOptions, Retriever, SearchOptions,
    };
    use crate::artifact::{build_artifact, BuildArtifactOptions};
    use crate::embedding::{Embedder, EmbeddingBackend};
    use crate::types::{NormalizedDocument, SourceRoot};
    use std::collections::BTreeMap;
    use tempfile::tempdir;

    #[test]
    fn returns_document_hits_and_reads_source_content() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("docs");
        std::fs::create_dir_all(&source).unwrap();
        let file = source.join("guide.md");
        std::fs::write(&file, "# Intro\nRust embeddings and retrieval.").unwrap();

        let artifact = dir.path().join("index.sqlite");
        build_artifact(
            &artifact,
            &[NormalizedDocument {
                original_path: file.display().to_string(),
                relative_path: "guide.md".to_string(),
                title: Some("Intro".to_string()),
                content: "# Intro\nRust embeddings and retrieval.".to_string(),
                metadata: BTreeMap::new(),
            }],
            &BuildArtifactOptions {
                source_root: SourceRoot {
                    id: "root".to_string(),
                    original_path: source.display().to_string(),
                },
                embedding_backend: EmbeddingBackend::Hashing { dimensions: 128 },
                chunking: Default::default(),
            },
        )
        .unwrap();

        let mut retriever = Retriever::open(&artifact, None).unwrap();
        let hits = retriever
            .search("rust retrieval", SearchOptions::default())
            .unwrap();

        assert_eq!(hits.len(), 1);
        assert!(hits[0].original_path.ends_with("guide.md"));
        let loaded = retriever.read_document(&hits[0]).unwrap();
        assert!(loaded.content.contains("Rust embeddings"));
    }

    #[test]
    fn filters_hits_by_path_prefix_and_metadata() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("docs");
        std::fs::create_dir_all(source.join("guides")).unwrap();
        std::fs::create_dir_all(source.join("notes")).unwrap();

        let guide = source.join("guides").join("rust.md");
        std::fs::write(&guide, "# Rust Guide\nDocument retrieval in Rust.").unwrap();
        let note = source.join("notes").join("python.md");
        std::fs::write(&note, "# Python Note\nDocument retrieval in Python.").unwrap();

        let artifact = dir.path().join("index.sqlite");
        let mut guide_metadata = BTreeMap::new();
        guide_metadata.insert("lang".to_string(), "rust".to_string());
        let mut note_metadata = BTreeMap::new();
        note_metadata.insert("lang".to_string(), "python".to_string());

        build_artifact(
            &artifact,
            &[
                NormalizedDocument {
                    original_path: guide.display().to_string(),
                    relative_path: "guides/rust.md".to_string(),
                    title: Some("Rust Guide".to_string()),
                    content: "# Rust Guide\nDocument retrieval in Rust.".to_string(),
                    metadata: guide_metadata.clone(),
                },
                NormalizedDocument {
                    original_path: note.display().to_string(),
                    relative_path: "notes/python.md".to_string(),
                    title: Some("Python Note".to_string()),
                    content: "# Python Note\nDocument retrieval in Python.".to_string(),
                    metadata: note_metadata,
                },
            ],
            &BuildArtifactOptions {
                source_root: SourceRoot {
                    id: "root".to_string(),
                    original_path: source.display().to_string(),
                },
                embedding_backend: EmbeddingBackend::Hashing { dimensions: 128 },
                chunking: Default::default(),
            },
        )
        .unwrap();

        let mut retriever = Retriever::open(&artifact, None).unwrap();
        let hits = retriever
            .search(
                "document retrieval",
                SearchOptions {
                    relative_path_prefix: Some("guides/".to_string()),
                    metadata: guide_metadata,
                    ..SearchOptions::default()
                },
            )
            .unwrap();

        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].relative_path, "guides/rust.md");
    }

    #[test]
    fn heuristic_reranker_prefers_title_and_heading_matches() {
        let hits = vec![
            DocumentHit {
                doc_id: "doc-1".to_string(),
                original_path: "/tmp/guides/rust.md".to_string(),
                relative_path: "guides/rust.md".to_string(),
                title: Some("Rust Guide".to_string()),
                score: 0.05,
                best_match: BestMatch {
                    chunk_id: 1,
                    excerpt: "rust guide quickstart and installation".to_string(),
                    heading_path: vec!["Rust Guide".to_string()],
                    char_start: 0,
                    char_end: 10,
                    score: 0.05,
                },
                metadata: BTreeMap::new(),
            },
            DocumentHit {
                doc_id: "doc-2".to_string(),
                original_path: "/tmp/notes/setup.md".to_string(),
                relative_path: "notes/setup.md".to_string(),
                title: Some("Setup Notes".to_string()),
                score: 0.08,
                best_match: BestMatch {
                    chunk_id: 2,
                    excerpt: "rust guide quickstart walkthrough".to_string(),
                    heading_path: vec!["Reference".to_string()],
                    char_start: 0,
                    char_end: 10,
                    score: 0.08,
                },
                metadata: BTreeMap::new(),
            },
        ];

        let reranked = rerank_documents_with_heuristic(
            "rust guide",
            &hits,
            &RerankerOptions {
                kind: RerankerKind::HeuristicV1,
                candidate_pool_size: 10,
            },
            2,
        );

        assert_eq!(reranked[0].doc_id, "doc-1");
    }

    #[test]
    fn embedding_reranker_prefers_document_level_match() {
        let hits = vec![
            DocumentHit {
                doc_id: "doc-1".to_string(),
                original_path: "/tmp/guides/rust.md".to_string(),
                relative_path: "guides/rust.md".to_string(),
                title: Some("Rust Guide".to_string()),
                score: 0.05,
                best_match: BestMatch {
                    chunk_id: 1,
                    excerpt: "installation and setup".to_string(),
                    heading_path: vec!["Quickstart".to_string()],
                    char_start: 0,
                    char_end: 10,
                    score: 0.05,
                },
                metadata: BTreeMap::new(),
            },
            DocumentHit {
                doc_id: "doc-2".to_string(),
                original_path: "/tmp/notes/network.md".to_string(),
                relative_path: "notes/network.md".to_string(),
                title: Some("Network Notes".to_string()),
                score: 0.08,
                best_match: BestMatch {
                    chunk_id: 2,
                    excerpt: "latency budgeting and packet traces".to_string(),
                    heading_path: vec!["Operations".to_string()],
                    char_start: 0,
                    char_end: 10,
                    score: 0.08,
                },
                metadata: BTreeMap::new(),
            },
        ];

        let mut embedder = Embedder::new(EmbeddingBackend::Hashing { dimensions: 2048 }).unwrap();
        let reranked = rerank_documents_with_embeddings(
            &mut embedder,
            "rust guide",
            &hits,
            &RerankerOptions::default(),
            2,
        )
        .unwrap();

        assert_eq!(reranked[0].doc_id, "doc-1");
    }
}
