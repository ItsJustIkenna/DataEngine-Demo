use std::{
    collections::HashMap, sync::Arc
};

use databaseschema::{
    models::{
        exchange::{
            Exchange,
            NewExchange
        },
        orderbook::{
            NewOrderBook,
            OrderBook
        },
        security::{
            NewSecurity,
            Security
        }
    },
    ops::{
        exchanges_ops::{
            create_exchange,
            exchange_exists,
            get_exchanges_by_name
        },
        order_book_ops::{
            create_orderbook,
            get_orderbook_by_exchange_id_and_security_id,
            orderbook_exists
        },
        securities_ops::{
            create_security,
            get_security_by_symbol,
            security_exists
        },
    },
    CustomAsyncPgConnectionManager,
};
use deadpool::managed::Pool;
use deadpool_redis::{
    Connection,
    Manager
};
use anyhow::Error;
use redis::cmd;
use redis_utils::get_redis_connection;
use tokio_retry::{strategy::{jitter, ExponentialBackoff}, Retry};

pub async fn get_exchange(
    redis_pool: Arc<Pool<Manager, Connection>>,
    postgres_pool: Arc<Pool<CustomAsyncPgConnectionManager>>,
    name: String,
) -> Result<Exchange, Error> {
    let retry_strategy = ExponentialBackoff::from_millis(10).map(jitter).take(3);

    // Acquire a Redis connection
    let connection = get_redis_connection(&redis_pool).await?;
    
    // Check if the exchange exists in the database
    if !exchange_exists(postgres_pool.clone(), &name).await {
        println!("Exchange does not exist");
        let new_exchange = NewExchange::new(&name);
        let exchange = create_exchange(postgres_pool.clone(), new_exchange).await?;
        let redis_key = format!("exchange:{}", exchange.exchange);
        Retry::spawn(retry_strategy.clone(), || async {
            cmd("HSET")
                .arg(&redis_key)
                .arg("created_at")
                .arg(&exchange.created_at.to_string())
                .arg("exchange_id")
                .arg(&exchange.exchange_id.to_string())
                .arg("exchange")
                .arg(&exchange.exchange)
                .query_async::<_, ()>(&mut connection.clone())
                .await
        }).await?;        
        Ok(exchange)
    } else {
        println!("Exchange already exists");

        // Retrieve the exchange from the database
        let exchange = get_exchanges_by_name(postgres_pool.clone(), &name).await?;
        let redis_key = format!("exchange:{}", exchange.exchange);

        // Check if the exchange exists in Redis
        let exists= Retry::spawn(retry_strategy.clone(), || async {
            cmd("HEXISTS")
                .arg(&redis_key)
                .arg("exchange")
                .query_async::<_, bool>(&mut connection.clone())
                .await
        }).await?;

        if !exists {
            // Save the exchange to Redis if it doesn't exist
            Retry::spawn(retry_strategy.clone(), || async {
                cmd("HSET")
                    .arg(&redis_key)
                    .arg("created_at")
                    .arg(&exchange.created_at.to_string())
                    .arg("exchange_id")
                    .arg(&exchange.exchange_id.to_string())
                    .arg("exchange")
                    .arg(&exchange.exchange)
                    .query_async::<_, ()>(&mut connection.clone())
                    .await
            }).await?;
            println!("Saved exchange to Redis with key {}", redis_key);
        } else {
            println!("Exchange already exists in Redis with key {}", redis_key);
        }
        Ok(exchange)
    }
}



pub async fn get_securities(
    redis_pool: Arc<Pool<Manager, Connection>>,
    postgres_pool: Arc<Pool<CustomAsyncPgConnectionManager>>,
    symbols: &Vec<String>,
) -> Result<Vec<Security>, Error> {
    let retry_strategy = ExponentialBackoff::from_millis(10).map(jitter).take(3);

    let connection = get_redis_connection(&redis_pool).await?;
    let mut pipe = redis::pipe();
    let mut securities = vec![];

    for symbol in symbols.iter() {
        if !security_exists(postgres_pool.clone(), symbol).await {
            println!("Security does not exist in the database: {}", symbol);
            let new_security = NewSecurity::new(symbol);
            let security = create_security(postgres_pool.clone(), new_security).await?;
            pipe.atomic()
                .cmd("HSET")
                .arg(format!("security:{}", security.security_id))
                .arg("created_at")
                .arg(&security.created_at.to_string())
                .arg("security_id")
                .arg(&security.security_id.to_string())
                .arg("symbol")
                .arg(&security.symbol);
            securities.push(security);
        } else {
            println!("Security already exists in the database: {}", symbol);

            let security = get_security_by_symbol(postgres_pool.clone(), symbol).await?;

            let exists = Retry::spawn(retry_strategy.clone(), || async {
                cmd("HEXISTS")
                    .arg(format!("security:{}", security.security_id))
                    .arg("symbol")
                    .query_async::<_, bool>(&mut connection.clone())
                    .await
            }).await?;
            
            if !exists {
                println!("Security does not exist in Redis. Adding security: {}", security.symbol);
                pipe.atomic()
                .cmd("HSET")
                .arg(format!("security:{}", security.security_id))
                .arg("created_at")
                .arg(&security.created_at.to_string())
                .arg("security_id")
                .arg(&security.security_id.to_string())
                .arg("symbol")
                .arg(&security.symbol);
            } else {
                println!("Security already exists in Redis: {}", security.symbol);
            }
            securities.push(security);
        }
    }

    Retry::spawn(retry_strategy.clone(), || async {
        // Execute the pipe
        pipe.query_async::<_, ()>(&mut connection.clone()).await?;
        println!("Securities saved to Redis");
        Ok::<(), Error>(())
    }).await?;

    Ok(securities)
}


pub async fn get_orderbooks(
    redis_pool: Arc<Pool<Manager, Connection>>,
    postgres_pool: Arc<Pool<CustomAsyncPgConnectionManager>>,
    securities: Vec<Security>,
    exchange: &Exchange,
) -> Result<HashMap<String, OrderBook>, Error> {
    // Get a Redis connection
    let connection = match get_redis_connection(&redis_pool).await {
        Ok(conn) => conn,
        Err(e) => {
            println!("Failed to create Redis connection: {}", e);
            return Err(e);
        }
    };

    println!("Successfully created Redis connection");

    let retry_strategy = ExponentialBackoff::from_millis(10).map(jitter).take(3);
    let mut pipe = redis::pipe();
    let mut orderbooks = HashMap::new();

    for security in securities.iter() {
        // Check if orderbook exists in PostgreSQL
        if !orderbook_exists(postgres_pool.clone(), security.security_id).await {
            println!("Orderbook does not exist for security: {}", security.symbol);

            // Create new orderbook in PostgreSQL
            let new_orderbook = NewOrderBook::new(
                &security.symbol,
                &exchange.exchange,
                security.security_id,
                exchange.exchange_id,
            );

            match create_orderbook(postgres_pool.clone(), new_orderbook).await {
                Ok(order_book) => {
                    add_orderbook_to_redis_pipeline(&mut pipe, &order_book);
                    orderbooks.insert(security.symbol.clone(), order_book);
                }
                Err(e) => {
                    println!("Failed to create orderbook: {}", e);
                }
            }
        } else {
            println!("Orderbook exists for security: {}", security.symbol);

            // Retrieve existing orderbook from PostgreSQL
            let order_book = get_orderbook_by_exchange_id_and_security_id(
                postgres_pool.clone(),
                exchange.exchange_id,
                security.security_id,
            )
            .await?;

            let redis_key = format!("order_book:{}", order_book.order_book_id);

            // Check if the orderbook exists in Redis
            let exists = Retry::spawn(retry_strategy.clone(), || async {
                cmd("HEXISTS")
                    .arg(&redis_key)
                    .arg("symbol")
                    .query_async::<_, bool>(&mut connection.clone())
                    .await
            })
            .await?;

            if !exists {
                println!("Orderbook does not exist in Redis. Adding key {}", redis_key);
                add_orderbook_to_redis_pipeline(&mut pipe, &order_book);
            } else {
                println!("Orderbook already exists in Redis with key {}", redis_key);
            }
            orderbooks.insert(security.symbol.clone(), order_book);
        }
    }

    // Execute the Redis pipeline
    Retry::spawn(retry_strategy, || async {
        pipe.query_async::<_, ()>(&mut connection.clone()).await?;
        println!("Orderbooks saved to Redis");
        Ok::<(), Error>(())
    }).await?;

    Ok(orderbooks)
}

/// Add orderbook details to the Redis pipeline
fn add_orderbook_to_redis_pipeline(pipe: &mut redis::Pipeline, order_book: &OrderBook) {
    pipe.atomic()
        .cmd("HSET")
        .arg(format!("order_book:{}", order_book.order_book_id))
        .arg("created_at")
        .arg(&order_book.created_at.to_string())
        .arg("updated_at")
        .arg(&order_book.updated_at.unwrap_or_default().to_string())
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
}
