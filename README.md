# inkdex

`inkdex` is a working codename for a local-first document retrieval library.

The project goal is deliberately narrow:

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

## Non-Goals

`inkdex` is not intended to be:

- a chat application
- a hosted vector database
- a full search product with UI, auth, sync, and server infrastructure
- an MCP server by default
- a workflow engine for ingestion pipelines
- a replacement for general-purpose RAG frameworks

## Scope

The initial prototype should only answer these questions:

1. What should the index artifact format be?
2. How should documents be chunked and represented?
3. Which local embedding/runtime options are practical?
4. What is the smallest useful query API?

Everything else should stay secondary until those decisions are stable.

## Proposed Shape

The likely shape of the project is:

- a build step that accepts normalized document inputs
- a local embedding step
- a compact persisted index artifact
- a runtime library that can open that artifact and return ranked matches

## Current Workflow

Build an artifact from a local docs directory:

```bash
cargo run -p inkdex-build -- build ./docs ./index.sqlite
```

Inspect an existing artifact:

```bash
cargo run -p inkdex-build -- inspect ./index.sqlite
```

Run the bundled benchmark fixture:

```bash
cargo run -p inkdex-build -- build fixtures/benchmark/basic/docs /tmp/inkdex-basic.sqlite hashing
cargo run -p inkdex-build -- benchmark /tmp/inkdex-basic.sqlite fixtures/benchmark/basic/queries.json
```

From Node, open the artifact and run document-first search:

```ts
import { openIndex } from 'inkdex';

const index = await openIndex('./index.sqlite');
const hits = await index.search('rust guide', {
  reranker: { candidatePoolSize: 25 },
});
```

Current native loading behavior:

- local development prefers `native/inkdex.<platform>.node`
- packaged installs can fall back to platform packages such as `@inkdex/native-darwin-x64`
- unsupported or missing native targets now return an explicit platform-specific error

## Design Constraints

- local-first
- deterministic builds
- library-first
- artifact-first
- simple enough to embed in other systems

## Naming

`inkdex` is a codename, not a final public name.

The project should earn a permanent name after the artifact format, API, and scope are proven.
