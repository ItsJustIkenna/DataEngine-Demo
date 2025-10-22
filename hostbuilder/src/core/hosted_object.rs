//! HostedObject implementation
//! 
//! This module contains the main application object that orchestrates
//! all components of the DataEngine

use anyhow::Result;
use async_trait::async_trait;
use config::Config;
use databaseschema::create_timescale_connection_pool;
use diesel_async::AsyncPgConnection;
use dotenv::dotenv;
use mockall::automock;
use redis_utils::create_redis_pool;
use serde_json::json;
use std::{
    env,
    sync::Arc,
    sync::atomic::{AtomicBool, AtomicU64},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
#[cfg(unix)]
use tokio::signal::unix::{signal, SignalKind};
use tokio::{signal, sync::broadcast::channel};

use crate::{
    data::get_info::{get_exchange, get_orderbooks, get_securities},
    infrastructure::{
        monitoring::{DatabaseHealthChecker, PerformanceHealthChecker, SystemHealthMonitor, WebSocketHealthChecker},
        resilience::ErrorHandler,
    },
    data::caching::CacheMaintenanceService,
    network::{
        endpoint::EndpointHandler,
        handlers::{
            rest_apis::kraken::kraken_rest_api::KrakenRestHandler,
            websockets::kraken::kraken_websocket_handler::KrakenWebSocketHandler,
        },
    },
    security::{SecurityConfig, SecurityManager},
    // Import logging macros (exported at crate root) and logger instance
    log_info, log_error,
    infrastructure::logging_facade::MAIN_LOGGER,
};

#[automock]
#[async_trait]
pub trait HostedObjectTrait {
    async fn run(&mut self) -> Result<()>;
}

pub struct HostedObject {
    config: Config,
    redis_pool: Arc<deadpool_redis::Pool>,
    postgres_pool: Arc<diesel_async::pooled_connection::deadpool::Pool<AsyncPgConnection>>,
    health_monitor: Arc<SystemHealthMonitor>,
    security_manager: Arc<SecurityManager>,
    error_handler: Arc<ErrorHandler>,
}

impl HostedObject {
    pub async fn new() -> Result<Self> {
        dotenv().ok();
        let config_path = env::var("CONFIG_PATH")
            .map_err(|_| anyhow::anyhow!("CONFIG_PATH environment variable must be set"))?;
        let config = Config::new(&config_path)?;
        let redis_pool = Arc::new(
            create_redis_pool().map_err(|e| anyhow::anyhow!("Failed to create Redis pool: {}", e))?,
        );
        let postgres_pool = Arc::new(create_timescale_connection_pool());

        // Initialize performance monitoring
        let global_metrics = performance::metrics::global_metrics();

        // Initialize security manager
        let security_config = SecurityConfig::default();
        let security_manager = Arc::new(SecurityManager::new(security_config));

        // Initialize error handler
        let error_handler = Arc::new(ErrorHandler::new());

        // Initialize health monitoring
        let mut health_monitor = SystemHealthMonitor::new(Duration::from_secs(30));

        // Add health checkers
        let ws_is_connected = Arc::new(AtomicBool::new(false));
        let ws_last_message = Arc::new(AtomicU64::new(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        ));
        let ws_health_checker = Arc::new(WebSocketHealthChecker::new(
            Arc::clone(&ws_is_connected),
            Arc::clone(&ws_last_message),
        ));
        health_monitor.add_health_checker(ws_health_checker);

        let db_health_checker = Arc::new(DatabaseHealthChecker::new(
            Arc::clone(&redis_pool),
            Arc::clone(&postgres_pool),
        ));
        health_monitor.add_health_checker(db_health_checker);

        let perf_health_checker =
            Arc::new(PerformanceHealthChecker::new(Arc::clone(global_metrics)));
        health_monitor.add_health_checker(perf_health_checker);

        let health_monitor = Arc::new(health_monitor);

        // Start background services
        health_monitor.start().await?;

        let cache_service = CacheMaintenanceService::new(Duration::from_secs(300)); // 5 minutes
        cache_service.start().await;

        Ok(Self {
            config,
            redis_pool,
            postgres_pool,
            health_monitor,
            security_manager,
            error_handler,
        })
    }
}

#[async_trait]
impl HostedObjectTrait for HostedObject {
    async fn run(&mut self) -> Result<()> {
        let (shutdown_tx, _) = channel(1);
        let mut handles = vec![];
        let config = self.config.clone();

        // Process each configured exchange
        for exchange in config.exchanges.iter().cloned() {
            if exchange.name != "Kraken" {
                log_info!(MAIN_LOGGER, "Skipping unsupported exchange: {}", exchange.name);
                continue;
            }

            // Initialize exchange (fetch data, authenticate, load securities & orderbooks)
            let (kraken_exchange, api_response, _securities, orderbooks) = match initialize_exchange(
                Arc::clone(&self.redis_pool),
                Arc::clone(&self.postgres_pool),
                exchange.name.clone(),
                exchange.websocket_token.clone(),
                &exchange.symbols,
            )
            .await
            {
                Ok(data) => data,
                Err(e) => {
                    log_error!(MAIN_LOGGER, "Failed to initialize exchange {}: {}", exchange.name, e);
                    continue;
                }
            };

            log_info!(
                MAIN_LOGGER,
                "Successfully initialized exchange: {}",
                kraken_exchange.exchange
            );

            // Spawn handlers for each configured WebSocket channel
            for websocket in exchange.websockets.iter().cloned() {
                let handle = spawn_websocket_handler(
                    &websocket.channel,
                    &websocket.endpoint,
                    &exchange.symbols,
                    &api_response.get_token(),
                    Some(exchange.depth),
                    orderbooks.clone(),
                    Arc::clone(&self.redis_pool),
                    Arc::clone(&self.postgres_pool),
                    shutdown_tx.subscribe(),
                );

                handles.push(handle);
            }
        }

        // Spawn shutdown signal handler
        let shutdown_handle = tokio::spawn(async move {
            log_info!(MAIN_LOGGER, "Shutdown handler spawned and listening for signals");
            
            #[cfg(unix)]
            {
                // Use SIGTERM signal handler for Unix-based systems (Kubernetes)
                match signal(SignalKind::terminate()) {
                    Ok(mut term_signal) => match term_signal.recv().await {
                        Some(_) => {
                            log_info!(MAIN_LOGGER, "Received SIGTERM, initiating graceful shutdown");
                            if let Err(e) = shutdown_tx.send(()) {
                                log_error!(MAIN_LOGGER, "Failed to send shutdown signal: {}", e);
                            }
                        }
                        None => {
                            log_error!(MAIN_LOGGER, "SIGTERM signal stream ended unexpectedly");
                        }
                    },
                    Err(e) => {
                        log_error!(MAIN_LOGGER, "Failed to register SIGTERM handler: {}", e);
                    }
                }
            }

            #[cfg(not(unix))]
            {
                // Use ctrl_c for non-Unix systems (Windows testing)
                match signal::ctrl_c().await {
                    Ok(_) => {
                        log_info!(MAIN_LOGGER, "Received Ctrl+C, initiating graceful shutdown");
                        if let Err(e) = shutdown_tx.send(()) {
                            log_error!(MAIN_LOGGER, "Failed to send shutdown signal: {}", e);
                        }
                    }
                    Err(e) => {
                        log_error!(MAIN_LOGGER, "Failed to register Ctrl+C handler: {}", e);
                    }
                }
            }
        });

        handles.push(shutdown_handle);

        // Wait for all handlers to complete
        log_info!(MAIN_LOGGER, "Application running with {} active handlers", handles.len());
        for handle in handles {
            let _ = handle.await;
        }
        
        log_info!(MAIN_LOGGER, "All handlers completed, application shutting down");
        Ok(())
    }
}

// =============================================================================
// Helper Functions for Refactored Architecture
// =============================================================================

/// Builds a subscription message for Kraken WebSocket channels
/// 
/// # Arguments
/// * `channel` - The channel name (level3, trade, balances)
/// * `symbols` - List of trading symbols to subscribe to
/// * `token` - Authentication token (required for authenticated channels)
/// * `depth` - Order book depth (optional, used for level3)
fn build_subscribe_message(
    channel: &str,
    symbols: &[String],
    token: &str,
    depth: Option<u32>,
) -> Value {
    match channel {
        "level3" => json!({
            "method": "subscribe",
            "params": {
                "channel": channel,
                "symbol": symbols,
                "depth": depth,
                "snapshot": true,
                "token": token,
            }
        }),
        "trade" => json!({
            "method": "subscribe",
            "params": {
                "channel": channel,
                "symbol": symbols,
                "snapshot": true
            }
        }),
        "balances" => json!({
            "method": "subscribe",
            "params": {
                "channel": channel,
                "token": token,
            }
        }),
        _ => {
            log_warn!(MAIN_LOGGER, "Unknown channel type: {}, using default format", channel);
            json!({
                "method": "subscribe",
                "params": {
                    "channel": channel,
                    "symbol": symbols,
                    "token": token,
                }
            })
        }
    }
}

/// Builds an unsubscription message for Kraken WebSocket channels
fn build_unsubscribe_message(
    channel: &str,
    symbols: &[String],
    token: &str,
    depth: Option<u32>,
) -> Value {
    match channel {
        "level3" => json!({
            "method": "unsubscribe",
            "params": {
                "channel": channel,
                "symbol": symbols,
                "depth": depth,
                "token": token,
            }
        }),
        "trade" => json!({
            "method": "unsubscribe",
            "params": {
                "channel": channel,
                "symbol": symbols,
                "snapshot": true
            }
        }),
        "balances" => json!({
            "method": "unsubscribe",
            "params": {
                "channel": channel,
                "token": token,
            }
        }),
        _ => {
            log_warn!(MAIN_LOGGER, "Unknown channel type: {}, using default format", channel);
            json!({
                "method": "unsubscribe",
                "params": {
                    "channel": channel,
                    "symbol": symbols,
                    "token": token,
                }
            })
        }
    }
}

/// Initialize an exchange by fetching exchange data, authenticating, and retrieving securities/orderbooks
/// 
/// This consolidates multiple database queries that were previously called once per websocket channel
/// into a single call per exchange, reducing database load by ~67%.
/// 
/// # Arguments
/// * `redis_pool` - Redis connection pool
/// * `postgres_pool` - PostgreSQL connection pool
/// * `exchange_name` - Name of the exchange (e.g., "Kraken")
/// * `auth_endpoint` - Authentication endpoint URL
/// * `symbols` - List of trading symbols to fetch
/// 
/// # Returns
/// Tuple containing (Exchange, TokenResponse, Vec<Security>, HashMap<String, OrderBook>)
async fn initialize_exchange(
    redis_pool: Arc<deadpool::managed::Pool<deadpool_redis::Manager>>,
    postgres_pool: Arc<deadpool::managed::Pool<diesel_async::pooled_connection::AsyncDieselConnectionManager<diesel_async::AsyncPgConnection>>>,
    exchange_name: String,
    auth_endpoint: String,
    symbols: &[String],
) -> Result<(
    databaseschema::models::exchange::Exchange,
    crate::network::handlers::models::kraken::token_response::TokenResponse,
    Vec<databaseschema::models::security::Security>,
    std::collections::HashMap<String, databaseschema::models::order_book::OrderBook>,
)> {
    // Get exchange information
    let exchange = get_exchange(
        Arc::clone(&redis_pool),
        Arc::clone(&postgres_pool),
        exchange_name.clone(),
    )
    .await?;

    log_info!(
        MAIN_LOGGER,
        "Successfully retrieved exchange: {}",
        exchange.exchange
    );

    // Authenticate with exchange
    let api_response = tokio::time::timeout(
        tokio::time::Duration::from_secs(30),
        KrakenRestHandler::authenticate(&auth_endpoint)
    )
    .await
    .map_err(|_| anyhow::anyhow!("Authentication timeout after 30 seconds"))??;

    log_info!(MAIN_LOGGER, "Successfully authenticated with {}", exchange_name);

    // Get securities (once per exchange, not per websocket)
    let securities = get_securities(
        Arc::clone(&redis_pool),
        Arc::clone(&postgres_pool),
        symbols,
    )
    .await?;

    log_info!(
        MAIN_LOGGER,
        "Successfully retrieved {} securities for {}",
        securities.len(),
        exchange_name
    );

    // Get orderbooks (once per exchange, not per websocket)
    let orderbooks = get_orderbooks(
        Arc::clone(&redis_pool),
        Arc::clone(&postgres_pool),
        securities.clone(),
        &exchange,
    )
    .await?;

    log_info!(
        MAIN_LOGGER,
        "Successfully retrieved {} orderbooks for {}",
        orderbooks.len(),
        exchange_name
    );

    Ok((exchange, api_response, securities, orderbooks))
}

/// Spawn a WebSocket handler for a specific channel
/// 
/// Creates subscription/unsubscription messages, initializes the WebSocket handler,
/// and spawns it as an async task. This function encapsulates all the channel-specific
/// logic and error handling.
/// 
/// # Arguments
/// * `channel` - Channel name ("level3", "trade", "balances")
/// * `endpoint` - WebSocket endpoint URL
/// * `symbols` - Trading symbols to subscribe to
/// * `token` - Authentication token
/// * `depth` - Order book depth (for level3 channel)
/// * `orderbooks` - Order book data
/// * `redis_pool` - Redis connection pool
/// * `postgres_pool` - PostgreSQL connection pool
/// * `shutdown_rx` - Shutdown signal receiver
/// 
/// # Returns
/// JoinHandle for the spawned task
fn spawn_websocket_handler(
    channel: &str,
    endpoint: &str,
    symbols: &[String],
    token: &str,
    depth: Option<u32>,
    orderbooks: std::collections::HashMap<String, databaseschema::models::order_book::OrderBook>,
    redis_pool: Arc<deadpool::managed::Pool<deadpool_redis::Manager>>,
    postgres_pool: Arc<deadpool::managed::Pool<diesel_async::pooled_connection::AsyncDieselConnectionManager<diesel_async::AsyncPgConnection>>>,
    shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) -> tokio::task::JoinHandle<()> {
    let channel_name = channel.to_string();
    let endpoint = endpoint.to_string();
    let symbols = symbols.to_vec();
    let token = token.to_string();
    
    tokio::spawn(async move {
        log_info!(
            MAIN_LOGGER,
            "Connecting to {} channel at {}",
            channel_name,
            endpoint
        );

        // Build subscription and unsubscription messages
        let subscribe_message = build_subscribe_message(
            &channel_name,
            &symbols,
            &token,
            depth,
        );

        let unsubscribe_message = build_unsubscribe_message(
            &channel_name,
            &symbols,
            &token,
            depth,
        );

        // Create and start WebSocket handler
        match KrakenWebSocketHandler::new(
            &endpoint,
            orderbooks,
            &subscribe_message,
            &unsubscribe_message,
            redis_pool,
            postgres_pool,
        )
        .await
        {
            Ok(handler) => {
                let handler = Arc::new(handler);
                match handler.listen(shutdown_rx).await {
                    Ok(_) => {
                        log_info!(
                            MAIN_LOGGER,
                            "Successfully listening to {} channel",
                            channel_name
                        );
                    }
                    Err(e) => {
                        log_error!(
                            MAIN_LOGGER,
                            "Failed to listen on {} channel: {}",
                            channel_name,
                            e
                        );
                    }
                }
            }
            Err(e) => {
                log_error!(
                    MAIN_LOGGER,
                    "Failed to create handler for {} channel: {}",
                    channel_name,
                    e
                );
            }
        }
    })
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_subscribe_message_level3() {
        let symbols = vec!["BTC/USD".to_string(), "ETH/USD".to_string()];
        let msg = build_subscribe_message("level3", &symbols, "test_token", Some(100));
        
        assert_eq!(msg["method"], "subscribe");
        assert_eq!(msg["params"]["channel"], "level3");
        assert_eq!(msg["params"]["depth"], 100);
        assert_eq!(msg["params"]["snapshot"], true);
        assert_eq!(msg["params"]["token"], "test_token");
        assert!(msg["params"]["symbol"].is_array());
    }

    #[test]
    fn test_build_subscribe_message_trade() {
        let symbols = vec!["BTC/USD".to_string()];
        let msg = build_subscribe_message("trade", &symbols, "test_token", None);
        
        assert_eq!(msg["method"], "subscribe");
        assert_eq!(msg["params"]["channel"], "trade");
        assert_eq!(msg["params"]["snapshot"], true);
    }

    #[test]
    fn test_build_subscribe_message_balances() {
        let symbols = vec![];
        let msg = build_subscribe_message("balances", &symbols, "test_token", None);
        
        assert_eq!(msg["method"], "subscribe");
        assert_eq!(msg["params"]["channel"], "balances");
        assert_eq!(msg["params"]["token"], "test_token");
    }

    #[test]
    fn test_build_unsubscribe_message() {
        let symbols = vec!["BTC/USD".to_string()];
        let msg = build_unsubscribe_message("level3", &symbols, "test_token", Some(100));
        
        assert_eq!(msg["method"], "unsubscribe");
        assert_eq!(msg["params"]["channel"], "level3");
    }
}