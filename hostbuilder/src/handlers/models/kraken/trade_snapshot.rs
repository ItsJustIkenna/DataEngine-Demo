use std::sync::Arc;

use anyhow::Result;
use databaseschema::{
    models::trade::NewTrade,
    models::orderbook::OrderBook,
    ops::
        trades_ops::create_trades
    ,
    CustomAsyncPgConnectionManager,
};
use deadpool::managed::Pool;
use deadpool_redis::{Connection, Manager};
use redis_utils::get_redis_connection;
use serde::{Deserialize, Serialize};
use tokio_retry::{strategy::FixedInterval, Retry};

use super::trade_data::TradeData;

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
        order_book: &OrderBook
    ) -> Result<()> {
        let retry_strategy = FixedInterval::from_millis(1).take(15);
        
        let trade_connection = get_redis_connection(&redis_pool)
            .await
            .expect("Failed to create redis connection");

        if self.channel == "trade" {
            if self.data.is_empty() {
                println!("Trade data is empty");
            }

            let mut trades = Vec::new();
            let mut pipe = redis::pipe();

            for trade in self.data.iter() {
                let new_trade = NewTrade::new(
                    &trade.symbol,
                    &order_book.exchange,
                    order_book.security_id,
                    order_book.exchange_id,
                    &trade.side,
                    &trade.price,
                    &trade.qty,
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
                    .arg(&trade.timestamp);

                trades.push(new_trade);
            }

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

            Ok(())
        } else {
            println!("Channel is not trade");
            Ok(())
        }
    }
}