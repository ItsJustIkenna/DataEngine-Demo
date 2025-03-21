use std::sync::Arc;

use anyhow::Result;
use databaseschema::{
    models::{historical_order::NewHistoricalOrder, order_book::OrderBook}, ops::historical_order_ops::create_historical_orders, CustomAsyncPgConnectionManager
};
use deadpool::managed::Pool;
use deadpool_redis::{
    Connection,
    Manager
};
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
                let mut pipe = redis::pipe();

                for trade in self.data.iter() {
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
                        &trade.symbol,
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

                    trades.push(historical_trade);
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

                match create_historical_orders(postgres_pool.clone(), trades).await {
                    Ok(_) => println!("Added historical orders to postgres"),
                    Err(e) => println!("Failed to add historical orders to postgres: {}", e),
                };

                Ok(())
            }
        }
    }
}