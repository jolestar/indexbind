---
title: Web and Cloudflare
order: 20
date: 2026-03-25
summary: Load canonical bundles in browsers, workers, and Cloudflare Workers.
---

# Web and Cloudflare

`indexbind` has two web-facing entrypoints:

- `indexbind/web`
- `indexbind/cloudflare`

They both query canonical bundles, but Cloudflare Workers need a dedicated entry so wasm can load through a static Worker module import.

## Browser or Standard Worker

```ts
import { openWebIndex } from 'indexbind/web';

const index = await openWebIndex('/search/index.bundle');
const hits = await index.search('cloudflare wasm');
```

## Cloudflare Worker

```ts
import { openWebIndex } from 'indexbind/cloudflare';

export default {
  async fetch(request: Request): Promise<Response> {
    const index = await openWebIndex('https://assets.example.com/index.bundle');
    const hits = await index.search(new URL(request.url).searchParams.get('q') ?? '');
    return Response.json(hits);
  },
};
```

## Embedding Backends

Canonical bundles can currently be built with:

- `hashing`
- `model2vec`

For `model2vec`, the build step copies these files into the bundle:

- `model/tokenizer.json`
- `model/config.json`
- `model/model.safetensors`

That lets the wasm runtime embed queries without host filesystem access.

## Package Boundary

The npm package contains runtime code and wasm files.

The bundle artifact contains your actual index data, vectors, postings, and optional `model2vec` model files.
