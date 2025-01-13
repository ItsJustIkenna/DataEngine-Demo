use std::{collections::HashMap, sync::Arc};

use databaseschema::{
    models::{exchange::{Exchange, NewExchange}, orderbook::{NewOrderBook, OrderBook}, security::{NewSecurity, Security}},
    ops::{
        exchanges_ops::{create_exchange, exchange_exists, get_exchanges_by_name},
        order_book_ops::{create_orderbook, get_orderbook_by_exchange_id_and_security_id, orderbook_exists},
        securities_ops::{create_security, get_security_by_symbol, security_exists},
    },
    CustomAsyncPgConnectionManager,
};
use deadpool::managed::Pool;
use deadpool_redis::{Connection, Manager};
use diesel::result::Error;
use redis::cmd;
use redis_utils::get_redis_connection;

pub async fn get_exchange(
    redis_pool: Arc<Pool<Manager, Connection>>,
    postgres_pool: Arc<Pool<CustomAsyncPgConnectionManager>>,
    name: String,
) -> Result<Exchange, Error> {
    let mut connection = get_redis_connection(&redis_pool)
        .await
        .expect("Failed to create redis connection");

    match exchange_exists(postgres_pool.clone(), &name).await {
        true => {
            println!("Exchange exists");

            match get_exchanges_by_name(postgres_pool.clone(), &name).await {
                Ok(exchange) => {
                    match cmd("HSET")
                        .arg(format!("exchange:{}", &exchange.exchange_id))
                        .arg("created_at")
                        .arg(&exchange.created_at.to_string())
                        .arg("exchange_id")
                        .arg(&exchange.exchange_id.to_string())
                        .arg("exchange")
                        .arg(&exchange.exchange)
                        .query_async::<_, ()>(&mut connection)
                        .await
                    {
                        Ok(_) => {
                            println!("Successfully saved exchange to Redis with key exchange:{}", exchange.exchange_id);
                            Ok(exchange)
                        }
                        Err(e) => {
                            println!("Failed to save exchange to redis: {}", e);
                            Ok(exchange)
                        }
                    }
                }
                Err(e) => {
                    println!("Failed to get exchange by name: {}", e);
                    Err(e)
                }
            }
        }
        false => {
            println!("Exchange does not exist");
            let new_exchange = NewExchange::new(&name);

            match create_exchange(postgres_pool.clone(), new_exchange).await {
                Ok(exchange) => {
                    match cmd("HSET")
                        .arg(format!("exchange:{}", &exchange.exchange_id))
                        .arg("created_at")
                        .arg(&exchange.created_at.to_string())
                        .arg("exchange_id")
                        .arg(&exchange.exchange_id.to_string())
                        .arg("exchange")
                        .arg(&exchange.exchange)
                        .query_async::<_, ()>(&mut connection)
                        .await
                    {
                        Ok(_) => {
                            println!("Successfully saved exchange to Redis with key exchange:{}", exchange.exchange_id);
                            Ok(exchange)
                        }
                        Err(e) => {
                            println!("Failed to save exchange to redis: {}", e);
                            Ok(exchange)
                        }
                    }
                }
                Err(e) => {
                    println!("Failed to create exchange: {}", e);
                    Err(e)
                }
            }
        }
    }
}

pub async fn get_securities(
    redis_pool: Arc<Pool<Manager, Connection>>,
    postgres_pool: Arc<Pool<CustomAsyncPgConnectionManager>>,
    symbols: &Vec<String>,
) -> Result<Vec<Security>, Error> {
    let mut connection = get_redis_connection(&redis_pool)
        .await
        .expect("Failed to create redis connection");

    let mut pipe = redis::pipe();

    let mut securities = vec![];
    for symbol in symbols.iter() {
        match security_exists(postgres_pool.clone(), symbol).await {
            true => {
                println!("Security exists");
                match get_security_by_symbol(postgres_pool.clone(), symbol).await {
                    Ok(security) => {
                        pipe.atomic()
                            .cmd("HSET")
                            .arg(format!("security:{}", &security.security_id))
                            .arg("created_at")
                            .arg(&security.created_at.to_string())
                            .arg("security_id")
                            .arg(&security.security_id.to_string())
                            .arg("symbol")
                            .arg(&security.symbol);
                        securities.push(security);
                    }
                    Err(e) => {
                        println!("Failed to get security by symbol: {}", e);
                    }
                };
            }
            false => {
                println!("Security does not exist");
                let new_security = NewSecurity::new(symbol);
                match create_security(postgres_pool.clone(), new_security).await {
                    Ok(security) => {
                        pipe.atomic()
                            .cmd("HSET")
                            .arg(format!("security:{}", &security.security_id))
                            .arg("created_at")
                            .arg(&security.created_at.to_string())
                            .arg("security_id")
                            .arg(&security.security_id.to_string())
                            .arg("symbol")
                            .arg(&security.symbol);
                        securities.push(security);
                    }
                    Err(e) => {
                        println!("Failed to create security: {}", e);
                    }
                };
            }
        };
    }

    match pipe.query_async::<_, ()>(&mut connection).await {
        Ok(_) => {
            println!("Securities saved to redis");
            Ok(securities)
        }
        Err(e) => {
            println!("Failed to save securities to redis: {}", e);
            Ok(securities)
        }
    }
}

pub async fn get_orderbooks(
    redis_pool: Arc<Pool<Manager, Connection>>,
    postgres_pool: Arc<Pool<CustomAsyncPgConnectionManager>>,
    securities: Vec<Security>,
    exchange: &Exchange,
) -> Result<HashMap<String, OrderBook>, Error> {
    let mut connection = get_redis_connection(&redis_pool)
        .await
        .expect("Failed to create redis connection");

    let mut pipe = redis::pipe();

    let mut orderbooks = HashMap::new();
    for security in securities.iter() {
        match orderbook_exists(postgres_pool.clone(), security.security_id).await {
            true => {
                println!("Orderbook exists for security: {}", security.symbol);
                match get_orderbook_by_exchange_id_and_security_id(postgres_pool.clone(), exchange.exchange_id, security.security_id).await {
                    Ok(order_book) => {
                        pipe.atomic()
                            .cmd("HSET")
                            .arg(format!("order_book:{}", &order_book.order_book_id))
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
                            .arg("order_book_id")
                            .arg(&order_book.order_book_id.to_string())
                            .arg("buy_order_book_id")
                            .arg(&order_book.buy_order_book_id.to_string())
                            .arg("sell_order_book_id")
                            .arg(&order_book.sell_order_book_id.to_string())
                            .arg("total_volume")
                            .arg(&order_book.total_volume.to_string());

                        orderbooks.insert(security.symbol.clone(), order_book);
                    }
                    Err(e) => {
                        println!("Failed to get orderbook by symbol: {}", e);
                    }
                };
            }
            false => {
                println!("Orderbook does not exist for security: {}", security.symbol);
                let new_orderbook = NewOrderBook::new(
                    &security.symbol,
                    &exchange.exchange,
                    security.security_id,
                    exchange.exchange_id,
                );
                match create_orderbook(postgres_pool.clone(), new_orderbook).await {
                    Ok(order_book) => {
                        pipe.atomic()
                            .cmd("HSET")
                            .arg(format!("order_book:{}", &order_book.order_book_id))
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
                            .arg("order_book_id")
                            .arg(&order_book.order_book_id.to_string())
                            .arg("buy_order_book_id")
                            .arg(&order_book.buy_order_book_id.to_string())
                            .arg("sell_order_book_id")
                            .arg(&order_book.sell_order_book_id.to_string())
                            .arg("total_volume")
                            .arg(&order_book.total_volume.to_string());

                        orderbooks.insert(security.symbol.clone(), order_book);
                    }
                    Err(e) => {
                        println!("Failed to create orderbook: {}", e);
                    }
                };
            }
        };
    }

    match pipe.query_async::<_, ()>(&mut connection).await {
        Ok(_) => {
            println!("Orderbooks saved to redis");
            Ok(orderbooks)
        }
        Err(e) => {
            println!("Failed to save orderbooks to redis: {}", e);
            Ok(orderbooks)
        }
    }
}
