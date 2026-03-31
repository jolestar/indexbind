use indexbind_core::{lexical_tokenize, normalize_for_heuristic as core_normalize_for_heuristic};
use model2vec_rs::model::StaticModel;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use wasm_bindgen::prelude::*;

type MetadataMap = BTreeMap<String, Value>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    schema_version: String,
    artifact_format: String,
    built_at: String,
    embedding_backend: Value,
    document_count: usize,
    chunk_count: usize,
    vector_dimensions: usize,
    chunking: Value,
    #[serde(default)]
    files: ManifestFiles,
    features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ManifestFiles {
    model: Option<ModelFiles>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelFiles {
    tokenizer: String,
    config: String,
    weights: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum EmbeddingBackend {
    Model2Vec { model: String, batch_size: usize },
    Hashing { dimensions: usize },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DocumentRecord {
    doc_id: String,
    relative_path: String,
    canonical_url: Option<String>,
    title: Option<String>,
    summary: Option<String>,
    #[serde(default)]
    metadata: MetadataMap,
    first_chunk_index: usize,
    chunk_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChunkRecord {
    chunk_id: f64,
    doc_id: String,
    ordinal: usize,
    heading_path: Vec<String>,
    char_start: usize,
    char_end: usize,
    token_count: usize,
    excerpt: String,
    chunk_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Postings {
    tokenizer: String,
    avg_chunk_length: f32,
    document_frequency: BTreeMap<String, usize>,
    postings: BTreeMap<String, Vec<Posting>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Posting {
    chunk_index: usize,
    term_frequency: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct SearchOptions {
    top_k: Option<usize>,
    mode: Option<String>,
    min_score: Option<f32>,
    reranker: Option<RerankerOptions>,
    relative_path_prefix: Option<String>,
    #[serde(default)]
    metadata: MetadataMap,
    #[serde(default)]
    score_adjustment: Option<ScoreAdjustmentOptions>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct RerankerOptions {
    kind: Option<String>,
    candidate_pool_size: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ScoreAdjustmentOptions {
    metadata_numeric_multiplier: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BestMatch {
    chunk_id: f64,
    excerpt: String,
    heading_path: Vec<String>,
    char_start: usize,
    char_end: usize,
    score: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DocumentHit {
    doc_id: String,
    relative_path: String,
    canonical_url: Option<String>,
    title: Option<String>,
    summary: Option<String>,
    #[serde(default)]
    metadata: MetadataMap,
    score: f32,
    best_match: BestMatch,
}

#[derive(Debug, Clone)]
struct RankedDocument {
    doc_id: String,
    score: f32,
    best_match: BestMatch,
}

#[wasm_bindgen]
pub struct WasmIndex {
    manifest: Manifest,
    documents: Vec<DocumentRecord>,
    documents_by_id: HashMap<String, DocumentRecord>,
    chunks: Vec<ChunkRecord>,
    vectors: Vec<Vec<f32>>,
    postings: Postings,
    model2vec: Option<StaticModel>,
}

#[wasm_bindgen]
impl WasmIndex {
    #[wasm_bindgen(constructor)]
    pub fn new(
        manifest: JsValue,
        documents: JsValue,
        chunks: JsValue,
        vectors: Vec<u8>,
        postings: JsValue,
        tokenizer_bytes: Option<Vec<u8>>,
        model_bytes: Option<Vec<u8>>,
        config_bytes: Option<Vec<u8>>,
    ) -> Result<WasmIndex, JsValue> {
        let manifest: Manifest = serde_wasm_bindgen::from_value(manifest).map_err(to_js_error)?;
        let documents: Vec<DocumentRecord> =
            serde_wasm_bindgen::from_value(documents).map_err(to_js_error)?;
        let chunks: Vec<ChunkRecord> =
            serde_wasm_bindgen::from_value(chunks).map_err(to_js_error)?;
        let postings: Postings = serde_wasm_bindgen::from_value(postings).map_err(to_js_error)?;
        let decoded_vectors = decode_vectors(&vectors, chunks.len(), manifest.vector_dimensions)
            .map_err(to_js_error)?;
        let documents_by_id = documents
            .iter()
            .cloned()
            .map(|document| (document.doc_id.clone(), document))
            .collect();
        let embedding_backend: EmbeddingBackend =
            serde_json::from_value(manifest.embedding_backend.clone()).map_err(to_js_error)?;
        let model2vec = match embedding_backend {
            EmbeddingBackend::Model2Vec { .. } => {
                let tokenizer =
                    tokenizer_bytes.ok_or_else(|| to_js_error("missing model tokenizer bytes"))?;
                let model =
                    model_bytes.ok_or_else(|| to_js_error("missing model weights bytes"))?;
                let config =
                    config_bytes.ok_or_else(|| to_js_error("missing model config bytes"))?;
                Some(StaticModel::from_bytes(tokenizer, model, config, None).map_err(to_js_error)?)
            }
            EmbeddingBackend::Hashing { .. } => None,
        };
        Ok(Self {
            manifest,
            documents,
            documents_by_id,
            chunks,
            vectors: decoded_vectors,
            postings,
            model2vec,
        })
    }

    pub fn info(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.manifest).map_err(to_js_error)
    }

    pub fn search(&self, query: String, options: JsValue) -> Result<JsValue, JsValue> {
        let options: SearchOptions = if options.is_undefined() || options.is_null() {
            SearchOptions::default()
        } else {
            serde_wasm_bindgen::from_value(options).map_err(to_js_error)?
        };
        let top_k = options.top_k.unwrap_or(10);
        let allowed_doc_ids = self.allowed_doc_ids(&options);
        if allowed_doc_ids.is_empty() {
            return serde_wasm_bindgen::to_value(&Vec::<DocumentHit>::new()).map_err(to_js_error);
        }

        let rerank_candidate_limit = options
            .reranker
            .as_ref()
            .and_then(|value| value.candidate_pool_size)
            .unwrap_or(top_k)
            .max(top_k);
        let limit = (top_k * 8).max(rerank_candidate_limit).max(top_k);
        let mode = retrieval_mode(&options)?;
        let vector_docs = match mode {
            "hybrid" | "vector" => {
                let query_embedding = self.embed_query(&query)?;
                self.rank_documents_by_vector(&query_embedding, limit, &allowed_doc_ids)
            }
            "lexical" => Vec::new(),
            _ => unreachable!(),
        };
        let lexical_docs = match mode {
            "hybrid" | "lexical" => self.rank_documents_by_lexical(&query, limit, &allowed_doc_ids),
            "vector" => Vec::new(),
            _ => unreachable!(),
        };
        let fused = self.fuse_documents(&vector_docs, &lexical_docs, limit);
        let reranked = self.rerank_documents(
            &query,
            fused,
            options.reranker.as_ref(),
            rerank_candidate_limit,
        )?;
        let adjusted = finalize_hits(
            reranked,
            options.score_adjustment.as_ref(),
            options.min_score,
            top_k,
        );
        serde_wasm_bindgen::to_value(&adjusted).map_err(to_js_error)
    }
}

fn retrieval_mode(options: &SearchOptions) -> Result<&str, JsValue> {
    match options.mode.as_deref() {
        Some("hybrid") | None => Ok("hybrid"),
        Some("vector") => Ok("vector"),
        Some("lexical") => Ok("lexical"),
        Some(other) => Err(to_js_error(format!("unsupported retrieval mode: {other}"))),
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
    hits.sort_by(|left, right| right.score.partial_cmp(&left.score).unwrap());
    hits.truncate(top_k);
    hits
}

impl WasmIndex {
    fn allowed_doc_ids(&self, options: &SearchOptions) -> HashSet<String> {
        self.documents
            .iter()
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
            .zip(self.vectors.iter())
            .filter(|(chunk, _)| allowed_doc_ids.contains(&chunk.doc_id))
            .map(|(chunk, vector)| (cosine_similarity(query_embedding, vector), chunk))
            .filter(|(score, _)| *score > 0.0)
            .collect::<Vec<_>>();
        chunk_scores.sort_by(|left, right| right.0.partial_cmp(&left.0).unwrap());
        aggregate_ranked_documents(chunk_scores.into_iter().take(limit * 2), limit)
    }

    fn rank_documents_by_lexical(
        &self,
        query: &str,
        limit: usize,
        allowed_doc_ids: &HashSet<String>,
    ) -> Vec<RankedDocument> {
        let tokens = tokenize(query)
            .into_iter()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        if tokens.is_empty() {
            return Vec::new();
        }

        let mut scored_chunks: HashMap<usize, f32> = HashMap::new();
        let chunk_count = self.chunks.len() as f32;
        let avg_chunk_length = self.postings.avg_chunk_length.max(1.0);
        let k1 = 1.2_f32;
        let b = 0.75_f32;

        for token in tokens {
            let Some(postings) = self.postings.postings.get(&token) else {
                continue;
            };
            let document_frequency = *self
                .postings
                .document_frequency
                .get(&token)
                .unwrap_or(&postings.len());
            let idf = (1.0
                + (chunk_count - document_frequency as f32 + 0.5)
                    / (document_frequency as f32 + 0.5))
                .ln();
            for posting in postings {
                let chunk = &self.chunks[posting.chunk_index];
                if !allowed_doc_ids.contains(&chunk.doc_id) {
                    continue;
                }
                let chunk_length = chunk.token_count.max(1) as f32;
                let tf = posting.term_frequency as f32;
                let numerator = tf * (k1 + 1.0);
                let denominator = tf + k1 * (1.0 - b + b * (chunk_length / avg_chunk_length));
                let score = idf * (numerator / denominator);
                *scored_chunks.entry(posting.chunk_index).or_default() += score;
            }
        }

        let mut chunk_scores = scored_chunks
            .into_iter()
            .map(|(chunk_index, score)| (score, &self.chunks[chunk_index]))
            .collect::<Vec<_>>();
        chunk_scores.sort_by(|left, right| right.0.partial_cmp(&left.0).unwrap());
        aggregate_ranked_documents(chunk_scores.into_iter().take(limit * 2), limit)
    }

    fn fuse_documents(
        &self,
        vector_docs: &[RankedDocument],
        lexical_docs: &[RankedDocument],
        top_k: usize,
    ) -> Vec<DocumentHit> {
        const RRF_K: f32 = 60.0;
        let mut fused: HashMap<String, (f32, Option<BestMatch>, Option<BestMatch>)> =
            HashMap::new();

        for (rank, entry) in vector_docs.iter().enumerate() {
            let score = 1.0 / (RRF_K + rank as f32 + 1.0);
            let value = fused
                .entry(entry.doc_id.clone())
                .or_insert((0.0, None, None));
            value.0 += score;
            value.1 = Some(entry.best_match.clone());
        }

        for (rank, entry) in lexical_docs.iter().enumerate() {
            let score = 1.0 / (RRF_K + rank as f32 + 1.0);
            let value = fused
                .entry(entry.doc_id.clone())
                .or_insert((0.0, None, None));
            value.0 += score;
            value.2 = Some(entry.best_match.clone());
        }

        let mut hits = fused
            .into_iter()
            .filter_map(|(doc_id, (score, vector_best, lexical_best))| {
                let document = self.documents_by_id.get(&doc_id)?;
                Some(DocumentHit {
                    doc_id: document.doc_id.clone(),
                    relative_path: document.relative_path.clone(),
                    canonical_url: document.canonical_url.clone(),
                    title: document.title.clone(),
                    summary: document.summary.clone(),
                    metadata: document.metadata.clone(),
                    score,
                    best_match: vector_best.or(lexical_best).unwrap_or(BestMatch {
                        chunk_id: 0.0,
                        excerpt: String::new(),
                        heading_path: Vec::new(),
                        char_start: 0,
                        char_end: 0,
                        score: 0.0,
                    }),
                })
            })
            .collect::<Vec<_>>();
        hits.sort_by(|left, right| right.score.partial_cmp(&left.score).unwrap());
        hits.truncate(top_k);
        hits
    }

    fn rerank_documents(
        &self,
        query: &str,
        hits: Vec<DocumentHit>,
        reranker: Option<&RerankerOptions>,
        top_k: usize,
    ) -> Result<Vec<DocumentHit>, String> {
        let Some(reranker) = reranker else {
            return Ok(hits.into_iter().take(top_k).collect());
        };
        let candidate_limit = reranker.candidate_pool_size.unwrap_or(50).max(top_k);
        if reranker.kind.as_deref() == Some("heuristic-v1") {
            return Ok(rerank_documents_with_heuristic(
                query,
                &hits,
                candidate_limit,
                top_k,
            ));
        }
        rerank_documents_with_embeddings(query, &hits, candidate_limit, top_k, |input| {
            self.embed_text(&input)
        })
    }

    fn embed_query(&self, query: &str) -> Result<Vec<f32>, String> {
        self.embed_text(&format_query_for_embedding(query))
    }

    fn embed_text(&self, input: &str) -> Result<Vec<f32>, String> {
        let embedding_backend: EmbeddingBackend =
            serde_json::from_value(self.manifest.embedding_backend.clone())
                .map_err(|e| e.to_string())?;
        match embedding_backend {
            EmbeddingBackend::Hashing { dimensions } => Ok(hashing_embedding(input, dimensions)),
            EmbeddingBackend::Model2Vec { .. } => {
                let model = self
                    .model2vec
                    .as_ref()
                    .ok_or_else(|| "model2vec runtime not initialized".to_string())?;
                Ok(model.encode_single(input))
            }
        }
    }
}

fn document_matches(document: &DocumentRecord, options: &SearchOptions) -> bool {
    if let Some(prefix) = &options.relative_path_prefix {
        if !document.relative_path.starts_with(prefix) {
            return false;
        }
    }

    options
        .metadata
        .iter()
        .all(|(key, value)| metadata_matches(document.metadata.get(key), value))
}

fn metadata_matches(candidate: Option<&Value>, filter: &Value) -> bool {
    let Some(candidate) = candidate else {
        return false;
    };
    candidate.is_boolean() == filter.is_boolean()
        && candidate.is_number() == filter.is_number()
        && candidate.is_string() == filter.is_string()
        && candidate == filter
}

fn aggregate_ranked_documents<'a, I>(chunk_scores: I, limit: usize) -> Vec<RankedDocument>
where
    I: Iterator<Item = (f32, &'a ChunkRecord)>,
{
    let mut by_document: HashMap<String, Vec<(f32, &ChunkRecord)>> = HashMap::new();
    for (score, chunk) in chunk_scores {
        by_document
            .entry(chunk.doc_id.clone())
            .or_default()
            .push((score, chunk));
    }

    let mut documents = by_document
        .into_iter()
        .filter_map(|(doc_id, mut scores)| {
            scores.sort_by(|left, right| right.0.partial_cmp(&left.0).unwrap());
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

    documents.sort_by(|left, right| right.score.partial_cmp(&left.score).unwrap());
    documents.truncate(limit);
    documents
}

fn rerank_documents_with_heuristic(
    query: &str,
    hits: &[DocumentHit],
    candidate_limit: usize,
    top_k: usize,
) -> Vec<DocumentHit> {
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
    reranked.sort_by(|left, right| right.score.partial_cmp(&left.score).unwrap());
    reranked.truncate(top_k);
    reranked
}

fn rerank_documents_with_embeddings<F>(
    query: &str,
    hits: &[DocumentHit],
    candidate_limit: usize,
    top_k: usize,
    embed_text: F,
) -> Result<Vec<DocumentHit>, String>
where
    F: Fn(String) -> Result<Vec<f32>, String>,
{
    let query_tokens = tokenize(query);
    let normalized_query = normalize_for_heuristic(query);
    let query_embedding = embed_text(format_query_for_embedding(query))?;
    let mut reranked = hits
        .iter()
        .take(candidate_limit)
        .cloned()
        .map(|mut hit| {
            let document_embedding = embed_text(format_document_for_reranking(
                &hit.relative_path,
                hit.title.as_deref(),
                &hit.best_match.heading_path,
                &hit.best_match.excerpt,
                &hit.metadata,
            ))?;
            let embedding_score = cosine_similarity(&query_embedding, &document_embedding).max(0.0);
            let heuristic_score = score_document_heuristic(&hit, &query_tokens, &normalized_query);
            let rerank_score = embedding_score * 0.8 + heuristic_score * 0.2;
            hit.score = hit.score * 0.2 + rerank_score * 0.8;
            Ok(hit)
        })
        .collect::<Result<Vec<_>, String>>()?;
    reranked.sort_by(|left, right| right.score.partial_cmp(&left.score).unwrap());
    reranked.truncate(top_k);
    Ok(reranked)
}

fn score_document_heuristic(
    hit: &DocumentHit,
    query_tokens: &[String],
    normalized_query: &str,
) -> f32 {
    let title_norm = normalize_for_heuristic(hit.title.as_deref().unwrap_or_default());
    let path_norm = normalize_for_heuristic(&hit.relative_path);
    let heading_norm = normalize_for_heuristic(&hit.best_match.heading_path.join(" "));
    let excerpt_norm = normalize_for_heuristic(&hit.best_match.excerpt);

    let title_coverage = score_token_coverage(query_tokens, &title_norm);
    let heading_coverage = score_token_coverage(query_tokens, &heading_norm);
    let excerpt_coverage = score_token_coverage(query_tokens, &excerpt_norm);
    let path_coverage = score_token_coverage(query_tokens, &path_norm);

    let phrase_bonus = contains_phrase(&title_norm, normalized_query, 0.30)
        + contains_phrase(&heading_norm, normalized_query, 0.20)
        + contains_phrase(&excerpt_norm, normalized_query, 0.15)
        + contains_phrase(&path_norm, normalized_query, 0.05);

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

fn tokenize(input: &str) -> Vec<String> {
    lexical_tokenize(input)
}

fn normalize_for_heuristic(input: &str) -> String {
    core_normalize_for_heuristic(input)
}

fn format_query_for_embedding(query: &str) -> String {
    format!("query: {}", normalize_whitespace(query))
}

fn format_document_for_reranking(
    relative_path: &str,
    title: Option<&str>,
    heading_path: &[String],
    excerpt: &str,
    metadata: &MetadataMap,
) -> String {
    let mut lines = vec![format!("path: {}", normalize_whitespace(relative_path))];
    if let Some(title) = title.filter(|value| !value.trim().is_empty()) {
        lines.push(format!("title: {}", normalize_whitespace(title)));
    }
    if !heading_path.is_empty() {
        lines.push(format!(
            "headings: {}",
            normalize_whitespace(&heading_path.join(" > "))
        ));
    }
    if !metadata.is_empty() {
        let metadata_line = metadata
            .iter()
            .map(|(key, value)| format!("{key}={}", format_metadata_value(value)))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!(
            "metadata: {}",
            normalize_whitespace(&metadata_line)
        ));
    }
    lines.push(format!("excerpt: {}", normalize_whitespace(excerpt)));
    lines.join("\n")
}

fn format_metadata_value(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        other => other.to_string(),
    }
}

fn normalize_whitespace(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
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
    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in &mut vector {
            *value /= norm;
        }
    }
    vector
}

fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    if left.is_empty() || left.len() != right.len() {
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

fn decode_vectors(
    bytes: &[u8],
    chunk_count: usize,
    dimensions: usize,
) -> Result<Vec<Vec<f32>>, String> {
    let expected_len = chunk_count * dimensions * 4;
    if bytes.len() != expected_len {
        return Err(format!(
            "vector blob length mismatch: expected {expected_len} bytes, got {}",
            bytes.len()
        ));
    }
    let mut vectors = Vec::with_capacity(chunk_count);
    let mut offset = 0usize;
    for _ in 0..chunk_count {
        let mut vector = Vec::with_capacity(dimensions);
        for _ in 0..dimensions {
            let bytes = [
                bytes[offset],
                bytes[offset + 1],
                bytes[offset + 2],
                bytes[offset + 3],
            ];
            vector.push(f32::from_le_bytes(bytes));
            offset += 4;
        }
        vectors.push(vector);
    }
    Ok(vectors)
}

fn to_js_error(error: impl ToString) -> JsValue {
    JsValue::from_str(&error.to_string())
}
