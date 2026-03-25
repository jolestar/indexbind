---
title: indexbind
type: page
order: 0
date: 2026-03-25
summary: Embedded retrieval artifacts for Node, browsers, and Workers.
---

# indexbind

`indexbind` is a retrieval library for fixed document sets.

It builds an artifact offline, then opens that artifact locally in Node, browsers, Web Workers, or Cloudflare Workers.

If you want to start with the shortest path, go to [Getting Started](./guides/getting-started.md). If you want to understand the architecture direction, read [Canonical Artifact and WASM](./architecture/canonical-artifact-and-wasm.md).

## Why It Exists

Most search infrastructure is designed around services, crawlers, or runtime-managed indexes.

`indexbind` takes a different position:

- the document set is fixed at build time
- the artifact is deterministic and portable
- the runtime API is small enough to embed into another product
- the same retrieval model can work in Node, browsers, and Workers

That makes it a better fit for docs systems, local tools, static deployments, and products such as `mdorigin`.

## What You Build

`indexbind` currently supports two artifact shapes:

- a native SQLite artifact for Node
- a canonical file bundle for web and worker runtimes

The bundle shape is the long-term cross-runtime contract. The SQLite path stays valuable for native performance and local ergonomics.

## What It Does

- builds deterministic retrieval artifacts from a document collection
- supports a native SQLite artifact for Node
- supports a canonical file bundle for web and worker runtimes
- provides a Node build API and query APIs for Node, web, and Cloudflare
- keeps search as an embeddable library concern rather than a hosted service

## Product Position

`indexbind` is a standalone retrieval engine, but it is also the search foundation for `mdorigin`.

That means the project stays library-first while still proving itself in a real publishing product.

## Runtime Surface

- `indexbind`
  Node API for native SQLite artifact querying
- `indexbind/build`
  programmatic canonical bundle build API
- `indexbind/web`
  browser and worker query runtime backed by wasm
- `indexbind/cloudflare`
  Cloudflare Worker entry with static wasm module loading

## Current Release

The npm package ships the wasm runtime in `dist/wasm` and `dist/wasm-bundler`.

`model2vec` model files are not part of the npm package itself. They are copied into the canonical bundle artifact when you build a bundle with the `model2vec` backend.

## Local Preview

If you want to preview this documentation site itself with `mdorigin`:

```bash
npm run docs:index
npm run docs:dev
```

<!-- INDEX:START -->

- [Guides](./guides/)
- [Concepts](./concepts/)
- [Reference](./reference/)
- [Architecture](./architecture/)

<!-- INDEX:END -->
