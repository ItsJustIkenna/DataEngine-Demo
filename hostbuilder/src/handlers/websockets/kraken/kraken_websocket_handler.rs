use anyhow::Result;
use async_trait::async_trait;
use config::Config;
use databaseschema::{
    models::order_book::OrderBook, 
    CustomAsyncPgConnectionManager,
};
use deadpool::managed::Pool;
use deadpool_redis::{
    Connection,
    Manager
};
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

use crate::{
    endpoint::EndpointHandler,
    handlers::{
        models::kraken::{
            responses::{parse_message, Response},
            fast_parser::FastMessageParser,
        }
    }
};
use performance::{CpuAffinityManager, cpu_affinity::{set_high_priority, prefault_stack}};
//-------------------------------------------------------------------------

pub struct KrakenWebSocketHandler {
    config: Config,
    writer: RwLock<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, Message>>,
    reader: RwLock<SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>>,
    publisher: Arc<tokio::sync::Mutex<Publisher>>, // Keep Mutex for compatibility 
    unsubscribe_message: Value,
    order_books: DashMap<String, OrderBook>, // Use DashMap for concurrent access
    redis_pool: Arc<Pool<Manager, Connection>>,
    postgres_pool: Arc<Pool<CustomAsyncPgConnectionManager>>,
    fast_parser: FastMessageParser, // SIMD-accelerated message parsing
}

impl KrakenWebSocketHandler {
    pub async fn new(endpoint: &str, order_books: HashMap<String, OrderBook>, subscribe_message: &Value, unsubscribe_message: &Value, redis_pool: Arc<Pool<Manager, Connection>>, postgres_pool: Arc<Pool<CustomAsyncPgConnectionManager>>) -> Result<Self> {
        // Use default config for now, optimize later
        let (ws_stream, _) = match connect_async(endpoint).await {
            Ok(val) => {
                tracing::info!("Connected to endpoint: {}", endpoint);
                val
            }
            Err(e) => {
                tracing::error!("Failed to connect to endpoint: {}", e);
                return Err(e.into());
            }
        };

        let config_path = env::var("CONFIG_PATH")
            .map_err(|_| anyhow::anyhow!("CONFIG_PATH environment variable must be set"))?;
        let config = Config::new(&config_path)?;

        let (mut writer, reader) = ws_stream.split();
        
        writer
            .send(Message::Text(subscribe_message.to_string()))
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send subscription message: {}", e))?;

        let reader = RwLock::new(reader);
        let writer = RwLock::new(writer);

        let addr = format!("{}:{}", config.message_broker.address, config.message_broker.port);
        let publisher_config = PublisherConfig::new(&addr);
        let publisher = Arc::new(tokio::sync::Mutex::new(
            Publisher::new(publisher_config)
                .map_err(|e| anyhow::anyhow!("Failed to create publisher: {:?}", e))?
        ));
        
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
        
        tokio::spawn(async move {
            // Pin WebSocket handler to dedicated high-priority core
            if let Err(e) = cpu_manager.pin_websocket_handler() {
                tracing::warn!("Failed to set CPU affinity: {}", e);
            }
            
            // Set maximum priority for critical trading thread
            if let Err(e) = set_high_priority() {
                tracing::warn!("Failed to set high priority: {}", e);
            }
            
            // Pre-fault memory to avoid page faults in critical path
            if let Err(e) = prefault_stack(8) {
                tracing::warn!("Failed to prefault stack: {}", e);
            }

            tokio::select! {
                // Listening to WebSocket Messages
                _ = async {
                    loop {
                        let message = handler.reader.write().await.next().await;

                        match message {
                            Some(Ok(Message::Text(text))) => {
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
                                                    Arc::clone(&self.publisher), 
                                                    &order_book
                                                ).await {
                                                    tracing::error!("Failed to process order book update: {}", e);
                                                }
                                            }
                                        }
                                    }
                                    continue; // Skip fallback parsing for critical path
                                }
                                
                                // Fallback to standard parsing for non-critical messages
                                match parse_message(&text) {
                                    Ok(Response::Level3Snapshot(snapshot)) => {
                                        tracing::debug!("Received level 3 snapshot");
                                        if let Some(order_book) = self.order_books.get(&snapshot.data[0].symbol) {
                                            if let Err(e) = snapshot.process_snapshot(
                                                Arc::clone(&self.redis_pool), 
                                                Arc::clone(&self.postgres_pool), 
                                                &self.config, 
                                                Arc::clone(&self.publisher), 
                                                &order_book
                                            ).await {
                                                tracing::error!("Failed to process order book snapshot: {}", e);
                                            }
                                        }
                                    },
                                    Ok(Response::Level3Update(update)) => {
                                        tracing::debug!("Received level 3 update");
                                        if let Some(order_book) = self.order_books.get(&update.data[0].symbol) {
                                            if let Err(e) = update.process_update(
                                                Arc::clone(&self.redis_pool), 
                                                Arc::clone(&self.postgres_pool), 
                                                &self.config, 
                                                Arc::clone(&self.publisher), 
                                                &order_book
                                            ).await {
                                                tracing::error!("Failed to process order book update: {}", e);
                                            }
                                        }
                                    },
                                    Ok(Response::TradeSnapshot(snapshot)) => {
                                        tracing::debug!("Received trade snapshot");
                                        if let Some(order_book) = self.order_books.get(&snapshot.data[0].symbol) {
                                            if let Err(e) = snapshot.process_snapshot(
                                                Arc::clone(&self.redis_pool), 
                                                Arc::clone(&self.postgres_pool), 
                                                &self.config, 
                                                Arc::clone(&self.publisher), 
                                                &order_book
                                            ).await {
                                                tracing::error!("Failed to process trade snapshot: {}", e);
                                            }
                                        }
                                    },
                                    Ok(Response::TradeUpdate(update)) => {
                                        tracing::debug!("Received trade update");
                                        if let Some(order_book) = self.order_books.get(&update.data[0].symbol) {
                                            if let Err(e) = update.process_update(
                                                Arc::clone(&self.redis_pool), 
                                                Arc::clone(&self.postgres_pool), 
                                                &self.config, 
                                                Arc::clone(&self.publisher), 
                                                &order_book
                                            ).await {
                                                tracing::error!("Failed to process trade update: {}", e);
                                            }
                                        }
                                    },
                                    Ok(Response::HeartBeat(_)) => {
                                        // Handle heartbeat silently
                                    },
                                    Ok(_) => {
                                        // Handle other message types silently
                                    },
                                    Err(e) => {
                                        tracing::error!("Failed to parse message: {}", e);
                                    }
                                }
                            },
                            Some(Ok(Message::Close(_))) => {
                                tracing::info!("WebSocket connection closed");
                                break;
                            },
                            Some(Ok(Message::Ping(data))) => {
                                // Send pong response
                                if let Err(e) = handler.writer.write().await
                                    .send(Message::Pong(data)).await {
                                    tracing::error!("Failed to send pong: {}", e);
                                }
                            },
                            Some(Ok(_)) => {
                                // Handle other message types silently
                            },
                            Some(Err(e)) => {
                                tracing::error!("Error receiving message: {}", e);
                            },
                            None => {
                                tracing::info!("WebSocket connection closed");
                                break;
                            }
                        }
                    }
                } => {},

                // Handling Shutdown
                _ = shutdown_rx.recv() => {
                    tracing::info!("Shutting down KrakenWebSocketHandler");
                    // Send unsubscribe message
                    if let Err(e) = handler.writer.write().await
                        .send(Message::Text(handler.unsubscribe_message.to_string())).await {
                        tracing::error!("Failed to send unsubscribe message: {}", e);
                    }
                },
            }
        });

        Ok(())
    }
}
