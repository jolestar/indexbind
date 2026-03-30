---
title: Getting Started
order: 10
date: 2026-03-25
summary: Install indexbind, build a native artifact and canonical bundle, and run your first query end to end.
---

# Getting Started

This guide takes the shortest full path:

1. install `indexbind`
2. build a native SQLite artifact from a tiny docs folder
3. query it from Node
4. build a canonical bundle from the same documents
5. query that bundle from the web runtime
6. see the incremental cache path for repeated local rebuilds

## Install

```bash
npm install indexbind
```

Supported prebuilt native targets:

- macOS arm64
- macOS x64
- Linux x64 (glibc)

On Windows, use WSL for install, build, and local Node query flows. Native Windows prebuilds are not published.

If your platform does not have a prebuilt native addon, build the native package locally in a Rust toolchain environment:

```bash
npm run build:native:release
```

## Create a Tiny Document Set

Create a minimal folder:

```text
docs/
  rust.md
  workers.md
```

Example content:

```md
# Rust Guide

Rust retrieval guide for local search.
```

```md
# Cloudflare Workers Guide

Workers deployment notes for retrieval.
```

## Build a Native SQLite Artifact

For a local docs folder:

```bash
npx indexbind build ./docs ./index.sqlite
```

## Query It from Node

```ts
import { openIndex } from 'indexbind';

const index = await openIndex('./index.sqlite');
const hits = await index.search('rust guide', {
  topK: 5,
  hybrid: true,
  reranker: {
    kind: 'embedding-v1',
    candidatePoolSize: 25,
  },
});

console.log(hits[0]);
```

You should see a hit shaped roughly like:

```ts
{
  relativePath: 'rust.md',
  title: 'Rust Guide',
  score: 0.9,
  bestMatch: {
    excerpt: 'Rust retrieval guide for local search.',
    ...
  },
  ...
}
```

Use the native SQLite artifact when your runtime is Node and you want the simplest local setup.

You can also sanity-check the artifact from the CLI:

```bash
npx indexbind search ./index.sqlite "rust guide"
npx indexbind search ./index.sqlite "rust guide" --text
```

CLI commands print JSON by default, which is useful for scripts and agents. Add `--text` for a shorter terminal summary.

## Build a Canonical Bundle

The canonical bundle is the portable artifact for browsers and workers:

```bash
npx indexbind build-bundle ./docs ./index.bundle
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
  embeddingBackend: 'model2vec',
});
```

`model2vec` is the default recommended backend when you want the best retrieval quality from `indexbind`. `hashing` remains available as a lighter compatibility-oriented backend.

## Optional Incremental Build Cache

If you rebuild the same local corpus repeatedly, keep a mutable cache and export fresh artifacts from it:

```bash
npx indexbind update-cache ./docs ./.indexbind-cache.sqlite --git-diff
npx indexbind export-artifact ./.indexbind-cache.sqlite ./index.sqlite
npx indexbind export-bundle ./.indexbind-cache.sqlite ./index.bundle
```

Use this path when:

- the corpus is mostly stable
- you are iterating on local content repeatedly
- a host application or script wants to trigger rebuilds incrementally

## Query the Bundle in Web Runtimes

```ts
import { openWebIndex } from 'indexbind/web';

const index = await openWebIndex('./index.bundle');
const hits = await index.search('rust guide');
```

`indexbind/web` requires wasm initialization to succeed. It does not silently fall back to a separate JS query engine.

Use the canonical bundle when you want the same retrieval data to work in browsers, standard workers, or Cloudflare Workers.

## Choose the Artifact

- Use the native SQLite artifact for local Node retrieval.
- Use the canonical bundle for browser and worker runtimes.
- Use the incremental build cache when you want repeated local rebuilds without treating the runtime artifact itself as mutable.
- If your product spans both environments, build both from the same document set.

## Inspect and Benchmark

Inspect a native SQLite artifact:

```bash
npx indexbind inspect ./index.sqlite
npx indexbind inspect ./index.sqlite --text
```

The Rust `indexbind-build` binary still exists for Rust-native workflows, but the npm package now ships the public `indexbind` CLI.

Run the bundled regression fixture:

```bash
npm run benchmark:basic
```
