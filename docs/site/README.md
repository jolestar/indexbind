---
title: indexbind
type: page
order: 0
date: 2026-03-25
summary: Embedded retrieval artifacts for Node, browsers, and Workers.
---

# indexbind

`indexbind` is a retrieval library for fixed document sets.

It builds an artifact offline, then opens that artifact locally in Node, browsers, Web Workers, or Cloudflare Workers.

If you want to start with the shortest path, go to [Getting Started](./guides/getting-started.md). If you want to understand the architecture direction, read [Canonical Artifact and WASM](./concepts/canonical-artifact-and-wasm.md).

## Why It Exists

Most search infrastructure is designed around services, crawlers, or runtime-managed indexes.

`indexbind` takes a different position:

- the document set is fixed at build time
- the artifact is deterministic and portable
- the runtime API is small enough to embed into another product
- the same retrieval model can work in Node, browsers, and Workers

That makes it a better fit for docs systems, local tools, static deployments, and products such as [`mdorigin`](https://mdorigin.jolestar.workers.dev), where embedded retrieval is part of a larger publishing flow.

## What It Does

- builds deterministic retrieval artifacts from a document collection
- supports a native SQLite artifact for Node
- supports a canonical file bundle for web and worker runtimes
- provides a Node build API and query APIs for Node, web, and Cloudflare
- keeps search as an embeddable library concern rather than a hosted service

## Start Here

- [Getting Started](./guides/getting-started.md)
- [Web and Cloudflare](./guides/web-and-cloudflare.md)
- [API](./reference/api.md)
- [CLI](./reference/cli.md)
- [Packaging](./reference/packaging.md)
- [Canonical Bundles](./concepts/canonical-bundles.md)
- [Runtime Model](./concepts/runtime-model.md)
- [Canonical Artifact and WASM](./concepts/canonical-artifact-and-wasm.md)

## Local Preview

If you want to preview this documentation site itself with [`mdorigin`](https://mdorigin.jolestar.workers.dev):

```bash
npm run docs:index
npm run docs:dev
```

<!-- INDEX:START -->

- [Guides](./guides/)
- [Concepts](./concepts/)
- [Reference](./reference/)

<!-- INDEX:END -->
