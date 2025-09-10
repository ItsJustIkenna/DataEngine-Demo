# syntax=docker/dockerfile:1
# Ultra-optimized production Dockerfile for DataEngine
# Build from TradingPlatform root: docker build -f DataEngine/Dockerfile -t dataengine:latest .

ARG RUST_VERSION=1.82.0
ARG APP_NAME=program

################################################################################
# Stage 1: Dependency builder with maximum caching efficiency
FROM rust:${RUST_VERSION}-slim-bullseye AS deps

# Install minimal essential dependencies in single layer
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    libssl-dev \
    libpq-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Optimize Cargo environment for speed and caching
ENV CARGO_HOME=/usr/local/cargo \
    CARGO_TARGET_DIR=/workspace/target \
    CARGO_INCREMENTAL=0 \
    CARGO_NET_RETRY=10 \
    RUST_BACKTRACE=1

# Set up complete workspace structure to match local development
WORKDIR /app

# Copy the entire dependency structure to match relative paths
COPY databaseschema/ ./databaseschema/
COPY redisutils/ ./redisutils/
COPY LoggingEngine/ ./LoggingEngine/
COPY MessageBrokerEngine/ ./MessageBrokerEngine/

# Copy DataEngine workspace
COPY DataEngine/ ./DataEngine/

# Change to DataEngine directory for build
WORKDIR /app/DataEngine

################################################################################
# Stage 2: Application builder
FROM deps AS build
ARG APP_NAME

# Ultra-aggressive optimization flags for production
ENV RUSTFLAGS="-C opt-level=3 -C codegen-units=1 -C panic=abort -C target-cpu=native -C link-arg=-s"

# Build optimized binary
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/DataEngine/target \
    cargo build --release --locked --bin $APP_NAME && \
    cp target/release/$APP_NAME /bin/dataengine && \
    strip --strip-all /bin/dataengine && \
    chmod +x /bin/dataengine

################################################################################
# Stage 3: Ultra-minimal distroless runtime
FROM gcr.io/distroless/cc-debian11:nonroot AS runtime

# Copy only the optimized binary
COPY --from=build /bin/dataengine /usr/local/bin/dataengine

# Use non-root user for security
USER nonroot:nonroot

# Expose application ports
EXPOSE 8080 9090

# Health check for container orchestration
HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --retries=3 \
  CMD ["/usr/local/bin/dataengine", "--health-check"] || exit 1

# Set optimal entrypoint
ENTRYPOINT ["/usr/local/bin/dataengine"]
CMD ["--production"]