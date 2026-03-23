# Benchmark Fixtures

These fixtures provide a minimal retrieval regression baseline.

Current layout:

- `basic/docs/`: fixed source corpus used to build an artifact
- `basic/queries.json`: fixed queries and expected top document path

Example:

```bash
cargo run -p indexbind-build -- build fixtures/benchmark/basic/docs /tmp/indexbind-basic.sqlite hashing
cargo run -p indexbind-build -- benchmark /tmp/indexbind-basic.sqlite fixtures/benchmark/basic/queries.json
```
