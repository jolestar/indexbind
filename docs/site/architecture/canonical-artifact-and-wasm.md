---
title: Canonical Artifact and WASM
order: 10
date: 2026-03-25
summary: Retrieval-only contracts, canonical file artifacts, and wasm-backed query runtimes.
---

# Canonical Artifact and WASM

This page is the short architecture version of the longer design work tracked in the repository.

## Direction

The long-term design is:

1. keep `indexbind` retrieval-only
2. define a canonical file artifact for cross-runtime querying
3. use wasm as the shared web and worker query runtime
4. keep SQLite as a native optimization path rather than the cross-runtime public contract

## Main Decisions

### Retrieval-only API

The runtime returns ranked hits and metadata, not direct document reads.

### Canonical Artifact

The portable artifact is a file bundle containing manifest, documents, chunks, vectors, postings, and optional model assets.

### WASM Query Runtime

`indexbind/web` and `indexbind/cloudflare` use wasm-backed query execution over that canonical bundle.

### Native Path

Node still supports SQLite artifacts through the native addon, but that is no longer the only product-level artifact shape.

## Why This Matters

This split gives the project a cleaner product shape:

- native querying can stay fast and practical
- web and worker runtimes can share a real retrieval engine
- the public API no longer depends on host filesystem assumptions

For the full design notes, see [the repository architecture document](../../architecture/canonical-artifact-and-wasm.md).
