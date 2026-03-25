---
title: Getting Started
order: 10
date: 2026-03-25
summary: Install indexbind, build a native artifact or canonical bundle, and execute your first query.
---

# Getting Started

## Install

```bash
npm install indexbind
```

Supported prebuilt native targets:

- macOS arm64
- macOS x64
- Linux x64 (glibc)

If your platform does not have a prebuilt native addon, build the native package locally in a Rust toolchain environment:

```bash
npm run build:native:release
```

## Build a Native SQLite Artifact

For a local docs folder:

```bash
cargo run -p indexbind-build -- build ./docs ./index.sqlite
```

Query it from Node:

```ts
import { openIndex } from 'indexbind';

const index = await openIndex('./index.sqlite');
const hits = await index.search('rust guide', {
  reranker: { candidatePoolSize: 25 },
});
```

## Build a Canonical Bundle

The canonical bundle is the portable artifact for browsers and workers:

```bash
cargo run -p indexbind-build -- build-bundle ./docs ./index.bundle
```

You can also build the same bundle programmatically:

```ts
import { buildCanonicalBundle } from 'indexbind/build';

await buildCanonicalBundle('./index.bundle', [
  {
    relativePath: 'guides/rust.md',
    canonicalUrl: '/guides/rust',
    title: 'Rust Guide',
    summary: 'A minimal retrieval guide.',
    content: '# Rust Guide\n\nRust retrieval guide.',
    metadata: { lang: 'rust' },
  },
], {
  embeddingBackend: 'hashing',
});
```

## Query the Bundle in Web Runtimes

```ts
import { openWebIndex } from 'indexbind/web';

const index = await openWebIndex('./index.bundle');
const hits = await index.search('rust guide');
```

`indexbind/web` requires wasm initialization to succeed. It does not silently fall back to a separate JS query engine.

## Inspect and Benchmark

Inspect a native SQLite artifact:

```bash
cargo run -p indexbind-build -- inspect ./index.sqlite
```

Run the bundled regression fixture:

```bash
npm run benchmark:basic
```
