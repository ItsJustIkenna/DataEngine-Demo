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

# Build the workspace
RUN cargo build

# Run tests
RUN cargo test

# Set the default command to run the application
CMD ["cargo", "run", "--bin", "program"]