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
use crate::logging_facade::KRAKEN_LOGGER;
use crate::{log_debug, log_warn, log_error};
use std::time::Instant;

use crate::handlers::logging::{KRAKEN_SNAPSHOT_LOGGER, SnapshotProcessingLog};


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
        redis_pool: Arc<deadpool_redis::Pool>,
        postgres_pool: Arc<diesel_async::pooled_connection::deadpool::Pool<AsyncPgConnection>>,
        config: &Config,
        _publisher: Option<Arc<Mutex<Publisher>>>, // Optional for database-only mode
        order_book: &OrderBook,
    ) -> Result<()> {
        let start = Instant::now();
        
        // Reduce retry attempts for lower latency
        let retry_strategy = ExponentialBackoff::from_millis(5).map(jitter).take(2);
        
        // Fetch connections concurrently
        let book_connection_future = get_redis_connection(&redis_pool);
        let bid_connection_future = get_redis_connection(&redis_pool);
        let ask_connection_future = get_redis_connection(&redis_pool);
        
        let (book_connection_result, bid_connection_result, ask_connection_result) = 
            tokio::join!(book_connection_future, bid_connection_future, ask_connection_future);
            
        let book_connection = book_connection_result.expect("Failed to create redis connection");
        let bid_connection = bid_connection_result.expect("Failed to create redis connection");
        let ask_connection = ask_connection_result.expect("Failed to create redis connection");

        if self.data.is_empty() {
            log_debug!(KRAKEN_LOGGER, "Snapshot data is empty");
            
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
            log_debug!(KRAKEN_LOGGER, "Snapshot data has empty bids and asks");
            return Ok(());
        }
        
        // Pre-allocate orders collection with appropriate capacity
        let total_order_count = snapshot_data.bids.len() + snapshot_data.asks.len();
        let orders = Arc::new(Mutex::new(Vec::with_capacity(total_order_count)));
        
        let mut open_bid_volume = BigDecimal::zero();
        let mut open_ask_volume = BigDecimal::zero();

        // Process bids concurrently
        let bid_futures = async {
            if !snapshot_data.bids.is_empty() {
                let orders = Arc::clone(&orders);
                
                // Pre-allocate all collections with appropriate capacity
                let bid_count = snapshot_data.bids.len();
                let mut buy_orders = Vec::with_capacity(bid_count);
                let mut historical_orders = Vec::with_capacity(bid_count);
                let mut pipe = redis::pipe();
                pipe.atomic();
                
                // Process bids and collect them in pre-allocated vectors
                for bid in snapshot_data.bids.iter() {
                    let price_level = &bid.limit_price;
                    let buy_quantity = &bid.order_qty;
                    let unique_id = &bid.order_id;
                    let timestamp = bid.timestamp;

                    open_bid_volume += buy_quantity;

                    let message = Order {
                        unique_id: unique_id.clone(),
                        symbol: order_book.symbol.clone(),
                        exchange: order_book.exchange.clone(),
                        price_level: price_level.to_f32().unwrap_or(0.0),
                        quantity: buy_quantity.to_f32().unwrap_or(0.0),
                        side: SIDE_BUY.to_string(),
                        event: EVENT_NEW.to_string(),
                    };

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

                    // Prepare key once for reuse
                    let key = format!("buy_order:{}", order.unique_id);
                    
                    pipe.cmd("HSET")
                        .arg(key)
                        .arg("symbol").arg(&order.symbol)
                        .arg("exchange").arg(&order.exchange)
                        .arg("security_id").arg(&order.security_id.to_string())
                        .arg("exchange_id").arg(&order.exchange_id.to_string())
                        .arg("buy_order_book_id").arg(&order.buy_order_book_id.to_string())
                        .arg("unique_id").arg(&order.unique_id)
                        .arg("price_level").arg(&order.price_level.to_string())
                        .arg("buy_quantity").arg(&order.buy_quantity.to_string());

                    buy_orders.push(order);
                    orders.lock().await.push(message);
                    historical_orders.push(historical_order);
                }

                // Execute Redis pipe
                let result = Retry::spawn(retry_strategy.clone(), || async {
                    match pipe.query_async::<_, ()>(&mut bid_connection.clone()).await {
                        Ok(_) => {
                            log_debug!(KRAKEN_LOGGER, "Added buy orders to redis");
                            Ok(())
                        },
                        Err(e) => {
                            log_warn!(KRAKEN_LOGGER, "Failed to add buy orders to redis: {}", e);
                            Err(e)
                        },
                    }
                }).await;

                if let Err(e) = result {
                    log_error!(KRAKEN_LOGGER, "All retries failed for redis commands: {}", e);
                }                    

                // Execute database operations concurrently
                let create_buy_orders_future = create_open_buy_orders(postgres_pool.clone(), buy_orders);
                let create_historical_future = create_historical_orders(postgres_pool.clone(), historical_orders);
                
                let (buy_result, historical_result) = tokio::join!(create_buy_orders_future, create_historical_future);
                
                if let Err(e) = buy_result {
                    log_error!(KRAKEN_LOGGER, "Failed to add buy orders to postgres: {}", e);
                } else {
                    log_debug!(KRAKEN_LOGGER, "Added buy orders to postgres");
                }
                
                if let Err(e) = historical_result {
                    log_error!(KRAKEN_LOGGER, "Failed to add historical orders to postgres: {}", e);
                } else {
                    log_debug!(KRAKEN_LOGGER, "Added historical orders to postgres");
                }
            }
        };

        // Process asks concurrently
        let ask_futures = async {
            if !snapshot_data.asks.is_empty() {
                let orders = Arc::clone(&orders);
                
                // Pre-allocate all collections with appropriate capacity
                let ask_count = snapshot_data.asks.len();
                let mut sell_orders = Vec::with_capacity(ask_count);
                let mut historical_orders = Vec::with_capacity(ask_count);
                let mut pipe = redis::pipe();
                pipe.atomic();
                
                for ask in snapshot_data.asks.iter() {
                    let price_level = &ask.limit_price;
                    let sell_quantity = &ask.order_qty;
                    let unique_id = &ask.order_id;
                    let timestamp = ask.timestamp;

                    open_ask_volume += sell_quantity;

                    let message = Order {
                        unique_id: ask.order_id.clone(),
                        symbol: order_book.symbol.clone(),
                        exchange: order_book.exchange.clone(),
                        price_level: price_level.to_f32().unwrap_or(0.0),
                        quantity: sell_quantity.to_f32().unwrap_or(0.0),
                        side: SIDE_SELL.to_string(),
                        event: EVENT_NEW.to_string(),
                    };

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

                    // Prepare key once for reuse
                    let key = format!("sell_order:{}", order.unique_id);
                    
                    pipe.cmd("HSET")
                        .arg(key)
                        .arg("symbol").arg(&order.symbol)
                        .arg("exchange").arg(&order.exchange)
                        .arg("security_id").arg(&order.security_id.to_string())
                        .arg("exchange_id").arg(&order.exchange_id.to_string())
                        .arg("sell_order_book_id").arg(&order.sell_order_book_id.to_string())
                        .arg("unique_id").arg(&order.unique_id)
                        .arg("price_level").arg(&order.price_level.to_string())
                        .arg("sell_quantity").arg(&order.sell_quantity.to_string());

                    sell_orders.push(order);
                    orders.lock().await.push(message);
                    historical_orders.push(historical_order);
                }

                // Execute Redis pipe
                let result = Retry::spawn(retry_strategy.clone(), || async {
                    match pipe.query_async::<_, ()>(&mut ask_connection.clone()).await {
                        Ok(_) => {
                            log_debug!(KRAKEN_LOGGER, "Added sell orders to redis");
                            Ok(())
                        },
                        Err(e) => {
                            log_warn!(KRAKEN_LOGGER, "Failed to add sell orders to redis: {}", e);
                            Err(e)
                        },
                    }
                }).await;

                if let Err(e) = result {
                    log_error!(KRAKEN_LOGGER, "All retries failed for redis commands: {}", e);
                }  

                // Execute database operations concurrently
                let create_sell_orders_future = create_open_sell_orders(postgres_pool.clone(), sell_orders);
                let create_historical_future = create_historical_orders(postgres_pool.clone(), historical_orders);
                
                let (sell_result, historical_result) = tokio::join!(create_sell_orders_future, create_historical_future);
                
                if let Err(e) = sell_result {
                    log_error!(KRAKEN_LOGGER, "Failed to add sell orders to postgres: {}", e);
                } else {
                    log_debug!(KRAKEN_LOGGER, "Added sell orders to postgres");
                }
                
                if let Err(e) = historical_result {
                    log_error!(KRAKEN_LOGGER, "Failed to add historical orders to postgres: {}", e);
                } else {
                    log_debug!(KRAKEN_LOGGER, "Added historical orders to postgres");
                }
            }
        };

        // Execute bid and ask processing concurrently
        tokio::join!(bid_futures, ask_futures);
        
        // Prepare orders message
        let orders_data = {
            let locked_orders = orders.lock().await;
            locked_orders.clone()
        };
        
        let message_orders = Orders {
            orders: orders_data
        };
        
        let market_message = MarketMessage {
            market_id: "kraken_level3_snapshot".to_string(),
            payload: Some(market_message::Payload::OrdersPayload(message_orders)),
        };
        
        let topics = &config.topics;
        let message = PublishRequest {
            topic: topics[0].clone(), // Use first topic as string
            payload: Some(publish_request::Payload::MarketPayload(market_message)),
        };
        
        // Connect to publisher and send message in parallel with other work (if enabled)
        let publish_future = if let Some(publisher_ref) = _publisher {
            let publisher_clone = Arc::clone(&publisher_ref);
            let message_clone = message.clone();
            let topic_name = config.topic_routing
                .get("level3_snapshot")
                .cloned()
                .unwrap_or_else(|| "market.data.kraken.snapshots".to_string());
                
            Some(async move {
                match publisher_clone.lock().await.publish(message_clone, &topic_name) {
                    Ok(_) => {
                        log_debug!(KRAKEN_LOGGER, "Successfully published level3 snapshot message");
                    },
                    Err(e) => {
                        log_error!(KRAKEN_LOGGER, "Failed to publish level3 snapshot message: {:?}", e);
                    }
                }
            })
        } else {
            log_debug!(KRAKEN_LOGGER, "🔒 DATABASE-ONLY MODE: Skipping message broker publishing for level3 snapshot");
            None
        };

        // Calculate total volume
        let total_volume = open_bid_volume.clone() + open_ask_volume.clone();
        log_debug!(KRAKEN_LOGGER, "Total volume: {}", total_volume);

        // Update orderbook in Redis and Postgres concurrently with publishing
        let orderbook_key = format!("order_book:{}", order_book.order_book_id);
        
        let redis_future = Retry::spawn(retry_strategy.clone(), || async {
            match cmd("HSET")
                .arg(&orderbook_key)
                .arg("created_at").arg(&order_book.created_at.to_string())
                .arg("updated_at").arg(&order_book.updated_at.expect("").to_string())
                .arg("symbol").arg(&order_book.symbol)
                .arg("exchange").arg(&order_book.exchange)
                .arg("security_id").arg(&order_book.security_id.to_string())
                .arg("exchange_id").arg(&order_book.exchange_id.to_string())
                .arg("buy_order_book_id").arg(&order_book.buy_order_book_id.to_string())
                .arg("sell_order_book_id").arg(&order_book.sell_order_book_id.to_string())
                .arg("total_volume").arg(&total_volume.to_string())
                .query_async::<_, ()>(&mut book_connection.clone())
                .await
            {
                Ok(_) => {
                    log_debug!(KRAKEN_LOGGER, "Added order book to redis");
                    Ok(())
                }
                Err(e) => {
                    log_warn!(KRAKEN_LOGGER, "Failed to add order book to redis: {}", e);
                    Err(e)
                }
            }
        });
        
        let postgres_future = update_orderbook(postgres_pool.clone(), order_book.clone(), total_volume.clone());
        
        // Wait for all operations (conditionally including publish_future)
        let (redis_result, postgres_result) = if let Some(pub_future) = publish_future {
            let (_, redis_result, postgres_result) = 
                tokio::join!(pub_future, redis_future, postgres_future);
            (redis_result, postgres_result)
        } else {
            let (redis_result, postgres_result) = 
                tokio::join!(redis_future, postgres_future);
            (redis_result, postgres_result)
        };
        
        if let Err(e) = redis_result {
            log_error!(KRAKEN_LOGGER, "All retries failed for redis commands: {}", e);
            processing_error = Some(format!("Redis error: {}", e));
        }
        
        if let Err(e) = postgres_result {
            log_error!(KRAKEN_LOGGER, "Failed to update order book in postgres: {}", e);
            if processing_error.is_none() {
                processing_error = Some(format!("Postgres error: {}", e));
            }
        } else {
            log_debug!(KRAKEN_LOGGER, "Updated order book in postgres");
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

