# High-Performance Market Data Processing System

A low-latency, high-throughput market data processing system designed for cryptocurrency exchanges, with a focus on Kraken.

## Overview

This system provides a high-performance pipeline for consuming, processing, and distributing market data from cryptocurrency exchanges. It features optimized components for:

- Real-time processing of order book data (L3 market depth)
- Trade data ingestion and distribution
- Persistence of market data for both real-time access and historical analysis
- Efficient message distribution via a publish-subscribe architecture

## Architecture

The system consists of several components working together:

### HostBuilder

The core server component that manages connections to data sources and coordinates the processing pipeline:

- Configurable TCP settings for optimal network performance
- Connection management with health monitoring
- Efficient thread and resource management
- Signal handling for graceful shutdowns

### Market Data Handlers

Specialized handlers for different types of market data:

- Level3Snapshot: Processes full order book snapshots
- Level3Update: Processes incremental updates to order books
- TradeSnapshot: Processes historical trade data
- TradeUpdate: Processes real-time trade updates

### Message Broker

A high-performance pub/sub system for distributing market data:

- Topic-based message routing
- Low-latency message delivery
- Optimized for financial market data
- Support for thousands of concurrent subscribers

### Persistence Layer

Dual-layer persistence strategy for optimal performance:

- Redis for high-speed access to current market state
- PostgreSQL (TimescaleDB) for durable storage and historical analysis

## Performance Optimizations

The system incorporates numerous optimizations for low-latency, high-throughput processing:

### Memory Management
- Pre-allocated buffers and collections
- Minimized heap allocations in critical paths
- Careful string handling to reduce allocations

### Concurrency
- Asynchronous processing with Tokio
- Concurrent database operations
- Lock-free data structures where possible
- Fine-grained locking to minimize contention

### Network Optimizations
- TCP socket tuning (TCP_NODELAY, TCP_QUICKACK)
- Connection pooling
- Efficient binary protocols

### Database Optimizations
- Batch operations for Redis and PostgreSQL
- Connection pooling
- Concurrent queries

### Error Handling
- Fast failure detection
- Retry mechanisms with exponential backoff
- Graceful degradation under load

## Configuration

The system is configured via environment variables and a configuration file:

```
CONFIG_PATH=/path/to/config.json
```

The configuration file specifies:
- Exchange connections
- Symbols to monitor
- Database connection parameters
- Performance tuning parameters

Example configuration:
```json
{
  "exchanges": [
    {
      "name": "Kraken",
      "symbols": ["BTC/USD", "ETH/USD"],
      "depth": 100,
      "websocket_token": "https://api.kraken.com/0/private/GetWebSocketsToken",
      "websockets": [
        {
          "channel": "level3",
          "endpoint": "wss://ws.kraken.com"
        },
        {
          "channel": "trade",
          "endpoint": "wss://ws.kraken.com"
        }
      ]
    }
  ],
  "message_broker": {
    "address": "localhost",
    "port": 8080
  },
  "topics": ["market.data.kraken"]
}
```

## Usage

### Prerequisites

- Rust 1.56+
- PostgreSQL with TimescaleDB extension
- Redis 6.0+
- Access to exchange APIs (API keys configured as needed)

### Building

```bash
cargo build --release
```

### Running

```bash
export CONFIG_PATH=/path/to/config.json
./target/release/program
```

### Docker

```bash
docker build -t market-data-processor .
docker run -v /path/to/config.json:/app/config.json -e CONFIG_PATH=/app/config.json market-data-processor
```

## Monitoring and Metrics

The system provides detailed metrics for monitoring:

- Connection states
- Message throughput
- Processing latencies
- Error rates
- Queue depths

These metrics can be integrated with standard monitoring solutions via logs.

## Performance Tuning

For optimal performance:

1. Set appropriate TCP buffer sizes in the OS
2. Configure thread counts according to available CPU cores
3. Adjust retry strategies based on network conditions
4. Consider CPU affinity settings for critical components
5. Tune Redis and PostgreSQL for your workload

## Testing

Run the test suite:

```bash
cargo test
```

Performance benchmarks:
```bash
cargo bench
```

## Contributing

Contributions are welcome! Please ensure all performance optimizations are benchmarked.

When contributing, focus on:
- Maintaining or improving latency characteristics
- Reducing memory allocations
- Improving error handling
- Adding test coverage

## License

[Your license information here]