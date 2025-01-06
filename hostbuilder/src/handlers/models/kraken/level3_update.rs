use std::{str::FromStr, sync::Arc};

use anyhow::Result;
use bigdecimal::{BigDecimal, Zero};
use databaseschema::{
    models::{modified_buy_order::NewModifiedBuyOrder,
        modified_sell_order::NewModifiedSellOrder,
        open_buy_order::NewOpenBuyOrder,
        open_sell_order::NewOpenSellOrder,
        orderbook::OrderBook},
        ops::{modified_buy_order_ops::{create_modified_buy_orders, delete_modified_buy_orders},
        modified_sell_order_ops::{create_modified_sell_orders, delete_modified_sell_orders},
        open_buy_order_ops::{create_open_buy_orders, delete_open_buy_orders},
        open_sell_order_ops::{create_open_sell_orders, delete_open_sell_orders},
        order_book_ops::update_orderbook},
        CustomAsyncPgConnectionManager
};
use deadpool::managed::Pool;
use deadpool_redis::{Connection, Manager};
use redis::cmd;
use redis_utils::get_redis_connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Level3Update {
    channel: String,
    r#type: String,
    data: Vec<Level3UpdateData>,
}

impl Level3Update {
    pub fn from_json(json: &serde_json::Value) -> Result<Level3Update> {
        let update: Level3Update = serde_json::from_value(json.clone())?;
        Ok(update)
    }

    pub async fn process_update(
        &self,
        redis_pool: Arc<Pool<Manager, Connection>>,
        postgres_pool: Arc<Pool<CustomAsyncPgConnectionManager>>,
        order_book: OrderBook,
    ) -> Result<()> {
        let mut book_connection = get_redis_connection(&redis_pool)
            .await
            .expect("Failed to create redis connection");
        let mut bid_connection = get_redis_connection(&redis_pool)
            .await
            .expect("Failed to create redis connection");
        let mut ask_connection = get_redis_connection(&redis_pool)
            .await
            .expect("Failed to create redis connection");

        if self.channel == "level3" {
            if self.data.is_empty() {
                println!("update data is empty");
            }

            let update_data = &self.data[0];

            if update_data.bids.is_empty() && update_data.asks.is_empty() {
                println!("Both bids and asks are empty");
            }

            let total_volume = cmd("HGET")
            .arg(format!("order_book:{}", order_book.order_book_id))
            .arg("total_volume")
            .query_async::<_, String>(&mut book_connection).await.expect("Failed to get total volume");

            let mut bid_volume = BigDecimal::zero();
            let mut ask_volume = BigDecimal::zero();

            let mut total_volume = BigDecimal::from_str(&total_volume).expect("Failed to convert total volume to BigDecimal");

            let bid_futures = async {
                let mut buy_orders = Vec::new();
                let mut modified_orders = Vec::new();
                let mut delete_orders = Vec::new();
                let mut pipe = redis::pipe();

                for bid in update_data.bids.iter() {
                    if bid.event == "add" {
                        let price_level = &bid.limit_price;
                        let buy_quantity = &bid.order_qty;
                        let unique_id = &bid.order_id;

                        bid_volume += buy_quantity;

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
                    } else if bid.event == "modify" {
                        let price_level = &bid.limit_price;
                        let new_buy_quantity = &bid.order_qty;
                        let unique_id = &bid.order_id;

                        let buy_quantity = cmd("HGET")
                            .arg(format!("buy_order:{}", unique_id))
                            .arg("buy_quantity")
                            .query_async::<_, String>(&mut bid_connection).await.expect("Failed to get total volume");

                        let buy_quantity = BigDecimal::from_str(&buy_quantity).expect("Failed to convert total volume to BigDecimal");

                        bid_volume -= buy_quantity.clone();

                        let buy_quantity = (new_buy_quantity - buy_quantity).abs();

                        bid_volume += buy_quantity;

                        let order = NewModifiedBuyOrder::new(
                            &order_book.symbol,
                            &order_book.exchange,
                            order_book.security_id,
                            order_book.exchange_id,
                            order_book.buy_order_book_id,
                            unique_id,
                            price_level,
                            new_buy_quantity,
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
                            .arg(&order.new_buy_quantity.to_string());

                        delete_orders.push(unique_id);
                        modified_orders.push(order);
                    } else if bid.event == "delete" {
                        pipe.atomic()
                            .cmd("HDEL")
                            .arg(format!("buy_order:{}", bid.order_id))
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
                            .arg("unique_id")
                            .arg(&bid.order_id)
                            .arg("price_level")
                            .arg(&bid.limit_price.to_string())
                            .arg("buy_quantity")
                            .arg(&bid.order_qty.to_string());

                        bid_volume -= &bid.order_qty;

                        delete_orders.push(&bid.order_id);
                    }
                }
                match pipe.query_async::<_, ()>(&mut bid_connection).await {
                    Ok(_) => {
                        println!("Batch added, modified, and deleted buy orders to/from redis")
                    }
                    Err(e) => println!(
                        "Failed to batch add, modify, and delete buy orders to/from redis: {}",
                        e
                    ),
                }
                create_open_buy_orders(postgres_pool.clone(), buy_orders).await;
                create_modified_buy_orders(postgres_pool.clone(), modified_orders).await;
                delete_open_buy_orders(postgres_pool.clone(), &delete_orders).await;
                delete_modified_buy_orders(postgres_pool.clone(), &delete_orders).await;
            };

            let ask_futures = async {
                let mut sell_orders = Vec::new();
                let mut modified_orders = Vec::new();
                let mut delete_orders = Vec::new();
                let mut pipe = redis::pipe();

                for ask in update_data.asks.iter() {
                    if ask.event == "add" {
                        let price_level = &ask.limit_price;
                        let sell_quantity = &ask.order_qty;
                        let unique_id = &ask.order_id;

                        ask_volume += sell_quantity;

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

                        sell_orders.push(order);
                    } else if ask.event == "modify" {
                        let price_level = &ask.limit_price;
                        let new_sell_quantity = &ask.order_qty;
                        let unique_id = &ask.order_id;

                        let ask_quantity = cmd("HGET")
                            .arg(format!("sell_order:{}", unique_id))
                            .arg("sell_quantity")
                            .query_async::<_, String>(&mut ask_connection).await.expect("Failed to get total volume");

                        let ask_quantity = BigDecimal::from_str(&ask_quantity).expect("Failed to convert total volume to BigDecimal");

                        ask_volume -= ask_quantity.clone();

                        let ask_quantity = (new_sell_quantity - ask_quantity).abs();

                        ask_volume += ask_quantity;

                        let order = NewModifiedSellOrder::new(
                            &order_book.symbol,
                            &order_book.exchange,
                            order_book.security_id,
                            order_book.exchange_id,
                            order_book.sell_order_book_id,
                            unique_id,
                            price_level,
                            new_sell_quantity,
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
                            .arg(&order.new_sell_quantity.to_string());

                        delete_orders.push(unique_id);
                        modified_orders.push(order);
                    } else if ask.event == "delete" {
                        pipe.atomic()
                            .cmd("HDEL")
                            .arg(format!("sell_order:{}", ask.order_id))
                            .arg("symbol")
                            .arg(&order_book.symbol)
                            .arg("exchange")
                            .arg(&order_book.exchange)
                            .arg("security_id")
                            .arg(&order_book.security_id.to_string())
                            .arg("exchange_id")
                            .arg(&order_book.exchange_id.to_string())
                            .arg("sell_order_book_id")
                            .arg(&order_book.sell_order_book_id.to_string())
                            .arg("unique_id")
                            .arg(&ask.order_id)
                            .arg("price_level")
                            .arg(&ask.limit_price.to_string())
                            .arg("sell_quantity")
                            .arg(&ask.order_qty.to_string());

                        ask_volume -= &ask.order_qty;

                        delete_orders.push(&ask.order_id);
                    }
                }
                match pipe.query_async::<_, ()>(&mut ask_connection).await {
                    Ok(_) => {
                        println!("Batch added, modified, and deleted buy orders to/from redis")
                    }
                    Err(e) => println!(
                        "Failed to batch add, modify, and delete buy orders to/from redis: {}",
                        e
                    ),
                }
                create_open_sell_orders(postgres_pool.clone(), sell_orders).await;
                create_modified_sell_orders(postgres_pool.clone(), modified_orders).await;
                delete_open_sell_orders(postgres_pool.clone(), &delete_orders).await;
                delete_modified_sell_orders(postgres_pool.clone(), &delete_orders).await;
            };

            tokio::join!(bid_futures, ask_futures);

            total_volume += bid_volume + ask_volume;

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
                    println!("Updated order book in redis");
                }
                Err(e) => {
                    println!("Failed to update order book in redis: {}", e);
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
struct Level3UpdateData {
    symbol: String,
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
    timestamp: String,
}