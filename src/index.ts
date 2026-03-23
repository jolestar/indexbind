import {
  loadNativeModule,
  type NativeArtifactInfo,
  type NativeBestMatch,
  type NativeDocumentHit,
  type NativeIndex,
  type NativeLoadedDocument,
  type NativeSearchOptions,
} from './native.js';

export interface SearchOptions {
  topK?: number;
  hybrid?: boolean;
  reranker?: RerankerOptions;
  relativePathPrefix?: string;
  metadata?: Record<string, string>;
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
  originalPath: string;
  relativePath: string;
  title?: string;
  score: number;
  bestMatch: BestMatch;
}

export interface ArtifactInfo {
  schemaVersion: string;
  builtAt: string;
  embeddingBackend: unknown;
  sourceRoot: unknown;
  documentCount: number;
  chunkCount: number;
}

export interface LoadedDocument {
  originalPath: string;
  relativePath: string;
  content: string;
}

export interface OpenIndexOptions {
  sourceRootOverride?: string;
}

export class Index {
  readonly #nativeIndex: NativeIndex;

  private constructor(nativeIndex: NativeIndex) {
    this.#nativeIndex = nativeIndex;
  }

  static async open(artifactPath: string, options: OpenIndexOptions = {}): Promise<Index> {
    const module = loadNativeModule();
    const nativeIndex = module.NativeIndex.open(artifactPath, options.sourceRootOverride);
    return new Index(nativeIndex);
  }

  info(): ArtifactInfo {
    return mapArtifactInfo(this.#nativeIndex.info());
  }

  async search(query: string, options: SearchOptions = {}): Promise<DocumentHit[]> {
    const nativeOptions: NativeSearchOptions = {
      topK: options.topK,
      hybrid: options.hybrid,
      reranker: options.reranker,
      relativePathPrefix: options.relativePathPrefix,
      metadata: options.metadata,
    };
    return this.#nativeIndex.search(query, nativeOptions).map(mapHit);
  }

  async readDocument(hit: DocumentHit): Promise<LoadedDocument> {
    return mapLoaded(
      this.#nativeIndex.readDocument(
        hit.docId,
        hit.originalPath,
        hit.relativePath,
        hit.title,
        hit.score,
        mapBestMatchBack(hit.bestMatch),
      ),
    );
  }
}

export function openIndex(artifactPath: string, options?: OpenIndexOptions): Promise<Index> {
  return Index.open(artifactPath, options);
}

function mapArtifactInfo(info: NativeArtifactInfo): ArtifactInfo {
  return {
    schemaVersion: info.schemaVersion,
    builtAt: info.builtAt,
    embeddingBackend: JSON.parse(info.embeddingBackend),
    sourceRoot: JSON.parse(info.sourceRoot),
    documentCount: info.documentCount,
    chunkCount: info.chunkCount,
  };
}

function mapHit(hit: NativeDocumentHit): DocumentHit {
  return {
    docId: hit.docId,
    originalPath: hit.originalPath,
    relativePath: hit.relativePath,
    title: hit.title,
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

function mapBestMatchBack(bestMatch: BestMatch): NativeBestMatch {
  return {
    chunkId: bestMatch.chunkId,
    excerpt: bestMatch.excerpt,
    headingPath: bestMatch.headingPath,
    charStart: bestMatch.charStart,
    charEnd: bestMatch.charEnd,
    score: bestMatch.score,
  };
}

function mapLoaded(loaded: NativeLoadedDocument): LoadedDocument {
  return {
    originalPath: loaded.originalPath,
    relativePath: loaded.relativePath,
    content: loaded.content,
  };
}
