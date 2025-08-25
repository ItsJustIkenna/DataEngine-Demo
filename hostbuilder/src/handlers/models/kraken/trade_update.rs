use std::sync::Arc;

use anyhow::Result;
use bigdecimal::ToPrimitive;
use config::Config;
use databaseschema::{
    models::{order_book::OrderBook, trade::NewTrade},
    ops::trades_ops::create_trades,
    CustomAsyncPgConnectionManager,
};
use deadpool::managed::Pool;
use deadpool_redis::{
    Connection,
    Manager
};
use protocol::broker::messages::{market_message, publish_request, MarketMessage, PublishRequest, Trade, Trades};
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
        redis_pool: Arc<Pool<Manager, Connection>>,
        postgres_pool: Arc<Pool<CustomAsyncPgConnectionManager>>,
        config: &Config,
        _publisher: Arc<Mutex<Publisher>>,
        order_book: &OrderBook
    ) -> Result<()> {
        // Reduce retry attempts for lower latency
        let retry_strategy = ExponentialBackoff::from_millis(5).map(jitter).take(2);
        
        // Get Redis connection
        let trade_connection_future = get_redis_connection(&redis_pool);
        let trade_connection = trade_connection_future.await.expect("Failed to create redis connection");

        if self.data.is_empty() {
            debug!("Trade data is empty");
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
            let timestamp_rfc = trade.timestamp.to_rfc3339();
            
            let trade_message = Trade {
                symbol: trade.symbol.clone(),
                exchange: order_book.exchange.clone(),
                side: trade.side.clone(),
                price: price_f32,
                qty: qty_f32,
                ord_type: trade.ord_type.clone(),
                timestamp: timestamp_rfc.clone(),
                trade_id: trade.trade_id,
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
                .arg("timestamp").arg(&timestamp_rfc);

            trades.push(new_trade);
            trade_messages.push(trade_message);
        }        // Prepare message payload outside of any async operations
        let trade_messages_payload = Trades {
            trades: trade_messages,
        };

        let market_message = MarketMessage {
            payload: Some(market_message::Payload::TradesPayload(trade_messages_payload)),
        };

        let topics = &config.topics;
        let message = PublishRequest {
            topics: topics.clone(),
            payload: Some(publish_request::Payload::MarketPayload(market_message)),
        };
        
        // Execute Redis operations
        let redis_future = Retry::spawn(retry_strategy, || async {
            match pipe.query_async::<_, ()>(&mut trade_connection.clone()).await {
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
        
        // Only store in trades table - historical_orders is redundant for trades
        let create_trades_future = create_trades(postgres_pool.clone(), trades);
        // Removed duplicate historical_orders storage for trades
        
        // Prepare publisher operation
        let publisher_clone = _publisher.clone();
        let message_clone = message.clone();
        let topic_name = config.topic_routing
            .get("trades")
            .cloned()
            .unwrap_or_else(|| "market.data.kraken.trades".to_string());
        let publisher_future = async move {
            match publisher_clone.lock().await.publish(message_clone, &topic_name) {
                Ok(_) => {
                    debug!("Successfully published trade message");
                },
                Err(e) => {
                    error!("Failed to publish trade message: {:?}", e);
                }
            }
        };
        
        // Execute operations concurrently - only trades table needed
        let (redis_result, trades_result, _) = tokio::join!(
            redis_future,
            create_trades_future,
            publisher_future
        );

        if let Err(e) = redis_result {
            error!("Failed to add trades to redis after retries: {}", e);
        }

        if let Err(e) = trades_result {
            error!("Failed to add trades to postgres: {}", e);
        } else {
            debug!("Added trades to postgres");
        }

        Ok(())
    }
}