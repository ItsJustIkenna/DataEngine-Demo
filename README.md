# Data Engine

This application is a high-performance, asynchronous data ingestion engine written in Rust. It connects to the Kraken cryptocurrency exchange via REST and WebSocket APIs, authenticates, and subscribes to both Level 3 order book and trade data channels. The data is stored in a Redis cache and TimescaleDB database for downstream use in trading systems or analytics platforms.

## Features

- 🔒 Secure, authenticated WebSocket and REST API communication with Kraken
- 📦 Ingests both order book and trade data (Level 3 depth)
- 🧠 Uses Redis for caching and TimescaleDB for persistent storage
- 🧪 Modular and testable via `mockall`
- 🧵 Asynchronous task management using `tokio`
- 🌐 Environment-based configuration with support for multiple exchanges

## Project Structure

- `main.rs` — Launches the data engine
- `lib.rs` — Core implementation including setup and orchestration
- `handlers/` — API and WebSocket handlers
- `endpoint/` — Endpoint handler definitions
- `get_info/` — Functions to retrieve securities, order books, and exchange info
- `config/` — Configuration parsing

## Prerequisites

- Rust (1.70+ recommended)
- Redis instance
- TimescaleDB (PostgreSQL with TimescaleDB extension)
- `.env` file with the following:
  ```env
  CONFIG_PATH=./path/to/appsettings.json
