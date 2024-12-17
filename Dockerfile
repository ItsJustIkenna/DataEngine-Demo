# syntax=docker/dockerfile:1.3

# Use the official Rust image
FROM rust:alpine AS builder

# Install necessary build dependencies
RUN apk update && apk add --no-cache pkgconfig openssl-dev build-base

ENV PKG_CONFIG_PATH=/usr/lib/aarch64-linux-gnu/pkgconfig
ENV OPENSSL_DIR=/usr
ENV OPENSSL_LIB_DIR=$OPENSSL_DIR/lib
ENV OPENSSL_INCLUDE_DIR=$OPENSSL_DIR/include

# Set the working directory to DataEngine
WORKDIR /usr/src/app/DataEngine

# Copy the entire project
COPY . .

# Copy the DatabaseEngine
COPY DatabaseEngine .

# Build and test the application.
RUN cargo build --locked --release --workspace --bin program && \
    cargo test --locked --release --workspace

# Debug step: List the contents of the target/release directory
RUN ls -la /usr/src/app/DataEngine/target/release

# Create a new stage for running the application that contains the minimal
# runtime dependencies for the application.
FROM alpine:3.18 AS final

# Create a non-privileged user that the app will run under.
ARG UID=10001
RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --uid "$UID" \
    "appuser"

# Copy the built binary from the builder stage
COPY --from=builder /usr/src/app/DataEngine/target/release/program /bin/server

# Set the user to the non-privileged user
USER appuser

# Set the entrypoint to the binary
ENTRYPOINT ["/bin/server"]