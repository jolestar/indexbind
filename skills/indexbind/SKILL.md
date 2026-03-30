---
name: indexbind
description: Use when an agent needs to install or use indexbind from Node, browsers, Web Workers, or Cloudflare Workers. This skill helps choose the right package, CLI, artifact, and entrypoint, and points to the live markdown docs for details.
---

# Indexbind

Use this skill when the task is about using `indexbind` from a host application or environment.

## Install

Install the package:

```bash
npm install indexbind
```

Optional global install when the goal is using `indexbind` as a shell command from arbitrary directories:

```bash
npm install -g indexbind
```

Then use either:

- `npx indexbind ...` for local installs and per-project workflows
- `indexbind ...` after a global install
- `import ... from 'indexbind'` or `indexbind/build` for programmatic usage

Platform notes:
- native prebuilds exist for macOS arm64, macOS x64, and Linux x64 (glibc)
- Windows usage should go through WSL

Install and packaging docs:
- `https://indexbind.jolestar.workers.dev/guides/getting-started.md`
- `https://indexbind.jolestar.workers.dev/reference/packaging.md`

## Choose the right interface

- Index a local docs folder or local knowledge-base directory from the shell:
  use `npx indexbind ...`
- Local Node querying over a built SQLite artifact:
  use `indexbind`
- Programmatic build, incremental cache update, inspect, or benchmark:
  use `indexbind/build`
- Mixed local knowledge bases that need host-defined document classification, metadata, or directory weighting:
  normalize documents in the host first, then pass them to `indexbind/build`
- Browser or standard worker querying over a canonical bundle:
  use `indexbind/web`
- Cloudflare Worker querying:
  use `indexbind/cloudflare`
- Shell-driven build/update/export/inspect flows:
  use `npx indexbind ...`

API docs:
- `https://indexbind.jolestar.workers.dev/reference/api.md`
- `https://indexbind.jolestar.workers.dev/reference/cli.md`

## Choose the artifact

- Local directory indexing for later Node queries:
  build a native SQLite artifact
- Local directory indexing for browser or worker delivery:
  build a canonical bundle
- Node runtime:
  use a native SQLite artifact
- Browser, Web Worker, Cloudflare Worker:
  use a canonical bundle
- Repeated rebuilds over a stable corpus:
  use the build cache, then export fresh artifacts or bundles

Concepts:
- `https://indexbind.jolestar.workers.dev/concepts/runtime-model.md`
- `https://indexbind.jolestar.workers.dev/concepts/canonical-bundles.md`

## Common commands

Typical CLI commands:

- `npx indexbind build ./docs ./index.sqlite`
- `npx indexbind build-bundle ./docs ./index.bundle`
- `npx indexbind update-cache ./docs ./.indexbind-cache.sqlite --git-diff`
- `npx indexbind build <input-dir> <output-file>`
- `npx indexbind build-bundle <input-dir> <output-dir>`
- `npx indexbind update-cache <input-dir> <cache-file> [--git-diff] [--git-base <rev>]`
- `npx indexbind export-artifact <cache-file> <output-file>`
- `npx indexbind export-bundle <cache-file> <output-dir>`
- `npx indexbind inspect <artifact-file>`
- `npx indexbind benchmark <artifact-file> <queries-json>`

Use `indexbind/build` instead when the host already has documents in memory or wants tighter control from code.

## Common APIs

Use these APIs when the host already has documents or wants tighter control:

- `openIndex(...)` from `indexbind`
- `buildFromDirectory(...)` from `indexbind/build`
- `buildCanonicalBundle(...)` from `indexbind/build`
- `buildCanonicalBundleFromDirectory(...)` from `indexbind/build`
- `updateBuildCache(...)` from `indexbind/build`
- `updateBuildCacheFromDirectory(...)` from `indexbind/build`
- `exportArtifactFromBuildCache(...)` from `indexbind/build`
- `exportCanonicalBundleFromBuildCache(...)` from `indexbind/build`
- `inspectArtifact(...)` from `indexbind/build`
- `benchmarkArtifact(...)` from `indexbind/build`
- `openWebIndex(...)` from `indexbind/web`
- `openWebIndex(...)` from `indexbind/cloudflare`

Docs:
- `https://indexbind.jolestar.workers.dev/reference/api.md`
- `https://indexbind.jolestar.workers.dev/guides/adoption-examples.md`

## Cloudflare rule

Inside Cloudflare Workers:

- prefer `indexbind/cloudflare`
- if bundle files are not directly exposed as public URLs, pass a custom `fetch` to `openWebIndex(...)`
- use the host asset loader such as `ASSETS.fetch(...)` rather than monkey-patching global fetch

Docs:
- `https://indexbind.jolestar.workers.dev/guides/web-and-cloudflare.md`
- `https://indexbind.jolestar.workers.dev/reference/api.md`

## Read in this order when unsure

1. `https://indexbind.jolestar.workers.dev/guides/getting-started.md`
2. `https://indexbind.jolestar.workers.dev/reference/api.md`
3. `https://indexbind.jolestar.workers.dev/reference/cli.md`
4. `https://indexbind.jolestar.workers.dev/guides/web-and-cloudflare.md`
