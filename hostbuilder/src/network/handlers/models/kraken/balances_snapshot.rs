use std::sync::Arc;
use tokio::sync::Mutex; // Add tokio::sync::Mutex for optional publisher

use anyhow::Result;
use bigdecimal::ToPrimitive;
use config::Config;
use databaseschema::{
    models::order_book::OrderBook
};
use diesel_async::AsyncPgConnection;
use protocol::broker::messages::{portfolio_message, publish_request, PortfolioMessage, PublishRequest};
use protocol::generated::Wallet;
use publisher::Publisher;
// Redis imports removed - no longer using Redis for caching
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
use crate::infrastructure::logging_facade::KRAKEN_LOGGER;
use crate::{log_debug, log_warn, log_error};

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
        _postgres_pool: Arc<diesel_async::pooled_connection::deadpool::Pool<AsyncPgConnection>>,
        config: &Config,
        _publisher: Option<Arc<Mutex<Publisher>>>, // Optional for database-only mode
        order_book: &OrderBook
    ) -> Result<()> {
        // Reduce retry attempts for lower latency (currently unused but available for future use)
        let _retry_strategy = ExponentialBackoff::from_millis(5).map(jitter).take(2);

        if self.data.is_empty() {
            log_debug!(KRAKEN_LOGGER, "balances data is empty");
            return Ok(());
        }

        // Pre-allocate arrays with capacity based on data size
        let balances_count = self.data.len();
        let mut balances = Vec::with_capacity(balances_count);

        // Process the balance snapshot data here
        // For example, you can log the balance data or perform any other operations needed
        for balance in self.data.iter() {
            println!("Asset: {}, Balance: {}", balance.asset, balance.balance);
            for wallet in &balance.wallets {
                println!("Wallet Type: {}, ID: {}, Balance: {}", wallet.r#type, wallet.id, wallet.balance);
                
                let w = Wallet {
                    user_id: "default_user".to_string(),
                    symbol: balance.asset.clone(),
                    balance: wallet.balance.to_f32().unwrap_or(0.0),
                    currency: balance.asset.clone(),
                    last_updated: chrono::Utc::now().timestamp(),
                };

                balances.push(w);
            }
        }

        let wallets_payload = protocol::generated::WalletData {
            exchange: order_book.exchange.clone(),
            wallets: balances,
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

        // Handle publisher conditionally for database-only mode
        if let Some(publisher_ref) = _publisher {
            let topic_name = config.topic_routing
                .get("balances_snapshot")
                .cloned()
                .unwrap_or_else(|| "portfolio.updates.snapshots".to_string());
            
            // Acquire the lock and publish immediately (minimize lock hold time)
            let mut publisher_guard = publisher_ref.lock().await;
            match publisher_guard.publish(message, &topic_name) {
                Ok(_) => {
                    log_debug!(KRAKEN_LOGGER, "Successfully published balances snapshot message");
                },
                Err(e) => {
                    log_error!(KRAKEN_LOGGER, "Failed to publish balances snapshot message: {:?}", e);
                }
            }
        } else {
            log_debug!(KRAKEN_LOGGER, "🔒 DATABASE-ONLY MODE: Skipping message broker publishing for balances snapshot");
        }

        Ok(())
    }
}
