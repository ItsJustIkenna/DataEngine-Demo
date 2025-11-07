use std::sync::Arc;
use tokio::sync::Mutex; // Add tokio::sync::Mutex for optional publisher

use anyhow::Result;
use bigdecimal::{BigDecimal, Zero, ToPrimitive};
use chrono::{DateTime, Utc};
use config::Config;
use databaseschema::{
    models::{historical_order::NewHistoricalOrder, open_buy_order::NewOpenBuyOrder, open_sell_order::NewOpenSellOrder, order_book::OrderBook}, ops::{
        historical_order_ops::create_historical_orders, open_buy_order_ops::create_open_buy_orders, open_sell_order_ops::create_open_sell_orders, order_book_ops::update_orderbook
    }
};
use diesel_async::AsyncPgConnection;
use protocol::broker::messages::{market_message, publish_request, MarketMessage, Order, Orders, PublishRequest};
use publisher::Publisher;
use redis::cmd;
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
use crate::infrastructure::logging_facade::KRAKEN_LOGGER;
use crate::{log_debug, log_info, log_warn, log_error};
use std::time::Instant;

use crate::network::handlers::logging::{KRAKEN_WEBSOCKET_LOGGER, KRAKEN_SNAPSHOT_LOGGER, Level3SnapshotLog, SnapshotProcessingLog};


// Common strings to avoid allocations
const SIDE_BUY: &str = "buy";
const SIDE_SELL: &str = "sell";
const EVENT_NEW: &str = "new";
const STATUS_OPEN: &str = "open";

#[derive(Debug, Serialize, Deserialize)]
pub struct Level3Snapshot {
    channel: String,
    r#type: String,
    pub data: Vec<Level3SnapshotData>,
}

impl Level3Snapshot {
    pub fn from_json(json: &serde_json::Value) -> Result<Level3Snapshot> {
        let snapshot: Level3Snapshot = serde_json::from_value(json.clone())?;
        Ok(snapshot)
    }

    pub async fn process_snapshot(
        &self,
        _redis_pool: Arc<deadpool_redis::Pool>, // Unused - Redis caching removed
        postgres_pool: Arc<diesel_async::pooled_connection::deadpool::Pool<AsyncPgConnection>>,
        config: &Config,
        _publisher: Option<Arc<Mutex<Publisher>>>, // Optional for database-only mode
        order_book: &OrderBook,
    ) -> Result<()> {
        eprintln!("DEBUG: process_snapshot - Entry point (Redis caching disabled)");
        let start = Instant::now();

        if self.data.is_empty() {
            log_info!(KRAKEN_LOGGER, "⚠️  Level3 snapshot has NO data - empty snapshot received");
            
            // Log empty snapshot
            let log_entry = SnapshotProcessingLog {
                symbol: "UNKNOWN".to_string(),
                snapshot_type: "level3_snapshot".to_string(),
                data_count: 0,
                processing_time_micros: start.elapsed().as_micros() as u64,
                data_size_bytes: 0,
                error: Some("Snapshot data is empty".to_string()),
            };
            let _ = KRAKEN_SNAPSHOT_LOGGER.log_snapshot_processing(log_entry).await;
            
            return Ok(());
        }

        let symbol = &self.data[0].symbol;
        let mut processing_error: Option<String> = None;
        
        let snapshot_data = &self.data[0];

        if snapshot_data.bids.is_empty() && snapshot_data.asks.is_empty() {
            log_info!(KRAKEN_LOGGER, "⚠️  Level3 snapshot for {} has EMPTY bids and asks - no orderbook data received", symbol);
            return Ok(());
        }
        
        eprintln!("DEBUG: process_snapshot - Processing {} bids and {} asks", snapshot_data.bids.len(), snapshot_data.asks.len());
        
        let mut open_bid_volume = BigDecimal::zero();
        let mut open_ask_volume = BigDecimal::zero();

        // Process bids concurrently
        let bid_futures = async {
            if !snapshot_data.bids.is_empty() {
                eprintln!("DEBUG: process_snapshot - Processing bids");
                
                // Pre-allocate all collections with appropriate capacity
                let bid_count = snapshot_data.bids.len();
                let mut buy_orders = Vec::with_capacity(bid_count);
                let mut historical_orders = Vec::with_capacity(bid_count);
                
                // Process bids and collect them in pre-allocated vectors
                for bid in snapshot_data.bids.iter() {
                    let price_level = &bid.limit_price;
                    let buy_quantity = &bid.order_qty;
                    let unique_id = &bid.order_id;
                    let timestamp = bid.timestamp;

                    open_bid_volume += buy_quantity;

                    let order = NewOpenBuyOrder::new(
                        timestamp.clone(),
                        &order_book.symbol,
                        &order_book.exchange,
                        order_book.security_id,
                        order_book.exchange_id,
                        order_book.buy_order_book_id,
                        unique_id,
                        price_level,
                        buy_quantity,
                    );

                    let historical_order = NewHistoricalOrder::new(
                        timestamp.clone(),
                        unique_id,
                        EVENT_NEW,
                        SIDE_BUY,
                        price_level,
                        buy_quantity,
                        None,
                        None,
                        STATUS_OPEN,
                        &order_book.exchange,
                        &order_book.symbol,
                        order_book.exchange_id,
                        order_book.security_id,
                    );

                    buy_orders.push(order);
                    historical_orders.push(historical_order);
                }

                eprintln!("DEBUG: process_snapshot - About to persist {} buy orders to database", buy_orders.len());
                
                // Execute database operations concurrently
                let buy_count = buy_orders.len();
                let historical_count = historical_orders.len();

                let create_buy_orders_future = create_open_buy_orders(postgres_pool.clone(), buy_orders);
                let create_historical_future = create_historical_orders(postgres_pool.clone(), historical_orders);

                let (buy_result, historical_result) = tokio::join!(create_buy_orders_future, create_historical_future);

                if let Err(e) = buy_result {
                    log_error!(KRAKEN_LOGGER, "Failed to add buy orders to postgres: {}", e);
                } else {
                    log_info!(KRAKEN_LOGGER, "✅ Successfully persisted {} buy orders to database", buy_count);
                }

                if let Err(e) = historical_result {
                    log_error!(KRAKEN_LOGGER, "Failed to add historical orders to postgres: {}", e);
                } else {
                    log_info!(KRAKEN_LOGGER, "✅ Successfully persisted {} historical orders to database", historical_count);
                }
            }
        };

        // Process asks concurrently
        let ask_futures = async {
            if !snapshot_data.asks.is_empty() {
                eprintln!("DEBUG: process_snapshot - Processing asks");
                
                // Pre-allocate all collections with appropriate capacity
                let ask_count = snapshot_data.asks.len();
                let mut sell_orders = Vec::with_capacity(ask_count);
                let mut historical_orders = Vec::with_capacity(ask_count);
                
                for ask in snapshot_data.asks.iter() {
                    let price_level = &ask.limit_price;
                    let sell_quantity = &ask.order_qty;
                    let unique_id = &ask.order_id;
                    let timestamp = ask.timestamp;

                    open_ask_volume += sell_quantity;

                    let order = NewOpenSellOrder::new(
                        timestamp.clone(),
                        &order_book.symbol,
                        &order_book.exchange,
                        order_book.security_id,
                        order_book.exchange_id,
                        order_book.sell_order_book_id,
                        unique_id,
                        price_level,
                        sell_quantity,
                    );

                    let historical_order = NewHistoricalOrder::new(
                        timestamp.clone(),
                        unique_id,
                        EVENT_NEW,
                        SIDE_SELL,
                        price_level,
                        sell_quantity,
                        None,
                        None,
                        STATUS_OPEN,
                        &order_book.exchange,
                        &order_book.symbol,
                        order_book.exchange_id,
                        order_book.security_id,
                    );

                    sell_orders.push(order);
                    historical_orders.push(historical_order);
                }

                eprintln!("DEBUG: process_snapshot - About to persist {} sell orders to database", sell_orders.len());
                
                // Execute database operations concurrently
                let sell_count = sell_orders.len();
                let historical_count = historical_orders.len();

                let create_sell_orders_future = create_open_sell_orders(postgres_pool.clone(), sell_orders);
                let create_historical_future = create_historical_orders(postgres_pool.clone(), historical_orders);

                let (sell_result, historical_result) = tokio::join!(create_sell_orders_future, create_historical_future);

                if let Err(e) = sell_result {
                    log_error!(KRAKEN_LOGGER, "Failed to add open sell orders to postgres: {}", e);
                } else {
                    log_info!(KRAKEN_LOGGER, "✅ Successfully persisted {} sell orders to database", sell_count);
                }

                if let Err(e) = historical_result {
                    log_error!(KRAKEN_LOGGER, "Failed to add historical orders to postgres: {}", e);
                } else {
                    log_info!(KRAKEN_LOGGER, "✅ Successfully persisted {} historical orders to database", historical_count);
                }
            }
        };

        // Execute bid and ask processing concurrently
        tokio::join!(bid_futures, ask_futures);
        
        // Skip message broker publishing in DATABASE-ONLY mode (orders collection removed with Redis)
        log_debug!(KRAKEN_LOGGER, "🔒 DATABASE-ONLY MODE: Skipping message broker publishing for level3 snapshot");
        let publish_future: Option<std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>> = None;

        // Calculate total volume
        let total_volume = open_bid_volume.clone() + open_ask_volume.clone();
        log_debug!(KRAKEN_LOGGER, "Total volume: {}", total_volume);

        eprintln!("DEBUG: process_snapshot - About to update orderbook in postgres");
        
        // Update orderbook in Postgres (Redis caching removed)
        let postgres_future = update_orderbook(postgres_pool.clone(), order_book.clone(), total_volume.clone());
        
        // Wait for database update (and optional publish)
        let postgres_result = if let Some(pub_future) = publish_future {
            let (_, postgres_result) = 
                tokio::join!(pub_future, postgres_future);
            postgres_result
        } else {
            postgres_future.await
        };
        
        if let Err(e) = postgres_result {
            eprintln!("DEBUG: process_snapshot - Failed to update orderbook: {}", e);
            log_error!(KRAKEN_LOGGER, "Failed to update order book in postgres: {}", e);
            processing_error = Some(format!("Postgres error: {}", e));
        } else {
            eprintln!("DEBUG: process_snapshot - Successfully updated orderbook in postgres");
            log_info!(KRAKEN_LOGGER, "✅ Successfully updated orderbook in postgres for {}", symbol);
        }

        // Log snapshot processing completion
        let data_size = std::mem::size_of_val(&self.data);
        let log_entry = SnapshotProcessingLog {
            symbol: symbol.clone(),
            snapshot_type: "level3_snapshot".to_string(),
            data_count: self.data.len(),
            processing_time_micros: start.elapsed().as_micros() as u64,
            data_size_bytes: data_size,
            error: processing_error,
        };
        let _ = KRAKEN_SNAPSHOT_LOGGER.log_snapshot_processing(log_entry).await;
        
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Level3SnapshotData {
    pub symbol: String,
    checksum: i64,
    bids: Vec<SnapshotOrder>,
    asks: Vec<SnapshotOrder>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SnapshotOrder {
    order_id: String,
    limit_price: BigDecimal,
    order_qty: BigDecimal,
    timestamp: DateTime<Utc>,
}

