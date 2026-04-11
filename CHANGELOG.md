# Changelog

## 0.6.3

- Removed the vendored `forks/model2vec-rs` copy and switched the workspace to a pinned upstream `model2vec-rs` git dependency.
- Replaced the previous fork-private canonical bundle model asset helper with a local `hf-hub`-backed resolver for native bundle builds.
- Fixed wasm-target canonical asset compilation so the upstream dependency switch builds cleanly across the existing Node and wasm package pipeline.

## 0.6.2

- Added index-scoped `indexbind.build.js` and `indexbind.search.js` conventions so one indexed root can attach document shaping, default search profiles, and lightweight query rewrites to the native `indexbind` pipeline.
- Applied these conventions across CLI and Node directory/search flows without replacing the default directory scanner, incremental cache engine, or artifact contract.
- Expanded docs, skills, and smoke/install coverage to describe and verify the new convention-based extension path.

## 0.6.1

- Fixed the release workflow smoke artifact build step to use the current `indexbind build --backend ...` CLI shape.

## 0.6.0

- Added default CLI build paths under `<input-dir>/.indexbind/`, defaulted build command input roots to the current directory, and switched build/cache selection to explicit `--backend` and `--cache-file` flags.
- Made directory-based ingestion honor hidden paths, `.gitignore`, and common generated/dependency directories such as `node_modules/`, `target/`, `dist/`, and `build/`.
- Removed the legacy root CLI build form so all supported CLI flows now use explicit subcommands.

## 0.5.1

- Fixed the release workflow after the Rust CLI removal by switching the release smoke artifact build step to the npm-first `indexbind` CLI.
- Updated package and workspace versions to `0.5.1` so the hotfix can publish cleanly after the failed `0.5.0` release attempt.

## 0.5.0

- Retired the Rust `indexbind-build` binary and unified all supported CLI workflows on the npm-first `indexbind` command.
- Made explicit CLI help paths succeed with exit code `0` for both the top-level command and subcommands, and added dedicated CLI smoke coverage in CI.
- Replaced the old boolean `hybrid` search flag with explicit retrieval `mode: 'hybrid' | 'vector'` across the Node API, CLI, native bridge, wasm/web runtime, docs, and packaged-install smoke tests.

## 0.4.0

- Added an npm-first `indexbind` CLI so `npm install indexbind` can drive `build`, `build-bundle`, `update-cache`, `export-*`, `inspect`, and `benchmark` flows through `npx indexbind ...`.
- Added directory-oriented build, inspect, and benchmark helpers to `indexbind/build`, keeping the npm CLI and programmatic APIs aligned on the same native implementation.
- Expanded docs with a host-controlled custom index builder example, refreshed the local `indexbind` skill to match the npm CLI and artifact model, and filtered CI so docs-only changes can skip the full Rust and browser validation path.

## 0.3.3

- Fixed the release-time root package verification added in `0.3.2` to avoid requiring non-contract local `dist/` leftovers such as `dist/cloudflare/worker.mjs`, which caused clean CI release builds to fail.

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
