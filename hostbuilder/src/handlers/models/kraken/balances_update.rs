use std::sync::Arc;
use tokio::sync::Mutex; // Add tokio::sync::Mutex for optional publisher

use anyhow::Result;
use bigdecimal::ToPrimitive;
use chrono::{DateTime, Utc};
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
use tokio_retry::{
    strategy::{
        jitter,
        ExponentialBackoff
    },
    Retry
};
use tracing::{error, warn, debug};
use std::time::Instant;

use crate::handlers::logging::{KRAKEN_BALANCE_LOGGER, BalanceUpdateLog};


#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceUpdate {
    channel: String,
    r#type: String,
    data: Vec<BalanceData>,
    sequence: u64
}

impl BalanceUpdate {
    pub fn get_data(&self) -> &[BalanceData] {
        &self.data
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceData {
    ledger_id: String,
    ref_id: String,
    timestamp: DateTime<Utc>,
    r#type: String,
    asset: String,
    asset_class: String,
    category: String,
    wallet_type: String,
    wallet_id: String,
    amount: f64,
    fee: f64,
    balance: f64,
}

impl BalanceData {
    pub fn get_asset(&self) -> &str {
        &self.asset
    }
}

impl BalanceUpdate {
    pub fn from_json(json: &serde_json::Value) -> Result<BalanceUpdate> {
        let update: BalanceUpdate = serde_json::from_value(json.clone())?;
        Ok(update)
    }

    pub async fn process_update(
        &self,
        redis_pool: Arc<Pool<Manager, Connection>>,
        _postgres_pool: Arc<Pool<CustomAsyncPgConnectionManager>>,
        config: &Config,
        _publisher: Option<Arc<Mutex<Publisher>>>, // Optional for database-only mode
        order_book: &OrderBook
    ) -> Result<()> {
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
            let wallet = Wallet {
                user_id: "default_user".to_string(),
                symbol: balance.asset.clone(),
                balance: balance.amount.to_f32().unwrap_or(0.0),
                currency: balance.asset.clone(),
                last_updated: chrono::Utc::now().timestamp(),
            };
            balances.push(wallet.clone());
        }

        let wallets_payload = protocol::generated::WalletData {
            exchange: order_book.exchange.clone(),
            wallets: balances.clone(),
        };

        let portfolio_message = PortfolioMessage {
            portfolio_id: "kraken_balances_update".to_string(),
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

        // Handle publisher conditionally for database-only mode
        let publisher_future = if let Some(publisher_ref) = _publisher {
            let publisher_clone = Arc::clone(&publisher_ref);
            let message_clone = message.clone();
            let topic_name = config.topic_routing
                .get("balances")
                .cloned()
                .unwrap_or_else(|| "portfolio.updates.balances".to_string());
                
            Some(async move {
                match publisher_clone.lock().await.publish(message_clone, &topic_name) {
                    Ok(_) => {
                        debug!("Successfully published portfolio message");
                    },
                    Err(e) => {
                        error!("Failed to publish portfolio message: {:?}", e);
                    }
                }
            })
        } else {
            debug!("🔒 DATABASE-ONLY MODE: Skipping message broker publishing for balances update");
            None
        };

        // Wait for all futures to complete
        let redis_result = if let Some(pub_future) = publisher_future {
            let (redis_result, _) = tokio::join!(
                redis_future,
                pub_future
            );
            redis_result
        } else {
            redis_future.await
        };

        if let Err(e) = redis_result {
            error!("Failed to add trades to redis after retries: {}", e);
        }
        Ok(())
    }
}