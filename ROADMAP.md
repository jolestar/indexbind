# Indexbind Roadmap

`indexbind` should stay narrower than tools like `qmd`.

The core product is:

- a local-first retrieval artifact
- a small embeddable library
- document-first retrieval, not chunk-first retrieval

The roadmap below is organized to protect those constraints while still borrowing the parts of `qmd` that are actually useful.

## Design Position

What `indexbind` should borrow from `qmd`:

- local embedding models
- hybrid retrieval
- explicit query/document formatting before embedding
- clean separation between embedding, retrieval, and optional reranking

What `indexbind` should not copy from `qmd`:

- a chunk-first result model
- a multi-model runtime as the default path
- product/CLI concerns driving core architecture
- a Node-first core runtime

## Phase 1: Stable Core

Goal:

- make artifact format, document identity, and document-first retrieval stable

Required work:

- finalize SQLite artifact schema and schema versioning
- keep `doc_id` stable from source root + relative path
- preserve `original_path` and `relative_path` in the artifact
- keep retrieval results document-first, with chunk evidence only in `bestMatch`
- formalize chunking rules for markdown headings, paragraphs, code blocks, and tables
- define a stable Rust API for build/open/search/read-document
- define a stable Node API on top of the Rust core

Quality target:

- given the same input set, build output is deterministic enough for stable IDs and stable rankings within expected floating point variance

Public API target:

- `buildArtifact(...)`
- `openIndex(...)`
- `search(query, options?) -> DocumentHit[]`
- `readDocument(hit) -> LoadedDocument`

Do not add yet:

- reranker
- query expansion
- multi-model pipeline
- generation features

## Phase 2: Retrieval Quality

Goal:

- improve recall and ranking without changing the document-first API

Required work:

- replace the temporary hashing backend with a real local embedding backend
- add explicit embedding formatting for queries and documents
- add lexical + vector hybrid retrieval as the default scoring path
- add weighted score fusion or RRF internally
- add basic metadata/path filters
- evaluate a better vector backend if brute-force SQLite blobs become a bottleneck

Suggested model/runtime paths:

- ONNX-based embeddings in Rust if distribution is stable
- GGUF-based embeddings only if they fit the library-first packaging model

Quality target:

- improved relevance on natural language queries without returning chunk lists as the primary API

Do not add yet:

- query expansion by default
- reranking by default

## Phase 3: Optional Ranking Layer

Goal:

- make ranking stronger for ambiguous or broad queries while keeping it optional

Required work:

- add an optional reranker stage after initial document candidate generation
- rerank documents, not raw chunks, though chunk evidence may still feed the reranker input
- expose reranking as configuration, not as a required dependency
- keep artifact compatibility with Phase 1/2 indexes

Quality target:

- better top-k ordering on mixed or underspecified queries

Public API target:

- keep `search()` stable
- allow optional runtime config such as reranker enable/disable and candidate pool size

## Phase 4: Tooling

Goal:

- improve usability without changing core architecture

Required work:

- package prebuilt native binaries for mainstream npm platforms
- improve native module loading and unsupported-platform errors
- provide a documented Rust CLI for build workflows
- add artifact inspection/debug commands
- add benchmark fixtures and retrieval regression suites

Possible additions:

- collection management
- incremental rebuild tooling
- artifact stats/health output

These should remain secondary to library correctness.

## Phase 5: Advanced Retrieval Features

Goal:

- selectively adopt higher-complexity ideas from tools like `qmd`

Candidate features:

- query expansion
- multiple embedding model profiles
- language-specific retrieval presets
- alternate ANN backends behind the same document-first API

Rules for this phase:

- every feature must preserve document-first output
- every feature must be optional
- no feature should require turning `indexbind` into a chat app or workflow framework

## Acceptance Criteria By Stage

Phase 1 is complete when:

- artifact schema is versioned
- document identity and path preservation are stable
- Rust and Node can build, open, search, and read documents end-to-end

Phase 2 is complete when:

- a real local embedding backend replaces hashing for normal use
- hybrid retrieval materially improves quality
- public APIs stay stable

Phase 3 is complete when:

- reranking is optional, useful, and does not redefine the primary result object

Phase 4 is complete when:

- third-party installation feels close to a normal npm dependency on mainstream platforms

Phase 5 is complete when:

- advanced features improve quality without distorting the core scope

## Current Priority

The next concrete priority should be:

1. Replace the temporary hashing embedding backend with a real local embedding backend.
2. Add explicit query/document embedding formatting.
3. Upgrade the current retrieval path from simple weighted merge to a more principled hybrid fusion.
4. Keep the document-first API unchanged while doing all of the above.
