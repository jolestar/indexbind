---
title: Search Quality Controls
order: 30
date: 2026-03-26
summary: Understand which search options affect recall, reranking, filtering, and final ordering.
---

# Search Quality Controls

`indexbind` exposes a small set of search knobs. The important distinction is where each knob applies:

- recall: which candidates are allowed into the pool
- reranking: how the candidate pool is reordered
- final ordering: how already-ranked hits are adjusted before returning

## Recall Controls

### `mode`

```ts
const hits = await index.search('rust guide', {
  mode: 'hybrid',
});
```

`mode: 'hybrid'` combines vector and lexical retrieval before the final ranked list is built.

Use it when you want a safer default across exact matches and semantic matches.

```ts
const hits = await index.search('rust guide', {
  mode: 'vector',
});
```

`mode: 'vector'` means vector-only retrieval.

```ts
const hits = await index.search('rust guide', {
  mode: 'lexical',
});
```

`mode: 'lexical'` means lexical-only retrieval.

### `relativePathPrefix`

```ts
const hits = await index.search('rust guide', {
  relativePathPrefix: 'guides/',
});
```

This limits candidate documents to a path prefix before ranking is finalized.

Use it when your application already knows which subtree should be searched.

### `metadata`

```ts
const hits = await index.search('rust guide', {
  metadata: {
    lang: 'rust',
    visibility: 'public',
  },
});
```

Metadata filtering is exact-match filtering. It narrows the candidate set before the final result list is returned.

Use it when your host application needs product-level filtering such as:

- language
- tenant
- content type
- publication state

## Reranking Controls

### `reranker.kind`

```ts
const hits = await index.search('rust guide', {
  reranker: { kind: 'heuristic-v1' },
});
```

Available kinds:

- `heuristic-v1`
- `embedding-v1`

`heuristic-v1` is a lightweight reranker that prefers strong title and heading matches.

`embedding-v1` uses the embedding layer to rerank the candidate pool with stronger semantic judgment.

### `reranker.candidatePoolSize`

```ts
const hits = await index.search('rust guide', {
  topK: 5,
  reranker: {
    kind: 'embedding-v1',
    candidatePoolSize: 25,
  },
});
```

This controls how many candidates reach the reranker before the final `topK` cut.

Increase it when:

- your target document is relevant but keeps missing the final top results
- your collection is noisy
- you use a semantic reranker and need more room for promotion

## Final Ordering Controls

### `scoreAdjustment.metadataNumericMultiplier`

```ts
const hits = await index.search('rust guide', {
  scoreAdjustment: {
    metadataNumericMultiplier: 'directory_weight',
  },
});
```

This multiplies the final score by a numeric metadata field on each hit.

Use it for host-defined ranking priors such as:

- source importance
- content quality
- trust score
- directory or collection weight

This is intentionally generic. `indexbind` does not define what the metadata field means.

### `minScore`

```ts
const hits = await index.search('rust guide', {
  topK: 10,
  minScore: 0.05,
});
```

This drops low-scoring tail hits after reranking and score adjustment.

Use it when you want:

- fewer weak semantic neighbors in the tail
- fewer than `topK` hits when confidence is low
- a stable cutoff for a fixed retrieval profile

`minScore` is most useful when your index, embedding backend, reranker choice, and score-adjustment profile are fixed. Treat it as a profile-tuned tail cutoff, not a universal confidence value across every retrieval configuration.

## Recommended Defaults

For many embedded products, a good starting point is:

```ts
const hits = await index.search(query, {
  topK: 10,
  mode: 'hybrid',
  reranker: {
    kind: 'embedding-v1',
    candidatePoolSize: 25,
  },
});
```

Then layer on `metadata` filters and `scoreAdjustment` only when your host application has clear product rules that should influence ranking.

Add `minScore` once you have a stable search profile and want to trim weak tail matches.

## What These Knobs Do Not Solve

These controls help retrieval quality, but they do not replace:

- domain-specific document normalization
- host-side query rewriting
- custom snippet rendering
- product-level navigation or rendering logic

Those concerns still belong in the application that embeds `indexbind`.
