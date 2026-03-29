---
title: CLI
order: 20
date: 2026-03-25
summary: Commands for building, inspecting, and benchmarking indexbind artifacts.
---

# CLI

Install the npm package, then run the public CLI through `npx indexbind ...` or your package manager's local bin shim.

Main commands:

- `npx indexbind build <input-dir> <output-file> [hashing|<model-id>]`
- `npx indexbind build-bundle <input-dir> <output-dir> [hashing|<model-id>]`
- `npx indexbind update-cache <input-dir> <cache-file> [hashing|<model-id>] [--git-diff] [--git-base <rev>]`
- `npx indexbind export-artifact <cache-file> <output-file>`
- `npx indexbind export-bundle <cache-file> <output-dir>`
- `npx indexbind inspect <artifact-file>`
- `npx indexbind benchmark <artifact-file> <queries-json>`

Examples:

```bash
npx indexbind build ./docs ./index.sqlite
npx indexbind build-bundle ./docs ./index.bundle
npx indexbind update-cache ./docs ./.indexbind-cache.sqlite --git-diff
npx indexbind export-artifact ./.indexbind-cache.sqlite ./index.sqlite
npx indexbind export-bundle ./.indexbind-cache.sqlite ./index.bundle
npx indexbind inspect ./index.sqlite
npx indexbind benchmark ./index.sqlite fixtures/benchmark/basic/queries.json
```

Embedding backend selection:

- `hashing`
- any other string is treated as a `model2vec` model id

If the backend argument is omitted, the current default backend is used.

## Incremental Cache Flow

Recommended sequence:

1. `update-cache` to refresh the mutable build cache
2. `export-artifact` to write a fresh SQLite artifact
3. `export-bundle` to write a fresh canonical bundle when needed

`update-cache` defaults to a full directory scan. Add `--git-diff` to use Git as a change-detection fast path. Add `--git-base <rev>` when you want to diff against a specific revision and still reuse the same cache.

## Trigger Example

One simple local hook pattern is updating the cache after branch changes:

```bash
#!/usr/bin/env bash
set -euo pipefail

npx indexbind update-cache ./docs ./.indexbind-cache.sqlite --git-diff
npx indexbind export-artifact ./.indexbind-cache.sqlite ./index.sqlite
```

This is only an adapter example. The cache logic still lives in the shared incremental engine, so the same flow can also be called from agent scripts, task runners, or a file watcher.

The original Rust `indexbind-build` binary remains available for Rust-native environments and contributor workflows.
