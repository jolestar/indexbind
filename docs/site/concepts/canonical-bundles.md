---
title: Canonical Bundles
order: 10
date: 2026-03-25
summary: The portable file-bundle artifact shared by web and worker runtimes.
---

# Canonical Bundles

The canonical bundle is the product-level artifact for cross-runtime querying.

It is a directory bundle, not a SQLite database. The current minimum shape is:

- `manifest.json`
- `documents.json`
- `chunks.json`
- `vectors.bin`
- `postings.json`
- optional `model/`

## Why It Exists

SQLite works well for native local querying, but it is the wrong format to treat as the long-term cross-runtime contract.

The canonical bundle is easier to:

- serve from static hosting
- inspect directly during development
- load in browsers and workers
- evolve without leaking SQLite-specific schema decisions into the public API

## What the Bundle Stores

- normalized document metadata
- chunk boundaries and excerpts
- dense vectors for semantic search
- postings for lexical scoring
- optional `model2vec` assets for query embedding in wasm

The bundle is retrieval-only. It is not meant to expose a host-specific file read contract.
