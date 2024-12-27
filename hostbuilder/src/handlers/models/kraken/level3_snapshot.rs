use std::sync::Arc;

use anyhow::Result;
use bigdecimal::{BigDecimal, Zero};
use databaseschema::{
    models::orderbook::OrderBook,
    models::open_buy_order::NewOpenBuyOrder,
    models::open_sell_order::NewOpenSellOrder,
    ops::{
        open_buy_order_ops::
            create_open_buy_orders
        ,
        open_sell_order_ops::
            create_open_sell_orders
        ,
        order_book_ops::update_orderbook,
    },
    CustomAsyncPgConnectionManager,
};
use deadpool::managed::Pool;
use deadpool_redis::{Connection, Manager};
use redis::cmd;
use redis_utils::create_redis_connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Level3Snapshot {
    channel: String,
    r#type: String,
    data: Vec<Level3SnapshotData>,
}

impl Level3Snapshot {
    pub fn from_json(json: &serde_json::Value) -> Result<Level3Snapshot> {
        let snapshot: Level3Snapshot = serde_json::from_value(json.clone())?;
        Ok(snapshot)
    }

    pub async fn process_snapshot(
        &self,
        redis_pool: Arc<Pool<Manager, Connection>>,
        postgres_pool: Arc<Pool<CustomAsyncPgConnectionManager>>,
        order_book: OrderBook,
    ) -> Result<()> {
        let mut book_connection = create_redis_connection(&redis_pool)
            .await
            .expect("Failed to create redis connection");
        let mut bid_connection = create_redis_connection(&redis_pool)
            .await
            .expect("Failed to create redis connection");
        let mut ask_connection = create_redis_connection(&redis_pool)
            .await
            .expect("Failed to create redis connection");

        if self.channel == "level3" {
            if self.data.is_empty() {
                println!("Snapshot data is empty");
            }

            let snapshot_data = &self.data[0];

            if snapshot_data.bids.is_empty() && snapshot_data.asks.is_empty() {
                println!("Both bids and asks are empty");
            }

            let mut open_bid_volume = BigDecimal::zero();
            let mut open_ask_volume = BigDecimal::zero();

            let bid_futures = async {
                if !snapshot_data.bids.is_empty() {
                    let mut buy_orders = Vec::new();
                    let mut pipe = redis::pipe();

                    for bid in snapshot_data.bids.iter() {
                        let price_level = &bid.limit_price;
                        let buy_quantity = &bid.order_qty;
                        let unique_id = &bid.order_id;

                        open_bid_volume += buy_quantity;

                        let order = NewOpenBuyOrder::new(
                            &order_book.symbol,
                            &order_book.exchange,
                            order_book.security_id,
                            order_book.exchange_id,
                            order_book.buy_order_book_id,
                            unique_id,
                            price_level,
                            buy_quantity,
                        );

                        pipe.atomic()
                            .cmd("HSET")
                            .arg(format!("buy_order:{}", order.unique_id))
                            .arg("symbol")
                            .arg(&order.symbol)
                            .arg("exchange")
                            .arg(&order.exchange)
                            .arg("security_id")
                            .arg(&order.security_id.to_string())
                            .arg("exchange_id")
                            .arg(&order.exchange_id.to_string())
                            .arg("buy_order_book_id")
                            .arg(&order.buy_order_book_id.to_string())
                            .arg("unique_id")
                            .arg(&order.unique_id)
                            .arg("price_level")
                            .arg(&order.price_level.to_string())
                            .arg("buy_quantity")
                            .arg(&order.buy_quantity.to_string());

                        buy_orders.push(order);
                    }

                    match pipe.query_async::<_, ()>(&mut bid_connection).await {
                        Ok(_) => println!("Added buy orders to redis"),
                        Err(e) => println!("Failed to add buy orders to redis: {}", e),
                    }

                    create_open_buy_orders(postgres_pool.clone(), buy_orders).await;
                }
            };

            let ask_futures = async {
                if !snapshot_data.asks.is_empty() {
                    let mut add_sell_orders = Vec::new();
                    let mut pipe = redis::pipe();
                    for ask in snapshot_data.asks.iter() {
                        let price_level = &ask.limit_price;
                        let sell_quantity = &ask.order_qty;
                        let unique_id = &ask.order_id;

                        open_ask_volume += sell_quantity;

                        let order = NewOpenSellOrder::new(
                            &order_book.symbol,
                            &order_book.exchange,
                            order_book.security_id,
                            order_book.exchange_id,
                            order_book.sell_order_book_id,
                            unique_id,
                            price_level,
                            sell_quantity,
                        );

                        pipe.atomic()
                            .cmd("HSET")
                            .arg(format!("sell_order:{}", order.unique_id))
                            .arg("symbol")
                            .arg(&order.symbol)
                            .arg("exchange")
                            .arg(&order.exchange)
                            .arg("security_id")
                            .arg(&order.security_id.to_string())
                            .arg("exchange_id")
                            .arg(&order.exchange_id.to_string())
                            .arg("sell_order_book_id")
                            .arg(&order.sell_order_book_id.to_string())
                            .arg("unique_id")
                            .arg(&order.unique_id)
                            .arg("price_level")
                            .arg(&order.price_level.to_string())
                            .arg("sell_quantity")
                            .arg(&order.sell_quantity.to_string());

                        add_sell_orders.push(order);
                    }

                    match pipe.query_async::<_, ()>(&mut ask_connection).await {
                        Ok(_) => println!("Added sell orders to redis"),
                        Err(e) => println!("Failed to add sell orders to redis: {}", e),
                    }

                    create_open_sell_orders(postgres_pool.clone(), add_sell_orders).await;
                }
            };

            tokio::join!(bid_futures, ask_futures);


            let total_volume = open_bid_volume + open_ask_volume;

            println!("Total volume: {}", total_volume);

            match cmd("HSET")
                .arg(format!("order_book:{}", order_book.order_book_id))
                .arg("created_at")
                .arg(&order_book.created_at.to_string())
                .arg("updated_at")
                .arg(&order_book.updated_at.expect("").to_string())
                .arg("symbol")
                .arg(&order_book.symbol)
                .arg("exchange")
                .arg(&order_book.exchange)
                .arg("security_id")
                .arg(&order_book.security_id.to_string())
                .arg("exchange_id")
                .arg(&order_book.exchange_id.to_string())
                .arg("buy_order_book_id")
                .arg(&order_book.buy_order_book_id.to_string())
                .arg("sell_order_book_id")
                .arg(&order_book.sell_order_book_id.to_string())
                .arg("total_volume")
                .arg(&total_volume.to_string())
                .query_async::<_, ()>(&mut book_connection)
                .await
            {
                Ok(_) => {
                    println!("Added order book to redis");
                }
                Err(e) => {
                    println!("Failed to add order book to redis: {}", e);
                }
            }
            update_orderbook(postgres_pool.clone(), order_book, total_volume).await;
            Ok(())
        } else {
            println!("Channel is not level3");
            Ok(())
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Level3SnapshotData {
    symbol: String,
    checksum: i64,
    bids: Vec<SnapshotOrder>,
    asks: Vec<SnapshotOrder>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SnapshotOrder {
    order_id: String,
    limit_price: BigDecimal,
    order_qty: BigDecimal,
    timestamp: String,
}