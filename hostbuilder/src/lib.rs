pub mod caching;
pub mod data_pipeline;
pub mod endpoint;
pub mod get_info;
pub mod handlers;
pub mod kernel_bypass;
pub mod logging_facade;
pub mod monitoring;
pub mod order_book;
pub mod resilience;
pub mod security;
pub mod simd_optimizations;
pub mod tests;
pub mod ultra_metrics;
pub mod api_endpoints;

// Re-export important functions
pub use api_endpoints::start_api_server;

use anyhow::Result;
use crate::logging_facade::MAIN_LOGGER;

use async_trait::async_trait;
use config::Config;
use databaseschema::{
    create_timescale_connection_pool,
    CustomAsyncPgConnectionManager
};
use deadpool::managed::Pool;
use deadpool_redis::{
    Connection,
    Manager
};
use dotenv::dotenv;
use get_info::{
    get_exchange,
    get_orderbooks,
    get_securities
};
use handlers::{
    rest_apis::kraken::kraken_rest_api::KrakenRestHandler,
    websockets::kraken::kraken_websocket_handler::KrakenWebSocketHandler,
};
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
use tokio::signal::unix::{
    signal,
    SignalKind
};
use tokio::{
    signal,
    sync::broadcast::channel
};
use tracing; // Use tracing instead of println

use crate::{
    endpoint::EndpointHandler,
    resilience::ErrorHandler,
    monitoring::{SystemHealthMonitor, WebSocketHealthChecker, DatabaseHealthChecker, PerformanceHealthChecker},
    caching::CacheMaintenanceService,
    security::{SecurityManager, SecurityConfig},
};

#[automock]
#[async_trait]
pub trait HostedObjectTrait {
    async fn run(&mut self) -> Result<()>;
}

pub struct HostedObject {
    config: Config,
    redis_pool: Arc<Pool<Manager, Connection>>,
    postgres_pool: Arc<Pool<CustomAsyncPgConnectionManager>>,
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
        let redis_pool = Arc::new(create_redis_pool()
            .map_err(|e| anyhow::anyhow!("Failed to create Redis pool: {}", e))?);
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
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64
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
        
        let perf_health_checker = Arc::new(PerformanceHealthChecker::new(
            Arc::clone(global_metrics),
        ));
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

        for exchange in config.exchanges.iter().cloned() {
            if exchange.name == "Kraken" {
                match get_exchange(
                    Arc::clone(&self.redis_pool),
                    Arc::clone(&self.postgres_pool),
                    exchange.name,
                )
                .await {
                    Ok(kraken_exchange) => {
                        log_info!(MAIN_LOGGER, "Successfully retrieved exchange: {}", kraken_exchange.exchange);
                        let auth_endpoint = exchange.websocket_token.clone();
                        let api_response = KrakenRestHandler::authenticate(&auth_endpoint).await?;

                        for websocket in exchange.websockets.iter().cloned() {
                            let api_response = api_response.clone();

                            match get_securities(
                                Arc::clone(&self.redis_pool),
                                Arc::clone(&self.postgres_pool),
                                &exchange.symbols,
                            )
                            .await {
                                Ok(securities) => {
                                    log_info!(MAIN_LOGGER, "Successfully retrieved securities: {:?}", securities);
                                    match get_orderbooks(
                                        Arc::clone(&self.redis_pool),
                                        Arc::clone(&self.postgres_pool),
                                        securities,
                                        &kraken_exchange,
                                    )
                                    .await {
                                        Ok(orderbooks) => {
                                            log_info!(MAIN_LOGGER, "Successfully retrieved orderbooks");
                                            let orderbooks = orderbooks.clone();
                                            let shutdown_tx = shutdown_tx.clone();
                                            let redis_pool = Arc::clone(&self.redis_pool);
                                            let postgres_pool = Arc::clone(&self.postgres_pool);
                                            let symbols = exchange.symbols.clone();
                                            let handle = tokio::spawn(async move {
                                                log_info!(MAIN_LOGGER, "Connecting to {:?}", &websocket.endpoint);
            
                                                match websocket.channel.as_str() {
                                                    "level3" => {
                                                        let subscribe_message = json!({
                                                            "method": "subscribe",
                                                            "params": {
                                                                "channel": &websocket.channel,
                                                                "symbol": &symbols,
                                                                "depth": &exchange.depth,
                                                                "snapshot": true,
                                                                "token": &api_response.get_token(),
                                                            }
                                                        });
            
                                                        let unsubscribe_message = json!({
                                                            "method": "unsubscribe",
                                                            "params": {
                                                                "channel": &websocket.channel,
                                                                "symbol": &symbols,
                                                                "token": &api_response.get_token(),
                                                            }
                                                        });
            
                                                        match KrakenWebSocketHandler::new(
                                                            &websocket.endpoint,
                                                            orderbooks.clone(),
                                                            &subscribe_message,
                                                            &unsubscribe_message,
                                                            redis_pool.clone(),
                                                            postgres_pool.clone(),
                                                        )
                                                        .await
                                                        {
                                                            Ok(handler) => {
                                                                let handler = Arc::new(handler);
                                                                match handler.listen(shutdown_tx.subscribe()).await
                                                                {
                                                                    Ok(_) => {
                                                                        log_info!(MAIN_LOGGER, 
                                                                            "Listening to Kraken level 3"
                                                                        );
                                                                    }
                                                                    Err(e) => log_error!(MAIN_LOGGER, "Failed to listen: {}", e),
                                                                }
                                                            }
                                                            Err(e) => log_error!(MAIN_LOGGER, "Failed to create handler: {}", e),
                                                        }
                                                    }
                                                    "trade" => {
                                                        let subscribe_message = json!({
                                                            "method": "subscribe",
                                                            "params": {
                                                                "channel": &websocket.channel,
                                                                "symbol": &symbols,
                                                                "snapshot": true
                                                            }
                                                        });
            
                                                        let unsubscribe_message = json!({
                                                            "method": "unsubscribe",
                                                            "params": {
                                                                "channel": &websocket.channel,
                                                                "symbol": symbols,
                                                                "snapshot": true
                                                            }
                                                        });
            
                                                        match KrakenWebSocketHandler::new(
                                                            &websocket.endpoint,
                                                            orderbooks.clone(),
                                                            &subscribe_message,
                                                            &unsubscribe_message,
                                                            redis_pool.clone(),
                                                            postgres_pool.clone(),
                                                        )
                                                        .await
                                                        {
                                                            Ok(handler) => {
                                                                let handler = Arc::new(handler);
                                                                match handler.listen(shutdown_tx.subscribe()).await
                                                                {
                                                                    Ok(_) => {
                                                                        println!(
                                                                            "Listening to Kraken trade"
                                                                        );
                                                                    }
                                                                    Err(e) => eprintln!("Failed to listen: {}", e),
                                                                }
                                                            }
                                                            Err(e) => eprintln!("Failed to create handler: {}", e),
                                                        }
                                                    }
                                                    "balances" => {
                                                        let subscribe_message = json!({
                                                            "method": "subscribe",
                                                            "params": {
                                                                "channel": &websocket.channel,
                                                                "token": &api_response.get_token(),
                                                            }
                                                        });
            
                                                        let unsubscribe_message = json!({
                                                            "method": "unsubscribe",
                                                            "params": {
                                                                "channel": &websocket.channel,
                                                                "token": &api_response.get_token(),
                                                            }
                                                        });
            
                                                        match KrakenWebSocketHandler::new(
                                                            &websocket.endpoint,
                                                            orderbooks.clone(),
                                                            &subscribe_message,
                                                            &unsubscribe_message,
                                                            redis_pool.clone(),
                                                            postgres_pool.clone(),
                                                        )
                                                        .await
                                                        {
                                                            Ok(handler) => {
                                                                let handler = Arc::new(handler);
                                                                match handler.listen(shutdown_tx.subscribe()).await
                                                                {
                                                                    Ok(_) => {
                                                                        println!(
                                                                            "Listening to Kraken portfolio balances"
                                                                        );
                                                                    }
                                                                    Err(e) => eprintln!("Failed to listen: {}", e),
                                                                }
                                                            }
                                                            Err(e) => eprintln!("Failed to create handler: {}", e),
                                                        }
                                                    }
                                                    _ => {
                                                        println!("Unknown websocket type: {}", websocket.channel.as_str());
                                                    }
                                                };
                                            });
                                            handles.push(handle);
                                        }
                                        Err(e) => {
                                            println!("Failed to retrieve orderbooks: {}", e);
                                        }
                                    }; 
                                }
                                Err(e) => {
                                    println!("Failed to retrieve securities: {}", e);
                                }
                            }; 
                        }
                    }
                    Err(e) => {
                        println!("Failed to retrieve exchange: {}", e);
                    }
                }
            } else {
                println!("Unknown exchange: {}", exchange.name);
            }
        }

        let shutdown_handle = tokio::spawn(async move {
            println!("Shutdown handle spawned");
            #[cfg(unix)]
            {
                // Use SIGTERM signal handler for Unix-based systems (Kubernetes)
                match signal(SignalKind::terminate()) {
                    Ok(mut term_signal) => {
                        match term_signal.recv().await {
                            Some(_) => {
                                println!("Received SIGTERM, initiating shutdown...");
                                if let Err(e) = shutdown_tx.send(()) {
                                    eprintln!("Failed to send shutdown signal: {}", e);
                                }
                            }
                            None => {
                                eprintln!("Failed to receive SIGTERM signal");
                            }
                        };
                    }
                    Err(e) => {
                        eprintln!("Failed to listen for SIGTERM shutdown signal: {}", e);
                    }
                }
            }

            #[cfg(not(unix))]
            {
                // Use ctrl_c for non-Unix systems (Windows testing)
                match signal::ctrl_c().await {
                    Ok(_) => {
                        println!("Received Ctrl+C, initiating shutdown...");
                        if let Err(e) = shutdown_tx.send(()) {
                            eprintln!("Failed to send shutdown signal: {}", e);
                        }
                    }
                    Err(e) => eprintln!("Failed to listen for ctrl_c shutdown signal: {}", e),
                }
                
            }
        });

        handles.push(shutdown_handle);

        for handle in handles {
            let _ = handle.await;
        }
        Ok(())
    }
}
