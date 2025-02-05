mod config;
pub mod endpoint;
pub mod get_info;
pub mod handlers;

use anyhow::Result;
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
    sync::Arc
};
#[cfg(unix)]
use tokio::signal::unix::{
    signal,
    SignalKind
};
use tokio::{
    signal,
    sync::broadcast::channel,
    time::{
        timeout,
        Duration
    }
};

use crate::endpoint::EndpointHandler;

#[automock]
#[async_trait]
pub trait HostedObjectTrait {
    async fn run(&mut self) -> Result<()>;
}

pub struct HostedObject {
    config: Config,
    redis_pool: Arc<Pool<Manager, Connection>>,
    postgres_pool: Arc<Pool<CustomAsyncPgConnectionManager>>,
}

impl HostedObject {
    pub async fn new() -> Result<Self> {
        dotenv().ok();
        let config_path = env::var("CONFIG_PATH").expect("CONFIG_PATH must be set");
        let config = Config::new(&config_path)?;
        let redis_pool = Arc::new(create_redis_pool().expect("Failed to create Redis pool"));
        let postgres_pool = Arc::new(create_timescale_connection_pool());

        Ok(Self {
            config,
            redis_pool,
            postgres_pool,
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
                    self.redis_pool.clone(),
                    self.postgres_pool.clone(),
                    exchange.name,
                )
                .await {
                    Ok(kraken_exchange) => {
                        println!("Successfully retrieved exchange: {}", kraken_exchange.exchange);
                        let auth_endpoint = exchange.websocket_token.clone();
                        let api_response = KrakenRestHandler::authenticate(&auth_endpoint).await?;

                        for websocket in exchange.websockets.iter().cloned() {
                            let api_response = api_response.clone();

                            match get_securities(
                                self.redis_pool.clone(),
                                self.postgres_pool.clone(),
                                &exchange.symbols,
                            )
                            .await {
                                Ok(securities) => {
                                    println!("Successfully retrieved securities: {:?}", securities);
                                    match get_orderbooks(
                                        self.redis_pool.clone(),
                                        self.postgres_pool.clone(),
                                        securities,
                                        &kraken_exchange,
                                    )
                                    .await {
                                        Ok(orderbooks) => {
                                            println!("Successfully retrieved orderbooks");
                                            let orderbooks = orderbooks.clone();
                                            let shutdown_tx = shutdown_tx.clone();
                                            let redis_pool = self.redis_pool.clone();
                                            let postgres_pool = self.postgres_pool.clone();
                                            let symbols = exchange.symbols.clone();
                                            let handle = tokio::spawn(async move {
                                                println!("Connecting to {:?}", &websocket.endpoint);
            
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
                                                                        println!(
                                                                            "Listening to Kraken level 3"
                                                                        );

                                                                    }
                                                                    Err(e) => eprintln!("Failed to listen: {}", e),
                                                                }
                                                            }
                                                            Err(e) => eprintln!("Failed to create handler: {}", e),
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
            match  timeout(Duration::from_secs(30), async {
                #[cfg(unix)]
                {
                    // Use SIGTERM signal handler for Unix-based systems (Kubernetes)
                    match signal(SignalKind::terminate()) {
                        Ok(mut term_signal) => {
                            match term_signal.recv().await {
                                Some(_) => {
                                    println!("Received SIGTERM, initiating shutdown...");
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
                        Ok(_) => println!("Received Ctrl+C, initiating shutdown..."),
                        Err(e) => eprintln!("Failed to listen for ctrl_c shutdown signal: {}", e),
                    }
                    
                }

                if let Err(e) = shutdown_tx.send(()) {
                    eprintln!("Failed to send shutdown signal: {}", e);
                }
            }).await {
                Ok(_) => {
                    println!("Shutdown completed successfully");
                }
                Err(_) => {
                    eprintln!("Shutdown timed out");
                }
            }
        });

        for handle in handles {
            let _ = handle.await;
        }

        let _ = shutdown_handle.await;
        Ok(())
    }
}