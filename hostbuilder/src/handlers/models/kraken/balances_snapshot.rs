use std::sync::Arc;

use anyhow::Result;
use bigdecimal::ToPrimitive;
use config::Config;
use databaseschema::{
    models::order_book::OrderBook, CustomAsyncPgConnectionManager
};
use deadpool::managed::Pool;
use deadpool_redis::{
    Connection,
    Manager
};
use protocol::broker::messages::{portfolio_message, publish_request, PortfolioMessage, PublishRequest, Wallet, Wallets};
use publisher::Publisher;
use redis_utils::get_redis_connection;
use serde::{
    Deserialize,
    Serialize
};
use tokio::sync::Mutex;
use tokio_retry::{
    strategy::{
        jitter,
        ExponentialBackoff
    },
    Retry
};
use tracing::{error, warn, debug};
use std::time::Instant;

use crate::handlers::logging::{KRAKEN_BALANCE_LOGGER, BalanceUpdateLog, KRAKEN_SNAPSHOT_LOGGER, SnapshotProcessingLog};

#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceSnapshot {
    channel: String,
    pub data: Vec<BalanceData>,
    r#type: String,
    sequence: u64
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceData {
    asset: String,
    asset_class: String,
    balance: f64,
    wallets: Vec<WalletData>,
}

impl BalanceData {
    pub fn get_asset(&self) -> &str {
        &self.asset
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletData {
    r#type: String,
    id: String,
    balance: f64,
}

impl BalanceSnapshot {
    pub fn from_json(json: &serde_json::Value) -> Result<BalanceSnapshot> {
        let snapshot: BalanceSnapshot = serde_json::from_value(json.clone())?;
        Ok(snapshot)
    }

    pub async fn process_snapshot(
        &self,
        redis_pool: Arc<Pool<Manager, Connection>>,
        _postgres_pool: Arc<Pool<CustomAsyncPgConnectionManager>>,
        config: &Config,
        _publisher: Arc<Mutex<Publisher>>,
        order_book: &OrderBook
    ) -> Result<()> {
        let start = Instant::now();
        let mut processing_error: Option<String> = None;
        
        // Reduce retry attempts for lower latency
        let retry_strategy = ExponentialBackoff::from_millis(5).map(jitter).take(2);

        // Get Redis connection
        let balances_connection_future = get_redis_connection(&redis_pool);
        let balances_connection = balances_connection_future.await.expect("Failed to create redis connection");

        if self.data.is_empty() {
            debug!("balances data is empty");
            return Ok(());
        }

        // Pre-allocate arrays with capacity based on data size
        let balances_count = self.data.len();
        let mut balances = Vec::with_capacity(balances_count);
        let mut pipe = redis::pipe();
        pipe.atomic();

        // Process the balance snapshot data here
        // For example, you can log the balance data or perform any other operations needed
        for balance in self.data.iter() {
            println!("Asset: {}, Balance: {}", balance.asset, balance.balance);
            for wallet in &balance.wallets {
                println!("Wallet Type: {}, ID: {}, Balance: {}", wallet.r#type, wallet.id, wallet.balance);
                
                let wallet = Wallet {
                    user_id: "default_user".to_string(),
                    symbol: balance.asset.clone(),
                    balance: wallet.balance.to_f32().unwrap_or(0.0),
                    currency: balance.asset.clone(),
                    last_updated: chrono::Utc::now().timestamp(),
                };

                // Use key once
                let key = format!("wallet:{}", wallet.symbol);
                
                pipe.cmd("HSET")
                    .arg(key)
                    .arg("symbol").arg(&wallet.symbol)
                    .arg("balance").arg(&wallet.balance);

                balances.push(wallet.clone());
            }
        }

        let wallets_payload = protocol::generated::WalletData {
            exchange: order_book.exchange.clone(),
            wallets: balances.clone(),
        };

        let portfolio_message = PortfolioMessage {
            portfolio_id: "kraken_balances".to_string(),
            payload: Some(portfolio_message::Payload::WalletsPayload(wallets_payload)),
        };

        let topics = &config.topics;
        let message = PublishRequest {
            topic: topics[0].clone(), // Use first topic as string
            payload: Some(publish_request::Payload::PortfolioPayload(portfolio_message)),
        };

        // Execute Redis, Postgres and Publisher operations concurrently
        let redis_future = Retry::spawn(retry_strategy, || async {
            match pipe.query_async::<_, ()>(&mut balances_connection.clone()).await {
                Ok(_) => {
                    debug!("Added trades to redis");
                    Ok(())
                }
                Err(e) => {
                    warn!("Failed to add trades to redis: {}", e);
                    Err(e)
                }
            }
        });

        let publisher_clone = _publisher.clone();
        let message_clone = message.clone();
        let topic_name = config.topic_routing
            .get("balances_snapshot")
            .cloned()
            .unwrap_or_else(|| "portfolio.updates.snapshots".to_string());
        let publisher_future = async move {
            match publisher_clone.lock().await.publish(message_clone, &topic_name) {
                Ok(_) => {
                    debug!("Successfully published balances snapshot message");
                },
                Err(e) => {
                    error!("Failed to publish balances snapshot message: {:?}", e);
                }
            }
        };

        // Wait for all futures to complete
        let (redis_result, _) = tokio::join!(
            redis_future,
            publisher_future
        );

        if let Err(e) = redis_result {
            error!("Failed to add trades to redis after retries: {}", e);
        }
        Ok(())
    }
}