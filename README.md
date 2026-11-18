# DataEngine (demo)

A Rust market-data ingestion and normalization engine. It connects to Kraken over WebSocket and REST, parses incoming frames with a SIMD-JSON based parser, normalizes them into internal message types, keeps order-book and cache state, and exposes a publisher client for handing the results to a message broker. Components are assembled by a configuration-driven host builder, and a `performance` crate provides latency and throughput counters plus a `benchmark` binary. It is a research and paper-trading codebase, and the configuration in this public demo is sanitized example data.

## Layout

```
hostbuilder/   host builder and runtime
  network/       REST and WebSocket handlers (Kraken), message models, fast_parser (SIMD JSON)
  data/          order book, caching, pipeline
  optimization/  memory-pool and low-level tuning modules
  security/      input validation and error-handling helpers
  infrastructure/ logging and monitoring
config/        typed, validated configuration
performance/   latency and throughput counters
program/       binaries: `program` and `benchmark`
local/         in-repo protocol, publisher, ultra-logger and databaseschema crates
k8s/           Helm chart
Dockerfile, Makefile
```

## Build and test

Requires a recent stable Rust toolchain (edition 2021).

```bash
cargo build --workspace
cargo test --workspace
```

The release profile uses fat LTO, one codegen unit and `panic = "abort"`; the test profile keeps unwinding so tests can run. The `database` feature of `program` is on by default. `Dockerfile`, `Makefile` and `k8s/data-engine-helm` cover container and Kubernetes deployment.

## Tests

`cargo test --workspace` runs about 110 test functions (static count): unit tests in `hostbuilder/src/tests.rs` and the other crates, plus `tests/integration_test.rs`, `tests/system_integration_tests.rs`, `tests/performance_integration_tests.rs` and `tests/memory_pool_validation.rs`.
