import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { existsSync } from 'node:fs';

const require = createRequire(import.meta.url);
const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, '..');

function platformKey(): string {
  return `${process.platform}-${process.arch}`;
}

const SUPPORTED_PREBUILT_TARGETS = new Map<string, string>([
  ['darwin-arm64', '@indexbind/native-darwin-arm64'],
  ['darwin-x64', '@indexbind/native-darwin-x64'],
  ['linux-x64', '@indexbind/native-linux-x64-gnu'],
  ['linux-arm64', '@indexbind/native-linux-arm64-gnu'],
]);

function resolveNativeModule(): NativeModule {
  const key = platformKey();
  const attempted: string[] = [];
  const candidates = [
    path.join(root, 'native', `indexbind.${key}.node`),
    path.join(root, 'native', 'indexbind.node'),
  ];

  for (const candidate of candidates) {
    if (existsSync(candidate)) {
      attempted.push(candidate);
      try {
        return require(candidate) as NativeModule;
      } catch (error) {
        throw nativeLoadError(key, attempted, error);
      }
    }
    attempted.push(candidate);
  }

  const prebuiltPackage = SUPPORTED_PREBUILT_TARGETS.get(key);
  if (prebuiltPackage) {
    attempted.push(prebuiltPackage);
    try {
      return require(prebuiltPackage) as NativeModule;
    } catch {
      throw nativeLoadError(key, attempted);
    }
  }

  throw nativeLoadError(key, attempted);
}

function nativeLoadError(key: string, attempted: string[], cause?: unknown): Error {
  const lines = [
    `indexbind native addon could not be loaded for ${key}.`,
    `Attempted: ${attempted.join(', ')}`,
    `Supported prebuilt targets: ${Array.from(SUPPORTED_PREBUILT_TARGETS.keys()).join(', ')}`,
    'For local development, run "npm run build:native".',
  ];

  if (cause instanceof Error && cause.message) {
    lines.push(`Load failure: ${cause.message}`);
  } else if (!SUPPORTED_PREBUILT_TARGETS.has(key)) {
    lines.push('This platform is not in the current prebuilt matrix.');
  }

  return new Error(lines.join('\n'));
}

export interface NativeBestMatch {
  chunkId: number;
  excerpt: string;
  headingPath: string[];
  charStart: number;
  charEnd: number;
  score: number;
}

export interface NativeDocumentHit {
  docId: string;
  relativePath: string;
  canonicalUrl?: string;
  title?: string;
  summary?: string;
  metadata: string;
  score: number;
  bestMatch: NativeBestMatch;
}

export interface NativeArtifactInfo {
  schemaVersion: string;
  builtAt: string;
  embeddingBackend: string;
  lexicalTokenizer: string;
  sourceRoot: string;
  documentCount: number;
  chunkCount: number;
}

export interface NativeBuildDocument {
  docId?: string;
  sourcePath?: string;
  relativePath: string;
  canonicalUrl?: string;
  title?: string;
  summary?: string;
  content: string;
  metadataJson?: string;
}

export interface NativeBuildOptions {
  embeddingBackend?: string;
  hashingDimensions?: number;
  model?: string;
  batchSize?: number;
  sourceRootId?: string;
  sourceRootPath?: string;
  targetTokens?: number;
  overlapTokens?: number;
}

export interface NativeCanonicalBuildStats {
  documentCount: number;
  chunkCount: number;
  vectorDimensions: number;
}

export interface NativeBuildStats {
  documentCount: number;
  chunkCount: number;
}

export interface NativeIncrementalBuildStats {
  scannedDocumentCount: number;
  newDocumentCount: number;
  changedDocumentCount: number;
  unchangedDocumentCount: number;
  removedDocumentCount: number;
  activeDocumentCount: number;
  activeChunkCount: number;
}

export interface NativeDirectoryUpdateMode {
  mode?: 'full-scan' | 'git-diff';
  baseRevision?: string;
}

export interface NativeBenchmarkCaseResult {
  name: string;
  query: string;
  expectedTopHit: string;
  actualTopHit?: string;
  passed: boolean;
}

export interface NativeBenchmarkSummary {
  fixture: string;
  total: number;
  passed: number;
  failed: number;
  results: NativeBenchmarkCaseResult[];
}

export interface NativeSearchOptions {
  topK?: number;
  mode?: 'hybrid' | 'vector' | 'lexical';
  minScore?: number;
  reranker?: NativeRerankerOptions;
  relativePathPrefix?: string;
  metadata?: Record<string, string>;
  scoreAdjustment?: NativeScoreAdjustmentOptions;
}

export interface NativeOpenIndexOptions {
  modeProfile?: 'hybrid' | 'lexical';
}

export interface NativeRerankerOptions {
  kind?: 'embedding-v1' | 'heuristic-v1';
  candidatePoolSize?: number;
}

export interface NativeScoreAdjustmentOptions {
  metadataNumericMultiplier?: string;
}

export interface NativeIndex {
  info(): NativeArtifactInfo;
  search(query: string, options?: NativeSearchOptions): NativeDocumentHit[];
}

export interface NativeModule {
  NativeIndex: {
    open(artifactPath: string, options?: NativeOpenIndexOptions): NativeIndex;
  };
  buildArtifactFromDirectory(
    inputDir: string,
    outputPath: string,
    options?: NativeBuildOptions,
  ): NativeBuildStats;
  buildCanonicalBundle(
    outputDir: string,
    documents: NativeBuildDocument[],
    options?: NativeBuildOptions,
  ): NativeCanonicalBuildStats;
  buildCanonicalBundleFromDirectory(
    inputDir: string,
    outputDir: string,
    options?: NativeBuildOptions,
  ): NativeCanonicalBuildStats;
  updateBuildCacheFromDocuments(
    cachePath: string,
    documents: NativeBuildDocument[],
    removedRelativePaths?: string[],
    options?: NativeBuildOptions,
  ): NativeIncrementalBuildStats;
  updateBuildCacheFromDirectory(
    inputDir: string,
    cachePath: string,
    options?: NativeBuildOptions,
    updateMode?: NativeDirectoryUpdateMode,
  ): NativeIncrementalBuildStats;
  exportArtifactFromCache(cachePath: string, outputPath: string): NativeBuildStats;
  exportCanonicalBundleFromCache(
    cachePath: string,
    outputDir: string,
  ): NativeCanonicalBuildStats;
  inspectArtifact(artifactPath: string): NativeArtifactInfo;
  benchmarkArtifact(artifactPath: string, queriesJsonPath: string): NativeBenchmarkSummary;
}

export function loadNativeModule(): NativeModule {
  return resolveNativeModule();
}
