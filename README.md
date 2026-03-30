# indexbind

[![npm version](https://img.shields.io/npm/v/indexbind)](https://www.npmjs.com/package/indexbind)
[![docs](https://img.shields.io/badge/docs-live-0f5bd7)](https://indexbind.jolestar.workers.dev)
[![license](https://img.shields.io/npm/l/indexbind)](./LICENSE)

`indexbind` builds retrieval artifacts offline, then opens them across Node, browsers, and Workers.

Docs: [indexbind.jolestar.workers.dev](https://indexbind.jolestar.workers.dev)

The release history is tracked in [CHANGELOG.md](./CHANGELOG.md).

## What It Is

`indexbind` is an embedded retrieval library for fixed document sets.

Use it when:

- the document collection is known at build time
- retrieval should ship with your product or artifact
- the host application wants to control routing, filtering, and ranking policy
- the same retrieval data should work across Node, browsers, and Workers
- you do not want a hosted search dependency at query time

`indexbind` is not a hosted search service and not a turnkey knowledge-base product. It is a retrieval layer you embed into your own docs system, blog system, local tool, agent workflow, or publishing pipeline.

## Public Contract

- build retrieval artifacts offline
- open a native SQLite artifact in Node
- open a canonical bundle in browsers, Workers, and Cloudflare Workers
- keep results document-first, with chunk evidence in `bestMatch`
- expose build and query APIs through a small library surface

## Install

Install the Node package:

```bash
npm install indexbind
```

On supported platforms, npm also installs the matching native package automatically through `optionalDependencies`.

Current published prebuilt targets:

- macOS arm64
- macOS x64
- Linux x64 (glibc)

Windows native prebuilds are not included. On Windows, use WSL for install, build, and local Node query flows.

If a prebuilt addon is unavailable for your platform, install from source in a Rust toolchain environment and run:

```bash
npm run build:native:release
```

## Start Quickly

1. Build a native SQLite artifact for Node:

```bash
npx indexbind build ./docs ./index.sqlite
```

2. Query it from Node:

```ts
import { openIndex } from 'indexbind';

const index = await openIndex('./index.sqlite');
const hits = await index.search('rust guide');
```

Or query it directly from the CLI:

```bash
npx indexbind search ./index.sqlite "rust guide"
npx indexbind search ./index.sqlite "rust guide" --text
```

3. Build a canonical bundle for browsers and Workers:

```bash
npx indexbind build-bundle ./docs ./index.bundle
```

4. Or keep a mutable build cache and export fresh artifacts from it:

```bash
npx indexbind update-cache ./docs ./.indexbind-cache.sqlite --git-diff
npx indexbind export-artifact ./.indexbind-cache.sqlite ./index.sqlite
```

The npm package now includes the public CLI. Rust users can still run the original `indexbind-build` binary directly.

CLI commands print JSON by default. Add `--text` when you want scan-friendly terminal output.

## Artifact Paths

- Native SQLite artifact:
  best fit for local Node retrieval
- Canonical bundle:
  best fit for browsers, Workers, and Cloudflare Workers
- Incremental build cache:
  best fit for repeated local rebuilds and host-controlled indexing workflows

## Project Shape

The project scope is deliberately narrow:

- take a fixed document collection as input
- build a reusable retrieval artifact offline
- use local embedding models instead of hosted APIs
- expose a small library API that other systems can embed

## Best Fit

`indexbind` works best when you want to:

- build search artifacts from a deterministic document set
- embed retrieval into another product, CLI, or publishing system
- index a local knowledge base while still owning the surrounding workflow
- ship search without depending on a hosted search service
- reuse the same retrieval model across Node, browsers, and Workers

## Not the Best Fit

`indexbind` is usually not the right first choice when you want:

- a hosted search service with dashboards, analytics, and server-side index management
- a turnkey local knowledge-base product with its own end-user workflow
- a static-site search tool where the main requirement is dropping in a prebuilt UI and search script

## Positioning

The easiest way to understand `indexbind` is by comparison:

- `Pagefind` is optimized for static-site search as a packaged product. `indexbind` is a lower-level retrieval library you embed into your own site, app, CLI, or worker.
- `qmd` overlaps with `indexbind` on local knowledge-base search. The main difference is product boundary: `qmd` is closer to a ready-made local search product, while `indexbind` is closer to a retrieval engine you embed into your own system. If your host wants to own routing, filtering, ranking policy, and artifact distribution, `indexbind` is usually the better fit.
- `Meilisearch` is a hosted or self-hosted search service. `indexbind` avoids the service boundary by building an artifact offline and opening it locally at runtime.

That makes `indexbind` a good fit for doc systems, local tools, local knowledge bases with host-defined workflow, publishing pipelines, and agent-facing products that want a reusable retrieval layer.

## Documentation Paths

Use the docs by task:

- [Getting Started](./docs/site/guides/getting-started.md)
- [Choosing indexbind](./docs/site/guides/choosing-indexbind.md)
- [Adoption Examples](./docs/site/guides/adoption-examples.md)
- [Benchmarks and Case Studies](./docs/site/guides/benchmarks-and-case-studies.md)
- [API Reference](./docs/site/reference/api.md)
- [CLI Reference](./docs/site/reference/cli.md)
- [Web and Cloudflare](./docs/site/guides/web-and-cloudflare.md)
- [Packaging](./docs/site/reference/packaging.md)
- [Search Quality Controls](./docs/site/guides/search-quality-controls.md)

## Documentation Site

- [Documentation site](https://indexbind.jolestar.workers.dev)
- [Architecture](./docs/site/concepts/canonical-artifact-and-wasm.md)
- [Documentation site source](./docs/site)

## Name

`Indexbind` reflects the library's shape: build a fixed document set into a reusable retrieval artifact, then open that artifact locally to rank documents.
