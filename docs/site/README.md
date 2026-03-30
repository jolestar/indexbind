---
title: indexbind
type: page
order: 0
date: 2026-03-25
summary: Embedded retrieval artifacts for Node, browsers, and Workers.
---

# indexbind

`indexbind` is a retrieval library for fixed document sets.

It builds retrieval artifacts offline, then opens them locally in Node, browsers, Web Workers, or Cloudflare Workers.

If you want the shortest path, start with [Getting Started](./guides/getting-started.md). If you first need to decide whether `indexbind` is the right tool, read [Choosing indexbind](./guides/choosing-indexbind.md).

## What It Optimizes For

Most search infrastructure is designed around services, crawlers, or runtime-managed indexes.

`indexbind` takes a different position:

- the document set is fixed at build time
- the retrieval artifact is deterministic and portable
- the runtime API is small enough to embed into another product
- the same retrieval model can work in Node, browsers, and Workers
- the host application can still own routing, filtering, and ranking policy

That makes it a better fit for docs systems, local tools, local knowledge bases with host-defined workflow, static deployments, and products such as [`mdorigin`](https://mdorigin.jolestar.workers.dev), where embedded retrieval is part of a larger publishing flow.

## Choose The Right Tool

`indexbind` is a better fit when you need an embedded retrieval layer. It is not trying to be:

- a hosted search service
- a turnkey knowledge-base product
- a static-site-only search widget

If that decision is still unclear, go to [Choosing indexbind](./guides/choosing-indexbind.md).

## Main Paths

- builds deterministic retrieval artifacts from a document collection
- supports a native SQLite artifact for Node
- supports a canonical file bundle for web and worker runtimes
- provides a Node build API and query APIs for Node, web, and Cloudflare
- supports an incremental build cache with fresh export to artifacts and bundles
- keeps search as an embeddable library concern rather than a hosted service

## Start By Need

- Want the shortest end-to-end path:
  [Getting Started](./guides/getting-started.md)
- Want to decide whether it fits better than Pagefind, qmd, or Meilisearch:
  [Choosing indexbind](./guides/choosing-indexbind.md)
- Want concrete integration shapes for docs, publishing, or local knowledge-base workflows:
  [Adoption Examples](./guides/adoption-examples.md)
- Want an indicative local baseline and current in-house usage patterns:
  [Benchmarks and Case Studies](./guides/benchmarks-and-case-studies.md)
- Want to integrate from code:
  [API](./reference/api.md)
- Want to drive builds from the CLI:
  [CLI](./reference/cli.md)
- Want browser or Worker usage:
  [Web and Cloudflare](./guides/web-and-cloudflare.md)
- Want to understand packaging and artifact shapes:
  [Packaging](./reference/packaging.md)

## Current Platform Support

- Native prebuilds are published for macOS arm64, macOS x64, and Linux x64 (glibc).
- Windows native prebuilds are not published; use WSL for install, build, and local Node query flows.
- Canonical bundle runtimes work across browsers, Workers, and Cloudflare Workers.

## Docs Map

- [Getting Started](./guides/getting-started.md)
- [Choosing indexbind](./guides/choosing-indexbind.md)
- [Adoption Examples](./guides/adoption-examples.md)
- [Benchmarks and Case Studies](./guides/benchmarks-and-case-studies.md)
- [Search Quality Controls](./guides/search-quality-controls.md)
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
  <!-- mdorigin:index kind=directory -->

- [Concepts](./concepts/)
  <!-- mdorigin:index kind=directory -->

- [Reference](./reference/)
  <!-- mdorigin:index kind=directory -->

- [skills](./skills/)
  <!-- mdorigin:index kind=directory -->

<!-- INDEX:END -->
