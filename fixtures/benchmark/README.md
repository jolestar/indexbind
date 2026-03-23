# Benchmark Fixtures

These fixtures provide a minimal retrieval regression baseline.

Current layout:

- `basic/docs/`: fixed source corpus used to build an artifact
- `basic/queries.json`: fixed queries and expected top document path

Example:

```bash
cargo run -p inkdex-build -- build fixtures/benchmark/basic/docs /tmp/inkdex-basic.sqlite hashing
cargo run -p inkdex-build -- benchmark /tmp/inkdex-basic.sqlite fixtures/benchmark/basic/queries.json
```
