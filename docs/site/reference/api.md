---
title: API
order: 10
date: 2026-03-25
summary: Node, build, web, and Cloudflare entrypoints.
---

# API

## `indexbind`

Native Node entrypoint for SQLite artifacts:

```ts
import { openIndex } from 'indexbind';

const index = await openIndex('./index.sqlite');
const hits = await index.search('rust guide', {
  topK: 5,
  hybrid: true,
  reranker: { kind: 'embedding-v1', candidatePoolSize: 25 },
  relativePathPrefix: 'guides/',
});
```

## `indexbind/build`

Programmatic canonical bundle build API:

```ts
import { buildCanonicalBundle } from 'indexbind/build';
```

Main input shape:

- `docId?`
- `sourcePath?`
- `relativePath`
- `canonicalUrl?`
- `title?`
- `summary?`
- `content`
- `metadata?`

## `indexbind/web`

Browser and worker entrypoint for canonical bundles:

```ts
import { openWebIndex } from 'indexbind/web';
```

This path requires wasm initialization to succeed.

## `indexbind/cloudflare`

Cloudflare Worker entrypoint:

```ts
import { openWebIndex } from 'indexbind/cloudflare';
```

Use this instead of `indexbind/web` inside Workers so wasm can be loaded through the Worker-compatible static module path.
