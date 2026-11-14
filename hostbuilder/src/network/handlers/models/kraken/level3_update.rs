use std::{
    str::FromStr,
    sync::Arc
};

use anyhow::{
    Result,
    Error
};
use bigdecimal::{
    BigDecimal,
    Zero,
    ToPrimitive
};
use chrono::{DateTime, Utc};
use config::Config;
use databaseschema::{
    models::{
        historical_order::NewHistoricalOrder, open_buy_order::NewOpenBuyOrder, open_sell_order::NewOpenSellOrder, order_book::OrderBook
    },
        ops::{
            historical_order_ops::create_historical_orders, open_buy_order_ops::{
                create_open_buy_orders,
                delete_open_buy_orders, modify_open_buy_orders
            }, open_sell_order_ops::{
                create_open_sell_orders,
                delete_open_sell_orders, modify_open_sell_orders
            }, order_book_ops::update_orderbook
        }
};
use diesel_async::AsyncPgConnection;
use protocol::broker::messages::{market_message, publish_request, MarketMessage, Order, Orders, PublishRequest};
use publisher::Publisher;
// Redis imports removed - no longer using Redis for caching
use serde::{
    Deserialize,
    Serialize
};
use tokio::sync::Mutex; // Keep Mutex for compatibility
use crate::infrastructure::logging_facade::KRAKEN_LOGGER;
use crate::{log_debug, log_error, log_info};
use std::time::Instant;

// Import the Kraken logger
use crate::network::handlers::logging::{KRAKEN_ORDERBOOK_LOGGER, OrderBookUpdateLog};
use crate::network::handlers::logging::{KRAKEN_HISTORICAL_LOGGER, HistoricalOrderLog};

// Constants - optimized for cache lines
const INITIAL_ORDERS_CAPACITY: usize = 512; // Increased for better performance
const EVENT_ADD: &str = "add";
const EVENT_MODIFY: &str = "modify";
const EVENT_DELETE: &str = "delete";
const SIDE_BUY: &str = "buy";
const SIDE_SELL: &str = "sell";
const EVENT_NEW: &str = "new";
const EVENT_CANCEL: &str = "cancel";
const STATUS_OPEN: &str = "open";
const STATUS_FILLED: &str = "filled";
const STATUS_PARTIALLY_FILLED: &str = "partially_filled";

#[derive(Debug, Serialize, Deserialize)]
pub struct Level3Update {
    channel: String,
    r#type: String,
    pub data: Vec<Level3UpdateData>,
}

impl Level3Update {
    pub fn from_json(json: &serde_json::Value) -> Result<Level3Update> {
        let update: Level3Update = serde_json::from_value(json.clone())?;
        Ok(update)
    }

    pub async fn process_update(
        &self,
        postgres_pool: Arc<diesel_async::pooled_connection::deadpool::Pool<AsyncPgConnection>>,
        config: &Config,
        _publisher: Option<Arc<tokio::sync::Mutex<Publisher>>>, // Optional for database-only mode
        order_book: &OrderBook,
    ) -> Result<()> {
        let start_time = Instant::now();
        
        if self.data.is_empty() {
            log_debug!(KRAKEN_LOGGER, "Data is empty");
            
            // Log empty update
            let log_update = OrderBookUpdateLog {
                symbol: order_book.symbol.clone(),
                update_type: "empty_update".to_string(),
                bids_count: 0,
                asks_count: 0,
                processing_time_micros: start_time.elapsed().as_micros() as u64,
                total_volume: BigDecimal::zero(),
                error: None,
            };
            let _ = KRAKEN_ORDERBOOK_LOGGER.log_orderbook_update(log_update).await;
            
            return Ok(());
        }
        
        let update_data = &self.data[0];

        if update_data.bids.is_empty() && update_data.asks.is_empty() {
            log_debug!(KRAKEN_LOGGER, "Both bids and asks are empty");
            
            // Log empty bids/asks
            let log_update = OrderBookUpdateLog {
                symbol: update_data.symbol.clone(),
                update_type: "empty_bids_asks".to_string(),
                bids_count: 0,
                asks_count: 0,
                processing_time_micros: start_time.elapsed().as_micros() as u64,
                total_volume: BigDecimal::zero(),
                error: None,
            };
            let _ = KRAKEN_ORDERBOOK_LOGGER.log_orderbook_update(log_update).await;
            return Ok(());
        }

        // Use a pre-allocated Vec with larger capacity
        let orders = Arc::new(Mutex::new(Vec::with_capacity(INITIAL_ORDERS_CAPACITY)));

        let mut bid_volume = BigDecimal::zero();
        let mut ask_volume = BigDecimal::zero();

        // Process bids and asks concurrently
        let bid_futures = async {
            let orders = Arc::clone(&orders);
            // Pre-allocate vecs with appropriate capacity
            let bid_capacity = update_data.bids.len();
            let mut historical_orders = Vec::with_capacity(bid_capacity);
            let mut buy_orders = Vec::with_capacity(bid_capacity);
            let mut modify_orders = Vec::with_capacity(bid_capacity);
            let mut delete_orders: Vec<String> = Vec::with_capacity(bid_capacity);
            
            // Process bids
            for bid in update_data.bids.iter() {
                match bid.event.as_str() {
                    EVENT_ADD => {
                        let price_level = &bid.limit_price;
                        let buy_quantity = &bid.order_qty;
                        let unique_id = &bid.order_id;
                        let timestamp = bid.timestamp;

                        bid_volume += buy_quantity;

                        // Create structs without cloning strings where possible
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
                            timestamp,
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
                        orders.lock().await.push(message);
                        historical_orders.push(historical_order);
                    },
                    EVENT_MODIFY => {
                        let price_level = &bid.limit_price;
                        let new_buy_quantity = &bid.order_qty;
                        let unique_id = &bid.order_id;
                        let timestamp = bid.timestamp;

                        // Note: Without Redis cache, we don't have previous quantity for diff calculation
                        // The modify operation will update the database record with the new quantity
                        bid_volume += new_buy_quantity;

                        let message = Order {
                            unique_id: unique_id.clone(),
                            symbol: order_book.symbol.clone(),
                            exchange: order_book.exchange.clone(),
                            price_level: price_level.to_f32().unwrap_or(0.0),
                            quantity: new_buy_quantity.to_f32().unwrap_or(0.0),
                            side: SIDE_BUY.to_string(),
                            event: EVENT_MODIFY.to_string(),
                        };

                        let historical_order = NewHistoricalOrder::new(
                            timestamp.clone(),
                            unique_id,
                            EVENT_MODIFY,
                            SIDE_BUY,
                            price_level,
                            new_buy_quantity,
                            Some(price_level.clone()),
                            None, // Without Redis, we don't track quantity diff
                            STATUS_PARTIALLY_FILLED,
                            &order_book.exchange,
                            &order_book.symbol,
                            order_book.exchange_id,
                            order_book.security_id,
                        );

                        modify_orders.push((unique_id, price_level, new_buy_quantity));
                        orders.lock().await.push(message);
                        historical_orders.push(historical_order);
                    },
                    EVENT_DELETE => {
                        let timestamp = bid.timestamp;

                        let message = Order {
                            unique_id: bid.order_id.clone(),
                            symbol: order_book.symbol.clone(),
                            exchange: order_book.exchange.clone(),
                            price_level: bid.limit_price.to_f32().unwrap_or(0.0),
                            quantity: bid.order_qty.to_f32().unwrap_or(0.0),
                            side: SIDE_BUY.to_string(),
                            event: EVENT_DELETE.to_string(),
                        };

                        let historical_order = NewHistoricalOrder::new(
                            timestamp.clone(),
                            &bid.order_id,
                            EVENT_CANCEL,
                            SIDE_BUY,
                            &bid.limit_price,
                            &bid.order_qty,
                            None,
                            None,
                            STATUS_FILLED,
                            &order_book.exchange,
                            &order_book.symbol,
                            order_book.exchange_id,
                            order_book.security_id,
                        );

                        bid_volume -= &bid.order_qty;

                        delete_orders.push(bid.order_id.clone());
                        orders.lock().await.push(message);
                        historical_orders.push(historical_order);
                    },
                    _ => {}
                }
            }

            // Capture counts before moving vectors into futures
            let buy_count = buy_orders.len();
            let modify_buy_count = modify_orders.len();
            let delete_buy_count = delete_orders.len();
            let historical_buy_count = historical_orders.len();

            // Process database operations concurrently
            let hist_start = std::time::Instant::now();
            let create_buy_orders_future = create_open_buy_orders(postgres_pool.clone(), buy_orders);
            let modify_buy_orders_future = modify_open_buy_orders(postgres_pool.clone(), modify_orders);
            let delete_buy_orders_future = delete_open_buy_orders(postgres_pool.clone(), &delete_orders);
            let create_historical_future = create_historical_orders(postgres_pool.clone(), historical_orders);
            
            let (create_result, modify_result, delete_result, historical_result) = 
                tokio::join!(
                    create_buy_orders_future, 
                    modify_buy_orders_future, 
                    delete_buy_orders_future, 
                    create_historical_future
                );
                
            if let Err(e) = create_result {
                log_error!(KRAKEN_LOGGER, "Failed to add open buy orders to postgres: {}", e);
            } else {
                log_info!(KRAKEN_LOGGER, "✅ Persisted {} buy orders to database - Symbol: {}, Exchange: {}", 
                    buy_count, order_book.symbol, order_book.exchange);
            }
            
            if let Err(e) = modify_result {
                log_error!(KRAKEN_LOGGER, "Failed to modify open buy orders in postgres: {}", e);
            } else {
                log_info!(KRAKEN_LOGGER, "✏️ Modified {} buy orders in database - Symbol: {}", 
                    modify_buy_count, order_book.symbol);
            }
            
            if let Err(e) = delete_result {
                log_error!(KRAKEN_LOGGER, "Failed to delete open buy orders from postgres: {}", e);
            } else {
                log_info!(KRAKEN_LOGGER, "🗑️ Deleted {} buy orders from database - Symbol: {}", 
                    delete_buy_count, order_book.symbol);
            }
            
            // Log historical order storage with structured logging - granular per event type
            if let Err(e) = &historical_result {
                log_error!(KRAKEN_LOGGER, "Failed to add historical orders to postgres: {}", e);
                
                // Log failure for each event type that occurred
                if buy_count > 0 {
                    let hist_log = HistoricalOrderLog {
                        symbol: order_book.symbol.clone(),
                        event_type: "buy-add".to_string(),
                        order_count: buy_count,
                        processing_time_ms: hist_start.elapsed().as_millis(),
                        success: false,
                        error: Some(e.to_string()),
                    };
                    let _ = KRAKEN_HISTORICAL_LOGGER.log_historical_order_storage(hist_log).await;
                }
                if modify_buy_count > 0 {
                    let hist_log = HistoricalOrderLog {
                        symbol: order_book.symbol.clone(),
                        event_type: "buy-modify".to_string(),
                        order_count: modify_buy_count,
                        processing_time_ms: hist_start.elapsed().as_millis(),
                        success: false,
                        error: Some(e.to_string()),
                    };
                    let _ = KRAKEN_HISTORICAL_LOGGER.log_historical_order_storage(hist_log).await;
                }
                if delete_buy_count > 0 {
                    let hist_log = HistoricalOrderLog {
                        symbol: order_book.symbol.clone(),
                        event_type: "buy-delete".to_string(),
                        order_count: delete_buy_count,
                        processing_time_ms: hist_start.elapsed().as_millis(),
                        success: false,
                        error: Some(e.to_string()),
                    };
                    let _ = KRAKEN_HISTORICAL_LOGGER.log_historical_order_storage(hist_log).await;
                }
            } else {
                log_info!(KRAKEN_LOGGER, "📚 Archived {} historical orders to database", historical_buy_count);
                
                // Log success for each event type that occurred
                if buy_count > 0 {
                    let hist_log = HistoricalOrderLog {
                        symbol: order_book.symbol.clone(),
                        event_type: "buy-add".to_string(),
                        order_count: buy_count,
                        processing_time_ms: hist_start.elapsed().as_millis(),
                        success: true,
                        error: None,
                    };
                    let _ = KRAKEN_HISTORICAL_LOGGER.log_historical_order_storage(hist_log).await;
                }
                if modify_buy_count > 0 {
                    let hist_log = HistoricalOrderLog {
                        symbol: order_book.symbol.clone(),
                        event_type: "buy-modify".to_string(),
                        order_count: modify_buy_count,
                        processing_time_ms: hist_start.elapsed().as_millis(),
                        success: true,
                        error: None,
                    };
                    let _ = KRAKEN_HISTORICAL_LOGGER.log_historical_order_storage(hist_log).await;
                }
                if delete_buy_count > 0 {
                    let hist_log = HistoricalOrderLog {
                        symbol: order_book.symbol.clone(),
                        event_type: "buy-delete".to_string(),
                        order_count: delete_buy_count,
                        processing_time_ms: hist_start.elapsed().as_millis(),
                        success: true,
                        error: None,
                    };
                    let _ = KRAKEN_HISTORICAL_LOGGER.log_historical_order_storage(hist_log).await;
                }
            }

            Ok::<(), Error>(())
        };

        let ask_futures = async {
            let orders = Arc::clone(&orders);
            // Pre-allocate vecs with appropriate capacity
            let ask_capacity = update_data.asks.len();
            let mut historical_orders = Vec::with_capacity(ask_capacity);
            let mut sell_orders = Vec::with_capacity(ask_capacity);
            let mut modify_orders = Vec::with_capacity(ask_capacity);
            let mut delete_orders: Vec<String> = Vec::with_capacity(ask_capacity);

            for ask in update_data.asks.iter() {
                match ask.event.as_str() {
                    EVENT_ADD => {
                        let price_level = &ask.limit_price;
                        let sell_quantity = &ask.order_qty;
                        let unique_id = &ask.order_id;
                        let timestamp = ask.timestamp;

                        ask_volume += sell_quantity;

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
                            timestamp,
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
                        orders.lock().await.push(message);
                        historical_orders.push(historical_order);
                    },
                    EVENT_MODIFY => {
                        let price_level = &ask.limit_price;
                        let new_sell_quantity = &ask.order_qty;
                        let unique_id = &ask.order_id;
                        let timestamp = ask.timestamp;

                        // Note: Without Redis cache, we don't have previous quantity for diff calculation
                        ask_volume += new_sell_quantity;

                        let message = Order {
                            unique_id: ask.order_id.clone(),
                            symbol: order_book.symbol.clone(),
                            exchange: order_book.exchange.clone(),
                            price_level: price_level.to_f32().unwrap_or(0.0),
                            quantity: new_sell_quantity.to_f32().unwrap_or(0.0),
                            side: SIDE_SELL.to_string(),
                            event: EVENT_MODIFY.to_string(),
                        };

                        let historical_order = NewHistoricalOrder::new(
                            timestamp.clone(),
                            unique_id,
                            EVENT_MODIFY,
                            SIDE_SELL,
                            price_level,
                            new_sell_quantity,
                            Some(price_level.clone()),
                            None, // Without Redis, we don't track quantity diff
                            STATUS_PARTIALLY_FILLED,
                            &order_book.exchange,
                            &order_book.symbol,
                            order_book.exchange_id,
                            order_book.security_id,
                        );

                        modify_orders.push((unique_id, price_level, new_sell_quantity));
                        orders.lock().await.push(message);
                        historical_orders.push(historical_order);
                    },
                    EVENT_DELETE => {
                        let timestamp = ask.timestamp;

                        let message = Order {
                            unique_id: ask.order_id.clone(),
                            symbol: order_book.symbol.clone(),
                            exchange: order_book.exchange.clone(),
                            price_level: ask.limit_price.to_f32().unwrap_or(0.0),
                            quantity: ask.order_qty.to_f32().unwrap_or(0.0),
                            side: SIDE_SELL.to_string(),
                            event: EVENT_DELETE.to_string(),
                        };

                        let historical_order = NewHistoricalOrder::new(
                            timestamp.clone(),
                            &ask.order_id,
                            EVENT_CANCEL,
                            SIDE_SELL,
                            &ask.limit_price,
                            &ask.order_qty,
                            None,
                            None,
                            STATUS_FILLED,
                            &order_book.exchange,
                            &order_book.symbol,
                            order_book.exchange_id,
                            order_book.security_id,
                        );

                        ask_volume -= &ask.order_qty;

                        delete_orders.push(ask.order_id.clone());
                        orders.lock().await.push(message);
                        historical_orders.push(historical_order);
                    },
                    _ => {}
                }
            }

            // Capture counts before moving vectors into futures
            let sell_count = sell_orders.len();
            let modify_sell_count = modify_orders.len();
            let delete_sell_count = delete_orders.len();
            let historical_sell_count = historical_orders.len();

            // Process database operations concurrently
            let hist_start = std::time::Instant::now();
            let create_sell_orders_future = create_open_sell_orders(postgres_pool.clone(), sell_orders);
            let modify_sell_orders_future = modify_open_sell_orders(postgres_pool.clone(), modify_orders);
            let delete_sell_orders_future = delete_open_sell_orders(postgres_pool.clone(), &delete_orders);
            let create_historical_future = create_historical_orders(postgres_pool.clone(), historical_orders);
            
            let (create_result, modify_result, delete_result, historical_result) = 
                tokio::join!(
                    create_sell_orders_future, 
                    modify_sell_orders_future, 
                    delete_sell_orders_future, 
                    create_historical_future
                );
                
            if let Err(e) = create_result {
                log_error!(KRAKEN_LOGGER, "Failed to add open sell orders to postgres: {}", e);
            } else {
                log_info!(KRAKEN_LOGGER, "✅ Persisted {} sell orders to database - Symbol: {}, Exchange: {}", 
                    sell_count, order_book.symbol, order_book.exchange);
            }
            
            if let Err(e) = modify_result {
                log_error!(KRAKEN_LOGGER, "Failed to modify open sell orders in postgres: {}", e);
            } else {
                log_info!(KRAKEN_LOGGER, "✏️ Modified {} sell orders in database - Symbol: {}", 
                    modify_sell_count, order_book.symbol);
            }
            
            if let Err(e) = delete_result {
                log_error!(KRAKEN_LOGGER, "Failed to delete open sell orders from postgres: {}", e);
            } else {
                log_info!(KRAKEN_LOGGER, "🗑️ Deleted {} sell orders from database - Symbol: {}", 
                    delete_sell_count, order_book.symbol);
            }
            
            // Log historical order storage with structured logging - granular per event type
            if let Err(e) = &historical_result {
                log_error!(KRAKEN_LOGGER, "Failed to add historical orders to postgres: {}", e);
                
                // Log failure for each event type that occurred
                if sell_count > 0 {
                    let hist_log = HistoricalOrderLog {
                        symbol: order_book.symbol.clone(),
                        event_type: "sell-add".to_string(),
                        order_count: sell_count,
                        processing_time_ms: hist_start.elapsed().as_millis(),
                        success: false,
                        error: Some(e.to_string()),
                    };
                    let _ = KRAKEN_HISTORICAL_LOGGER.log_historical_order_storage(hist_log).await;
                }
                if modify_sell_count > 0 {
                    let hist_log = HistoricalOrderLog {
                        symbol: order_book.symbol.clone(),
                        event_type: "sell-modify".to_string(),
                        order_count: modify_sell_count,
                        processing_time_ms: hist_start.elapsed().as_millis(),
                        success: false,
                        error: Some(e.to_string()),
                    };
                    let _ = KRAKEN_HISTORICAL_LOGGER.log_historical_order_storage(hist_log).await;
                }
                if delete_sell_count > 0 {
                    let hist_log = HistoricalOrderLog {
                        symbol: order_book.symbol.clone(),
                        event_type: "sell-delete".to_string(),
                        order_count: delete_sell_count,
                        processing_time_ms: hist_start.elapsed().as_millis(),
                        success: false,
                        error: Some(e.to_string()),
                    };
                    let _ = KRAKEN_HISTORICAL_LOGGER.log_historical_order_storage(hist_log).await;
                }
            } else {
                log_info!(KRAKEN_LOGGER, "📚 Archived {} historical orders to database", historical_sell_count);
                
                // Log success for each event type that occurred
                if sell_count > 0 {
                    let hist_log = HistoricalOrderLog {
                        symbol: order_book.symbol.clone(),
                        event_type: "sell-add".to_string(),
                        order_count: sell_count,
                        processing_time_ms: hist_start.elapsed().as_millis(),
                        success: true,
                        error: None,
                    };
                    let _ = KRAKEN_HISTORICAL_LOGGER.log_historical_order_storage(hist_log).await;
                }
                if modify_sell_count > 0 {
                    let hist_log = HistoricalOrderLog {
                        symbol: order_book.symbol.clone(),
                        event_type: "sell-modify".to_string(),
                        order_count: modify_sell_count,
                        processing_time_ms: hist_start.elapsed().as_millis(),
                        success: true,
                        error: None,
                    };
                    let _ = KRAKEN_HISTORICAL_LOGGER.log_historical_order_storage(hist_log).await;
                }
                if delete_sell_count > 0 {
                    let hist_log = HistoricalOrderLog {
                        symbol: order_book.symbol.clone(),
                        event_type: "sell-delete".to_string(),
                        order_count: delete_sell_count,
                        processing_time_ms: hist_start.elapsed().as_millis(),
                        success: true,
                        error: None,
                    };
                    let _ = KRAKEN_HISTORICAL_LOGGER.log_historical_order_storage(hist_log).await;
                }
            }

            Ok::<(), Error>(())
        };
        
        // Initialize total volume from orderbook
        let mut total_volume = BigDecimal::from_str(&order_book.total_volume.to_string())
            .unwrap_or_else(|_| BigDecimal::zero());

        // Execute bid and ask futures concurrently
        let (bid_result, ask_result) = tokio::join!(bid_futures, ask_futures);

        if let Err(e) = bid_result {
            log_error!(KRAKEN_LOGGER, "Error processing bids: {}", e);
        }
        
        if let Err(e) = ask_result {
            log_error!(KRAKEN_LOGGER, "Error processing asks: {}", e);
        }

        // Prepare message outside the lock to minimize lock time
        let orders_data = {
            let locked_orders = orders.lock().await;
            locked_orders.clone()
        };
        
        let message_orders = Orders {
            orders: orders_data
        };
        
        // Create market message and publish request
        let market_message = MarketMessage {
            market_id: "kraken_level3_update".to_string(),
            payload: Some(market_message::Payload::OrdersPayload(message_orders)),
        };
        
        let topics = &config.topics;
        let message = PublishRequest {
            topic: topics[0].clone(), // Use first topic as string
            payload: Some(publish_request::Payload::MarketPayload(market_message)),
        };
        
        // Publish message using the new Publisher API
        let publisher_clone = _publisher.clone();
        let message_clone = message.clone();
        let topic_name = config.topic_routing
            .get("level3")
            .cloned()
            .unwrap_or_else(|| "market.data.kraken.level3".to_string());
        let publisher_future = async move {
            if let Some(ref publisher_clone) = publisher_clone {
                match publisher_clone.lock().await.publish(message_clone, &topic_name) {
                    Ok(_) => {
                        log_debug!(KRAKEN_LOGGER, "Successfully published level3 update message");
                    },
                    Err(e) => {
                        log_error!(KRAKEN_LOGGER, "Failed to publish level3 update message: {:?}", e);
                    }
                }
            }
        };

        // Update total volume
        total_volume += bid_volume + ask_volume;
        
        // Update Postgres and publish message in parallel
        let postgres_future = update_orderbook(postgres_pool.clone(), order_book.clone(), total_volume.clone());
        let (postgres_result, _) = tokio::join!(postgres_future, publisher_future);
        
        match postgres_result {
            Ok(_) => {
                log_debug!(KRAKEN_LOGGER, "Updated order book in postgres");
                
                // Log successful order book update processing
                let processing_time = start_time.elapsed().as_micros() as u64;
                let log_update = OrderBookUpdateLog {
                    symbol: update_data.symbol.clone(),
                    update_type: "level3_update".to_string(),
                    bids_count: update_data.bids.len(),
                    asks_count: update_data.asks.len(),
                    processing_time_micros: processing_time,
                    total_volume: total_volume.clone(),
                    error: None,
                };
                let _ = KRAKEN_ORDERBOOK_LOGGER.log_orderbook_update(log_update).await;
            }
            Err(e) => {
                log_error!(KRAKEN_LOGGER, "Failed to update order book in postgres: {}", e);
                
                // Log failed order book update processing
                let processing_time = start_time.elapsed().as_micros() as u64;
                let log_update = OrderBookUpdateLog {
                    symbol: update_data.symbol.clone(),
                    update_type: "level3_update".to_string(),
                    bids_count: update_data.bids.len(),
                    asks_count: update_data.asks.len(),
                    processing_time_micros: processing_time,
                    total_volume: total_volume.clone(),
                    error: Some(e.to_string()),
                };
                let _ = KRAKEN_ORDERBOOK_LOGGER.log_orderbook_update(log_update).await;
            }
        }
        
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Level3UpdateData {
    pub symbol: String,
    checksum: i64,
    bids: Vec<UpdateOrder>,
    asks: Vec<UpdateOrder>,
}

#[derive(Debug, Serialize, Deserialize)]
struct UpdateOrder {
   event: String,
   order_id: String,
   limit_price: BigDecimal,
   order_qty: BigDecimal,
   timestamp: DateTime<Utc>,
}