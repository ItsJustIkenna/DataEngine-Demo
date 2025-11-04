use anyhow::Result;
use async_trait::async_trait;
use config::Config;
use databaseschema::{
    models::order_book::OrderBook,
};
use diesel_async::AsyncPgConnection;
use futures_util::{
    stream::{
        SplitSink,
        SplitStream
    },
    SinkExt,
    StreamExt,
};
use publisher::{Publisher, PublisherConfig};
use serde_json::Value;
use std::{
    collections::HashMap, env, sync::Arc
};
use tokio::{
    net::TcpStream,
    sync::{
        broadcast::Receiver,
        RwLock
    },
};
use tokio_tungstenite::{
    connect_async,
    tungstenite::Message,
    MaybeTlsStream,
    WebSocketStream
};
use dashmap::DashMap;
use std::time::Instant;
use ultra_logger::LogLevel;
use crate::infrastructure::logging_facade::KRAKEN_LOGGER; use crate::{log_info, log_warn, log_error, log_debug}; // Add tracing info macro

use crate::{
    network::endpoint::EndpointHandler,
};
use crate::network::handlers::models::kraken::responses::{parse_message, Response};
use crate::network::handlers::models::kraken::fast_parser::FastMessageParser;
use crate::network::handlers::logging::{KRAKEN_WEBSOCKET_LOGGER, WebSocketEvent, CriticalError};
use performance::{CpuAffinityManager, cpu_affinity::{set_high_priority, prefault_stack}};
//-------------------------------------------------------------------------

pub struct KrakenWebSocketHandler {
    config: Config,
    writer: RwLock<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>,
    reader: RwLock<SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>>,
    publisher: Option<Arc<tokio::sync::Mutex<Publisher>>>, // Optional for database-only mode
    unsubscribe_message: Value,
    order_books: DashMap<String, OrderBook>, // Use DashMap for concurrent access
    redis_pool: Arc<deadpool_redis::Pool>,
    postgres_pool: Arc<diesel_async::pooled_connection::deadpool::Pool<AsyncPgConnection>>,
    fast_parser: FastMessageParser, // SIMD-accelerated message parsing
}

impl KrakenWebSocketHandler {
    pub async fn new(endpoint: &str, order_books: HashMap<String, OrderBook>, subscribe_message: &Value, unsubscribe_message: &Value, redis_pool: Arc<deadpool_redis::Pool>, postgres_pool: Arc<diesel_async::pooled_connection::deadpool::Pool<AsyncPgConnection>>) -> Result<Self> {
        // Use default config for now, optimize later
        let connection_start = Instant::now();
        let (ws_stream, _) = match connect_async(endpoint).await {
            Ok(val) => {
                let latency = connection_start.elapsed().as_micros() as u64;
                log_info!(KRAKEN_LOGGER, "Connected to endpoint: {}", endpoint);
                
                // Log successful WebSocket connection
                let connection_event = WebSocketEvent {
                    endpoint: endpoint.to_string(),
                    action: "connect".to_string(),
                    connection_id: None,
                    message: format!("Successfully connected to Kraken WebSocket at {}", endpoint),
                    level: LogLevel::Info,
                    latency_micros: Some(latency),
                };
                let _ = KRAKEN_WEBSOCKET_LOGGER.log_websocket_event(connection_event).await;
                
                val
            }
            Err(e) => {
                log_error!(KRAKEN_LOGGER, "Failed to connect to endpoint: {}", e);
                
                // Log WebSocket connection failure
                let connection_event = WebSocketEvent {
                    endpoint: endpoint.to_string(),
                    action: "connect_failed".to_string(),
                    connection_id: None,
                    message: format!("Failed to connect to Kraken WebSocket: {}", e),
                    level: LogLevel::Error,
                    latency_micros: None,
                };
                let _ = KRAKEN_WEBSOCKET_LOGGER.log_websocket_event(connection_event).await;
                
                return Err(e.into());
            }
        };

        let config_path = env::var("CONFIG_PATH")
            .map_err(|_| anyhow::anyhow!("CONFIG_PATH environment variable must be set"))?;
        let config = Config::new(&config_path)?;

        let (mut writer, reader) = ws_stream.split();
        
        log_info!(KRAKEN_LOGGER, "📤 Sending subscription message to {}: {}", endpoint, subscribe_message);
        
        writer
            .send(Message::Text(subscribe_message.to_string()))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send subscription message: {}", e))?;

        let reader = RwLock::new(reader);
        let writer = RwLock::new(writer);

        let addr = format!("{}:{}", config.message_broker.address, config.message_broker.port);
        let publisher_config = PublisherConfig::new(&addr);
        
        // Check if database-only mode is enabled
        let database_only_mode = env::var("DATABASE_ONLY_MODE")
            .unwrap_or_else(|_| "false".to_string())
            .to_lowercase() == "true"
            || env::var("DISABLE_MESSAGE_BROKER")
                .unwrap_or_else(|_| "false".to_string())
                .to_lowercase() == "true";
        
        let publisher = if database_only_mode {
            // In database-only mode, create a no-op publisher or handle gracefully
            log_info!(KRAKEN_LOGGER, "🔒 DATABASE-ONLY MODE: Message broker publishing disabled");
            None
        } else {
            Some(Arc::new(tokio::sync::Mutex::new(
                Publisher::new(publisher_config)
                    .map_err(|e| anyhow::anyhow!("Failed to create publisher: {:?}", e))?
            )))
        };
        
        // Convert HashMap to DashMap for concurrent access
        let order_books_map = DashMap::new();
        for (key, value) in order_books {
            order_books_map.insert(key, value);
        }
        
        Ok(Self {
            config,
            writer,
            reader,
            publisher,
            unsubscribe_message: unsubscribe_message.clone(),
            order_books: order_books_map,
            redis_pool,
            postgres_pool,
            fast_parser: FastMessageParser::new(),
        })
    }
}

#[async_trait]
impl EndpointHandler for KrakenWebSocketHandler {
    async fn listen(self: Arc<Self>, mut shutdown_rx: Receiver<()>) -> Result<()> {
        let handler = Arc::clone(&self);

        // Ultra-high performance optimizations
        let cpu_manager = CpuAffinityManager::new();
        
        let handle = tokio::spawn(async move {
            // Pin WebSocket handler to dedicated high-priority core
            if let Err(e) = cpu_manager.pin_websocket_handler() {
                log_warn!(KRAKEN_LOGGER, "Failed to set CPU affinity: {}", e);
            }
            
            // Set maximum priority for critical trading thread
            if let Err(e) = set_high_priority() {
                log_warn!(KRAKEN_LOGGER, "Failed to set high priority: {}", e);
            }
            
            // Pre-fault memory to avoid page faults in critical path
            if let Err(e) = prefault_stack(8) {
                log_warn!(KRAKEN_LOGGER, "Failed to prefault stack: {}", e);
            }

            tokio::select! {
                // Listening to WebSocket Messages
                _result = async {
                    loop {
                        let message = handler.reader.write().await.next().await;

                        match message {
                            Some(Ok(Message::Text(text))) => {
                                // Log all incoming messages for debugging
                                if text.contains("subscribe") || text.contains("status") || text.contains("error") {
                                    log_info!(KRAKEN_LOGGER, "📨 Received message: {}", text);
                                }
                                
                                // Use ultra-high performance SIMD-accelerated message parsing
                                let message_bytes = text.as_bytes();
                                
                                // Fast path: use SIMD to identify message type before expensive parsing
                                if self.fast_parser.is_level3_update(message_bytes) {
                                    // Critical path optimization: parse Level3 updates with minimal allocations
                                    if let Ok(update) = self.fast_parser.parse_level3_update_fast(&text) {
                                        if let Some(symbol) = self.fast_parser.extract_symbol_fast(message_bytes) {
                                            if let Some(order_book) = self.order_books.get(&symbol) {
                                                if let Err(e) = update.process_update(
                                                    Arc::clone(&self.redis_pool), 
                                                    Arc::clone(&self.postgres_pool), 
                                                    &self.config, 
                                                    self.publisher.as_ref().map(|p| Arc::clone(p)), 
                                                    &order_book
                                                ).await {
                                                    log_error!(KRAKEN_LOGGER, "Failed to process order book update: {}", e);
                                                }
                                            }
                                        }
                                    }
                                    continue; // Skip fallback parsing for critical path
                                }
                                
                                // Fallback to standard parsing for non-critical messages
                                match parse_message(&text) {
                                    Ok(Response::Level3Snapshot(snapshot)) => {
                                        log_info!(KRAKEN_LOGGER, "📥 Received Level3 snapshot for {} with {} bids and {} asks", 
                                            snapshot.data[0].symbol, 
                                            snapshot.data[0].bids.as_ref().map_or(0, |b| b.len()),
                                            snapshot.data[0].asks.as_ref().map_or(0, |a| a.len()));
                                        if let Some(order_book) = self.order_books.get(&snapshot.data[0].symbol) {
                                            if let Err(e) = snapshot.process_snapshot(
                                                Arc::clone(&self.redis_pool), 
                                                Arc::clone(&self.postgres_pool), 
                                                &self.config, 
                                                self.publisher.as_ref().map(|p| Arc::clone(p)), 
                                                &order_book
                                            ).await {
                                                log_error!(KRAKEN_LOGGER, "Failed to process order book snapshot: {}", e);
                                            }
                                        }
                                    },
                                    Ok(Response::Level3Update(update)) => {
                                        log_info!(KRAKEN_LOGGER, "📥 Received Level3 update for {} - {} bids, {} asks changed", 
                                            update.data[0].symbol,
                                            update.data[0].bids.as_ref().map_or(0, |b| b.len()),
                                            update.data[0].asks.as_ref().map_or(0, |a| a.len()));
                                        if let Some(order_book) = self.order_books.get(&update.data[0].symbol) {
                                            if let Err(e) = update.process_update(
                                                Arc::clone(&self.redis_pool), 
                                                Arc::clone(&self.postgres_pool), 
                                                &self.config, 
                                                self.publisher.as_ref().map(|p| Arc::clone(p)), 
                                                &order_book
                                            ).await {
                                                log_error!(KRAKEN_LOGGER, "Failed to process order book update: {}", e);
                                            }
                                        }
                                    },
                                    Ok(Response::TradeSnapshot(snapshot)) => {
                                        log_debug!(KRAKEN_LOGGER, "Received trade snapshot");
                                        if let Some(order_book) = self.order_books.get(&snapshot.data[0].symbol) {
                                            if let Err(e) = snapshot.process_snapshot(
                                                Arc::clone(&self.redis_pool), 
                                                Arc::clone(&self.postgres_pool), 
                                                &self.config, 
                                                self.publisher.as_ref().map(|p| Arc::clone(p)), 
                                                &order_book
                                            ).await {
                                                log_error!(KRAKEN_LOGGER, "Failed to process trade snapshot: {}", e);
                                            }
                                        }
                                    },
                                    Ok(Response::TradeUpdate(update)) => {
                                        log_debug!(KRAKEN_LOGGER, "Received trade update");
                                        if let Some(order_book) = self.order_books.get(&update.data[0].symbol) {
                                            if let Err(e) = update.process_update(
                                                Arc::clone(&self.redis_pool), 
                                                Arc::clone(&self.postgres_pool), 
                                                &self.config, 
                                                self.publisher.as_ref().map(|p| Arc::clone(p)), 
                                                &order_book
                                            ).await {
                                                log_error!(KRAKEN_LOGGER, "Failed to process trade update: {}", e);
                                            }
                                        }
                                    },
                                    Ok(Response::Level3SubscribeAck(ack)) => {
                                        log_info!(KRAKEN_LOGGER, "✅ Level3 subscription acknowledged: {:?}", ack);
                                    },
                                    Ok(Response::Level3Subscribe(sub)) => {
                                        log_info!(KRAKEN_LOGGER, "📡 Level3 subscription request: {:?}", sub);
                                    },
                                    Ok(Response::TradeSubscribeAck(ack)) => {
                                        log_info!(KRAKEN_LOGGER, "✅ Trade subscription acknowledged: {:?}", ack);
                                    },
                                    Ok(Response::TradeSubscribe(sub)) => {
                                        log_info!(KRAKEN_LOGGER, "📡 Trade subscription request: {:?}", sub);
                                    },
                                    Ok(Response::Status(status)) => {
                                        log_info!(KRAKEN_LOGGER, "📊 Status message received: {:?}", status);
                                    },
                                    Ok(Response::HeartBeat(_)) => {
                                        // Handle heartbeat silently
                                    },
                                    Ok(other) => {
                                        // Log unhandled message types to see what we're missing
                                        log_info!(KRAKEN_LOGGER, "📭 Unhandled message type: {:?}", other);
                                    },
                                    Err(e) => {
                                        log_error!(KRAKEN_LOGGER, "Failed to parse message: {}", e);
                                        
                                        // Log critical parsing error
                                        let parse_error = CriticalError {
                                            component: "KrakenWebSocketHandler".to_string(),
                                            error_code: "MESSAGE_PARSE_ERROR".to_string(),
                                            error_message: format!("Failed to parse WebSocket message: {}", e),
                                            stack_trace: Some(format!("{:?}", e)),
                                            context: Some(format!("Message length: {}", text.len())),
                                        };
                                        let _ = KRAKEN_WEBSOCKET_LOGGER.log_critical_error(parse_error).await;
                                    }
                                }
                            },
                            Some(Ok(Message::Close(_))) => {
                                log_warn!(KRAKEN_LOGGER, "WebSocket connection closed by server, will attempt reconnection");
                                
                                // Log WebSocket disconnect
                                let disconnect_event = WebSocketEvent {
                                    endpoint: "kraken-websocket".to_string(),
                                    action: "disconnect".to_string(),
                                    connection_id: None,
                                    message: "WebSocket connection closed by server, reconnecting...".to_string(),
                                    level: LogLevel::Warn,
                                    latency_micros: None,
                                };
                                let _ = KRAKEN_WEBSOCKET_LOGGER.log_websocket_event(disconnect_event).await;
                                
                                // Exit loop to trigger reconnection in outer loop
                                log_warn!(KRAKEN_LOGGER, "WebSocket closed by server, exiting listener loop");
                                break;
                            },
                            Some(Ok(Message::Ping(data))) => {
                                // Send pong response
                                if let Err(e) = handler.writer.write().await
                                    .send(Message::Pong(data)).await {
                                    log_error!(KRAKEN_LOGGER, "Failed to send pong: {}", e);
                                }
                            },
                            Some(Ok(_)) => {
                                // Handle other message types silently
                            },
                            Some(Err(e)) => {
                                log_error!(KRAKEN_LOGGER, "Error receiving message: {}, exiting listener loop", e);
                                break;
                            },
                            None => {
                                log_warn!(KRAKEN_LOGGER, "WebSocket connection closed unexpectedly, exiting listener loop");
                                break;
                            }
                        }
                    }
                } => {
                    // WebSocket listener completed (loop exited via break)
                    log_info!(KRAKEN_LOGGER, "WebSocket listener loop completed");
                },

                // Handling Shutdown
                _ = shutdown_rx.recv() => {
                    log_info!(KRAKEN_LOGGER, "Shutting down KrakenWebSocketHandler");
                    // Send unsubscribe message
                    if let Err(e) = handler.writer.write().await
                        .send(Message::Text(handler.unsubscribe_message.to_string())).await {
                        log_error!(KRAKEN_LOGGER, "Failed to send unsubscribe message: {}", e);
                    }
                },
            }
        });

        // Wait for the WebSocket listener task to complete
        handle.await.map_err(|e| anyhow::anyhow!("WebSocket listener task panicked: {}", e))?;
        
        Ok(())
    }
}

