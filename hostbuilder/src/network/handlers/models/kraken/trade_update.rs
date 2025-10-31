use std::sync::Arc;
use tokio::sync::Mutex; // Add tokio::sync::Mutex for optional publisher

use anyhow::Result;
use bigdecimal::ToPrimitive;
use config::Config;
use databaseschema::{
    models::{order_book::OrderBook, trade::NewTrade},
    ops::trades_ops::create_trades,
};
use diesel_async::AsyncPgConnection;
use protocol::broker::messages::{market_message, publish_request, MarketMessage, PublishRequest, Trade, Trades};
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
use crate::infrastructure::logging_facade::KRAKEN_LOGGER; 
use crate::{log_info, log_warn, log_error, log_debug};
use std::time::Instant;

// Import the Kraken logger
use crate::network::handlers::logging::{KRAKEN_TRADE_LOGGER, TradeProcessingLog};

use super::trade_data::TradeData;

#[derive(Debug, Serialize, Deserialize)]
pub struct TradeUpdate {
    channel: String,
    r#type: String,
    pub data: Vec<TradeData>,
}

impl TradeUpdate {
    pub fn from_json(json: &serde_json::Value) -> Result<TradeUpdate> {
        let update: TradeUpdate = serde_json::from_value(json.clone())?;
        Ok(update)
    }

    pub async fn process_update(
        &self,
        redis_pool: Arc<deadpool_redis::Pool>,
        postgres_pool: Arc<diesel_async::pooled_connection::deadpool::Pool<AsyncPgConnection>>,
        config: &Config,
        _publisher: Option<Arc<Mutex<Publisher>>>, // Optional for database-only mode
        order_book: &OrderBook
    ) -> Result<()> {
        let start_time = Instant::now();
        
        // Reduce retry attempts for lower latency
        let retry_strategy = ExponentialBackoff::from_millis(5).map(jitter).take(2);
        
        // Get Redis connection
        let trade_connection_future = get_redis_connection(&redis_pool);
        let trade_connection = trade_connection_future.await.expect("Failed to create redis connection");

        if self.data.is_empty() {
            log_debug!(KRAKEN_LOGGER, "Trade data is empty");
            return Ok(());
        }
        
        // Pre-allocate arrays with capacity based on data size
        let trade_count = self.data.len();
        let mut trades = Vec::with_capacity(trade_count);
        let mut trade_messages = Vec::with_capacity(trade_count);
        let mut pipe = redis::pipe();
        pipe.atomic();

        // Process all data first before async operations
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
            
            let new_trade = NewTrade::new(
                &trade.symbol,
                &order_book.exchange,
                &trade_id_str,
                order_book.security_id,
                order_book.exchange_id,
                &trade.side,
                &trade.price,
                &trade.qty,
            );
            // Format key once
            let key = format!("trade:{}", trade.trade_id);
            
            pipe.cmd("HSET")
                .arg(key)
                .arg("symbol").arg(&trade.symbol)
                .arg("side").arg(&trade.side)
                .arg("price").arg(&trade.price.to_string())
                .arg("qty").arg(&trade.qty.to_string())
                .arg("ord_type").arg(&trade.ord_type)
                .arg("trade_id").arg(&trade_id_str)
                .arg("timestamp").arg(&timestamp_i64.to_string());

            trades.push(new_trade);
            trade_messages.push(trade_message);
        }        // Prepare message payload outside of any async operations
        let trade_messages_payload = Trades {
            trades: trade_messages,
        };

        let market_message = MarketMessage {
            market_id: "kraken_trades_update".to_string(),
            payload: Some(market_message::Payload::TradesPayload(trade_messages_payload)),
        };

        let topics = &config.topics;
        let message = PublishRequest {
            topic: topics[0].clone(), // Use first topic as string
            payload: Some(publish_request::Payload::MarketPayload(market_message)),
        };
        
        // Execute Redis operations
        let redis_future = Retry::spawn(retry_strategy, || async {
            match pipe.query_async::<_, ()>(&mut trade_connection.clone()).await {
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
        
        // Only store in trades table - historical_orders is redundant for trades
        let create_trades_future = create_trades(postgres_pool.clone(), trades);
        // Removed duplicate historical_orders storage for trades
        
        // Handle publisher conditionally for database-only mode
        let publisher_future = if let Some(publisher_ref) = _publisher {
            let publisher_clone = Arc::clone(&publisher_ref);
            let message_clone = message.clone();
            let topic_name = config.topic_routing
                .get("trades")
                .cloned()
                .unwrap_or_else(|| "market.data.kraken.trades".to_string());
                
            Some(async move {
                match publisher_clone.lock().await.publish(message_clone, &topic_name) {
                    Ok(_) => {
                        log_debug!(KRAKEN_LOGGER, "Successfully published trade message");
                    },
                    Err(e) => {
                        log_error!(KRAKEN_LOGGER, "Failed to publish trade message: {:?}", e);
                    }
                }
            })
        } else {
            log_debug!(KRAKEN_LOGGER, "🔒 DATABASE-ONLY MODE: Skipping message broker publishing for trade update");
            None
        };
        
        // Execute operations concurrently - only trades table needed
        let (redis_result, trades_result) = if let Some(pub_future) = publisher_future {
            let (redis_result, trades_result, _) = tokio::join!(
                redis_future,
                create_trades_future,
                pub_future
            );
            (redis_result, trades_result)
        } else {
            let (redis_result, trades_result) = tokio::join!(
                redis_future,
                create_trades_future
            );
            (redis_result, trades_result)
        };

        let processing_time = start_time.elapsed().as_micros() as u64;

        if let Err(e) = redis_result {
            log_error!(KRAKEN_LOGGER, "Failed to add trades to redis after retries: {}", e);
        }

        match trades_result {
            Ok(_) => {
                log_info!(KRAKEN_LOGGER, "✅ Successfully persisted {} trades to database and Redis - Exchange: {}, OrderBook: {}, Processing: {}μs", 
                    trade_count, order_book.exchange, order_book.order_book_id, processing_time);
                
                // Log successful trade processing for each trade
                for trade in self.data.iter() {
                    let log_trade = TradeProcessingLog {
                        symbol: trade.symbol.clone(),
                        trade_id: trade.trade_id.to_string(),
                        side: trade.side.clone(),
                        price: trade.price.to_f32().unwrap_or(0.0),
                        quantity: trade.qty.to_f32().unwrap_or(0.0),
                        processing_time_micros: processing_time,
                        error: None,
                    };
                    let _ = KRAKEN_TRADE_LOGGER.log_trade_processing(log_trade).await;
                }
            }
            Err(e) => {
                log_error!(KRAKEN_LOGGER, "❌ Failed to persist {} trades to database - Exchange: {}, OrderBook: {}, Error: {}", 
                    trade_count, order_book.exchange, order_book.order_book_id, e);
                
                // Log failed trade processing for each trade
                for trade in self.data.iter() {
                    let log_trade = TradeProcessingLog {
                        symbol: trade.symbol.clone(),
                        trade_id: trade.trade_id.to_string(),
                        side: trade.side.clone(),
                        price: trade.price.to_f32().unwrap_or(0.0),
                        quantity: trade.qty.to_f32().unwrap_or(0.0),
                        processing_time_micros: processing_time,
                        error: Some(e.to_string()),
                    };
                    let _ = KRAKEN_TRADE_LOGGER.log_trade_processing(log_trade).await;
                }
            }
        }

        Ok(())
    }
}
