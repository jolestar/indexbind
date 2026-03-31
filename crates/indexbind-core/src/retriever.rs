use crate::embedding::{
    bytes_to_vector, cosine_similarity, format_document_for_reranking, format_query_for_embedding,
    Embedder, EmbeddingBackend,
};
use crate::lexical::{normalize_for_heuristic, tokenize};
use crate::types::{BestMatch, DocumentHit, MetadataMap, SourceRoot, StoredChunk, StoredDocument};
use crate::{IndexbindError, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactInfo {
    pub schema_version: String,
    pub built_at: String,
    pub embedding_backend: EmbeddingBackend,
    pub lexical_tokenizer: String,
    pub source_root: SourceRoot,
    pub document_count: usize,
    pub chunk_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOptions {
    pub top_k: usize,
    #[serde(default)]
    pub mode: RetrievalMode,
    pub candidate_multiplier: usize,
    #[serde(default)]
    pub min_score: Option<f32>,
    #[serde(default)]
    pub reranker: Option<RerankerOptions>,
    #[serde(default)]
    pub relative_path_prefix: Option<String>,
    #[serde(default)]
    pub metadata: MetadataMap,
    #[serde(default)]
    pub score_adjustment: Option<ScoreAdjustmentOptions>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RetrievalMode {
    #[default]
    Hybrid,
    Vector,
    Lexical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ModeProfile {
    #[default]
    Hybrid,
    Lexical,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RetrieverOpenOptions {
    #[serde(default)]
    pub mode_profile: ModeProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScoreAdjustmentOptions {
    #[serde(default)]
    pub metadata_numeric_multiplier: Option<String>,
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
            mode: RetrievalMode::Hybrid,
            candidate_multiplier: 8,
            min_score: None,
            reranker: None,
            relative_path_prefix: None,
            metadata: BTreeMap::new(),
            score_adjustment: None,
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
    embedder: Option<Embedder>,
    mode_profile: ModeProfile,
}

impl Retriever {
    pub fn open(path: &Path) -> Result<Self> {
        Self::open_with_options(path, RetrieverOpenOptions::default())
    }

    pub fn open_with_options(path: &Path, options: RetrieverOpenOptions) -> Result<Self> {
        let connection = Connection::open(path)?;
        let info = load_info(&connection)?;
        let documents = load_documents(&connection)?;
        let chunks = load_chunks(&connection)?;
        let chunks_by_id = chunks
            .iter()
            .map(|entry| (entry.chunk.chunk_id, entry.chunk.clone()))
            .collect::<HashMap<_, _>>();
        let embedder = if options.mode_profile == ModeProfile::Hybrid {
            Some(Embedder::new(info.embedding_backend.clone())?)
        } else {
            None
        };

        Ok(Self {
            connection,
            info,
            documents,
            chunks,
            chunks_by_id,
            embedder,
            mode_profile: options.mode_profile,
        })
    }

    pub fn info(&self) -> &ArtifactInfo {
        &self.info
    }

    pub fn search(&mut self, query: &str, options: SearchOptions) -> Result<Vec<DocumentHit>> {
        self.ensure_mode_supported(options.mode)?;
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
        let final_candidate_limit = if options.score_adjustment.is_some() {
            limit
        } else {
            rerank_candidate_limit
        };
        let vector_docs = match options.mode {
            RetrievalMode::Hybrid | RetrievalMode::Vector => {
                let formatted_query = format_query_for_embedding(query);
                let query_embedding = self
                    .embedder_mut()?
                    .embed_texts(&[formatted_query])?
                    .into_iter()
                    .next()
                    .unwrap_or_default();
                self.rank_documents_by_vector(&query_embedding, limit, &allowed_doc_ids)
            }
            RetrievalMode::Lexical => Vec::new(),
        };
        let lexical_docs = match options.mode {
            RetrievalMode::Hybrid | RetrievalMode::Lexical => {
                self.rank_documents_by_lexical(query, limit, &allowed_doc_ids)?
            }
            RetrievalMode::Vector => Vec::new(),
        };
        let fused_hits = fuse_documents(&self.documents, &vector_docs, &lexical_docs, limit);
        let reranked = self.rerank_documents(
            query,
            &fused_hits,
            options.reranker.as_ref(),
            final_candidate_limit,
        )?;
        Ok(finalize_hits(
            reranked,
            options.score_adjustment.as_ref(),
            options.min_score,
            options.top_k,
        ))
    }
}

fn finalize_hits(
    mut hits: Vec<DocumentHit>,
    config: Option<&ScoreAdjustmentOptions>,
    min_score: Option<f32>,
    top_k: usize,
) -> Vec<DocumentHit> {
    if let Some(field) = config.and_then(|value| value.metadata_numeric_multiplier.as_deref()) {
        for hit in &mut hits {
            let multiplier = hit
                .metadata
                .get(field)
                .and_then(Value::as_f64)
                .filter(|value| value.is_finite() && *value > 0.0)
                .unwrap_or(1.0) as f32;
            hit.score *= multiplier;
        }
    }

    if let Some(min_score) = min_score.filter(|value| value.is_finite()) {
        hits.retain(|hit| hit.score >= min_score);
    }
    hits.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(Ordering::Equal)
    });
    hits.truncate(top_k);
    hits
}

impl Retriever {
    fn ensure_mode_supported(&self, mode: RetrievalMode) -> Result<()> {
        if self.mode_profile == ModeProfile::Lexical && mode != RetrievalMode::Lexical {
            return Err(IndexbindError::InvalidSearchConfig(format!(
                "this index was opened with modeProfile: \"lexical\"; mode \"{}\" is unavailable. Re-open with modeProfile: \"hybrid\"",
                mode.as_str()
            )));
        }
        Ok(())
    }

    fn embedder_mut(&mut self) -> Result<&mut Embedder> {
        self.embedder.as_mut().ok_or_else(|| {
            IndexbindError::InvalidSearchConfig(
                "embedding resources are unavailable for this index instance".to_string(),
            )
        })
    }

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
                let embedder = self.embedder_mut()?;
                rerank_documents_with_embeddings(embedder, query, hits, config, top_k)
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

    options.metadata.iter().all(|(key, value)| {
        document
            .metadata
            .get(key)
            .is_some_and(|candidate| metadata_matches(candidate, value))
    })
}

fn metadata_matches(candidate: &Value, filter: &Value) -> bool {
    candidate.is_boolean() == filter.is_boolean()
        && candidate.is_number() == filter.is_number()
        && candidate.is_string() == filter.is_string()
        && candidate == filter
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
                relative_path: document.relative_path.clone(),
                canonical_url: document.canonical_url.clone(),
                title: document.title.clone(),
                summary: document.summary.clone(),
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
    let normalized_query = normalize_for_heuristic(query);
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
    let normalized_query = normalize_for_heuristic(query);
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

impl RetrievalMode {
    fn as_str(&self) -> &'static str {
        match self {
            RetrievalMode::Hybrid => "hybrid",
            RetrievalMode::Vector => "vector",
            RetrievalMode::Lexical => "lexical",
        }
    }
}

fn score_document_heuristic(
    hit: &DocumentHit,
    query_tokens: &[String],
    normalized_query: &str,
) -> f32 {
    let title = hit.title.as_deref().unwrap_or_default();
    let heading = hit.best_match.heading_path.join(" ");
    let title_norm = normalize_for_heuristic(title);
    let path_norm = normalize_for_heuristic(&hit.relative_path);
    let heading_norm = normalize_for_heuristic(&heading);
    let excerpt_norm = normalize_for_heuristic(&hit.best_match.excerpt);

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
    let lexical_tokenizer = values
        .get("lexical_tokenizer")
        .cloned()
        .unwrap_or_else(|| "alnum-lower-v1".to_string());
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
        lexical_tokenizer,
        source_root,
        document_count,
        chunk_count,
    })
}

fn load_documents(connection: &Connection) -> Result<HashMap<String, StoredDocument>> {
    let mut statement = connection.prepare(
        "SELECT doc_id, source_root_id, source_path, relative_path, canonical_url, title, summary, content_hash, modified_at, chunk_count, metadata_json FROM documents",
    )?;
    let documents = statement
        .query_map([], |row| {
            let metadata_json: String = row.get(10)?;
            Ok(StoredDocument {
                doc_id: row.get(0)?,
                source_root_id: row.get(1)?,
                source_path: row.get(2)?,
                relative_path: row.get(3)?,
                canonical_url: row.get(4)?,
                title: row.get(5)?,
                summary: row.get(6)?,
                content_hash: row.get(7)?,
                modified_at: row.get(8)?,
                chunk_count: row.get::<_, i64>(9)? as usize,
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
        finalize_hits, rerank_documents_with_embeddings, rerank_documents_with_heuristic,
        BestMatch, DocumentHit, ModeProfile, RerankerKind, RerankerOptions, RetrievalMode,
        Retriever, RetrieverOpenOptions, ScoreAdjustmentOptions, SearchOptions,
    };
    use crate::artifact::build_artifact;
    use crate::build::BuildArtifactOptions;
    use crate::embedding::{Embedder, EmbeddingBackend};
    use crate::types::{NormalizedDocument, SourceRoot};
    use crate::LEXICAL_TOKENIZER_VERSION;
    use serde_json::Value;
    use std::collections::BTreeMap;
    use tempfile::tempdir;

    #[test]
    fn returns_document_hits_with_runtime_neutral_fields() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("docs");
        std::fs::create_dir_all(&source).unwrap();
        let file = source.join("guide.md");
        std::fs::write(&file, "# Intro\nRust embeddings and retrieval.").unwrap();

        let artifact = dir.path().join("index.sqlite");
        build_artifact(
            &artifact,
            &[NormalizedDocument {
                doc_id: None,
                source_path: Some(file.display().to_string()),
                relative_path: "guide.md".to_string(),
                canonical_url: Some("/docs/guide".to_string()),
                title: Some("Intro".to_string()),
                summary: Some("Rust embeddings and retrieval".to_string()),
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

        let mut retriever = Retriever::open(&artifact).unwrap();
        let hits = retriever
            .search("rust retrieval", SearchOptions::default())
            .unwrap();

        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].relative_path, "guide.md");
        assert_eq!(hits[0].canonical_url.as_deref(), Some("/docs/guide"));
        assert_eq!(
            hits[0].summary.as_deref(),
            Some("Rust embeddings and retrieval")
        );
    }

    #[test]
    fn chinese_lexical_queries_match_expected_document() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("docs");
        std::fs::create_dir_all(&source).unwrap();

        let artifact = dir.path().join("index.sqlite");
        build_artifact(
            &artifact,
            &[
                NormalizedDocument {
                    doc_id: Some("modular".to_string()),
                    source_path: None,
                    relative_path: "modular.md".to_string(),
                    canonical_url: None,
                    title: Some("模块化区块链".to_string()),
                    summary: Some("模块化区块链与调用层设计".to_string()),
                    content: "# 模块化区块链\n调用层与模块化区块链架构。".to_string(),
                    metadata: BTreeMap::new(),
                },
                NormalizedDocument {
                    doc_id: Some("other".to_string()),
                    source_path: None,
                    relative_path: "other.md".to_string(),
                    canonical_url: None,
                    title: Some("存档".to_string()),
                    summary: Some("运维归档".to_string()),
                    content: "# 存档\n基础设施与归档材料。".to_string(),
                    metadata: BTreeMap::new(),
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

        let mut retriever = Retriever::open(&artifact).unwrap();
        assert_eq!(
            retriever.info().lexical_tokenizer,
            LEXICAL_TOKENIZER_VERSION
        );

        let hits = retriever
            .search("模块化区块链", SearchOptions::default())
            .unwrap();
        assert!(!hits.is_empty());
        assert_eq!(hits[0].doc_id, "modular");

        let mixed_hits = retriever
            .search("调用层 Layer2", SearchOptions::default())
            .unwrap();
        assert!(!mixed_hits.is_empty());
        assert_eq!(mixed_hits[0].doc_id, "modular");
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
        guide_metadata.insert("lang".to_string(), Value::String("rust".to_string()));
        let mut note_metadata = BTreeMap::new();
        note_metadata.insert("lang".to_string(), Value::String("python".to_string()));

        build_artifact(
            &artifact,
            &[
                NormalizedDocument {
                    doc_id: None,
                    source_path: Some(guide.display().to_string()),
                    relative_path: "guides/rust.md".to_string(),
                    canonical_url: Some("/guides/rust".to_string()),
                    title: Some("Rust Guide".to_string()),
                    summary: None,
                    content: "# Rust Guide\nDocument retrieval in Rust.".to_string(),
                    metadata: guide_metadata.clone(),
                },
                NormalizedDocument {
                    doc_id: None,
                    source_path: Some(note.display().to_string()),
                    relative_path: "notes/python.md".to_string(),
                    canonical_url: Some("/notes/python".to_string()),
                    title: Some("Python Note".to_string()),
                    summary: None,
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

        let mut retriever = Retriever::open(&artifact).unwrap();
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
                relative_path: "guides/rust.md".to_string(),
                canonical_url: Some("/guides/rust".to_string()),
                title: Some("Rust Guide".to_string()),
                summary: None,
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
                relative_path: "notes/setup.md".to_string(),
                canonical_url: Some("/notes/setup".to_string()),
                title: Some("Setup Notes".to_string()),
                summary: None,
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
                relative_path: "guides/rust.md".to_string(),
                canonical_url: Some("/guides/rust".to_string()),
                title: Some("Rust Guide".to_string()),
                summary: None,
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
                relative_path: "notes/network.md".to_string(),
                canonical_url: Some("/notes/network".to_string()),
                title: Some("Network Notes".to_string()),
                summary: None,
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

    #[test]
    fn metadata_numeric_multiplier_reorders_final_hits() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("docs");
        std::fs::create_dir_all(&source).unwrap();

        let artifact = dir.path().join("index.sqlite");
        let mut low_weight = BTreeMap::new();
        low_weight.insert("directory_weight".to_string(), Value::from(0.5));
        let mut high_weight = BTreeMap::new();
        high_weight.insert("directory_weight".to_string(), Value::from(2.0));

        build_artifact(
            &artifact,
            &[
                NormalizedDocument {
                    doc_id: Some("doc-low".to_string()),
                    source_path: None,
                    relative_path: "low.md".to_string(),
                    canonical_url: None,
                    title: Some("Calling Layer Overview".to_string()),
                    summary: None,
                    content: "Calling layer design for agents.".to_string(),
                    metadata: low_weight,
                },
                NormalizedDocument {
                    doc_id: Some("doc-high".to_string()),
                    source_path: None,
                    relative_path: "high.md".to_string(),
                    canonical_url: None,
                    title: Some("Calling Layer Notes".to_string()),
                    summary: None,
                    content: "Calling layer notes for agents.".to_string(),
                    metadata: high_weight,
                },
            ],
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

        let mut retriever = Retriever::open(&artifact).unwrap();
        let hits = retriever
            .search(
                "calling layer",
                SearchOptions {
                    mode: RetrievalMode::Hybrid,
                    reranker: Some(RerankerOptions {
                        kind: RerankerKind::HeuristicV1,
                        candidate_pool_size: 10,
                    }),
                    score_adjustment: Some(ScoreAdjustmentOptions {
                        metadata_numeric_multiplier: Some("directory_weight".to_string()),
                    }),
                    ..SearchOptions::default()
                },
            )
            .unwrap();

        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].doc_id, "doc-high");
    }

    #[test]
    fn metadata_numeric_multiplier_can_promote_hits_without_reranker() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("docs");
        std::fs::create_dir_all(&source).unwrap();

        let artifact = dir.path().join("index.sqlite");
        let mut low_weight = BTreeMap::new();
        low_weight.insert("directory_weight".to_string(), Value::from(0.5));
        let mut high_weight = BTreeMap::new();
        high_weight.insert("directory_weight".to_string(), Value::from(2.0));

        build_artifact(
            &artifact,
            &[
                NormalizedDocument {
                    doc_id: Some("doc-low".to_string()),
                    source_path: None,
                    relative_path: "low.md".to_string(),
                    canonical_url: None,
                    title: Some("Calling Layer Overview".to_string()),
                    summary: None,
                    content: "Calling layer design for agents.".to_string(),
                    metadata: low_weight,
                },
                NormalizedDocument {
                    doc_id: Some("doc-high".to_string()),
                    source_path: None,
                    relative_path: "high.md".to_string(),
                    canonical_url: None,
                    title: Some("Calling Layer Notes".to_string()),
                    summary: None,
                    content: "Calling layer notes for agents.".to_string(),
                    metadata: high_weight,
                },
            ],
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

        let mut retriever = Retriever::open(&artifact).unwrap();
        let hits = retriever
            .search(
                "calling layer",
                SearchOptions {
                    top_k: 1,
                    candidate_multiplier: 8,
                    mode: RetrievalMode::Hybrid,
                    score_adjustment: Some(ScoreAdjustmentOptions {
                        metadata_numeric_multiplier: Some("directory_weight".to_string()),
                    }),
                    ..SearchOptions::default()
                },
            )
            .unwrap();

        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].doc_id, "doc-high");
    }

    #[test]
    fn min_score_can_return_fewer_than_top_k_hits() {
        let hits = vec![
            DocumentHit {
                doc_id: "doc-strong".to_string(),
                relative_path: "strong.md".to_string(),
                canonical_url: None,
                title: Some("Strong".to_string()),
                summary: None,
                score: 0.18,
                best_match: BestMatch {
                    chunk_id: 1,
                    excerpt: "strong match".to_string(),
                    heading_path: Vec::new(),
                    char_start: 0,
                    char_end: 12,
                    score: 0.18,
                },
                metadata: BTreeMap::new(),
            },
            DocumentHit {
                doc_id: "doc-weak".to_string(),
                relative_path: "weak.md".to_string(),
                canonical_url: None,
                title: Some("Weak".to_string()),
                summary: None,
                score: 0.04,
                best_match: BestMatch {
                    chunk_id: 2,
                    excerpt: "weak match".to_string(),
                    heading_path: Vec::new(),
                    char_start: 0,
                    char_end: 10,
                    score: 0.04,
                },
                metadata: BTreeMap::new(),
            },
        ];

        let finalized = finalize_hits(hits, None, Some(0.05), 10);

        assert_eq!(finalized.len(), 1);
        assert_eq!(finalized[0].doc_id, "doc-strong");
    }

    #[test]
    fn min_score_is_applied_after_score_adjustment() {
        let mut promoted_metadata = BTreeMap::new();
        promoted_metadata.insert("directory_weight".to_string(), Value::from(2.0));

        let mut neutral_metadata = BTreeMap::new();
        neutral_metadata.insert("directory_weight".to_string(), Value::from(1.0));

        let hits = vec![
            DocumentHit {
                doc_id: "doc-promoted".to_string(),
                relative_path: "promoted.md".to_string(),
                canonical_url: None,
                title: Some("Promoted".to_string()),
                summary: None,
                score: 0.08,
                best_match: BestMatch {
                    chunk_id: 1,
                    excerpt: "promoted match".to_string(),
                    heading_path: Vec::new(),
                    char_start: 0,
                    char_end: 14,
                    score: 0.08,
                },
                metadata: promoted_metadata,
            },
            DocumentHit {
                doc_id: "doc-cut".to_string(),
                relative_path: "cut.md".to_string(),
                canonical_url: None,
                title: Some("Cut".to_string()),
                summary: None,
                score: 0.09,
                best_match: BestMatch {
                    chunk_id: 2,
                    excerpt: "cut match".to_string(),
                    heading_path: Vec::new(),
                    char_start: 0,
                    char_end: 9,
                    score: 0.09,
                },
                metadata: neutral_metadata,
            },
        ];

        let finalized = finalize_hits(
            hits,
            Some(&ScoreAdjustmentOptions {
                metadata_numeric_multiplier: Some("directory_weight".to_string()),
            }),
            Some(0.1),
            10,
        );

        assert_eq!(finalized.len(), 1);
        assert_eq!(finalized[0].doc_id, "doc-promoted");
        assert!(finalized[0].score >= 0.1);
    }

    #[test]
    fn lexical_mode_returns_lexical_matches() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("docs");
        std::fs::create_dir_all(&source).unwrap();

        let artifact = dir.path().join("index.sqlite");
        build_artifact(
            &artifact,
            &[
                NormalizedDocument {
                    doc_id: Some("rust-guide".to_string()),
                    source_path: None,
                    relative_path: "rust.md".to_string(),
                    canonical_url: None,
                    title: Some("Rust Guide".to_string()),
                    summary: None,
                    content: "# Rust Guide\nRust guide for local search.".to_string(),
                    metadata: BTreeMap::new(),
                },
                NormalizedDocument {
                    doc_id: Some("python-guide".to_string()),
                    source_path: None,
                    relative_path: "python.md".to_string(),
                    canonical_url: None,
                    title: Some("Python Guide".to_string()),
                    summary: None,
                    content: "# Python Guide\nPython notes for data tooling.".to_string(),
                    metadata: BTreeMap::new(),
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

        let mut retriever = Retriever::open(&artifact).unwrap();
        let hits = retriever
            .search(
                "rust guide",
                SearchOptions {
                    mode: RetrievalMode::Lexical,
                    ..SearchOptions::default()
                },
            )
            .unwrap();

        assert!(!hits.is_empty());
        assert_eq!(hits[0].doc_id, "rust-guide");
        assert!(hits.iter().all(|hit| hit.relative_path != "python.md"));
    }

    #[test]
    fn lexical_mode_profile_rejects_vector_search() {
        let dir = tempdir().unwrap();
        let source = dir.path().join("docs");
        std::fs::create_dir_all(&source).unwrap();

        let artifact = dir.path().join("index.sqlite");
        build_artifact(
            &artifact,
            &[NormalizedDocument {
                doc_id: Some("rust-guide".to_string()),
                source_path: None,
                relative_path: "rust.md".to_string(),
                canonical_url: None,
                title: Some("Rust Guide".to_string()),
                summary: None,
                content: "# Rust Guide\nRust guide for local search.".to_string(),
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

        let mut retriever = Retriever::open_with_options(
            &artifact,
            RetrieverOpenOptions {
                mode_profile: ModeProfile::Lexical,
            },
        )
        .unwrap();

        let hits = retriever
            .search(
                "rust guide",
                SearchOptions {
                    mode: RetrievalMode::Lexical,
                    ..SearchOptions::default()
                },
            )
            .unwrap();
        assert_eq!(hits[0].doc_id, "rust-guide");

        let error = retriever
            .search(
                "rust guide",
                SearchOptions {
                    mode: RetrievalMode::Vector,
                    ..SearchOptions::default()
                },
            )
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("this index was opened with modeProfile: \"lexical\""));
    }
}
