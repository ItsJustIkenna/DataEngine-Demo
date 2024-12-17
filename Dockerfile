# syntax=docker/dockerfile:1.3

# Use the official Rust image
FROM rust:alpine

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
RUN cargo build --locked --release --workspace && \
    cargo test --locked --release --workspace

CMD ["cargo", "run", "--release", "--bin", "program"]