import {
  loadNativeModule,
  type NativeArtifactInfo,
  type NativeBestMatch,
  type NativeDocumentHit,
  type NativeIndex,
  type NativeSearchOptions,
} from './native.js';

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
  metadata?: Record<string, string>;
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

export interface ArtifactInfo {
  schemaVersion: string;
  builtAt: string;
  embeddingBackend: unknown;
  lexicalTokenizer: string;
  sourceRoot: unknown;
  documentCount: number;
  chunkCount: number;
}

export class Index {
  readonly #nativeIndex: NativeIndex;

  private constructor(nativeIndex: NativeIndex) {
    this.#nativeIndex = nativeIndex;
  }

  static async open(artifactPath: string): Promise<Index> {
    const module = loadNativeModule();
    const nativeIndex = module.NativeIndex.open(artifactPath);
    return new Index(nativeIndex);
  }

  info(): ArtifactInfo {
    return mapArtifactInfo(this.#nativeIndex.info());
  }

  async search(query: string, options: SearchOptions = {}): Promise<DocumentHit[]> {
    assertNoLegacyHybridOption(options);
    const nativeOptions: NativeSearchOptions = {
      topK: options.topK,
      mode: options.mode,
      minScore: options.minScore,
      reranker: options.reranker,
      relativePathPrefix: options.relativePathPrefix,
      metadata: options.metadata,
      scoreAdjustment: options.scoreAdjustment,
    };
    return this.#nativeIndex.search(query, nativeOptions).map(mapHit);
  }
}

export function openIndex(artifactPath: string): Promise<Index> {
  return Index.open(artifactPath);
}

function assertNoLegacyHybridOption(options: SearchOptions): void {
  if (options && typeof options === 'object' && Object.prototype.hasOwnProperty.call(options, 'hybrid')) {
    throw new Error(
      'Search option "hybrid" has been removed. Use mode: "hybrid", "vector", or "lexical" instead.',
    );
  }
}

function mapArtifactInfo(info: NativeArtifactInfo): ArtifactInfo {
  return {
    schemaVersion: info.schemaVersion,
    builtAt: info.builtAt,
    embeddingBackend: JSON.parse(info.embeddingBackend),
    lexicalTokenizer: info.lexicalTokenizer,
    sourceRoot: JSON.parse(info.sourceRoot),
    documentCount: info.documentCount,
    chunkCount: info.chunkCount,
  };
}

function mapHit(hit: NativeDocumentHit): DocumentHit {
  return {
    docId: hit.docId,
    relativePath: hit.relativePath,
    canonicalUrl: hit.canonicalUrl,
    title: hit.title,
    summary: hit.summary,
    metadata: JSON.parse(hit.metadata) as Record<string, JsonValue>,
    score: hit.score,
    bestMatch: mapBestMatch(hit.bestMatch),
  };
}

function mapBestMatch(bestMatch: NativeBestMatch): BestMatch {
  return {
    chunkId: bestMatch.chunkId,
    excerpt: bestMatch.excerpt,
    headingPath: bestMatch.headingPath,
    charStart: bestMatch.charStart,
    charEnd: bestMatch.charEnd,
    score: bestMatch.score,
  };
}
