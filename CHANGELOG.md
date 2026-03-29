# Changelog

## 0.3.2

- Fixed the published root npm package to include the full `dist/` tree, restoring `indexbind/web` and `indexbind/cloudflare` imports that broke in `0.3.1` because `dist/web-core.js` was omitted.
- Added release-time root package verification to catch missing files and broken relative imports before publishing npm tarballs.

## 0.3.1

- Fixed `indexbind/cloudflare` and `indexbind/web` bundle loading in real Cloudflare Worker deployments by separating the Worker wasm bootstrap from the generic web runtime and allowing hosts to provide an explicit bundle `fetch` implementation.
- Added a deployable Cloudflare Worker manual testcase plus expanded smoke coverage for direct and virtual bundle loading modes.

## 0.3.0

- Added YAML frontmatter ingestion for directory builds, including `title`, `summary`, `canonical_url`, and metadata extraction.
- Added a document-level incremental build cache with fresh export to SQLite artifacts and canonical bundles.
- Added optional `git diff`-based change detection and trigger-friendly build/update APIs for CLI and programmatic workflows.
- Refreshed the README and documentation landing pages to clarify platform support, artifact paths, local knowledge-base fit, and public positioning.

## 0.2.3

- Improved Chinese lexical tokenization with a shared `mixed-cjk-bigram-v2` tokenizer across SQLite FTS, canonical postings, reranking, and web/wasm runtimes.
- Added explicit lexical tokenizer metadata to artifact inspection and runtime info.
- Fixed tokenizer coverage for newer CJK Unicode extension blocks and removed extra allocations from lexical token counting during chunking.

## 0.2.2

- Added and published a public documentation site at `https://indexbind.jolestar.workers.dev`.
- Reorganized docs into `Guides`, `Concepts`, and `Reference`, and added a packaging reference page.
- Refreshed public project metadata across npm, Cargo, GitHub, and the repository README.

## 0.2.1

- Fixed published root npm package metadata to preserve runtime `dependencies`, including `@noble/hashes` for `indexbind/web` and `indexbind/cloudflare`.
- Fixed CI and release workflows to install the wasm target and `wasm-bindgen-cli` before package builds.
- Upgraded GitHub Actions workflow action versions to Node 24 compatible release lines.

## 0.2.0

- Added canonical file-bundle build and runtime support for `indexbind/web`.
- Added programmatic bundle building via `indexbind/build`.
- Added wasm-backed query runtime coverage for Node workers, browsers, and Cloudflare Workers.
- Added `indexbind/cloudflare` for Cloudflare Worker environments that require static wasm module imports.
- Removed automatic JS fallback from `indexbind/web`; web runtimes now require wasm initialization to succeed.
- Added bundle smoke regressions for web, worker, browser, and Cloudflare Worker environments.
- `model2vec` web bundles now include model assets in the artifact bundle instead of relying on host filesystem access.
