import { blake3 } from '@noble/hashes/blake3.js';

export type JsonValue =
  | null
  | boolean
  | number
  | string
  | JsonValue[]
  | { [key: string]: JsonValue };

export interface SearchOptions {
  topK?: number;
  mode?: 'hybrid' | 'vector' | 'lexical';
  minScore?: number;
  reranker?: RerankerOptions;
  relativePathPrefix?: string;
  metadata?: Record<string, JsonValue>;
  scoreAdjustment?: ScoreAdjustmentOptions;
}

export interface RerankerOptions {
  kind?: 'embedding-v1' | 'heuristic-v1';
  candidatePoolSize?: number;
}

export interface ScoreAdjustmentOptions {
  metadataNumericMultiplier?: string;
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

export interface OpenWebIndexOptions {
  fetch?: (input: RequestInfo | URL, init?: RequestInit) => Promise<Response>;
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
    model?: {
      tokenizer: string;
      config: string;
      weights: string;
    };
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

interface WasmSearchBackend {
  info(): unknown;
  search(query: string, options?: unknown): unknown;
}

export interface WasmIndexBinding {
  new (
    manifest: unknown,
    documents: unknown,
    chunks: unknown,
    vectors: Uint8Array,
    postings: unknown,
    tokenizerBytes?: Uint8Array,
    modelBytes?: Uint8Array,
    configBytes?: Uint8Array,
  ): WasmSearchBackend;
}

export class WebIndex {
  readonly #manifest: CanonicalArtifactManifest;
  readonly #documents: CanonicalDocumentRecord[];
  readonly #documentsById: Map<string, CanonicalDocumentRecord>;
  readonly #wasmIndex: WasmSearchBackend;

  constructor(
    manifest: CanonicalArtifactManifest,
    documents: CanonicalDocumentRecord[],
    wasmIndex: WasmSearchBackend,
  ) {
    this.#manifest = manifest;
    this.#documents = documents;
    this.#documentsById = new Map(documents.map((document) => [document.docId, document]));
    this.#wasmIndex = wasmIndex;
  }

  static async open(base: string | URL): Promise<WebIndex> {
    return openWebIndexInternal(base, createBrowserWasmIndex);
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
    assertNoLegacyHybridOption(options);
    return this.#wasmIndex.search(query, options) as Promise<DocumentHit[]>;
  }
}

async function openWebIndexInternal(
  base: string | URL,
  createBackend: (
    manifest: CanonicalArtifactManifest,
    documents: CanonicalDocumentRecord[],
    chunks: CanonicalChunkRecord[],
    vectorsBuffer: ArrayBuffer,
    postings: CanonicalPostings,
    modelBuffers?: [ArrayBuffer, ArrayBuffer, ArrayBuffer],
  ) => Promise<WasmSearchBackend>,
  options: OpenWebIndexOptions = {},
): Promise<WebIndex> {
  const manifest = await loadJson<CanonicalArtifactManifest>(base, 'manifest.json', options);
  const documents = await loadJson<CanonicalDocumentRecord[]>(
    base,
    manifest.files.documents,
    options,
  );
  const chunks = await loadJson<CanonicalChunkRecord[]>(base, manifest.files.chunks, options);
  const postings = await loadJson<CanonicalPostings>(base, manifest.files.postings, options);
  const vectorsBuffer = await loadArrayBuffer(base, manifest.files.vectors, options);
  const modelBuffers = manifest.files.model
    ? await Promise.all([
        loadArrayBuffer(base, manifest.files.model.tokenizer, options),
        loadArrayBuffer(base, manifest.files.model.weights, options),
        loadArrayBuffer(base, manifest.files.model.config, options),
      ])
    : undefined;
  const wasmIndex = await createBackend(
    manifest,
    documents,
    chunks,
    vectorsBuffer,
    postings,
    modelBuffers,
  );
  return new WebIndex(manifest, documents, wasmIndex);
}

export async function openWebIndex(
  base: string | URL,
  options: OpenWebIndexOptions = {},
): Promise<WebIndex> {
  return openWebIndexInternal(base, createBrowserWasmIndex, options);
}

function assertNoLegacyHybridOption(options: SearchOptions): void {
  if (options && typeof options === 'object' && Object.prototype.hasOwnProperty.call(options, 'hybrid')) {
    throw new Error(
      'Search option "hybrid" has been removed. Use mode: "hybrid", "vector", or "lexical" instead.',
    );
  }
}

export async function openWebIndexWithBindings(
  base: string | URL,
  WasmIndex: WasmIndexBinding,
  options: OpenWebIndexOptions = {},
): Promise<WebIndex> {
  return openWebIndexInternal(
    base,
    async (manifest, documents, chunks, vectorsBuffer, postings, modelBuffers) =>
      createBoundWasmIndex(
        manifest,
        documents,
        chunks,
        vectorsBuffer,
        postings,
        modelBuffers,
        WasmIndex,
      ),
    options,
  );
}

async function createBrowserWasmIndex(
  manifest: CanonicalArtifactManifest,
  documents: CanonicalDocumentRecord[],
  chunks: CanonicalChunkRecord[],
  vectorsBuffer: ArrayBuffer,
  postings: CanonicalPostings,
  modelBuffers?: [ArrayBuffer, ArrayBuffer, ArrayBuffer],
): Promise<WasmSearchBackend> {
  if (!supportsWasmBackend(manifest.embeddingBackend)) {
    throw new Error('unsupported embedding backend for web runtime');
  }

  try {
    const wasmModuleUrl = new URL('./wasm/indexbind_wasm.js', import.meta.url).href;
    const wasmModule = (await import(wasmModuleUrl)) as {
      default: (input?: unknown) => Promise<void>;
      WasmIndex: new (
        manifest: unknown,
        documents: unknown,
        chunks: unknown,
        vectors: Uint8Array,
        postings: unknown,
        tokenizerBytes?: Uint8Array,
        modelBytes?: Uint8Array,
        configBytes?: Uint8Array,
      ) => WasmSearchBackend;
    };
    const wasmBinary = await loadWasmBinary();
    await wasmModule.default({ module_or_path: wasmBinary });
    return new wasmModule.WasmIndex(
      manifest,
      documents,
      chunks,
      new Uint8Array(vectorsBuffer),
      postings,
      modelBuffers ? new Uint8Array(modelBuffers[0]) : undefined,
      modelBuffers ? new Uint8Array(modelBuffers[1]) : undefined,
      modelBuffers ? new Uint8Array(modelBuffers[2]) : undefined,
    );
  } catch (error) {
    const detail =
      error instanceof Error ? error.stack ?? `${error.name}: ${error.message}` : String(error);
    throw new Error(`failed to initialize wasm web runtime: ${detail}`, {
      cause: error instanceof Error ? error : undefined,
    });
  }
}

function createBoundWasmIndex(
  manifest: CanonicalArtifactManifest,
  documents: CanonicalDocumentRecord[],
  chunks: CanonicalChunkRecord[],
  vectorsBuffer: ArrayBuffer,
  postings: CanonicalPostings,
  modelBuffers: [ArrayBuffer, ArrayBuffer, ArrayBuffer] | undefined,
  WasmIndex: WasmIndexBinding,
): WasmSearchBackend {
  if (!supportsWasmBackend(manifest.embeddingBackend)) {
    throw new Error('unsupported embedding backend for web runtime');
  }

  return new WasmIndex(
    manifest,
    documents,
    chunks,
    new Uint8Array(vectorsBuffer),
    postings,
    modelBuffers ? new Uint8Array(modelBuffers[0]) : undefined,
    modelBuffers ? new Uint8Array(modelBuffers[1]) : undefined,
    modelBuffers ? new Uint8Array(modelBuffers[2]) : undefined,
  );
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
  return Array.from(input)
    .map((ch) => (classifyChar(ch) ? ch.toLowerCase() : ' '))
    .join('')
    .split(/\s+/)
    .filter(Boolean)
    .join(' ');
}

function tokenize(input: string): string[] {
  const tokens: string[] = [];
  let current = '';
  let currentClass: 'alnum' | 'cjk' | null = null;

  const flush = () => {
    if (!current) {
      return;
    }
    if (currentClass === 'alnum') {
      tokens.push(current.toLowerCase());
    } else if (currentClass === 'cjk') {
      pushCjkTokens(tokens, current);
    }
    current = '';
  };

  for (const ch of Array.from(input)) {
    const nextClass = classifyChar(ch);
    if (!nextClass) {
      flush();
      currentClass = null;
      continue;
    }
    if (currentClass !== nextClass) {
      flush();
      currentClass = nextClass;
    }
    current += nextClass === 'alnum' ? ch.toLowerCase() : ch;
  }

  flush();
  return tokens;
}

function pushCjkTokens(tokens: string[], input: string): void {
  const chars = Array.from(input);
  if (chars.length === 1) {
    tokens.push(chars[0]);
    return;
  }
  if (chars.length === 2) {
    tokens.push(chars.join(''));
    return;
  }
  for (let index = 0; index < chars.length - 1; index += 1) {
    tokens.push(chars[index] + chars[index + 1]);
  }
}

function classifyChar(ch: string): 'alnum' | 'cjk' | null {
  if (isCjk(ch)) {
    return 'cjk';
  }
  return /[\p{L}\p{N}]/u.test(ch) ? 'alnum' : null;
}

function isCjk(ch: string): boolean {
  const codePoint = ch.codePointAt(0);
  if (codePoint === undefined) {
    return false;
  }
  return (
    (codePoint >= 0x3400 && codePoint <= 0x4dbf) ||
    (codePoint >= 0x4e00 && codePoint <= 0x9fff) ||
    (codePoint >= 0xf900 && codePoint <= 0xfaff) ||
    (codePoint >= 0x20000 && codePoint <= 0x2a6df) ||
    (codePoint >= 0x2a700 && codePoint <= 0x2b73f) ||
    (codePoint >= 0x2b740 && codePoint <= 0x2b81f) ||
    (codePoint >= 0x2b820 && codePoint <= 0x2ceaf) ||
    (codePoint >= 0x2ceb0 && codePoint <= 0x2ebef) ||
    (codePoint >= 0x2ebf0 && codePoint <= 0x2ee5f) ||
    (codePoint >= 0x30000 && codePoint <= 0x3134f) ||
    (codePoint >= 0x31350 && codePoint <= 0x323af) ||
    (codePoint >= 0x323b0 && codePoint <= 0x3347f)
  );
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

async function loadJson<T>(
  base: string | URL,
  fileName: string,
  options: OpenWebIndexOptions = {},
): Promise<T> {
  const buffer = await loadArrayBuffer(base, fileName, options);
  return JSON.parse(new TextDecoder().decode(buffer)) as T;
}

async function loadArrayBuffer(
  base: string | URL,
  fileName: string,
  options: OpenWebIndexOptions = {},
): Promise<ArrayBuffer> {
  const resource = resolveResource(base, fileName);
  if (resource.kind === 'url') {
    const fetchImpl = options.fetch ?? fetch;
    const response = await fetchImpl(resource.value);
    if (!response.ok) {
      throw new Error(`failed to load ${resource.value}: ${response.status} ${response.statusText}`);
    }
    return await response.arrayBuffer();
  }

  const fs = await import('node:fs/promises');
  const buffer = await fs.readFile(resource.value);
  return buffer.buffer.slice(buffer.byteOffset, buffer.byteOffset + buffer.byteLength);
}

async function loadWasmBinary(): Promise<Uint8Array | URL> {
  const wasmUrl = new URL('./wasm/indexbind_wasm_bg.wasm', import.meta.url);
  if (!isNodeRuntime()) {
    return wasmUrl;
  }

  const fs = await import('node:fs/promises');
  const buffer = await fs.readFile(wasmUrl);
  return new Uint8Array(buffer.buffer.slice(buffer.byteOffset, buffer.byteOffset + buffer.byteLength));
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

function supportsWasmBackend(backend: unknown): boolean {
  if (!backend || typeof backend !== 'object') {
    return false;
  }
  const record = backend as Record<string, unknown>;
  return 'Hashing' in record || 'dimensions' in record || 'Model2Vec' in record || 'model' in record;
}

function joinFilePath(base: string, fileName: string): string {
  const normalizedBase = base.endsWith('/') ? base.slice(0, -1) : base;
  return `${normalizedBase}/${fileName}`;
}

async function blake3HashBytes(input: string): Promise<Uint8Array> {
  return blake3(new TextEncoder().encode(input));
}
