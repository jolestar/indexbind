---
title: Packaging
order: 30
date: 2026-03-25
summary: npm package boundaries, native packages, wasm assets, and canonical bundle model files.
---

# Packaging

The published npm release is split into:

- `indexbind` for the TypeScript API, wasm runtime files, and native loader
- platform packages such as `@indexbind/native-darwin-x64` for prebuilt NAPI binaries

In normal use you install `indexbind` only. npm resolves the matching native package automatically when a supported prebuilt target exists.

## What Ships in the npm Package

The root package contains:

- runtime entrypoints such as `indexbind`, `indexbind/build`, `indexbind/web`, and `indexbind/cloudflare`
- wasm runtime files in `dist/wasm` and `dist/wasm-bundler`
- the native loader that resolves prebuilt platform packages when they exist

## What Ships in the Canonical Bundle

The canonical bundle contains your retrieval data:

- manifest
- documents
- chunks
- vectors
- postings
- optional model assets

When you build with `model2vec`, these files are copied into the bundle:

- `model/tokenizer.json`
- `model/config.json`
- `model/model.safetensors`

Those model files are not shipped inside the root npm package.
