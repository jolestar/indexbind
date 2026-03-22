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

function resolveNativeModule(): NativeModule {
  const candidates = [
    path.join(root, 'native', `inkdex.${platformKey()}.node`),
    path.join(root, 'native', 'inkdex.node'),
  ];

  for (const candidate of candidates) {
    if (existsSync(candidate)) {
      return require(candidate) as NativeModule;
    }
  }

  throw new Error(
    `inkdex native addon not found for ${platformKey()}. Run "npm run build:native" first.`,
  );
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
  originalPath: string;
  relativePath: string;
  title?: string;
  score: number;
  bestMatch: NativeBestMatch;
}

export interface NativeArtifactInfo {
  schemaVersion: string;
  builtAt: string;
  embeddingBackend: string;
  sourceRoot: string;
  documentCount: number;
  chunkCount: number;
}

export interface NativeLoadedDocument {
  originalPath: string;
  relativePath: string;
  content: string;
}

export interface NativeSearchOptions {
  topK?: number;
  hybrid?: boolean;
  relativePathPrefix?: string;
  metadata?: Record<string, string>;
}

export interface NativeIndex {
  info(): NativeArtifactInfo;
  search(query: string, options?: NativeSearchOptions): NativeDocumentHit[];
  readDocument(
    docId: string,
    originalPath: string,
    relativePath: string,
    title: string | undefined,
    score: number,
    bestMatch: NativeBestMatch,
  ): NativeLoadedDocument;
}

export interface NativeModule {
  NativeIndex: {
    open(artifactPath: string, sourceRootOverride?: string): NativeIndex;
  };
}

export function loadNativeModule(): NativeModule {
  return resolveNativeModule();
}
