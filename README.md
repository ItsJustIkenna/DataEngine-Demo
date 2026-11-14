# DataEngine - Demo Version

> **Note**: This is a public demonstration version with sanitized configurations and example parameters.

## Overview

Market data processing engine built in Rust, designed for low-latency ingestion and normalization.

## Performance

Measured on an 8-vCPU VM: Kraken normalization ~3.6 μs median at ~280k frames/s per core.

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
