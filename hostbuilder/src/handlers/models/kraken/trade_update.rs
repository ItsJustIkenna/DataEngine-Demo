use std::sync::Arc;

use anyhow::Result;
use bigdecimal::ToPrimitive;
use config::Config;
use databaseschema::{
    models::{historical_order::NewHistoricalOrder, order_book::OrderBook, trade::NewTrade},
    ops::{historical_order_ops::create_historical_orders, 
        trades_ops::create_trades}
    ,
    CustomAsyncPgConnectionManager,
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
        order_book: &OrderBook
    ) -> Result<()> {
        let retry_strategy = ExponentialBackoff::from_millis(10).map(jitter).take(3);
        
        let trade_connection = get_redis_connection(&redis_pool)
            .await
            .expect("Failed to create redis connection");

        match self.data.is_empty() {
            true => {
                println!("Trade data is empty");
                Ok(())
            },
            false => {
                let mut trades = Vec::new();
                let mut trade_messages = Vec::new();
                let mut historical_trades = Vec::new();
                let mut pipe = redis::pipe();

                for trade in self.data.iter() {
                    let trade_message = Trade {
                        symbol: trade.symbol.clone(),
                        exchange: order_book.exchange.clone(),
                        side: trade.side.clone(),
                        price: trade.price.to_f32().unwrap(),
                        qty: trade.qty.to_f32().unwrap(),
                        ord_type: trade.ord_type.clone(),
                        timestamp: trade.timestamp.to_rfc2822(),
                        trade_id: trade.trade_id.clone(),
                    };
                    
                    let new_trade = NewTrade::new(
                        &trade.symbol,
                        &order_book.exchange,
                        &trade.trade_id.to_string(),
                        order_book.security_id,
                        order_book.exchange_id,
                        &trade.side,
                        &trade.price,
                        &trade.qty,
                    );

                    let historical_trade = NewHistoricalOrder::new(
                        trade.timestamp,
                        &trade.trade_id.to_string(),
                        "trade",
                        &trade.side,
                        &trade.price,
                        &trade.qty,
                        None,
                        None,
                        "filled",
                        &order_book.exchange,
                        &order_book.symbol,
                    );

                    pipe.atomic()
                        .cmd("HSET")
                        .arg(format!("trade:{}", trade.trade_id))
                        .arg("symbol")
                        .arg(&trade.symbol)
                        .arg("side")
                        .arg(&trade.side)
                        .arg("price")
                        .arg(&trade.price.to_string())
                        .arg("qty")
                        .arg(&trade.qty.to_string())
                        .arg("ord_type")
                        .arg(&trade.ord_type)
                        .arg("trade_id")
                        .arg(&trade.trade_id)
                        .arg("timestamp")
                        .arg(&trade.timestamp.to_rfc3339());

                    trades.push(new_trade);
                    trade_messages.push(trade_message);
                    historical_trades.push(historical_trade);
                }

                let trade_messages = Trades {
                    trades: trade_messages,
                };

                let market_message = MarketMessage {
                    payload: Some(market_message::Payload::TradesPayload(trade_messages)),
                };

                let topics = &config.topics;
                let message = PublishRequest {
                    topics: topics.clone(),
                    payload: Some(publish_request::Payload::MarketPayload(market_message)),
                };
                
                
                let addr = format!("{}:{}", config.message_broker.address, config.message_broker.port);
                let mut publisher = Publisher::new(&addr).await?;
                
                publisher.publish(&message.topics, message.payload).await?;

                let result = Retry::spawn(retry_strategy, || async {
                    match pipe.query_async::<_, ()>(&mut trade_connection.clone()).await {
                        Ok(_) => {
                            println!("Added trades to redis");
                            Ok(())
                        }
                        Err(e) => {
                            println!("Failed to add trades to redis: {}", e);
                            Err(e)
                        }
                    }
                }).await;

                match result {
                    Ok(_) => println!("Added trades to redis"),
                    Err(e) => println!("Failed to add trades to redis: {}", e),
                }

                match create_trades(postgres_pool.clone(), trades).await {
                    Ok(_) => println!("Added trades to postgres"),
                    Err(e) => println!("Failed to add trades to postgres: {}", e),
                };

                match create_historical_orders(postgres_pool.clone(), historical_trades).await {
                    Ok(_) => {
                        println!("Added historical orders to postgres");
                    }
                    Err(e) => {
                        println!("Failed to add historical orders to postgres: {}", e);
                    }
                };

                Ok(())
            }
        }
    }
}