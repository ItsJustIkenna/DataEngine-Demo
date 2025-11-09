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
use diesel_async::AsyncPgConnection;
use anyhow::Error;
use crate::infrastructure::logging_facade::{MAIN_LOGGER, log_info, log_error};

pub async fn get_exchange(
    postgres_pool: Arc<diesel_async::pooled_connection::deadpool::Pool<AsyncPgConnection>>,
    name: String,
) -> Result<Exchange, Error> {
    // Check if the exchange exists in the database
    if !exchange_exists(postgres_pool.clone(), &name).await {
        log_info!(MAIN_LOGGER, "Exchange does not exist, creating new exchange: {}", name);
        let new_exchange = NewExchange::new(&name);
        let exchange = create_exchange(postgres_pool.clone(), new_exchange).await?;
        log_info!(MAIN_LOGGER, "Created new exchange: {}", exchange.exchange);
        Ok(exchange)
    } else {
        let exchange = get_exchanges_by_name(postgres_pool.clone(), &name).await?;
        log_info!(MAIN_LOGGER, "Retrieved existing exchange: {}", exchange.exchange);
        Ok(exchange)
    }
}



pub async fn get_securities(
    postgres_pool: Arc<diesel_async::pooled_connection::deadpool::Pool<AsyncPgConnection>>,
    symbols: &[String],
) -> Result<Vec<Security>, Error> {
    let mut securities = vec![];

    for symbol in symbols.iter() {
        if !security_exists(postgres_pool.clone(), symbol).await {
            log_info!(MAIN_LOGGER, "Security does not exist in database, creating: {}", symbol);
            let new_security = NewSecurity::new(symbol);
            let security = create_security(postgres_pool.clone(), new_security).await?;
            securities.push(security);
        } else {
            let security = get_security_by_symbol(postgres_pool.clone(), symbol).await?;
            securities.push(security);
        }
    }

    log_info!(MAIN_LOGGER, "Successfully processed {} securities", securities.len());
    Ok(securities)
}


pub async fn get_orderbooks(
    postgres_pool: Arc<diesel_async::pooled_connection::deadpool::Pool<AsyncPgConnection>>,
    securities: Vec<Security>,
    exchange: &Exchange,
) -> Result<HashMap<String, OrderBook>, Error> {
    let mut orderbooks = HashMap::new();

    log_info!(MAIN_LOGGER, "Processing {} securities for orderbooks", securities.len());

    for security in securities.iter() {
        // Check if orderbook exists in PostgreSQL
        if !orderbook_exists(postgres_pool.clone(), security.security_id).await {
            // Create new orderbook in PostgreSQL
            let new_orderbook = NewOrderBook::new(
                &security.symbol,
                &exchange.exchange,
                security.security_id,
                exchange.exchange_id,
            );

            match create_orderbook(postgres_pool.clone(), new_orderbook).await {
                Ok(order_book) => {
                    log_info!(MAIN_LOGGER, "Created new orderbook for security: {}", security.symbol);
                    orderbooks.insert(security.symbol.clone(), order_book);
                }
                Err(e) => {
                    log_error!(MAIN_LOGGER, "Failed to create orderbook for {}: {}", security.symbol, e);
                    return Err(anyhow::Error::from(e));
                }
            }
        } else {
            // Retrieve existing orderbook from PostgreSQL
            let order_book = get_orderbook_by_exchange_id_and_security_id(
                postgres_pool.clone(),
                exchange.exchange_id,
                security.security_id,
            )
            .await?;

            orderbooks.insert(security.symbol.clone(), order_book);
        }
    }

    log_info!(MAIN_LOGGER, "Successfully processed {} orderbooks", orderbooks.len());
    Ok(orderbooks)
}
