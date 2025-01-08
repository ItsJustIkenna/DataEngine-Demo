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
use redis_utils::create_redis_connection;
use serde::{Deserialize, Serialize};
use tokio_retry::{strategy::FixedInterval, Retry};

use super::trade_data::TradeData;

#[derive(Debug, Serialize, Deserialize)]
pub struct TradeUpdate {
    channel: String,
    r#type: String,
    data: Vec<TradeData>,
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
        order_book: OrderBook
    ) -> Result<()> {
        let retry_strategy = FixedInterval::from_millis(1).take(15);
        
        let trade_connection = create_redis_connection(&redis_pool)
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

            Retry::spawn(retry_strategy, || async {
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
            }).await?;

            create_trades(postgres_pool.clone(), trades).await;

            Ok(())
        } else {
            println!("Channel is not trade");
            Ok(())
        }
    }
}