# Ultra-optimized production Dockerfile for DataEngine
# ENTERPRISE BUILD STRATEGY: Always build from workspace root containing all dependencies
# Local:  docker build -f DataEngine/Dockerfile -t dataengine:latest .
# CI/CD:  docker build -f DataEngine/Dockerfile -t dataengine:latest .
# Build context MUST contain: DataEngine/, databaseschema/, redisutils/, LoggingEngine/, MessageBrokerEngine/

ARG RUST_VERSION=1.82.0
ARG APP_NAME=program

################################################################################
# Stage 1: Dependency builder with maximum caching efficiency
FROM rust:${RUST_VERSION}-alpine AS deps

# Install build dependencies for Alpine/musl including cross-compilation tools
RUN apk add --no-cache \
    build-base \
    pkgconfig \
    openssl-dev \
    openssl-libs-static \
    postgresql-dev \
    musl-dev \
    ca-certificates \
    perl \
    gcc \
    g++

# Optimize Cargo environment for speed and caching
ENV CARGO_HOME=/usr/local/cargo \
    CARGO_TARGET_DIR=/app/DataEngine/target \
    CARGO_INCREMENTAL=0 \
    CARGO_NET_RETRY=10 \
    RUST_BACKTRACE=1 \
    OPENSSL_DIR=/usr \
    OPENSSL_STATIC=1 \
    OPENSSL_LIB_DIR=/usr/lib \
    OPENSSL_INCLUDE_DIR=/usr/include \
    PKG_CONFIG_ALL_STATIC=1 \
    PKG_CONFIG_PATH=/usr/lib/pkgconfig

# Set up complete workspace structure to match local development
WORKDIR /app

# Copy only Cargo manifests first for maximum caching
COPY DataEngine/Cargo.toml DataEngine/Cargo.lock ./DataEngine/
COPY DataEngine/config/Cargo.toml ./DataEngine/config/
COPY DataEngine/hostbuilder/Cargo.toml ./DataEngine/hostbuilder/
COPY DataEngine/program/Cargo.toml ./DataEngine/program/
COPY DataEngine/performance/Cargo.toml ./DataEngine/performance/
COPY DataEngine/tests/Cargo.toml ./DataEngine/tests/

# Copy external dependency manifests and sources (complete structure)
COPY databaseschema/ ./databaseschema/
COPY redisutils/ ./redisutils/
COPY LoggingEngine/ ./LoggingEngine/
COPY MessageBrokerEngine/publisher/ ./MessageBrokerEngine/publisher/
COPY MessageBrokerEngine/protocol/ ./MessageBrokerEngine/protocol/

# Change to DataEngine directory for build
WORKDIR /app/DataEngine

# Create dummy source files for dependency caching
RUN mkdir -p config/src hostbuilder/src program/src performance/src tests/src && \
    echo "pub fn dummy() {}" > config/src/lib.rs && \
    echo "pub fn dummy() {}" > hostbuilder/src/lib.rs && \
    echo "fn main() { println!(\"Building dependencies...\"); }" > program/src/main.rs && \
    echo "fn main() { println!(\"Building benchmark...\"); }" > program/src/benchmark.rs && \
    echo "pub fn dummy() {}" > performance/src/lib.rs && \
    echo "pub fn dummy() {}" > tests/src/lib.rs

# Pre-compile dependencies for native musl (this layer will be cached!)
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/DataEngine/target \
    cargo build --release && \
    rm -rf config/src hostbuilder/src program/src performance/src tests/src

################################################################################
# Stage 2: Application builder
FROM deps AS build
ARG APP_NAME

COPY DataEngine/config/ ./config/
COPY DataEngine/hostbuilder/ ./hostbuilder/
COPY DataEngine/program/ ./program/
COPY DataEngine/performance/ ./performance/
COPY DataEngine/tests/ ./tests/

COPY databaseschema/ ./databaseschema/
COPY redisutils/ ./redisutils/
COPY LoggingEngine/ ./LoggingEngine/
COPY MessageBrokerEngine/publisher/ ./MessageBrokerEngine/publisher/
COPY MessageBrokerEngine/protocol/ ./MessageBrokerEngine/protocol/

# Set optimization flags for release build
ENV RUSTFLAGS="-C opt-level=3 -C codegen-units=1 -C panic=abort -C target-cpu=native -C link-arg=-s"

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/DataEngine/target \
    cargo build --release && \
    cp target/release/$APP_NAME /bin/dataengine && \
    strip --strip-all /bin/dataengine && \
    chmod +x /bin/dataengine

################################################################################
# Stage 3: Minimal Alpine runtime with built-in shell and process tools
FROM alpine:latest AS runtime

# Install essential runtime dependencies including procps for pgrep
RUN apk add --no-cache \
    ca-certificates \
    libgcc \
    procps \
    && adduser -D -u 65532 nonroot

# Copy only the optimized binary
COPY --from=build /bin/dataengine /usr/local/bin/dataengine

# Use non-root user for security
USER nonroot

# Expose application ports
EXPOSE 8080 9090

# Health check for container orchestration
HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --retries=3 \
  CMD ["/usr/local/bin/dataengine", "--health-check"] || exit 1

# Set optimal entrypoint
ENTRYPOINT ["/usr/local/bin/dataengine"]
CMD ["--production"]