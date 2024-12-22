# syntax=docker/dockerfile:1

ARG RUST_VERSION=1.82.0
ARG APP_NAME=program

################################################################################
# Stage 1: Build the application
FROM rust:${RUST_VERSION}-slim-bullseye AS build
ARG APP_NAME

# Install necessary build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Set the working directory inside the container
WORKDIR /app

# Copy the source code into the container
COPY DataEngine /app/DataEngine

COPY databaseschema /app/databaseschema

COPY RedisUtils /app/RedisUtils

# Ensure the program builds correctly from the workspace
WORKDIR /app/DataEngine

# Test the application in release mode
RUN cargo test --locked --release

# Build the application in release mode
RUN cargo build --locked --release

################################################################################
# Stage 2: Create a smaller runtime image
FROM debian:bullseye-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libc6 \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create a non-privileged user to run the app
ARG UID=10001
RUN adduser --disabled-password --gecos "" --home "/nonexistent" --shell "/sbin/nologin" --no-create-home --uid "${UID}" appuser

# Copy the built application from the build stage
COPY --from=build /app/DataEngine/target/release/$APP_NAME /bin/server

# Ensure the binary is executable
RUN chmod +x /bin/server

# Switch to non-privileged user
USER appuser

# Set the command to run the application
CMD ["/bin/server"]