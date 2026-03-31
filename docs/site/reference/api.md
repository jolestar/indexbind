---
title: API
order: 10
date: 2026-03-25
summary: Node, build, web, and Cloudflare entrypoints, plus the current search option surface.
---

# API

`indexbind` has four runtime-facing entrypoints:

- `indexbind`
- `indexbind/build`
- `indexbind/web`
- `indexbind/cloudflare`

## `indexbind`

Native Node entrypoint for SQLite artifacts:

```ts
import { openIndex } from 'indexbind';

const index = await openIndex('./index.sqlite');
const hits = await index.search('rust guide', {
  topK: 5,
  mode: 'hybrid',
  reranker: { kind: 'embedding-v1', candidatePoolSize: 25 },
  relativePathPrefix: 'guides/',
});
```

### `openIndex(artifactPath)`

Opens a native SQLite artifact and returns an `Index`.

### `index.info()`

Returns artifact metadata such as:

- `schemaVersion`
- `builtAt`
- `embeddingBackend`
- `lexicalTokenizer`
- `sourceRoot`
- `documentCount`
- `chunkCount`

### `index.search(query, options?)`

Main options:

- `topK?`: number of hits to return
- `mode?`: `'hybrid'`, `'vector'`, or `'lexical'`
- `minScore?`: prune low-confidence tail hits after final scoring
- `reranker?`: optional final reranking stage
- `relativePathPrefix?`: restrict retrieval to a path subtree
- `metadata?`: exact-match metadata filter
- `scoreAdjustment?`: adjust final ranking using metadata-driven multipliers

On the Node entrypoint, `metadata` is currently exposed as a string-to-string map in the TypeScript API.

Reranker options:

- `kind?`: `embedding-v1` or `heuristic-v1`
- `candidatePoolSize?`: candidate count forwarded into the reranker before final `topK`

Score-adjustment options:

- `metadataNumericMultiplier?`: metadata field name whose numeric value should multiply the final score

The returned hits include:

- `docId`
- `relativePath`
- `canonicalUrl?`
- `title?`
- `summary?`
- `metadata`
- `score`
- `bestMatch`

`bestMatch` contains:

- `chunkId`
- `excerpt`
- `headingPath`
- `charStart`
- `charEnd`
- `score`

## `indexbind/build`

Programmatic build and incremental cache API:

```ts
import {
  buildCanonicalBundle,
  updateBuildCache,
  exportArtifactFromBuildCache,
  exportCanonicalBundleFromBuildCache,
} from 'indexbind/build';
```

Main input shape:

- `docId?`
- `sourcePath?`
- `relativePath`
- `canonicalUrl?`
- `title?`
- `summary?`
- `content`
- `metadata?`

Use this entrypoint when your host application already has a normalized document set and wants to build directly from code instead of scanning a directory through the CLI.

For a larger mixed-content example where the host classifies documents and injects metadata before indexing, see [Adoption Examples](../guides/adoption-examples.md).

Available helpers:

- `buildCanonicalBundle(outputDir, documents, options?)`
- `buildFromDirectory(inputDir, outputPath, options?)`
- `buildCanonicalBundleFromDirectory(inputDir, outputDir, options?)`
- `updateBuildCache(cachePath, documents, options?, removedRelativePaths?)`
- `updateBuildCacheFromDirectory(inputDir, cachePath, options?, updateMode?)`
- `exportArtifactFromBuildCache(cachePath, outputPath)`
- `exportCanonicalBundleFromBuildCache(cachePath, outputDir)`
- `inspectArtifact(artifactPath)`
- `benchmarkArtifact(artifactPath, queriesJsonPath)`

`updateBuildCache(...)` returns:

- `scannedDocumentCount`
- `newDocumentCount`
- `changedDocumentCount`
- `unchangedDocumentCount`
- `removedDocumentCount`
- `activeDocumentCount`
- `activeChunkCount`

Typical incremental flow:

```ts
import {
  updateBuildCache,
  exportArtifactFromBuildCache,
  exportCanonicalBundleFromBuildCache,
} from 'indexbind/build';

await updateBuildCache(
  './.indexbind-cache.sqlite',
  [
    {
      relativePath: 'guides/rust.md',
      title: 'Rust Guide',
      content: '# Rust Guide\n\nRust retrieval guide.',
      metadata: { lang: 'rust' },
    },
  ],
  { embeddingBackend: 'hashing' },
  ['guides/old.md'],
);

await exportArtifactFromBuildCache('./.indexbind-cache.sqlite', './index.sqlite');
await exportCanonicalBundleFromBuildCache('./.indexbind-cache.sqlite', './index.bundle');
```

## `indexbind/web`

Browser and worker entrypoint for canonical bundles:

```ts
import { openWebIndex } from 'indexbind/web';
```

This path requires wasm initialization to succeed.

`openWebIndex(base, options?)` returns a `WebIndex`.

Optional open-time options:

- `fetch?`: override resource loading for canonical bundle files when the host wants to virtualize bundle storage

`WebIndex.info()` returns canonical bundle metadata such as:

- `schemaVersion`
- `artifactFormat`
- `builtAt`
- `embeddingBackend`
- `documentCount`
- `chunkCount`
- `vectorDimensions`
- `chunking`
- `features`

`WebIndex.search(query, options?)` accepts the same search options as the Node entrypoint, except metadata values can use the broader JSON value shape.

## `indexbind/cloudflare`

Cloudflare Worker entrypoint:

```ts
import { openWebIndex } from 'indexbind/cloudflare';
```

Use this instead of `indexbind/web` inside Workers so wasm can be loaded through the Worker-compatible static module path.

It accepts the same optional `fetch` override as `indexbind/web`, which is useful when the host wants to read bundle files through `ASSETS.fetch(...)` instead of public URLs.

## Search Defaults and Patterns

Reasonable starting point:

```ts
const hits = await index.search(query, {
  topK: 10,
  mode: 'hybrid',
  reranker: {
    kind: 'embedding-v1',
    candidatePoolSize: 25,
  },
});
```

Use metadata filtering when your host application has clear product boundaries:

```ts
const hits = await index.search(query, {
  metadata: {
    lang: 'rust',
    visibility: 'public',
  },
});
```

Use metadata-based score adjustment when your application wants a host-defined ranking prior:

```ts
const hits = await index.search(query, {
  scoreAdjustment: {
    metadataNumericMultiplier: 'directory_weight',
  },
});
```

Use `minScore` when your product wants to cut weak tail matches and allow fewer than `topK` hits:

```ts
const hits = await index.search(query, {
  topK: 10,
  minScore: 0.05,
});
```

- `mode: 'vector'` means vector-only retrieval.
- `mode: 'lexical'` means lexical-only retrieval.

For a fuller explanation of how these knobs interact, see [Search Quality Controls](../guides/search-quality-controls.md).
