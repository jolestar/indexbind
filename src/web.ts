import { blake3 } from 'hash-wasm';

export type JsonValue =
  | null
  | boolean
  | number
  | string
  | JsonValue[]
  | { [key: string]: JsonValue };

export interface SearchOptions {
  topK?: number;
  hybrid?: boolean;
  reranker?: RerankerOptions;
  relativePathPrefix?: string;
  metadata?: Record<string, JsonValue>;
}

export interface RerankerOptions {
  kind?: 'embedding-v1' | 'heuristic-v1';
  candidatePoolSize?: number;
}

export interface BestMatch {
  chunkId: number;
  excerpt: string;
  headingPath: string[];
  charStart: number;
  charEnd: number;
  score: number;
}

export interface DocumentHit {
  docId: string;
  relativePath: string;
  canonicalUrl?: string;
  title?: string;
  summary?: string;
  metadata: Record<string, JsonValue>;
  score: number;
  bestMatch: BestMatch;
}

export interface WebArtifactInfo {
  schemaVersion: string;
  artifactFormat: string;
  builtAt: string;
  embeddingBackend: unknown;
  documentCount: number;
  chunkCount: number;
  vectorDimensions: number;
  chunking: unknown;
  features: string[];
}

interface CanonicalArtifactManifest {
  schemaVersion: string;
  artifactFormat: string;
  builtAt: string;
  embeddingBackend: EmbeddingBackend;
  documentCount: number;
  chunkCount: number;
  vectorDimensions: number;
  chunking: unknown;
  files: {
    documents: string;
    chunks: string;
    vectors: string;
    postings: string;
  };
  features: string[];
}

interface CanonicalDocumentRecord {
  docId: string;
  relativePath: string;
  canonicalUrl?: string;
  title?: string;
  summary?: string;
  metadata: Record<string, JsonValue>;
  firstChunkIndex: number;
  chunkCount: number;
}

interface CanonicalChunkRecord {
  chunkId: number;
  docId: string;
  ordinal: number;
  headingPath: string[];
  charStart: number;
  charEnd: number;
  tokenCount: number;
  excerpt: string;
  chunkText: string;
}

interface CanonicalPostings {
  tokenizer: string;
  avgChunkLength: number;
  documentFrequency: Record<string, number>;
  postings: Record<string, CanonicalPosting[]>;
}

interface CanonicalPosting {
  chunkIndex: number;
  termFrequency: number;
}

type EmbeddingBackend = Record<string, unknown>;

type ResolvedResource =
  | { kind: 'url'; value: string }
  | { kind: 'file'; value: string };

interface RankedChunk {
  docId: string;
  score: number;
  chunk: CanonicalChunkRecord;
}

interface RankedDocument {
  docId: string;
  score: number;
  bestMatch: BestMatch;
}

interface FusedScore {
  score: number;
  vectorBest?: BestMatch;
  lexicalBest?: BestMatch;
}

export class WebIndex {
  readonly #manifest: CanonicalArtifactManifest;
  readonly #documents: CanonicalDocumentRecord[];
  readonly #documentsById: Map<string, CanonicalDocumentRecord>;
  readonly #chunks: CanonicalChunkRecord[];
  readonly #vectors: Float32Array[];
  readonly #postings: CanonicalPostings;

  private constructor(
    manifest: CanonicalArtifactManifest,
    documents: CanonicalDocumentRecord[],
    chunks: CanonicalChunkRecord[],
    vectors: Float32Array[],
    postings: CanonicalPostings,
  ) {
    this.#manifest = manifest;
    this.#documents = documents;
    this.#documentsById = new Map(documents.map((document) => [document.docId, document]));
    this.#chunks = chunks;
    this.#vectors = vectors;
    this.#postings = postings;
  }

  static async open(base: string | URL): Promise<WebIndex> {
    const manifest = await loadJson<CanonicalArtifactManifest>(base, 'manifest.json');
    const documents = await loadJson<CanonicalDocumentRecord[]>(base, manifest.files.documents);
    const chunks = await loadJson<CanonicalChunkRecord[]>(base, manifest.files.chunks);
    const postings = await loadJson<CanonicalPostings>(base, manifest.files.postings);
    const vectorsBuffer = await loadArrayBuffer(base, manifest.files.vectors);
    const vectors = decodeVectors(vectorsBuffer, chunks.length, manifest.vectorDimensions);
    return new WebIndex(manifest, documents, chunks, vectors, postings);
  }

  info(): WebArtifactInfo {
    return {
      schemaVersion: this.#manifest.schemaVersion,
      artifactFormat: this.#manifest.artifactFormat,
      builtAt: this.#manifest.builtAt,
      embeddingBackend: this.#manifest.embeddingBackend,
      documentCount: this.#manifest.documentCount,
      chunkCount: this.#manifest.chunkCount,
      vectorDimensions: this.#manifest.vectorDimensions,
      chunking: this.#manifest.chunking,
      features: this.#manifest.features,
    };
  }

  async search(query: string, options: SearchOptions = {}): Promise<DocumentHit[]> {
    const topK = options.topK ?? 10;
    const candidateMultiplier = 8;
    const allowedDocIds = this.allowedDocIds(options);
    if (allowedDocIds.size === 0) {
      return [];
    }

    const rerankCandidateLimit = Math.max(options.reranker?.candidatePoolSize ?? topK, topK);
    const limit = Math.max(topK * candidateMultiplier, rerankCandidateLimit, topK);
    const queryEmbedding = await this.embedQuery(query);
    const vectorDocs = this.rankDocumentsByVector(queryEmbedding, limit, allowedDocIds);
    const lexicalDocs = options.hybrid === false
      ? []
      : this.rankDocumentsByLexical(query, limit, allowedDocIds);
    const fused = fuseDocuments(this.#documentsById, vectorDocs, lexicalDocs, topK);
    return this.rerankDocuments(query, fused, options.reranker, topK);
  }

  private allowedDocIds(options: SearchOptions): Set<string> {
    return new Set(
      this.#documents
        .filter((document) => documentMatches(document, options))
        .map((document) => document.docId),
    );
  }

  private rankDocumentsByVector(
    queryEmbedding: Float32Array,
    limit: number,
    allowedDocIds: Set<string>,
  ): RankedDocument[] {
    const chunkScores = this.#chunks
      .map((chunk, index) => ({
        chunk,
        score: cosineSimilarity(queryEmbedding, this.#vectors[index]),
      }))
      .filter((entry) => allowedDocIds.has(entry.chunk.docId) && entry.score > 0)
      .sort((left, right) => right.score - left.score);

    return aggregateRankedDocuments(
      chunkScores.slice(0, limit * 2).map((entry) => ({
        docId: entry.chunk.docId,
        score: entry.score,
        chunk: entry.chunk,
      })),
      limit,
    );
  }

  private rankDocumentsByLexical(
    query: string,
    limit: number,
    allowedDocIds: Set<string>,
  ): RankedDocument[] {
    const tokens = [...new Set(tokenize(query))];
    if (tokens.length === 0) {
      return [];
    }

    const scoredChunks = new Map<number, number>();
    const chunkCount = this.#chunks.length;
    const avgChunkLength = this.#postings.avgChunkLength || 1;
    const k1 = 1.2;
    const b = 0.75;

    for (const token of tokens) {
      const postings = this.#postings.postings[token];
      if (!postings || postings.length === 0) {
        continue;
      }

      const documentFrequency = this.#postings.documentFrequency[token] ?? postings.length;
      const idf = Math.log(1 + (chunkCount - documentFrequency + 0.5) / (documentFrequency + 0.5));

      for (const posting of postings) {
        const chunk = this.#chunks[posting.chunkIndex];
        if (!allowedDocIds.has(chunk.docId)) {
          continue;
        }
        const chunkLength = chunk.tokenCount || 1;
        const numerator = posting.termFrequency * (k1 + 1);
        const denominator =
          posting.termFrequency + k1 * (1 - b + b * (chunkLength / avgChunkLength));
        const score = idf * (numerator / denominator);
        scoredChunks.set(posting.chunkIndex, (scoredChunks.get(posting.chunkIndex) ?? 0) + score);
      }
    }

    const chunkScores = [...scoredChunks.entries()]
      .map(([chunkIndex, score]) => ({
        docId: this.#chunks[chunkIndex].docId,
        score,
        chunk: this.#chunks[chunkIndex],
      }))
      .sort((left, right) => right.score - left.score)
      .slice(0, limit * 2);

    return aggregateRankedDocuments(chunkScores, limit);
  }

  private async rerankDocuments(
    query: string,
    hits: DocumentHit[],
    reranker: RerankerOptions | undefined,
    topK: number,
  ): Promise<DocumentHit[]> {
    if (!reranker) {
      return hits.slice(0, topK);
    }

    if (reranker.kind === 'heuristic-v1') {
      return rerankDocumentsWithHeuristic(query, hits, reranker, topK);
    }

    return rerankDocumentsWithEmbeddings(
      query,
      hits,
      reranker,
      topK,
      (input) => this.embedText(input),
    );
  }

  private async embedQuery(query: string): Promise<Float32Array> {
    return this.embedText(formatQueryForEmbedding(query));
  }

  private async embedText(input: string): Promise<Float32Array> {
    const backend = this.#manifest.embeddingBackend as Record<string, unknown>;
    if ('Hashing' in backend || 'dimensions' in backend) {
      const dimensions = extractHashingDimensions(this.#manifest.embeddingBackend);
      return hashingEmbedding(input, dimensions);
    }
    throw new Error(
      'web runtime does not support model2vec bundles yet; build a hashing bundle for now.',
    );
  }
}

export async function openWebIndex(base: string | URL): Promise<WebIndex> {
  return WebIndex.open(base);
}

function documentMatches(document: CanonicalDocumentRecord, options: SearchOptions): boolean {
  if (options.relativePathPrefix && !document.relativePath.startsWith(options.relativePathPrefix)) {
    return false;
  }

  return Object.entries(options.metadata ?? {}).every(([key, value]) =>
    metadataMatches(document.metadata[key], value),
  );
}

function metadataMatches(candidate: JsonValue | undefined, filter: JsonValue): boolean {
  if (candidate === undefined) {
    return false;
  }
  if (candidate === null || filter === null) {
    return candidate === filter;
  }
  if (Array.isArray(candidate) || Array.isArray(filter)) {
    return false;
  }
  if (typeof candidate === 'object' || typeof filter === 'object') {
    return false;
  }
  return candidate === filter;
}

function aggregateRankedDocuments(chunkScores: RankedChunk[], limit: number): RankedDocument[] {
  const byDocument = new Map<string, Array<{ score: number; chunk: CanonicalChunkRecord }>>();
  for (const entry of chunkScores) {
    const scores = byDocument.get(entry.docId) ?? [];
    scores.push({ score: entry.score, chunk: entry.chunk });
    byDocument.set(entry.docId, scores);
  }

  const documents: RankedDocument[] = [];
  for (const [docId, scores] of byDocument.entries()) {
    scores.sort((left, right) => right.score - left.score);
    const best = scores[0];
    if (!best) {
      continue;
    }
    const aggregate =
      best.score +
      scores
        .slice(1, 3)
        .reduce((sum, entry) => sum + entry.score, 0) *
        0.1;
    documents.push({
      docId,
      score: aggregate,
      bestMatch: {
        chunkId: best.chunk.chunkId,
        excerpt: best.chunk.excerpt,
        headingPath: best.chunk.headingPath,
        charStart: best.chunk.charStart,
        charEnd: best.chunk.charEnd,
        score: best.score,
      },
    });
  }

  documents.sort((left, right) => right.score - left.score);
  return documents.slice(0, limit);
}

function fuseDocuments(
  documents: Map<string, CanonicalDocumentRecord>,
  vectorDocs: RankedDocument[],
  lexicalDocs: RankedDocument[],
  topK: number,
): DocumentHit[] {
  const RRF_K = 60;
  const fused = new Map<string, FusedScore>();

  for (const [rank, entry] of vectorDocs.entries()) {
    const score = 1 / (RRF_K + rank + 1);
    const value = fused.get(entry.docId) ?? { score: 0 };
    value.score += score;
    value.vectorBest = entry.bestMatch;
    fused.set(entry.docId, value);
  }

  for (const [rank, entry] of lexicalDocs.entries()) {
    const score = 1 / (RRF_K + rank + 1);
    const value = fused.get(entry.docId) ?? { score: 0 };
    value.score += score;
    value.lexicalBest = entry.bestMatch;
    fused.set(entry.docId, value);
  }

  const hits: DocumentHit[] = [];
  for (const [docId, fusedScore] of fused.entries()) {
    const document = documents.get(docId);
    if (!document) {
      continue;
    }
    hits.push({
      docId: document.docId,
      relativePath: document.relativePath,
      canonicalUrl: document.canonicalUrl,
      title: document.title,
      summary: document.summary,
      metadata: document.metadata,
      score: fusedScore.score,
      bestMatch: fusedScore.vectorBest ?? fusedScore.lexicalBest ?? {
        chunkId: 0,
        excerpt: '',
        headingPath: [],
        charStart: 0,
        charEnd: 0,
        score: 0,
      },
    });
  }

  hits.sort((left, right) => right.score - left.score);
  return hits.slice(0, topK);
}

function rerankDocumentsWithHeuristic(
  query: string,
  hits: DocumentHit[],
  reranker: RerankerOptions,
  topK: number,
): DocumentHit[] {
  const candidateLimit = Math.max(reranker.candidatePoolSize ?? topK, topK);
  const queryTokens = tokenize(query);
  const normalizedQuery = normalizeForHeuristic(query);
  const reranked = hits
    .slice(0, candidateLimit)
    .map((hit) => {
      const rerankScore = scoreDocumentHeuristic(hit, queryTokens, normalizedQuery);
      return {
        ...hit,
        score: hit.score * 0.35 + rerankScore * 0.65,
      };
    });

  reranked.sort((left, right) => right.score - left.score);
  return reranked.slice(0, topK);
}

async function rerankDocumentsWithEmbeddings(
  query: string,
  hits: DocumentHit[],
  reranker: RerankerOptions,
  topK: number,
  embedText: (input: string) => Promise<Float32Array>,
): Promise<DocumentHit[]> {
  const candidateLimit = Math.max(reranker.candidatePoolSize ?? topK, topK);
  const queryTokens = tokenize(query);
  const normalizedQuery = normalizeForHeuristic(query);
  const queryEmbedding = await embedText(formatQueryForEmbedding(query));
  const reranked = await Promise.all(
    hits.slice(0, candidateLimit).map(async (hit) => {
      const documentEmbedding = await embedText(
        formatDocumentForReranking(
          hit.relativePath,
          hit.title,
          hit.bestMatch.headingPath,
          hit.bestMatch.excerpt,
          hit.metadata,
        ),
      );
      const embeddingScore = Math.max(cosineSimilarity(queryEmbedding, documentEmbedding), 0);
      const heuristicScore = scoreDocumentHeuristic(hit, queryTokens, normalizedQuery);
      const rerankScore = embeddingScore * 0.8 + heuristicScore * 0.2;
      return {
        ...hit,
        score: hit.score * 0.2 + rerankScore * 0.8,
      };
    }),
  );

  reranked.sort((left, right) => right.score - left.score);
  return reranked.slice(0, topK);
}

function scoreDocumentHeuristic(
  hit: DocumentHit,
  queryTokens: string[],
  normalizedQuery: string,
): number {
  const titleNorm = normalizeForHeuristic(hit.title ?? '');
  const pathNorm = normalizeForHeuristic(hit.relativePath);
  const headingNorm = normalizeForHeuristic(hit.bestMatch.headingPath.join(' '));
  const excerptNorm = normalizeForHeuristic(hit.bestMatch.excerpt);

  const titleCoverage = scoreTokenCoverage(queryTokens, titleNorm);
  const headingCoverage = scoreTokenCoverage(queryTokens, headingNorm);
  const excerptCoverage = scoreTokenCoverage(queryTokens, excerptNorm);
  const pathCoverage = scoreTokenCoverage(queryTokens, pathNorm);

  const phraseBonus =
    containsPhrase(titleNorm, normalizedQuery, 0.3) +
    containsPhrase(headingNorm, normalizedQuery, 0.2) +
    containsPhrase(excerptNorm, normalizedQuery, 0.15) +
    containsPhrase(pathNorm, normalizedQuery, 0.05);

  return (
    titleCoverage * 0.45 +
    headingCoverage * 0.2 +
    excerptCoverage * 0.25 +
    pathCoverage * 0.1 +
    phraseBonus
  );
}

function scoreTokenCoverage(queryTokens: string[], haystack: string): number {
  if (queryTokens.length === 0) {
    return 0;
  }
  const matched = queryTokens.filter((token) => haystack.includes(token)).length;
  return matched / queryTokens.length;
}

function containsPhrase(haystack: string, needle: string, weight: number): number {
  if (!needle || !haystack.includes(needle)) {
    return 0;
  }
  return weight;
}

function formatQueryForEmbedding(query: string): string {
  return `query: ${normalizeWhitespace(query)}`;
}

function formatDocumentForReranking(
  relativePath: string,
  title: string | undefined,
  headingPath: string[],
  excerpt: string,
  metadata: Record<string, JsonValue>,
): string {
  const lines = [`path: ${normalizeWhitespace(relativePath)}`];
  if (title && title.trim()) {
    lines.push(`title: ${normalizeWhitespace(title)}`);
  }
  if (headingPath.length > 0) {
    lines.push(`headings: ${normalizeWhitespace(headingPath.join(' > '))}`);
  }
  if (Object.keys(metadata).length > 0) {
    const metadataLine = Object.entries(metadata)
      .map(([key, value]) => `${key}=${formatMetadataValue(value)}`)
      .join(', ');
    lines.push(`metadata: ${normalizeWhitespace(metadataLine)}`);
  }
  lines.push(`excerpt: ${normalizeWhitespace(excerpt)}`);
  return lines.join('\n');
}

function formatMetadataValue(value: JsonValue): string {
  if (value === null) {
    return 'null';
  }
  if (typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean') {
    return String(value);
  }
  return JSON.stringify(value);
}

function normalizeWhitespace(input: string): string {
  return input.trim().split(/\s+/).join(' ');
}

function normalizeForHeuristic(input: string): string {
  return input
    .split('')
    .map((ch) => (/[\p{L}\p{N}]/u.test(ch) ? ch : ' '))
    .join('')
    .split(/\s+/)
    .filter(Boolean)
    .join(' ')
    .toLowerCase();
}

function tokenize(input: string): string[] {
  return input
    .split(/[^\p{L}\p{N}]+/u)
    .filter(Boolean)
    .map((segment) => segment.toLowerCase());
}

async function hashingEmbedding(input: string, dimensions: number): Promise<Float32Array> {
  const vector = new Float32Array(dimensions);
  for (const token of input.split(/\s+/).filter(Boolean)) {
    const hash = await blake3HashBytes(token);
    const bucket = hash[0] % dimensions;
    const sign = hash[1] % 2 === 0 ? 1 : -1;
    vector[bucket] += sign;
  }

  let norm = 0;
  for (const value of vector) {
    norm += value * value;
  }
  norm = Math.sqrt(norm);
  if (norm > 0) {
    for (let index = 0; index < vector.length; index += 1) {
      vector[index] /= norm;
    }
  }

  return vector;
}

function cosineSimilarity(left: Float32Array, right: Float32Array): number {
  if (left.length === 0 || left.length !== right.length) {
    return 0;
  }
  let dot = 0;
  let leftNorm = 0;
  let rightNorm = 0;
  for (let index = 0; index < left.length; index += 1) {
    dot += left[index] * right[index];
    leftNorm += left[index] * left[index];
    rightNorm += right[index] * right[index];
  }
  if (leftNorm === 0 || rightNorm === 0) {
    return 0;
  }
  return dot / (Math.sqrt(leftNorm) * Math.sqrt(rightNorm));
}

function decodeVectors(buffer: ArrayBuffer, chunkCount: number, dimensions: number): Float32Array[] {
  const view = new DataView(buffer);
  const vectors: Float32Array[] = [];
  let offset = 0;
  for (let chunkIndex = 0; chunkIndex < chunkCount; chunkIndex += 1) {
    const vector = new Float32Array(dimensions);
    for (let dimension = 0; dimension < dimensions; dimension += 1) {
      vector[dimension] = view.getFloat32(offset, true);
      offset += 4;
    }
    vectors.push(vector);
  }
  return vectors;
}

async function loadJson<T>(base: string | URL, fileName: string): Promise<T> {
  const buffer = await loadArrayBuffer(base, fileName);
  return JSON.parse(new TextDecoder().decode(buffer)) as T;
}

async function loadArrayBuffer(base: string | URL, fileName: string): Promise<ArrayBuffer> {
  const resource = resolveResource(base, fileName);
  if (resource.kind === 'url') {
    const response = await fetch(resource.value);
    if (!response.ok) {
      throw new Error(`failed to load ${resource.value}: ${response.status} ${response.statusText}`);
    }
    return await response.arrayBuffer();
  }

  const fs = await import('node:fs/promises');
  const buffer = await fs.readFile(resource.value);
  return buffer.buffer.slice(buffer.byteOffset, buffer.byteOffset + buffer.byteLength);
}

function resolveResource(base: string | URL, fileName: string): ResolvedResource {
  if (base instanceof URL) {
    return {
      kind: 'url',
      value: new URL(fileName, ensureTrailingSlash(base)).toString(),
    };
  }

  if (/^https?:\/\//.test(base)) {
    return {
      kind: 'url',
      value: new URL(fileName, ensureTrailingSlash(base)).toString(),
    };
  }

  if (!isNodeRuntime()) {
    const url = new URL(fileName, ensureTrailingSlash(base, globalThis.location?.href));
    return { kind: 'url', value: url.toString() };
  }

  return {
    kind: 'file',
    value: joinFilePath(base, fileName),
  };
}

function ensureTrailingSlash(value: string | URL, relativeTo?: string): string | URL {
  if (value instanceof URL) {
    return value.pathname.endsWith('/') ? value : new URL(`${value.pathname}/`, value);
  }
  const normalized = value.endsWith('/') ? value : `${value}/`;
  if (relativeTo) {
    return new URL(normalized, relativeTo);
  }
  return normalized;
}

function isNodeRuntime(): boolean {
  return typeof process !== 'undefined' && Boolean(process.versions?.node);
}

function extractHashingDimensions(backend: unknown): number {
  if (!backend || typeof backend !== 'object') {
    throw new Error('unsupported embedding backend shape');
  }
  const record = backend as Record<string, unknown>;
  if ('Hashing' in record) {
    const hashing = record.Hashing;
    if (hashing && typeof hashing === 'object' && 'dimensions' in (hashing as Record<string, unknown>)) {
      return Number((hashing as Record<string, unknown>).dimensions);
    }
  }
  if ('dimensions' in record) {
    return Number(record.dimensions);
  }
  throw new Error('web runtime only supports hashing embedding backend');
}

function joinFilePath(base: string, fileName: string): string {
  const normalizedBase = base.endsWith('/') ? base.slice(0, -1) : base;
  return `${normalizedBase}/${fileName}`;
}

async function blake3HashBytes(input: string): Promise<Uint8Array> {
  const hex = await blake3(input);
  return hexToBytes(hex);
}

function hexToBytes(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let index = 0; index < bytes.length; index += 1) {
    bytes[index] = Number.parseInt(hex.slice(index * 2, index * 2 + 2), 16);
  }
  return bytes;
}
