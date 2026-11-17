# DataEngine - Demo Version

> **Note**: This is a public demonstration version with sanitized configurations and example parameters.

## Overview

Market data processing engine built in Rust for ingesting and normalizing exchange market data.

## Performance

No benchmark results are published for this demo, and the benchmark harness is not part of this branch. Treat any performance figure as unmeasured until you run your own.

## Features

- Exchange market-data ingestion
- Message normalization
- Configuration-driven host builder

## Quick Start

```bash
# Build
cargo build --workspace --release

# Run
cargo run --release --bin program
```

## Disclaimer

For educational purposes only.
