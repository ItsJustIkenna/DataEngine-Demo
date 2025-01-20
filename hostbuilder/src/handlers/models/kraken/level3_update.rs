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
    Zero
};
use databaseschema::{
    models::{
        modified_buy_order::NewModifiedBuyOrder,
        modified_sell_order::NewModifiedSellOrder,
        open_buy_order::NewOpenBuyOrder,
        open_sell_order::NewOpenSellOrder,
        orderbook::OrderBook
    },
        ops::{
            modified_buy_order_ops::{
                create_modified_buy_orders,
                delete_modified_buy_orders
            },
            modified_sell_order_ops::{
                create_modified_sell_orders,
                delete_modified_sell_orders
            },
            open_buy_order_ops::{
                create_open_buy_orders,
                delete_open_buy_orders
            },
            open_sell_order_ops::{
                create_open_sell_orders,
                delete_open_sell_orders
            },
            order_book_ops::update_orderbook
        },
    CustomAsyncPgConnectionManager
};
use deadpool::managed::Pool;
use deadpool_redis::{
    Connection,
    Manager
};
use redis::cmd;
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
        redis_pool: Arc<Pool<Manager, Connection>>,
        postgres_pool: Arc<Pool<CustomAsyncPgConnectionManager>>,
        order_book: &OrderBook,
    ) -> Result<()> {
        let retry_strategy = ExponentialBackoff::from_millis(10).map(jitter).take(3);

        let book_connection = get_redis_connection(&redis_pool)
            .await
            .expect("Failed to create redis connection");
        let bid_connection = get_redis_connection(&redis_pool)
            .await
            .expect("Failed to create redis connection");
        let ask_connection = get_redis_connection(&redis_pool)
            .await
            .expect("Failed to create redis connection");

        match self.data.is_empty() {
            true => {
                println!("Data is empty");
                Ok(())
            },
            false => {
                let update_data = &self.data[0];

                match update_data.bids.is_empty() && update_data.asks.is_empty() {
                    true => {
                        println!("Both bids and asks are empty");
                        Ok(())
                    },
                    false => {
                        let total_volume = Retry::spawn(retry_strategy.clone(), || async {    
                            match cmd("HGET")
                            .arg(format!("order_book:{}", order_book.order_book_id))
                            .arg("total_volume")
                            .query_async::<_, String>(&mut book_connection.clone())
                            .await {
                                Ok(volume) => {
                                    println!("Successfully retrieved total volume");
                                    Ok(volume)
                                },
                                Err(e) => {
                                    println!("Failed to get total volume: {}", e);
                                    Err(e)
                                },
                            }
                        }).await;
        
                        let total_volume = match total_volume {
                            Ok(volume) => volume,
                            Err(e) => {
                                println!("All retries failed for redis commands: {}", e);
                                "0".to_string()
                            }
                        };  
        
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
        
                                    let buy_quantity = Retry::spawn(retry_strategy.clone(), || async {    
                                        match cmd("HGET")
                                        .arg(format!("buy_order:{}", unique_id))
                                        .arg("buy_quantity")
                                        .query_async::<_, String>(&mut bid_connection.clone()).await {
                                            Ok(volume) => {
                                                println!("Successfully retrieved buy quantity");
                                                Ok(volume)
                                            },
                                            Err(e) => {
                                                println!("Failed to get buy quantity: {}", e);
                                                Err(e)
                                            }
                                        }
                                    }).await;
        
                                    let buy_quantity = match buy_quantity {
                                        Ok(volume) => volume,
                                        Err(e) => {
                                            println!("All retries failed for redis commands: {}", e);
                                            "0".to_string()
                                        }
                                    };
        
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
                            let result = Retry::spawn(retry_strategy.clone(), || async {
                                match pipe.query_async::<_, ()>(&mut bid_connection.clone()).await {
                                    Ok(_) => {
                                        println!("Batch added, modified, and deleted buy orders to/from redis");
                                        Ok(())
                                    }
                                    Err(e) => {
                                        println!("Failed to batch add, modify, and delete buy orders to/from redis: {}", e);
                                        Err(e)
                                    }
                                }
                            }).await;
        
                            match result {
                                Ok(_) => {
                                    println!("Added, modified, and deleted buy orders to/from redis");
                                }
                                Err(e) => {
                                    println!("Failed to add, modify, and delete buy orders to/from redis: {}", e);
                                }
                            }
        
                            match create_open_buy_orders(postgres_pool.clone(), buy_orders).await {
                                Ok(_) => {
                                    println!("Added open buy orders to postgres");
                                }
                                Err(e) => {
                                    println!("Failed to add open buy orders to postgres: {}", e);
                                }
                            };
        
                            match create_modified_buy_orders(postgres_pool.clone(), modified_orders).await {
                                Ok(_) => {
                                    println!("Added modified buy orders to postgres");
                                }
                                Err(e) => {
                                    println!("Failed to add modified buy orders to postgres: {}", e);
                                }
                            };
        
                            match delete_open_buy_orders(postgres_pool.clone(), &delete_orders).await {
                                Ok(_) => {
                                    println!("Deleted open buy orders from postgres");
                                }
                                Err(e) => {
                                    println!("Failed to delete open buy orders from postgres: {}", e);
                                }
                            };
        
                            match delete_modified_buy_orders(postgres_pool.clone(), &delete_orders).await {
                                Ok(_) => {
                                    println!("Deleted modified buy orders from postgres");
                                }
                                Err(e) => {
                                    println!("Failed to delete modified buy orders from postgres: {}", e);
                                }
                            };
        
                            Ok::<(), Error>(())
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
        
                                    let ask_quantity = Retry::spawn(retry_strategy.clone(), || async {    
                                        match cmd("HGET")
                                        .arg(format!("ask_order:{}", unique_id))
                                        .arg("ask_quantity")
                                        .query_async::<_, String>(&mut bid_connection.clone()).await {
                                            Ok(volume) => {
                                                println!("Successfully retrieved ask quantity");
                                                Ok(volume)
                                            },
                                            Err(e) => {
                                                println!("Failed to get ask quantity: {}", e);
                                                Err(e)
                                            },
                                        }
                                    }).await;
        
                                    let ask_quantity = match ask_quantity {
                                        Ok(volume) => volume,
                                        Err(e) => {
                                            println!("All retries failed for redis commands: {}", e);
                                            "0".to_string()
                                        }
                                    };
        
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
                            let result = Retry::spawn(retry_strategy.clone(), || async {
                                match pipe.query_async::<_, ()>(&mut ask_connection.clone()).await {
                                    Ok(_) => {
                                        println!("Batch added, modified, and deleted sell orders to/from redis");
                                        Ok(())
                                    }
                                    Err(e) => {
                                        println!("Failed to batch add, modify, and delete sell orders to/from redis: {}", e);
                                        Err(e)
                                    }
                                }
                            }).await;
        
                            match result {
                                Ok(_) => {
                                    println!("Added, modified, and deleted sell orders to/from redis");
                                }
                                Err(e) => {
                                    println!("Failed to add, modify, and delete sell orders to/from redis: {}", e);
                                }
                            }
        
                            match create_open_sell_orders(postgres_pool.clone(), sell_orders).await {
                                Ok(_) => {
                                    println!("Added open sell orders to postgres");
                                }
                                Err(e) => {
                                    println!("Failed to add open sell orders to postgres: {}", e);
                                }
                            };
        
                            match create_modified_sell_orders(postgres_pool.clone(), modified_orders).await {
                                Ok(_) => {
                                    println!("Added modified sell orders to postgres");
                                }
                                Err(e) => {
                                    println!("Failed to add modified sell orders to postgres: {}", e);
                                }
                            };
        
                            match delete_open_sell_orders(postgres_pool.clone(), &delete_orders).await {
                                Ok(_) => {
                                    println!("Deleted open sell orders from postgres");
                                }
                                Err(e) => {
                                    println!("Failed to delete open sell orders from postgres: {}", e);
                                }
                            };
        
                            match delete_modified_sell_orders(postgres_pool.clone(), &delete_orders).await {
                                Ok(_) => {
                                    println!("Deleted modified sell orders from postgres");
                                }
                                Err(e) => {
                                    println!("Failed to delete modified sell orders from postgres: {}", e);
                                }
                            };
        
                            Ok::<(), Error>(())
                        };
        
                        let (bid_result, ask_result) = tokio::join!(bid_futures, ask_futures);
        
                        match bid_result {
                            Ok(_) => {
                                println!("Added, modified, and deleted buy orders to/from redis");
                            }
                            Err(e) => {
                                println!("Failed to add, modify, and delete buy orders to/from redis: {}", e);
                            }
                        }
                        
                        match ask_result {
                            Ok(_) => {
                                println!("Added, modified, and deleted sell orders to/from redis");
                            }
                            Err(e) => {
                                println!("Failed to add, modify, and delete sell orders to/from redis: {}", e);
                            }
                        }
        
                        total_volume += bid_volume + ask_volume;
        
                        let result = Retry::spawn(retry_strategy.clone(), || async {
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
                            .query_async::<_, ()>(&mut book_connection.clone())
                            .await
                            {
                                Ok(_) => {
                                    println!("Updated order book in redis");
                                    Ok(())
                                }
                                Err(e) => {
                                    println!("Failed to update order book in redis: {}", e);
                                    Err(e)
                                }
                            }
                        }).await;
        
                        match result {
                            Ok(_) => {
                                println!("Updated order book in redis");
                            }
                            Err(e) => {
                                println!("Failed to update order book in redis: {}", e);
                            }
                        }
        
                        match update_orderbook(postgres_pool.clone(), order_book.clone(), total_volume).await {
                            Ok(_) => {
                                println!("Updated order book in postgres");
                            }
                            Err(e) => {
                                println!("Failed to update order book in postgres: {}", e);
                            }
                        }
                        
                        Ok(())
                    }
                }
            }
        }
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
    timestamp: String,
}