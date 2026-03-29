import {
  loadNativeModule,
  type NativeArtifactInfo,
  type NativeBuildStats,
  type NativeBenchmarkSummary,
  type NativeBuildDocument,
  type NativeIncrementalBuildStats,
  type NativeBuildOptions,
  type NativeCanonicalBuildStats,
  type NativeDirectoryUpdateMode,
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

export interface BuildStats {
  documentCount: number;
  chunkCount: number;
}

export interface IncrementalBuildStats {
  scannedDocumentCount: number;
  newDocumentCount: number;
  changedDocumentCount: number;
  unchangedDocumentCount: number;
  removedDocumentCount: number;
  activeDocumentCount: number;
  activeChunkCount: number;
}

export interface DirectoryUpdateMode {
  mode?: 'full-scan' | 'git-diff';
  baseRevision?: string;
}

export interface BuildArtifactInfo {
  schemaVersion: string;
  builtAt: string;
  embeddingBackend: unknown;
  lexicalTokenizer: string;
  sourceRoot: unknown;
  documentCount: number;
  chunkCount: number;
}

export interface BenchmarkCaseResult {
  name: string;
  query: string;
  expectedTopHit: string;
  actualTopHit?: string;
  passed: boolean;
}

export interface BenchmarkSummary {
  fixture: string;
  total: number;
  passed: number;
  failed: number;
  results: BenchmarkCaseResult[];
}

export async function buildFromDirectory(
  inputDir: string,
  outputPath: string,
  options: BuildCanonicalBundleOptions = {},
): Promise<BuildStats> {
  const module = loadNativeModule();
  return mapPlainBuildStats(
    module.buildArtifactFromDirectory(inputDir, outputPath, mapBuildOptions(options)),
  );
}

export async function buildCanonicalBundle(
  outputDir: string,
  documents: BuildDocument[],
  options: BuildCanonicalBundleOptions = {},
): Promise<CanonicalBuildStats> {
  const module = loadNativeModule();
  const nativeDocuments = documents.map(mapBuildDocument);
  const nativeOptions = mapBuildOptions(options);
  return mapBuildStats(module.buildCanonicalBundle(outputDir, nativeDocuments, nativeOptions));
}

export async function buildCanonicalBundleFromDirectory(
  inputDir: string,
  outputDir: string,
  options: BuildCanonicalBundleOptions = {},
): Promise<CanonicalBuildStats> {
  const module = loadNativeModule();
  return mapBuildStats(
    module.buildCanonicalBundleFromDirectory(inputDir, outputDir, mapBuildOptions(options)),
  );
}

export async function updateBuildCache(
  cachePath: string,
  documents: BuildDocument[],
  options: BuildCanonicalBundleOptions = {},
  removedRelativePaths: string[] = [],
): Promise<IncrementalBuildStats> {
  const module = loadNativeModule();
  const nativeDocuments = documents.map(mapBuildDocument);
  return mapIncrementalBuildStats(
    module.updateBuildCacheFromDocuments(
      cachePath,
      nativeDocuments,
      removedRelativePaths,
      mapBuildOptions(options),
    ),
  );
}

export async function updateBuildCacheFromDirectory(
  inputDir: string,
  cachePath: string,
  options: BuildCanonicalBundleOptions = {},
  updateMode: DirectoryUpdateMode = {},
): Promise<IncrementalBuildStats> {
  const module = loadNativeModule();
  return mapIncrementalBuildStats(
    module.updateBuildCacheFromDirectory(
      inputDir,
      cachePath,
      mapBuildOptions(options),
      mapDirectoryUpdateMode(updateMode),
    ),
  );
}

export async function exportArtifactFromBuildCache(
  cachePath: string,
  outputPath: string,
): Promise<BuildStats> {
  const module = loadNativeModule();
  return mapPlainBuildStats(module.exportArtifactFromCache(cachePath, outputPath));
}

export async function exportCanonicalBundleFromBuildCache(
  cachePath: string,
  outputDir: string,
): Promise<CanonicalBuildStats> {
  const module = loadNativeModule();
  return mapBuildStats(module.exportCanonicalBundleFromCache(cachePath, outputDir));
}

export async function inspectArtifact(artifactPath: string): Promise<BuildArtifactInfo> {
  const module = loadNativeModule();
  return mapArtifactInfo(module.inspectArtifact(artifactPath));
}

export async function benchmarkArtifact(
  artifactPath: string,
  queriesJsonPath: string,
): Promise<BenchmarkSummary> {
  const module = loadNativeModule();
  return mapBenchmarkSummary(module.benchmarkArtifact(artifactPath, queriesJsonPath));
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

function mapPlainBuildStats(stats: NativeBuildStats): BuildStats {
  return {
    documentCount: stats.documentCount,
    chunkCount: stats.chunkCount,
  };
}

function mapIncrementalBuildStats(stats: NativeIncrementalBuildStats): IncrementalBuildStats {
  return {
    scannedDocumentCount: stats.scannedDocumentCount,
    newDocumentCount: stats.newDocumentCount,
    changedDocumentCount: stats.changedDocumentCount,
    unchangedDocumentCount: stats.unchangedDocumentCount,
    removedDocumentCount: stats.removedDocumentCount,
    activeDocumentCount: stats.activeDocumentCount,
    activeChunkCount: stats.activeChunkCount,
  };
}

function mapBuildOptions(options: BuildCanonicalBundleOptions): NativeBuildOptions {
  return {
    embeddingBackend: options.embeddingBackend,
    hashingDimensions: options.hashingDimensions,
    model: options.model,
    batchSize: options.batchSize,
    sourceRootId: options.sourceRootId,
    sourceRootPath: options.sourceRootPath,
    targetTokens: options.targetTokens,
    overlapTokens: options.overlapTokens,
  };
}

function mapDirectoryUpdateMode(updateMode: DirectoryUpdateMode): NativeDirectoryUpdateMode {
  return {
    mode: updateMode.mode,
    baseRevision: updateMode.baseRevision,
  };
}

function mapArtifactInfo(info: NativeArtifactInfo): BuildArtifactInfo {
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

function mapBenchmarkSummary(summary: NativeBenchmarkSummary): BenchmarkSummary {
  return {
    fixture: summary.fixture,
    total: summary.total,
    passed: summary.passed,
    failed: summary.failed,
    results: summary.results.map((result) => ({
      name: result.name,
      query: result.query,
      expectedTopHit: result.expectedTopHit,
      actualTopHit: result.actualTopHit,
      passed: result.passed,
    })),
  };
}
