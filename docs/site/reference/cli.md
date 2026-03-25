---
title: CLI
order: 20
date: 2026-03-25
summary: Commands for building, inspecting, and benchmarking indexbind artifacts.
---

# CLI

Current CLI lives in the Rust `indexbind-build` crate.

Main commands:

- `cargo run -p indexbind-build -- build <input-dir> <output-file> [hashing|<model-id>]`
- `cargo run -p indexbind-build -- build-bundle <input-dir> <output-dir> [hashing|<model-id>]`
- `cargo run -p indexbind-build -- inspect <artifact-file>`
- `cargo run -p indexbind-build -- benchmark <artifact-file> <queries-json>`

Examples:

```bash
cargo run -p indexbind-build -- build ./docs ./index.sqlite
cargo run -p indexbind-build -- build-bundle ./docs ./index.bundle
cargo run -p indexbind-build -- inspect ./index.sqlite
cargo run -p indexbind-build -- benchmark ./index.sqlite fixtures/benchmark/basic/queries.json
```

Embedding backend selection:

- `hashing`
- any other string is treated as a `model2vec` model id

If the backend argument is omitted, the current default backend is used.
