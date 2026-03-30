---
title: Adoption Examples
order: 15
date: 2026-03-29
summary: Map indexbind onto docs, publishing, and local knowledge-base workflows without giving up host control.
---

# Adoption Examples

These examples show where `indexbind` fits in a larger system.

The pattern is consistent across all of them:

- the host application decides what the document set is
- `indexbind` builds or updates retrieval artifacts
- the host application still owns routing, filtering, ranking policy, and rendering

## Docs Site With Browser Search

Use this shape when you want a docs site to ship its search artifact with the site itself.

Typical flow:

1. build a canonical bundle from the docs corpus
2. publish the bundle with the site
3. load it in the browser or worker runtime

```bash
cargo run -p indexbind-build -- build-bundle ./docs ./public/index.bundle
```

```ts
import { openWebIndex } from 'indexbind/web';

const index = await openWebIndex('/index.bundle');
const hits = await index.search('canonical bundle');
```

The host application still owns:

- route generation
- URL structure
- snippet rendering
- UI state and filters
- any host-specific ranking rules layered on top

This is close to how the `indexbind` documentation site is structured today.

## Publishing or Blog System

Use this shape when you already have a normalized content pipeline and want retrieval to stay inside that pipeline.

Typical flow:

1. parse frontmatter and markdown in the host application
2. pass normalized documents into `indexbind/build`
3. export the runtime artifact that matches the final deployment target

```ts
import {
  buildCanonicalBundle,
} from 'indexbind/build';

await buildCanonicalBundle('./dist/search.bundle', [
  {
    relativePath: 'posts/retrieval.md',
    canonicalUrl: '/posts/retrieval',
    title: 'Retrieval Notes',
    summary: 'How the publishing pipeline builds search artifacts.',
    content: '# Retrieval Notes\n\nBuild artifacts during publish.',
    metadata: {
      section: 'blog',
      visibility: 'public',
    },
  },
], {
  embeddingBackend: 'hashing',
});
```

This shape works well when the host already owns:

- frontmatter parsing
- canonical URLs
- taxonomies
- publication state
- product-specific ranking priors

This is also a good fit when the blog or publishing system wants search to be one build output rather than a separate search service.

## Custom Index Builder for a Mixed Local Knowledge Base

Use this shape when the host application wants to decide exactly which directories to scan, how to classify documents, and which metadata or weighting rules should be written into the index.

Typical flow:

1. walk the host-specific content roots
2. normalize each markdown file into a `BuildDocument`
3. infer metadata such as source root, content kind, visibility, or directory weight
4. pass the normalized documents into `indexbind/build`

```ts
import { buildCanonicalBundle } from 'indexbind/build';

const documents = [
  {
    docId: 'public/post-a/README.md',
    sourcePath: '/workspace/public/post-a/README.md',
    relativePath: 'public/post-a/README.md',
    canonicalUrl: 'https://example.com/post-a/',
    title: 'Post A',
    summary: 'Host-defined summary for workspace search.',
    content: '# Post A\n\nHost-controlled markdown content for the public post.',
    metadata: {
      source_root: 'public',
      content_kind: 'public_post',
      is_default_searchable: true,
      directory_weight: 1.0,
    },
  },
  {
    docId: 'research/notes/layer2.md',
    sourcePath: '/workspace/research/notes/layer2.md',
    relativePath: 'research/notes/layer2.md',
    title: 'Layer2 Notes',
    content: '# Layer2 Notes\n\nHost-controlled markdown content for research search.',
    metadata: {
      source_root: 'research',
      content_kind: 'research',
      is_default_searchable: true,
      directory_weight: 0.92,
    },
  },
];

await buildCanonicalBundle('./dist/workspace.bundle', documents, {
  embeddingBackend: 'model2vec',
  sourceRootId: 'workspace',
  sourceRootPath: process.cwd(),
});
```

This shape works well when the host wants to own:

- multi-root directory selection
- frontmatter parsing and custom title or summary rules
- content classification such as `public_post`, `draft`, `research`, or `archive_doc`
- metadata-driven ranking hints such as directory weights or visibility flags
- separate search profiles such as default vs exhaustive search

This is close to how the `workspace` project uses `indexbind`: the host normalizes heterogeneous content first, then hands a controlled document set into `indexbind/build`.

## Local Knowledge Base or Agent Workspace

Use this shape when documents change repeatedly and the host wants to trigger indexing incrementally.

Typical flow:

1. refresh the build cache
2. export a fresh runtime artifact from that cache
3. let the host tool open the new artifact locally

```bash
cargo run -p indexbind-build -- update-cache ./workspace-docs ./.indexbind-cache.sqlite --git-diff
cargo run -p indexbind-build -- export-artifact ./.indexbind-cache.sqlite ./workspace.sqlite
```

```ts
import { openIndex } from 'indexbind';

const index = await openIndex('./workspace.sqlite');
const hits = await index.search('incremental indexing');
```

This shape fits:

- local knowledge bases with host-defined workflow
- agent-driven documentation refreshes
- git-hook or task-runner triggered rebuilds
- products that want an embedded retrieval layer instead of a mutable local-store search product

## Picking Between the Three

- Prefer the docs-site pattern when the runtime target is browser or worker first.
- Prefer the publishing pattern when the host already has a structured content pipeline.
- Prefer the custom-builder pattern when the host needs to classify mixed local content before indexing.
- Prefer the local knowledge-base pattern when incremental rebuilds and local Node queries matter more than browser distribution.

If you need the full decision frame, go back to [Choosing indexbind](./choosing-indexbind.md). If you want indicative local measurements and current in-house usage notes, see [Benchmarks and Case Studies](./benchmarks-and-case-studies.md).
