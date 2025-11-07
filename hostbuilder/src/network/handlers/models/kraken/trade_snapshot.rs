use std::sync::Arc;
use tokio::sync::Mutex; // Add tokio::sync::Mutex for optional publisher

use anyhow::Result;
use bigdecimal::ToPrimitive;
use config::Config;
use databaseschema::{
    models::{historical_order::NewHistoricalOrder, order_book::OrderBook}, ops::historical_order_ops::create_historical_orders
};
use diesel_async::AsyncPgConnection;
use protocol::broker::messages::{market_message, publish_request, MarketMessage, PublishRequest, Trade, Trades};
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
use crate::network::handlers::logging::KRAKEN_TRADE_LOGGER;
use crate::{log_info, log_warn, log_error, log_debug};
use crate::network::handlers::logging::{KRAKEN_WEBSOCKET_LOGGER, TradeSnapshotLog};
use crate::infrastructure::logging_facade::KRAKEN_LOGGER;
use std::time::Instant;

use crate::network::handlers::logging::{KRAKEN_SNAPSHOT_LOGGER, SnapshotProcessingLog};

use super::trade_data::TradeData;

// Constants for optimization
const EVENT_TRADE: &str = "trade";
const STATUS_FILLED: &str = "filled";

#[derive(Debug, Serialize, Deserialize)]
pub struct TradeSnapshot {
    channel: String,
    r#type: String,
    pub data: Vec<TradeData>,
}

impl TradeSnapshot {
    pub fn from_json(json: &serde_json::Value) -> Result<TradeSnapshot> {
        let snapshot: TradeSnapshot = serde_json::from_value(json.clone())?;
        Ok(snapshot)
    }

    pub async fn process_snapshot(
        &self,
        postgres_pool: Arc<diesel_async::pooled_connection::deadpool::Pool<AsyncPgConnection>>,
        config: &Config,
        _publisher: Option<Arc<Mutex<Publisher>>>, // Optional for database-only mode
        order_book: &OrderBook
    ) -> Result<()> {
        let start = Instant::now();
        
        // Reduce retry attempts for lower latency
        let retry_strategy = ExponentialBackoff::from_millis(5).map(jitter).take(2);

        if self.data.is_empty() {
            log_debug!(KRAKEN_LOGGER, "Trade data is empty");
            
            // Log empty snapshot
            let log_entry = SnapshotProcessingLog {
                symbol: "UNKNOWN".to_string(),
                snapshot_type: "trade_snapshot".to_string(),
                data_count: 0,
                processing_time_micros: start.elapsed().as_micros() as u64,
                data_size_bytes: 0,
                error: Some("Trade snapshot data is empty".to_string()),
            };
            let _ = KRAKEN_SNAPSHOT_LOGGER.log_snapshot_processing(log_entry).await;
            
            return Ok(());
        }

        let symbol = &self.data[0].symbol;
        let mut processing_error: Option<String> = None;
        
        // Pre-allocate arrays with capacity based on data size
        let trade_count = self.data.len();
        let mut trades = Vec::with_capacity(trade_count);
        let mut trade_messages = Vec::with_capacity(trade_count);

        // Prepare all data first before any async operations
        for trade in self.data.iter() {
            // Convert values once and reuse
            let price_f32 = trade.price.to_f32().unwrap_or(0.0);
            let qty_f32 = trade.qty.to_f32().unwrap_or(0.0);
            let trade_id_str = trade.trade_id.to_string();
            let timestamp_i64 = trade.timestamp.timestamp();

            let trade_message = Trade {
                symbol: trade.symbol.clone(),
                exchange: order_book.exchange.clone(),
                side: trade.side.clone(),
                price: price_f32,
                quantity: qty_f32,
                qty: qty_f32,
                ord_type: trade.ord_type.clone(),
                timestamp: timestamp_i64,
                trade_id: trade_id_str.clone(),
            };

            let historical_trade = NewHistoricalOrder::new(
                trade.timestamp,
                &trade_id_str,
                EVENT_TRADE,
                &trade.side,
                &trade.price,
                &trade.qty,
                None,
                None,
                STATUS_FILLED,
                &order_book.exchange,
                &trade.symbol,
                order_book.exchange_id,
                order_book.security_id,
            );

            trades.push(historical_trade);
            trade_messages.push(trade_message);
        }

        // Prepare message payload outside of any async operations
        let trade_messages_payload = Trades {
            trades: trade_messages,
        };

        let market_message = MarketMessage {
            market_id: "kraken_trades".to_string(),
            payload: Some(market_message::Payload::TradesPayload(trade_messages_payload)),
        };

        let topics = &config.topics;
        let message = PublishRequest {
            topic: topics[0].clone(), // Use first topic as string
            payload: Some(publish_request::Payload::MarketPayload(market_message)),
        };
        
        // Only store trade data in historical_orders for snapshots (not real-time trades)
        let postgres_future = create_historical_orders(postgres_pool.clone(), trades);
        
        // Handle publisher conditionally for database-only mode
        let publisher_future = if let Some(publisher_ref) = _publisher {
            let publisher_clone = Arc::clone(&publisher_ref);
            let message_clone = message.clone();
            let topic_name = config.topic_routing
                .get("trade_snapshot")
                .cloned()
                .unwrap_or_else(|| "market.data.kraken.trade_snapshots".to_string());
                
            Some(async move {
                match publisher_clone.lock().await.publish(message_clone, &topic_name) {
                    Ok(_) => {
                        log_debug!(KRAKEN_LOGGER, "Successfully published trade snapshot message");
                    },
                    Err(e) => {
                        log_error!(KRAKEN_LOGGER, "Failed to publish trade snapshot message: {:?}", e);
                    }
                }
            })
        } else {
            log_debug!(KRAKEN_LOGGER, "🔒 DATABASE-ONLY MODE: Skipping message broker publishing for trade snapshot");
            None
        };
        
        // Wait for all futures to complete
        let postgres_result = if let Some(pub_future) = publisher_future {
            let (postgres_result, _) = tokio::join!(
                postgres_future,
                pub_future
            );
            postgres_result
        } else {
            postgres_future.await
        };

        if let Err(e) = postgres_result {
            log_error!(KRAKEN_LOGGER, "Failed to add historical orders to postgres: {}", e);
            processing_error = Some(format!("Postgres error: {}", e));
        } else {
            log_debug!(KRAKEN_LOGGER, "Added historical orders to postgres");
        }

        // Log trade snapshot processing completion
        let data_size = std::mem::size_of_val(&self.data);
        let log_entry = SnapshotProcessingLog {
            symbol: symbol.clone(),
            snapshot_type: "trade_snapshot".to_string(),
            data_count: self.data.len(),
            processing_time_micros: start.elapsed().as_micros() as u64,
            data_size_bytes: data_size,
            error: processing_error,
        };
        let _ = KRAKEN_SNAPSHOT_LOGGER.log_snapshot_processing(log_entry).await;

        Ok(())
    }
}
