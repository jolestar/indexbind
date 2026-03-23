import {
  loadNativeModule,
  type NativeBuildDocument,
  type NativeBuildOptions,
  type NativeCanonicalBuildStats,
} from './native.js';

export type JsonValue =
  | null
  | boolean
  | number
  | string
  | JsonValue[]
  | { [key: string]: JsonValue };

export interface BuildDocument {
  docId?: string;
  sourcePath?: string;
  relativePath: string;
  canonicalUrl?: string;
  title?: string;
  summary?: string;
  content: string;
  metadata?: Record<string, JsonValue>;
}

export interface BuildCanonicalBundleOptions {
  embeddingBackend?: 'hashing' | 'model2vec';
  hashingDimensions?: number;
  model?: string;
  batchSize?: number;
  sourceRootId?: string;
  sourceRootPath?: string;
  targetTokens?: number;
  overlapTokens?: number;
}

export interface CanonicalBuildStats {
  documentCount: number;
  chunkCount: number;
  vectorDimensions: number;
}

export async function buildCanonicalBundle(
  outputDir: string,
  documents: BuildDocument[],
  options: BuildCanonicalBundleOptions = {},
): Promise<CanonicalBuildStats> {
  const module = loadNativeModule();
  const nativeDocuments = documents.map(mapBuildDocument);
  const nativeOptions: NativeBuildOptions = {
    embeddingBackend: options.embeddingBackend,
    hashingDimensions: options.hashingDimensions,
    model: options.model,
    batchSize: options.batchSize,
    sourceRootId: options.sourceRootId,
    sourceRootPath: options.sourceRootPath,
    targetTokens: options.targetTokens,
    overlapTokens: options.overlapTokens,
  };
  return mapBuildStats(module.buildCanonicalBundle(outputDir, nativeDocuments, nativeOptions));
}

function mapBuildDocument(document: BuildDocument): NativeBuildDocument {
  return {
    docId: document.docId,
    sourcePath: document.sourcePath,
    relativePath: document.relativePath,
    canonicalUrl: document.canonicalUrl,
    title: document.title,
    summary: document.summary,
    content: document.content,
    metadataJson: JSON.stringify(document.metadata ?? {}),
  };
}

function mapBuildStats(stats: NativeCanonicalBuildStats): CanonicalBuildStats {
  return {
    documentCount: stats.documentCount,
    chunkCount: stats.chunkCount,
    vectorDimensions: stats.vectorDimensions,
  };
}
