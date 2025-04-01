use std::sync::Arc;

use anyhow::Result;
use bigdecimal::ToPrimitive;
use config::Config;
use databaseschema::{
    models::{historical_order::NewHistoricalOrder, order_book::OrderBook}, ops::historical_order_ops::create_historical_orders, CustomAsyncPgConnectionManager
};
use deadpool::managed::Pool;
use deadpool_redis::{
    Connection,
    Manager
};
use protocol::broker::messages::{market_message, publish_request, MarketMessage, PublishRequest, Trade, Trades};
use publisher::{Publisher, PublisherTrait};
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
        redis_pool: Arc<Pool<Manager, Connection>>,
        postgres_pool: Arc<Pool<CustomAsyncPgConnectionManager>>,
        config: &Config,
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

        // Prepare all data first before any async operations
        for trade in self.data.iter() {
            // Convert values once and reuse
            let price_f32 = trade.price.to_f32().unwrap_or(0.0);
            let qty_f32 = trade.qty.to_f32().unwrap_or(0.0);
            let trade_id_str = trade.trade_id.to_string();
            let timestamp_rfc = trade.timestamp.to_rfc2822();

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
            );

            // Use key once
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

            trades.push(historical_trade);
            trade_messages.push(trade_message);
        }

        // Prepare message payload outside of any async operations
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
        
        // Execute Redis, Postgres and Publisher operations concurrently
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
        
        let postgres_future = create_historical_orders(postgres_pool.clone(), trades);
        
        let publisher_future = async {
            let addr = format!("{}:{}", config.message_broker.address, config.message_broker.port);
            match Publisher::new(&addr).await {
                Ok(mut publisher) => {
                    match publisher.publish(&message.topics, message.payload).await {
                        Ok(_) => debug!("Published trade messages successfully"),
                        Err(e) => error!("Failed to publish trade messages: {}", e),
                    }
                },
                Err(e) => error!("Failed to create publisher: {}", e),
            }
        };
        
        // Wait for all futures to complete
        let (redis_result, postgres_result, _) = tokio::join!(
            redis_future,
            postgres_future,
            publisher_future
        );

        if let Err(e) = redis_result {
            error!("Failed to add trades to redis after retries: {}", e);
        }

        if let Err(e) = postgres_result {
            error!("Failed to add historical orders to postgres: {}", e);
        } else {
            debug!("Added historical orders to postgres");
        }

        Ok(())
    }
}