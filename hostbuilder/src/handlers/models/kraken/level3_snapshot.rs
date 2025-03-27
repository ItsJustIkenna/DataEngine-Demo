use std::sync::{Arc, Mutex};

use anyhow::Result;
use bigdecimal::{BigDecimal, Zero, ToPrimitive};
use chrono::{DateTime, Utc};
use config::Config;
use databaseschema::{
    models::{historical_order::NewHistoricalOrder, open_buy_order::NewOpenBuyOrder, open_sell_order::NewOpenSellOrder, order_book::OrderBook}, ops::{
        historical_order_ops::create_historical_orders, open_buy_order_ops::create_open_buy_orders, open_sell_order_ops::create_open_sell_orders, order_book_ops::update_orderbook
    }, CustomAsyncPgConnectionManager
};
use deadpool::managed::Pool;
use deadpool_redis::{
    Connection,
    Manager
};
use protocol::broker::messages::{market_message, publish_request, MarketMessage, Order, Orders, PublishRequest};
use publisher::{Publisher, PublisherTrait};
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
pub struct Level3Snapshot {
    channel: String,
    r#type: String,
    pub data: Vec<Level3SnapshotData>,
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
        config: &Config,
        order_book: &OrderBook,
    ) -> Result<()> {
        let retry_strategy = ExponentialBackoff::from_millis(10).map(jitter).take(3);
        
        let mut book_connection = get_redis_connection(&redis_pool)
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
                println!("Snapshot data is empty");
                Ok(())
            },
            false => {
                let snapshot_data = &self.data[0];

                match snapshot_data.bids.is_empty() && snapshot_data.asks.is_empty() {
                    true => {
                        println!("Snapshot data is empty");
                        Ok(())
                    },
                    false => {
                        let orders = Arc::new(Mutex::new(Vec::new()));
                        let mut open_bid_volume = BigDecimal::zero();
                        let mut open_ask_volume = BigDecimal::zero();

                        let bid_futures = async {
                            if !snapshot_data.bids.is_empty() {
                                let orders = Arc::clone(&orders);
                                let mut buy_orders = Vec::new();
                                let mut historical_orders = Vec::new();
                                let mut pipe = redis::pipe();

                                for bid in snapshot_data.bids.iter() {
                                    let price_level = &bid.limit_price;
                                    let buy_quantity = &bid.order_qty;
                                    let unique_id = &bid.order_id;
                                    let timestamp = bid.timestamp;

                                    open_bid_volume += buy_quantity;

                                    let message = Order {
                                        unique_id: unique_id.clone(),
                                        symbol: order_book.symbol.clone(),
                                        exchange: order_book.exchange.clone(),
                                        price_level: price_level.to_f32().unwrap(),
                                        quantity: buy_quantity.to_f32().unwrap(),
                                        side: "buy".to_string(),
                                        event: "new".to_string(),
                                    };

                                    let order = NewOpenBuyOrder::new(
                                        timestamp.clone(),
                                        &order_book.symbol,
                                        &order_book.exchange,
                                        order_book.security_id,
                                        order_book.exchange_id,
                                        order_book.buy_order_book_id,
                                        unique_id,
                                        price_level,
                                        buy_quantity,
                                    );

                                    let historical_order = NewHistoricalOrder::new(
                                        timestamp.clone(),
                                        &unique_id,
                                        "new",
                                        "buy",
                                        price_level,
                                        buy_quantity,
                                        None,
                                        None,
                                        "open",
                                        &order_book.exchange,
                                        &order_book.symbol,
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
                                    orders.lock().unwrap().push(message);
                                    historical_orders.push(historical_order);
                                }

                                let result = Retry::spawn(retry_strategy.clone(), || async {
                                    match pipe.query_async::<_, ()>(&mut bid_connection.clone()).await {
                                        Ok(_) => {
                                            println!("Added buy orders to redis");
                                            Ok(())
                                        },
                                        Err(e) => {
                                            println!("Failed to add buy orders to redis: {}", e);
                                            Err(e)
                                        },
                                    }
                                }).await;

                                if let Err(e) = result {
                                    println!("All retries failed for redis commands: {}", e);
                                }                    

                                match create_open_buy_orders(postgres_pool.clone(), buy_orders).await {
                                    Ok(_) => println!("Added buy orders to postgres"),
                                    Err(e) => println!("Failed to add buy orders to postgres: {}", e),
                                };

                                match create_historical_orders(postgres_pool.clone(), historical_orders).await {
                                    Ok(_) => println!("Added historical orders to postgres"),
                                    Err(e) => println!("Failed to add historical orders to postgres: {}", e),
                                };
                            }
                        };

                        let ask_futures = async {
                            if !snapshot_data.asks.is_empty() {
                                let orders = Arc::clone(&orders);
                                let mut sell_orders = Vec::new();
                                let mut historical_orders = Vec::new();
                                let mut pipe = redis::pipe();
                                for ask in snapshot_data.asks.iter() {
                                    let price_level = &ask.limit_price;
                                    let sell_quantity = &ask.order_qty;
                                    let unique_id = &ask.order_id;
                                    let timestamp = ask.timestamp;

                                    open_ask_volume += sell_quantity;

                                    let message = Order {
                                        unique_id: ask.order_id.clone(),
                                        symbol: order_book.symbol.clone(),
                                        exchange: order_book.exchange.clone(),
                                        price_level: price_level.to_f32().unwrap(),
                                        quantity: sell_quantity.to_f32().unwrap(),
                                        side: "sell".to_string(),
                                        event: "new".to_string(),
                                    };

                                    let order = NewOpenSellOrder::new(
                                        timestamp.clone(),
                                        &order_book.symbol,
                                        &order_book.exchange,
                                        order_book.security_id,
                                        order_book.exchange_id,
                                        order_book.sell_order_book_id,
                                        unique_id,
                                        price_level,
                                        sell_quantity,
                                    );

                                    let historical_order = NewHistoricalOrder::new(
                                        timestamp.clone(),
                                        &unique_id,
                                        "new",
                                        "sell",
                                        price_level,
                                        sell_quantity,
                                        None,
                                        None,
                                        "open",
                                        &order_book.exchange,
                                        &order_book.symbol,
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
                                    orders.lock().unwrap().push(message);
                                    historical_orders.push(historical_order);
                                }

                                let result = Retry::spawn(retry_strategy.clone(), || async {
                                    match pipe.query_async::<_, ()>(&mut ask_connection.clone()).await {
                                        Ok(_) => {
                                            println!("Added sell orders to redis");
                                            Ok(())
                                        },
                                        Err(e) => {
                                            println!("Failed to add sell orders to redis: {}", e);
                                            Err(e)
                                        },
                                    }
                                }).await;

                                if let Err(e) = result {
                                    println!("All retries failed for redis commands: {}", e);
                                }  

                                match create_open_sell_orders(postgres_pool.clone(), sell_orders).await {
                                    Ok(_) => println!("Added sell orders to postgres"),
                                    Err(e) => println!("Failed to add sell orders to postgres: {}", e),
                                };

                                match create_historical_orders(postgres_pool.clone(), historical_orders).await {
                                    Ok(_) => println!("Added historical orders to postgres"),
                                    Err(e) => println!("Failed to add historical orders to postgres: {}", e),
                                };
                            }
                        };

                        tokio::join!(bid_futures, ask_futures);
                        
                        let orders = Orders {
                            orders: orders.lock().unwrap().clone()
                        };
                        let market_message = MarketMessage {
                            payload: Some(market_message::Payload::OrdersPaylaod(orders)),
                        };
                        let topics = &config.topics;
                        let message = PublishRequest {
                            topics: topics.clone(),
                            payload: Some(publish_request::Payload::MarketPayload(market_message)),
                        };
                        
                        
                        let addr = format!("{}:{}", config.message_broker.address, config.message_broker.port);
                        let mut publisher = Publisher::new(&addr).await?;
                        
                        publisher.publish(&message.topics, message.payload).await?;


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
                                println!("Added order book to redis");
                                Ok(())
                            }
                            Err(e) => {
                                println!("Failed to add order book to redis: {}", e);
                                Err(e)
                            }
                        }
                        }).await;

                        if let Err(e) = result {
                            println!("All retries failed for redis commands: {}", e);
                        }  
                        
                        match update_orderbook(postgres_pool.clone(), order_book.clone(), total_volume).await {
                            Ok(_) => println!("Updated order book in postgres"),
                            Err(e) => println!("Failed to update order book in postgres: {}", e),
                        };
                        Ok(())
                    }
                }
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Level3SnapshotData {
    pub symbol: String,
    checksum: i64,
    bids: Vec<SnapshotOrder>,
    asks: Vec<SnapshotOrder>,
}

#[derive(Debug, Serialize, Deserialize)]
struct SnapshotOrder {
    order_id: String,
    limit_price: BigDecimal,
    order_qty: BigDecimal,
    timestamp: DateTime<Utc>,
}