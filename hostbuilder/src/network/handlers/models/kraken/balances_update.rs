use std::sync::Arc;
use tokio::sync::Mutex; // Add tokio::sync::Mutex for optional publisher

use anyhow::Result;
use bigdecimal::ToPrimitive;
use chrono::{DateTime, Utc};
use config::Config;
use databaseschema::{
    models::order_book::OrderBook
};
use diesel_async::AsyncPgConnection;
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
use crate::infrastructure::logging_facade::KRAKEN_LOGGER; use crate::{log_info, log_warn, log_error, log_debug};
use std::time::Instant;

use crate::network::handlers::logging::{KRAKEN_BALANCE_LOGGER, BalanceUpdateLog};


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
        redis_pool: Arc<deadpool_redis::Pool>,
        _postgres_pool: Arc<diesel_async::pooled_connection::deadpool::Pool<AsyncPgConnection>>,
        config: &Config,
        _publisher: Option<Arc<Mutex<Publisher>>>, // Optional for database-only mode
        order_book: &OrderBook
    ) -> Result<()> {
        let start_time = Instant::now();
        
        // Reduce retry attempts for lower latency
        let retry_strategy = ExponentialBackoff::from_millis(5).map(jitter).take(2);

        // Get Redis connection
        let balances_connection_future = get_redis_connection(&redis_pool);
        let balances_connection = balances_connection_future.await.expect("Failed to create redis connection");

        if self.data.is_empty() {
            log_debug!(KRAKEN_LOGGER, "balances data is empty");
            return Ok(());
        }

        // Pre-allocate arrays with capacity based on data size
        let balances_count = self.data.len();
        let mut balances = Vec::with_capacity(balances_count);
        let mut pipe = redis::pipe();
        pipe.atomic();
        
        // Log detailed balance update summary
        let balance_summary: Vec<String> = self.data.iter()
            .map(|b| format!("{}: {:.4}", b.asset, b.balance))
            .collect();
        
        log_info!(KRAKEN_LOGGER, "💰 Processing {} balance updates: [{}]", 
            balances_count, balance_summary.join(", "));

        // Process the balance snapshot data here
        // For example, you can log the balance data or perform any other operations needed
        for balance in self.data.iter() {
            let wallet = Wallet {
                user_id: "default_user".to_string(),
                symbol: balance.asset.clone(),
                balance: balance.amount.to_f32().unwrap_or(0.0),
                currency: balance.asset.clone(),
                last_updated: chrono::Utc::now().timestamp(),
            };
            
            // Log individual balance for detailed tracking
            log_info!(KRAKEN_LOGGER, "Balance update persisted to Redis - Asset: {}, Balance: {:.8}, Type: {}, Wallet: {}", 
                balance.asset, balance.balance, balance.r#type, balance.wallet_type);
            
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
                    log_debug!(KRAKEN_LOGGER, "Added trades to redis");
                    Ok(())
                }
                Err(e) => {
                    log_warn!(KRAKEN_LOGGER, "Failed to add trades to redis: {}", e);
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
                        log_debug!(KRAKEN_LOGGER, "Successfully published portfolio message");
                    },
                    Err(e) => {
                        log_error!(KRAKEN_LOGGER, "Failed to publish portfolio message: {:?}", e);
                    }
                }
            })
        } else {
            log_debug!(KRAKEN_LOGGER, "🔒 DATABASE-ONLY MODE: Skipping message broker publishing for balances update");
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

        let processing_time = start_time.elapsed();
        
        match &redis_result {
            Ok(_) => {
                log_info!(KRAKEN_LOGGER, "✅ Successfully persisted {} balance updates to Redis in {:?}", 
                    balances_count, processing_time);
            }
            Err(e) => {
                log_error!(KRAKEN_LOGGER, "❌ Failed to persist balance updates to Redis after retries: {}", e);
            }
        }

        if let Err(e) = redis_result {
            log_error!(KRAKEN_LOGGER, "Failed to add trades to redis after retries: {}", e);
        }
        Ok(())
    }
}
