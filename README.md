# indexbind

[![npm version](https://img.shields.io/npm/v/indexbind)](https://www.npmjs.com/package/indexbind)
[![docs](https://img.shields.io/badge/docs-live-0f5bd7)](https://indexbind.jolestar.workers.dev)
[![license](https://img.shields.io/npm/l/indexbind)](./LICENSE)

`indexbind` builds retrieval artifacts offline, then opens them across Node, browsers, and Workers.

Docs: [indexbind.jolestar.workers.dev](https://indexbind.jolestar.workers.dev)

The release history is tracked in [CHANGELOG.md](./CHANGELOG.md).

## Design Constraints

- local-first
- deterministic builds
- library-first
- artifact-first
- simple enough to embed in other systems

## Install

Install the Node package:

```bash
npm install indexbind
```

On supported platforms, npm will also install the matching prebuilt native package automatically through `optionalDependencies`.

Supported prebuilt targets in the initial release:

- macOS arm64
- macOS x64
- Linux x64 (glibc)

Windows native prebuilds are not included in the initial release. On Windows, use WSL and run `npm install indexbind` inside the WSL environment so the Linux x64 native package can be resolved there.

If a prebuilt addon is unavailable for your platform, install from source in a Rust toolchain environment and run:

```bash
npm run build:native:release
```

The project scope is deliberately narrow:

- take a fixed document collection as input
- build a reusable retrieval artifact offline
- use local embedding models instead of hosted APIs
- expose a small library API that other systems can embed

## Goals

- build indexes from deterministic document sets such as markdown repositories, docs folders, or exported knowledge bases
- generate a portable artifact, preferably a single-file index or a similarly compact package
- support local embedding generation with no required remote API dependency
- make retrieval available through a small library surface, not only a standalone app
- keep the output suitable for integration into CLIs, agents, local apps, and publishing systems

## Best Fit

`indexbind` works best when you want to:

- build search artifacts from a deterministic document set
- embed retrieval into another product, CLI, or publishing system
- ship search without depending on a hosted search service
- reuse the same retrieval model across Node, browsers, and Workers

## Documentation

- [Documentation site](https://indexbind.jolestar.workers.dev)
- [Getting Started](./docs/site/guides/getting-started.md)
- [Web and Cloudflare](./docs/site/guides/web-and-cloudflare.md)
- [Canonical Bundles](./docs/site/concepts/canonical-bundles.md)
- [Runtime Model](./docs/site/concepts/runtime-model.md)
- [API Reference](./docs/site/reference/api.md)
- [CLI Reference](./docs/site/reference/cli.md)
- [Architecture](./docs/site/concepts/canonical-artifact-and-wasm.md)
- [Documentation site source](./docs/site)

## Name

`Indexbind` reflects the library's shape: build a fixed document set into a reusable retrieval artifact, then open that artifact locally to rank documents.
