use std::{
    sync::Arc
};
use std::collections::HashMap;

use databaseschema::{
    models::{
        exchange::{
            Exchange,
            NewExchange
        },
        order_book::{
            NewOrderBook,
            OrderBook
        },
        security::{
            NewSecurity,
            Security
        }
    },
    ops::{
        exchange_ops::{
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
};
use diesel_async::AsyncPgConnection;
use anyhow::Error;
use redis::cmd;
use redis_utils::get_redis_connection;
use tokio_retry::{strategy::{jitter, ExponentialBackoff}, Retry};

pub async fn get_exchange(
    redis_pool: Arc<deadpool_redis::Pool>,
    postgres_pool: Arc<diesel_async::pooled_connection::deadpool::Pool<AsyncPgConnection>>,
    name: String,
) -> Result<Exchange, Error> {
    let retry_strategy = ExponentialBackoff::from_millis(10).map(jitter).take(3);
    
    // Check if the exchange exists in the database
    if !exchange_exists(postgres_pool.clone(), &name).await {
        println!("Exchange does not exist");
        let new_exchange = NewExchange::new(&name);
        let exchange = create_exchange(postgres_pool.clone(), new_exchange).await?;
        let redis_key = format!("exchange:{}", exchange.exchange);
        
        // Use a fresh Redis connection for each retry attempt
        Retry::spawn(retry_strategy.clone(), || async {
            let mut connection = get_redis_connection(&redis_pool).await.map_err(|e| anyhow::Error::from(e))?;
            cmd("HSET")
                .arg(&redis_key)
                .arg("created_at")
                .arg(&exchange.created_at.to_string())
                .arg("exchange_id")
                .arg(&exchange.exchange_id.to_string())
                .arg("exchange")
                .arg(&exchange.exchange)
                .query_async::<_, ()>(&mut connection)
                .await
                .map_err(|e| anyhow::Error::from(e))
        }).await?;        
        println!("Created and saved new exchange to Redis with key {}", redis_key);
        Ok(exchange)
    } else {
        println!("Exchange already exists");

        // Retrieve the exchange from the database
        let exchange = get_exchanges_by_name(postgres_pool.clone(), &name).await?;
        let redis_key = format!("exchange:{}", exchange.exchange);

        // Check if the exchange exists in Redis with fresh connection
        let exists = Retry::spawn(retry_strategy.clone(), || async {
            let mut connection = get_redis_connection(&redis_pool).await.map_err(|e| anyhow::Error::from(e))?;
            cmd("HEXISTS")
                .arg(&redis_key)
                .arg("exchange")
                .query_async::<_, bool>(&mut connection)
                .await
                .map_err(|e| anyhow::Error::from(e))
        }).await?;

        if !exists {
            // Save the exchange to Redis if it doesn't exist with fresh connection
            Retry::spawn(retry_strategy.clone(), || async {
                let mut connection = get_redis_connection(&redis_pool).await.map_err(|e| anyhow::Error::from(e))?;
                cmd("HSET")
                    .arg(&redis_key)
                    .arg("created_at")
                    .arg(&exchange.created_at.to_string())
                    .arg("exchange_id")
                    .arg(&exchange.exchange_id.to_string())
                    .arg("exchange")
                    .arg(&exchange.exchange)
                    .query_async::<_, ()>(&mut connection)
                    .await
                    .map_err(|e| anyhow::Error::from(e))
            }).await?;
            println!("Saved exchange to Redis with key {}", redis_key);
        } else {
            println!("Exchange already exists in Redis with key {}", redis_key);
        }
        Ok(exchange)
    }
}



pub async fn get_securities(
    redis_pool: Arc<deadpool_redis::Pool>,
    postgres_pool: Arc<diesel_async::pooled_connection::deadpool::Pool<AsyncPgConnection>>,
    symbols: &Vec<String>,
) -> Result<Vec<Security>, Error> {
    let retry_strategy = ExponentialBackoff::from_millis(10).map(jitter).take(3);
    let mut securities = vec![];

    for symbol in symbols.iter() {
        if !security_exists(postgres_pool.clone(), symbol).await {
            println!("Security does not exist in the database: {}", symbol);
            let new_security = NewSecurity::new(symbol);
            let security = create_security(postgres_pool.clone(), new_security).await?;
            
            // Save to Redis immediately after creating in database
            let redis_key = format!("security:{}", security.symbol);
            Retry::spawn(retry_strategy.clone(), || async {
                let mut connection = get_redis_connection(&redis_pool).await.map_err(|e| anyhow::Error::from(e))?;
                cmd("HSET")
                    .arg(&redis_key)
                    .arg("created_at")
                    .arg(&security.created_at.to_string())
                    .arg("security_id")
                    .arg(&security.security_id.to_string())
                    .arg("symbol")
                    .arg(&security.symbol)
                    .query_async::<_, ()>(&mut connection)
                    .await
                    .map_err(|e| anyhow::Error::from(e))
            }).await?;
            println!("Created and saved new security to Redis: {}", symbol);
            securities.push(security);
        } else {
            println!("Security already exists in the database: {}", symbol);

            let security = get_security_by_symbol(postgres_pool.clone(), symbol).await?;
            let redis_key = format!("security:{}", security.symbol);

            let exists = Retry::spawn(retry_strategy.clone(), || async {
                let mut connection = get_redis_connection(&redis_pool).await.map_err(|e| anyhow::Error::from(e))?;
                cmd("HEXISTS")
                    .arg(&redis_key)
                    .arg("symbol")
                    .query_async::<_, bool>(&mut connection)
                    .await
                    .map_err(|e| anyhow::Error::from(e))
            }).await?;
            
            if !exists {
                println!("Security does not exist in Redis. Adding security: {}", security.symbol);
                Retry::spawn(retry_strategy.clone(), || async {
                    let mut connection = get_redis_connection(&redis_pool).await.map_err(|e| anyhow::Error::from(e))?;
                    cmd("HSET")
                        .arg(&redis_key)
                        .arg("created_at")
                        .arg(&security.created_at.to_string())
                        .arg("security_id")
                        .arg(&security.security_id.to_string())
                        .arg("symbol")
                        .arg(&security.symbol)
                        .query_async::<_, ()>(&mut connection)
                        .await
                        .map_err(|e| anyhow::Error::from(e))
                }).await?;
                println!("Saved existing security to Redis: {}", security.symbol);
            } else {
                println!("Security already exists in Redis: {}", security.symbol);
            }
            securities.push(security);
        }
    }

    println!("Successfully processed {} securities", securities.len());
    Ok(securities)
}


pub async fn get_orderbooks(
    redis_pool: Arc<deadpool_redis::Pool>,
    postgres_pool: Arc<diesel_async::pooled_connection::deadpool::Pool<AsyncPgConnection>>,
    securities: Vec<Security>,
    exchange: &Exchange,
) -> Result<HashMap<String, OrderBook>, Error> {
    let retry_strategy = ExponentialBackoff::from_millis(10).map(jitter).take(3);
    let mut orderbooks = HashMap::new();

    println!("Processing {} securities for orderbooks", securities.len());

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
                    // Save to Redis immediately after creating in database
                    let save_result = Retry::spawn(retry_strategy.clone(), || async {
                        let mut connection = get_redis_connection(&redis_pool).await.map_err(|e| anyhow::Error::from(e))?;
                        save_orderbook_to_redis(&mut connection, &order_book).await
                    }).await;

                    match save_result {
                        Ok(()) => {
                            println!("Created and saved new orderbook to Redis for security: {}", security.symbol);
                            orderbooks.insert(security.symbol.clone(), order_book);
                        }
                        Err(e) => {
                            println!("Failed to save new orderbook to Redis for {}: {}", security.symbol, e);
                            // Still add to orderbooks since it exists in database
                            orderbooks.insert(security.symbol.clone(), order_book);
                        }
                    }
                }
                Err(e) => {
                    println!("Failed to create orderbook for {}: {}", security.symbol, e);
                    return Err(anyhow::Error::from(e));
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
                let mut connection = get_redis_connection(&redis_pool).await.map_err(|e| anyhow::Error::from(e))?;
                cmd("HEXISTS")
                    .arg(&redis_key)
                    .arg("symbol")
                    .query_async::<_, bool>(&mut connection)
                    .await
                    .map_err(|e| anyhow::Error::from(e))
            }).await?;

            if !exists {
                println!("Orderbook does not exist in Redis. Adding key {}", redis_key);
                let save_result = Retry::spawn(retry_strategy.clone(), || async {
                    let mut connection = get_redis_connection(&redis_pool).await.map_err(|e| anyhow::Error::from(e))?;
                    save_orderbook_to_redis(&mut connection, &order_book).await
                }).await;

                match save_result {
                    Ok(()) => {
                        println!("Saved existing orderbook to Redis for security: {}", security.symbol);
                    }
                    Err(e) => {
                        println!("Failed to save existing orderbook to Redis for {}: {}", security.symbol, e);
                    }
                }
            } else {
                println!("Orderbook already exists in Redis with key {}", redis_key);
            }
            orderbooks.insert(security.symbol.clone(), order_book);
        }
    }

    println!("Successfully processed {} orderbooks", orderbooks.len());
    Ok(orderbooks)
}

/// Save orderbook details directly to Redis
async fn save_orderbook_to_redis(
    connection: &mut deadpool_redis::Connection,
    order_book: &OrderBook,
) -> Result<(), Error> {
    let redis_key = format!("order_book:{}", order_book.order_book_id);
    
    cmd("HSET")
        .arg(&redis_key)
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
        .arg(&order_book.total_volume.to_string())
        .query_async::<_, ()>(connection)
        .await?;
    
    Ok(())
}
