# DataEngine

This file implements the **HostedObject**, a core component of the `DataEngine` application. It is responsible for managing connections to financial exchanges, handling WebSocket subscriptions, processing real-time data (e.g., Level 3 order books and trades), and ensuring a graceful shutdown of the application. The file is designed with scalability and reliability in mind, leveraging asynchronous programming with `tokio`.

---

## Key Features

1. **Exchange Management**:
   - Connects to multiple financial exchanges (e.g., Kraken).
   - Retrieves exchange details, securities, and order books using REST APIs.

2. **WebSocket Handling**:
   - Manages WebSocket connections for real-time data streams.
   - Subscribes to channels like `level3` (order books) and `trade` (trade data).
   - Processes incoming messages and updates Redis and PostgreSQL databases.

3. **Graceful Shutdown**:
   - Listens for termination signals (`SIGTERM` on Unix or `Ctrl+C` on Windows).
   - Ensures all WebSocket connections and tasks are properly closed before exiting.

4. **Asynchronous Design**:
   - Uses `tokio` to handle multiple WebSocket connections concurrently.
   - Spawns tasks for each WebSocket connection and manages them efficiently.

---

## Components

### 1. **HostedObject Struct**
The `HostedObject` struct encapsulates the application's configuration and connection pools:
- **Fields**:
  - `config`: Stores the application's configuration (e.g., exchange details, WebSocket endpoints).
  - `redis_pool`: A connection pool for Redis, used for caching and real-time data storage.
  - `postgres_pool`: A connection pool for PostgreSQL/TimescaleDB, used for persistent storage.

- **Initialization**:
  The `new` method initializes the `HostedObject` by:
  - Loading environment variables using `dotenv`.
  - Reading the configuration file path from the `CONFIG_PATH` environment variable.
  - Creating Redis and PostgreSQL connection pools.

---

### 2. **HostedObjectTrait**
The `HostedObject` struct implements the `HostedObjectTrait` trait, which defines the `run` method. This method orchestrates the application's runtime logic.

---

### 3. **`run` Method**
The `run` method is the main entry point for the application's runtime behavior:
1. **Setup**:
   - Creates a broadcast channel (`shutdown_tx`) for signaling shutdown events.
   - Initializes a vector (`handles`) to store asynchronous tasks.

2. **Exchange Handling**:
   - Iterates over the configured exchanges (e.g., Kraken).
   - For each exchange:
     - Retrieves exchange details using REST APIs.
     - Authenticates with the exchange's REST API to obtain a token.
     - Retrieves securities and order books for the exchange.

3. **WebSocket Connections**:
   - For each WebSocket endpoint in the exchange configuration:
     - Constructs subscription and unsubscription messages (e.g., for Level 3 order books or trades).
     - Creates a `KrakenWebSocketHandler` to manage the WebSocket connection.
     - Spawns an asynchronous task to listen to the WebSocket and process incoming messages.

4. **Shutdown Handling**:
   - Spawns a task to listen for termination signals:
     - On Unix systems, listens for `SIGTERM` (e.g., for Kubernetes deployments).
     - On non-Unix systems (e.g., Windows), listens for `Ctrl+C`.
     - Sends a shutdown signal via the broadcast channel when a termination signal is received.

5. **Task Management**:
   - Adds all spawned tasks (WebSocket handlers and the shutdown handler) to the `handles` vector.
   - Awaits the completion of all tasks to ensure a graceful shutdown.

---

## Shutdown Logic

The shutdown logic ensures that:
- All WebSocket connections are closed properly.
- Resources like Redis and PostgreSQL connections are released.
- The application exits cleanly when a termination signal is received.

---

## Usage

### Initialization
The `HostedObject` is initialized using the `new` method, which sets up the configuration and connection pools.

### Running the Application
The `run` method is called to start the application's runtime logic. It handles exchange connections, WebSocket subscriptions, and shutdown signals.

---

## Dependencies

This file relies on several key Rust crates:
- **`tokio`**: For asynchronous runtime and signal handling.
- **`redis`**: For Redis connection pooling and caching.
- **`diesel`**: For PostgreSQL/TimescaleDB integration.
- **`serde_json`**: For constructing JSON messages for WebSocket subscriptions.
- **`dotenv`**: For loading environment variables.
- **`anyhow`**: For error handling.

---

## Example Workflow

1. **Startup**:
   - The application reads the configuration file and initializes connection pools.
   - It connects to exchanges and retrieves necessary data (e.g., securities, order books).

2. **WebSocket Management**:
   - The application subscribes to WebSocket channels (e.g., `level3`, `trade`).
   - It processes incoming messages and updates Redis and PostgreSQL.

3. **Shutdown**:
   - When a termination signal is received, the application sends a shutdown signal to all tasks.
   - All WebSocket connections are closed, and resources are released.

---

## Purpose

This file serves as the backbone of the `DataEngine` application, enabling real-time data processing for financial exchanges. It ensures scalability, reliability, and proper resource management, making it suitable for high-performance trading platforms or market data aggregators.