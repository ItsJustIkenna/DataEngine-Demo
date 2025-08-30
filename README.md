# DataEngine

> **Ultra-High-Performance Real-Time Market Data Processing Engine**

A production-grade, institutional-quality market data processing engine built in Rust, designed for ultra-low latency trading systems. Processes real-time market data from cryptocurrency exchanges with sub-millisecond performance while providing flexible deployment options for both full messaging and database-only modes.

## 🚀 Key Features

### Performance Excellence
- **Sub-millisecond Processing**: < 1ms end-to-end latency for critical path operations
- **Ultra-Low Latency**: SIMD-optimized message parsing with zero-copy operations
- **Lock-Free Architecture**: Memory-efficient concurrent data structures
- **Kernel Bypass**: Direct hardware access for maximum throughput
- **RDTSC Precision**: CPU cycle-level timing measurements

### Market Data Processing
- **Real-Time WebSocket Streams**: Live market data from Kraken and other exchanges
- **Order Book Management**: Lock-free Level 3 order book processing
- **Trade Data Processing**: Real-time trade execution data handling
- **Balance Tracking**: Account balance snapshots and updates
- **Multi-Symbol Support**: Concurrent processing of multiple trading pairs

### Data Persistence
- **PostgreSQL + TimescaleDB**: Time-series optimized storage for historical data
- **Redis Caching**: High-performance in-memory caching layer
- **Connection Pooling**: Optimized database connection management
- **Transactional Integrity**: ACID compliance for critical financial data

### Deployment Flexibility
- **Database-Only Mode**: Data collection and persistence without message broadcasting
- **Full Messaging Mode**: Complete real-time data distribution pipeline
- **Environment-Based Configuration**: Simple deployment mode switching
- **Kubernetes Ready**: Production-grade container orchestration support

## 📊 Performance Benchmarks

| Metric | Performance | Use Case |
|--------|-------------|----------|
| **End-to-End Latency** | 402μs | Complete message processing pipeline |
| **Order Book Updates** | < 100μs | Level 3 order book modifications |
| **Message Parsing** | < 50μs | SIMD-optimized JSON parsing |
| **Database Writes** | < 200μs | PostgreSQL + TimescaleDB inserts |
| **Redis Operations** | < 10μs | Cache reads/writes |
| **Memory Usage** | < 500MB | Peak memory footprint |
| **CPU Utilization** | < 15% | At 10,000 messages/second |
| **Throughput** | 50,000+ msg/s | Sustained message processing |

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        DataEngine Architecture                   │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │   WebSocket     │  │   Fast Message  │  │   SIMD Message  │  │
│  │   Handlers      │──│     Parser      │──│    Processor    │  │
│  │                 │  │                 │  │                 │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘  │
│           │                       │                       │      │
│           └───────────────────────┼───────────────────────┘      │
│                                   │                              │
│  ┌─────────────────────────────────────────────────────────────┐ │
│  │                Order Book Manager                          │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐       │ │
│  │  │   Level 3   │  │   Balances  │  │    Trades   │       │ │
│  │  │  Processor  │  │  Processor  │  │  Processor  │       │ │
│  │  └─────────────┘  └─────────────┘  └─────────────┘       │ │
│  └─────────────────────────────────────────────────────────────┘ │
│           │                       │                       │      │
│           ▼                       ▼                       ▼      │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  │
│  │  PostgreSQL +   │  │     Redis       │  │  Message Broker │  │
│  │   TimescaleDB   │  │    Caching      │  │   (Optional)    │  │
│  │   Persistence   │  │                 │  │   Publishing    │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

### Core Components

1. **WebSocket Handler**: Real-time connection management with automatic reconnection
2. **Fast Message Parser**: SIMD-optimized JSON parsing with field caching
3. **Order Book Processor**: Lock-free Level 3 order book management
4. **Database Layer**: PostgreSQL + TimescaleDB with connection pooling
5. **Caching Layer**: Redis-based high-performance data caching
6. **Message Broker**: Optional real-time data distribution
7. **Metrics System**: Ultra-precision performance monitoring

## 🚀 Quick Start

### Prerequisites

- **Rust**: 1.70+ (stable toolchain)
- **PostgreSQL**: 12+ with TimescaleDB extension
- **Redis**: 6.0+
- **Docker**: 20.10+ (optional)
- **Kubernetes**: 1.20+ (for production deployment)

### Installation

```bash
# Clone the repository
git clone https://github.com/your-org/tradingplatform.git
cd tradingplatform/DataEngine

# Build the project
cargo build --release

# Run tests
cargo test

# Start the DataEngine
cargo run --release
```

### Docker Quick Start

```bash
# Build the Docker image
docker build -t dataengine:latest .

# Run with docker-compose
docker-compose up -d
```

### Configuration

Create a `config.toml` file:

```toml
[database]
host = "localhost"
port = 5432
username = "dataengine"
password = "your_password"
database = "trading_data"
max_connections = 20

[redis]
host = "localhost"
port = 6379
max_connections = 10

[kraken]
websocket_url = "wss://ws.kraken.com"
symbols = ["XBT/USD", "ETH/USD", "SOL/USD"]
reconnect_delay = 5000

[message_broker]
host = "localhost"
port = 9092
topics = ["market.data.kraken", "order.book.updates"]

[performance]
enable_simd = true
enable_kernel_bypass = true
memory_pool_size = 1048576
```

## 🔧 Deployment Modes

### Database-Only Mode (Helm)

Perfect for data collection and storage without real-time distribution:

```bash
# Deploy with database-only values
helm install dataengine-db-only ./k8s/data-engine-helm \
  -f ./k8s/data-engine-helm/values-database-only.yaml

# Or override values directly
helm install dataengine ./k8s/data-engine-helm \
  --set config.database_only_mode=true \
  --set config.disable_message_broker=true
```

**Features**:
- ✅ Full database operations (PostgreSQL + TimescaleDB + Redis)
- ✅ Real-time WebSocket data processing
- ✅ Order book management and caching
- ❌ Message broker publishing disabled
- 💡 ~100MB less memory usage, ~10% better CPU efficiency

### Full Messaging Mode (Helm)

Complete real-time data processing and distribution:

```bash
# Deploy with default values (full messaging)
helm install dataengine ./k8s/data-engine-helm

# Deploy with production values
helm install dataengine ./k8s/data-engine-helm \
  -f ./k8s/data-engine-helm/values-prod.yaml
```

**Features**:
- ✅ All database operations
- ✅ Real-time WebSocket data processing
- ✅ Order book management and caching
- ✅ Message broker publishing enabled
- ✅ Cross-service real-time data distribution

### Local Development

For local development without Kubernetes:

```bash
# Database-only mode
export DATABASE_ONLY_MODE=true
export DISABLE_MESSAGE_BROKER=true
cargo run --release

# Full messaging mode (default)
cargo run --release
```

### Kubernetes Deployment

#### Production Deployment (Full Messaging)

```bash
# Deploy with production values
helm install dataengine ./k8s/data-engine-helm \
  -f ./k8s/data-engine-helm/values-prod.yaml \
  --namespace trading-platform \
  --create-namespace

# Check deployment status
kubectl get pods -n trading-platform
kubectl logs -f deployment/dataengine -n trading-platform
```

#### Database-Only Deployment

```bash
# Deploy database-only mode
helm install dataengine-db-only ./k8s/data-engine-helm \
  -f ./k8s/data-engine-helm/values-database-only.yaml \
  --namespace data-collection \
  --create-namespace
```

#### Custom Configuration

```bash
# Override specific values
helm install dataengine ./k8s/data-engine-helm \
  --set config.database_only_mode=true \
  --set replicaCount=3 \
  --set resources.requests.memory=4Gi \
  --set postgresql.enabled=true \
  --set redis.enabled=true
```

#### Available Helm Values Files

- `values.yaml` - Default configuration (full messaging)
- `values-prod.yaml` - Production optimized settings
- `values-dev.yaml` - Development environment
- `values-qa.yaml` - QA environment settings  
- `values-database-only.yaml` - Database-only mode optimized

## 🔍 Monitoring and Observability

### Built-in Metrics

The DataEngine provides comprehensive metrics through the ultra-precision metrics system:

```rust
// Access real-time metrics
let metrics = UltraPrecisionMetrics::global();
println!("Processing latency: {}μs", metrics.average_latency_micros());
println!("Messages processed: {}", metrics.total_messages_processed());
println!("Error rate: {:.2}%", metrics.error_rate_percentage());
```

### Key Metrics

| Metric | Description | Typical Value |
|--------|-------------|---------------|
| `processing_latency_us` | End-to-end message processing time | < 500μs |
| `database_write_latency_us` | PostgreSQL write operations | < 200μs |
| `redis_operation_latency_us` | Redis cache operations | < 10μs |
| `messages_per_second` | Sustained throughput | 10,000+ |
| `error_rate_percentage` | Processing error rate | < 0.01% |
| `memory_usage_mb` | Peak memory consumption | < 500MB |
| `cpu_utilization_percentage` | CPU usage under load | < 15% |

### Health Endpoints

```bash
# Check system health
curl http://localhost:8080/health

# Get detailed metrics
curl http://localhost:8080/metrics

# View current configuration
curl http://localhost:8080/config
```

### Logging

Structured JSON logging with configurable levels:

```toml
[logging]
level = "info"
format = "json"
file = "/var/log/dataengine.log"
max_size = "100MB"
max_files = 10
```

## 🛠️ Development

### Project Structure

```
DataEngine/
├── Cargo.toml                 # Project configuration
├── Dockerfile                 # Container build configuration
├── docker-compose.yml         # Multi-service orchestration
├── k8s/                      # Kubernetes deployment manifests
├── config/                   # Configuration management
├── hostbuilder/              # Core engine implementation
│   ├── src/
│   │   ├── handlers/         # WebSocket and message handlers
│   │   │   ├── websockets/   # WebSocket connection management
│   │   │   ├── models/       # Data model processors
│   │   │   └── rest_apis/    # REST API integrations
│   │   ├── caching.rs        # Redis caching layer
│   │   ├── kernel_bypass.rs  # Low-level optimizations
│   │   ├── order_book.rs     # Lock-free order book
│   │   ├── ultra_metrics.rs  # Performance monitoring
│   │   └── lib.rs           # Main library entry
├── performance/              # Performance optimization modules
├── program/                  # Executable programs
└── tests/                   # Integration and unit tests
```

### Building from Source

```bash
# Development build
cargo build

# Release build with optimizations
cargo build --release

# Build with specific features
cargo build --features "simd,kernel-bypass"

# Cross-compilation for different targets
cargo build --target x86_64-unknown-linux-musl
```

### Running Tests

```bash
# Unit tests
cargo test

# Integration tests
cargo test --test integration

# Performance benchmarks
cargo bench

# End-to-end tests
./scripts/test-end-to-end-latency.ps1
```

### Debugging

```bash
# Debug build with symbols
cargo build --features debug-symbols

# Run with debug logging
RUST_LOG=debug cargo run

# Memory profiling with Valgrind
valgrind --tool=memcheck cargo run

# CPU profiling with perf
perf record cargo run
perf report
```

## 🔧 Configuration Reference

### Environment Variables

| Variable | Description | Default | Example |
|----------|-------------|---------|---------|
| `DATABASE_ONLY_MODE` | Enable database-only mode | `false` | `true` |
| `DISABLE_MESSAGE_BROKER` | Disable message publishing | `false` | `true` |
| `POSTGRES_HOST` | PostgreSQL server hostname | `localhost` | `postgres-cluster.internal` |
| `POSTGRES_PORT` | PostgreSQL server port | `5432` | `5432` |
| `POSTGRES_USER` | PostgreSQL username | `dataengine` | `trading_user` |
| `POSTGRES_PASSWORD` | PostgreSQL password | - | `secure_password` |
| `POSTGRES_DB` | PostgreSQL database name | `trading_data` | `market_data` |
| `REDIS_HOST` | Redis server hostname | `localhost` | `redis-cluster.internal` |
| `REDIS_PORT` | Redis server port | `6379` | `6379` |
| `RUST_LOG` | Logging level | `info` | `debug` |

### Performance Tuning

#### Memory Configuration

```toml
[performance]
# Memory pool size for zero-copy operations
memory_pool_size = 1048576  # 1MB

# Buffer sizes for different data types
order_book_buffer_size = 65536
trade_buffer_size = 32768
balance_buffer_size = 16384
```

#### CPU Optimizations

```toml
[cpu]
# Enable SIMD optimizations
enable_simd = true

# CPU affinity for critical threads
order_book_cpu_affinity = [0, 1]
websocket_cpu_affinity = [2, 3]
database_cpu_affinity = [4, 5]
```

#### Network Settings

```toml
[network]
# TCP buffer sizes
tcp_receive_buffer = 262144  # 256KB
tcp_send_buffer = 262144     # 256KB

# WebSocket settings
websocket_timeout = 30000    # 30 seconds
ping_interval = 10000        # 10 seconds
max_frame_size = 1048576     # 1MB
```

## 🔐 Security

### Authentication

```toml
[security]
# API key authentication
api_key_required = true
api_key_header = "X-API-Key"

# Rate limiting
rate_limit_requests_per_second = 1000
rate_limit_burst_size = 100
```

### Network Security

```toml
[network_security]
# TLS configuration
tls_enabled = true
tls_cert_path = "/etc/ssl/certs/dataengine.crt"
tls_key_path = "/etc/ssl/private/dataengine.key"

# Firewall rules
allowed_ips = ["10.0.0.0/8", "192.168.0.0/16"]
blocked_ips = []
```

## 🧪 Testing

### Performance Tests

```bash
# Latency benchmarks
cargo bench --bench latency

# Throughput tests
cargo bench --bench throughput

# Memory usage profiling
cargo bench --bench memory

# End-to-end integration
./scripts/test-end-to-end-latency.ps1
```

### Load Testing

```bash
# Simulate high-frequency trading load
cargo run --bin benchmark -- --clients 100 --messages 1000000

# WebSocket connection stress test
cargo run --bin websocket-stress -- --connections 1000

# Database write performance
cargo run --bin db-benchmark -- --writes 100000
```

## 📊 Observability

### Prometheus Metrics

```yaml
# Prometheus scrape configuration
- job_name: 'dataengine'
  static_configs:
  - targets: ['dataengine:8080']
  scrape_interval: 5s
  metrics_path: '/metrics'
```

### Grafana Dashboard

Key visualizations available:
- **Processing Latency**: Real-time latency percentiles
- **Throughput**: Messages processed per second
- **Error Rates**: System error rate over time
- **Resource Usage**: CPU, memory, and network utilization
- **Database Performance**: Query latency and connection pool stats

### Jaeger Tracing

```toml
[tracing]
enabled = true
jaeger_endpoint = "http://jaeger:14268/api/traces"
service_name = "dataengine"
sample_rate = 0.1
```

## 🚨 Troubleshooting

### Common Issues

#### High Memory Usage
```bash
# Check memory pool configuration
grep -i "memory_pool_size" config.toml

# Monitor memory allocations
RUST_LOG=trace cargo run 2>&1 | grep -i memory
```

#### Database Connection Issues
```bash
# Test database connectivity
psql -h $POSTGRES_HOST -U $POSTGRES_USER -c "SELECT 1;"

# Check connection pool status
curl http://localhost:8080/metrics | grep db_pool
```

#### WebSocket Disconnections
```bash
# Check network connectivity
ping ws.kraken.com

# Monitor WebSocket logs
tail -f /var/log/dataengine.log | grep -i websocket
```

#### Performance Degradation
```bash
# CPU profiling
perf top -p $(pgrep dataengine)

# Memory profiling
pmap -x $(pgrep dataengine)

# Check system limits
ulimit -a
```

### Debug Commands

```bash
# Enable verbose logging
export RUST_LOG=dataengine=trace

# Check configuration
cargo run -- --check-config

# Validate database schema
cargo run -- --validate-schema

# Test WebSocket connectivity
cargo run -- --test-websockets
```

## 🚀 Production Deployment

### Resource Requirements

| Component | CPU | Memory | Storage | Network |
|-----------|-----|--------|---------|---------|
| **DataEngine** | 2-4 cores | 1-2GB RAM | 10GB SSD | 1Gbps |
| **PostgreSQL** | 4-8 cores | 8-16GB RAM | 500GB NVMe | 1Gbps |
| **Redis** | 2 cores | 4-8GB RAM | 50GB SSD | 1Gbps |
| **Message Broker** | 2-4 cores | 2-4GB RAM | 100GB SSD | 1Gbps |

### High Availability Setup

```yaml
apiVersion: v1
kind: Service
metadata:
  name: dataengine-service
spec:
  selector:
    app: dataengine
  ports:
  - port: 8080
    targetPort: 8080
  type: LoadBalancer

---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: dataengine
spec:
  replicas: 3
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxSurge: 1
      maxUnavailable: 0
  selector:
    matchLabels:
      app: dataengine
  template:
    spec:
      containers:
      - name: dataengine
        image: dataengine:latest
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 5
```

### Backup and Recovery

```bash
# Database backup
pg_dump -h $POSTGRES_HOST -U $POSTGRES_USER trading_data > backup.sql

# Redis backup
redis-cli --rdb dump.rdb

# Configuration backup
tar -czf config-backup.tar.gz config/
```

## 📈 Roadmap

### Version 1.1 (Q4 2024)
- [ ] Support for additional exchanges (Binance, Coinbase Pro)
- [ ] GraphQL API for historical data queries
- [ ] Real-time analytics engine
- [ ] Machine learning integration for pattern detection

### Version 1.2 (Q1 2025)
- [ ] Multi-region deployment support
- [ ] Advanced order routing algorithms
- [ ] Risk management integration
- [ ] Compliance reporting tools

### Version 2.0 (Q2 2025)
- [ ] Equity markets support (NYSE, NASDAQ)
- [ ] Options and derivatives processing
- [ ] Advanced portfolio management
- [ ] AI-powered market prediction

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Development Setup

```bash
# Fork the repository
git clone https://github.com/your-username/tradingplatform.git

# Create a feature branch
git checkout -b feature/amazing-feature

# Make your changes and test
cargo test

# Commit and push
git commit -am 'Add amazing feature'
git push origin feature/amazing-feature

# Create a Pull Request
```

### Code Standards

- **Performance**: All changes must maintain sub-millisecond latency requirements
- **Safety**: Use `unsafe` code only when absolutely necessary with proper documentation
- **Testing**: Minimum 90% test coverage for all new features
- **Documentation**: Comprehensive documentation for all public APIs

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🆘 Support

- **Documentation**: [https://docs.tradingplatform.com](https://docs.tradingplatform.com)
- **Issues**: [GitHub Issues](https://github.com/your-org/tradingplatform/issues)
- **Discussions**: [GitHub Discussions](https://github.com/your-org/tradingplatform/discussions)
- **Email**: support@tradingplatform.com

---

**Built with ❤️ for ultra-high-frequency trading**

*DataEngine - Where microseconds matter™*
