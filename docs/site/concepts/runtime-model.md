---
title: Runtime Model
order: 20
date: 2026-03-25
summary: The retrieval-only runtime boundary across Node, browsers, and Cloudflare Workers.
---

# Runtime Model

`indexbind` deliberately focuses on retrieval, not content loading.

That means the runtime returns:

- `docId`
- `relativePath`
- `canonicalUrl`
- `title`
- `summary`
- `metadata`
- `score`
- `bestMatch`

It does not provide a public `readDocument()` API any more.

## Why Retrieval-Only

This keeps the runtime portable.

If the library tried to read content directly, it would need host-specific contracts for:

- local filesystem paths
- browser fetch locations
- Cloudflare asset routing

That would weaken the common API surface and make the web runtime much less clean.

Instead, the application takes the hit metadata and decides how to render, fetch, or navigate to the source content.

## Runtime Split

- `indexbind`
  native SQLite query runtime for Node
- `indexbind/build`
  Node build API for canonical bundles
- `indexbind/web`
  wasm-backed canonical bundle runtime for browsers and standard workers
- `indexbind/cloudflare`
  wasm-backed canonical bundle runtime for Cloudflare Workers
