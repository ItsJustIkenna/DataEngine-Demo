//! Benchmark-only access to the Kraken level3 `update` hot path, compiled only
//! with the `bench-internals` feature. No network, no Postgres, no broker.
//!
//! Mirrors, in order, what `KrakenWebSocketHandler::listen`'s fast path and
//! `Level3Update::process_update` do per text frame, minus the I/O:
//!   1. detect:    `FastMessageParser::is_level3_update` (real)
//!   2. parse:     `FastMessageParser::parse_level3_update_fast` (real, serde_json)
//!   3. route key: `FastMessageParser::extract_symbol_fast` + order-book lookup (real)
//!   4. normalize: per bid/ask, the protobuf `Order` plus the `NewOpenBuyOrder` /
//!                 `NewOpenSellOrder` / `NewHistoricalOrder` rows `process_update`
//!                 builds (mirrored here; `process_update` itself needs a live
//!                 Postgres pool and issues the DB writes inline)
//!   5. encode:    `Orders` -> `MarketMessage` -> `PublishRequest` -> prost bytes
//! NOT measured: the Postgres writes, the logger calls, and the broker publish.
use super::*;
use crate::network::handlers::models::kraken::fast_parser::FastMessageParser;
use dashmap::DashMap;

pub struct L3Pipeline {
    parser: FastMessageParser,
    books: DashMap<String, OrderBook>,
    topic: String,
}

pub struct Normalized {
    pub orders: usize,
    pub db_rows: usize,
    pub bytes: Vec<u8>,
}

impl L3Pipeline {
    pub fn new(symbols: &[&str]) -> Self {
        let books = DashMap::new();
        for s in symbols {
            books.insert(
                s.to_string(),
                OrderBook {
                    created_at: Utc::now(),
                    updated_at: None,
                    symbol: s.to_string(),
                    exchange: "kraken".to_string(),
                    security_id: uuid::Uuid::new_v4(),
                    exchange_id: uuid::Uuid::new_v4(),
                    order_book_id: uuid::Uuid::new_v4(),
                    buy_order_book_id: uuid::Uuid::new_v4(),
                    sell_order_book_id: uuid::Uuid::new_v4(),
                    total_volume: BigDecimal::zero(),
                },
            );
        }
        Self { parser: FastMessageParser::new(), books, topic: "market.data".to_string() }
    }

    pub fn process_frame(&self, text: &str) -> Option<Normalized> {
        let bytes = text.as_bytes();
        if !self.parser.is_level3_update(bytes) {
            return None;
        }
        let update = self.parser.parse_level3_update_fast(text).ok()?;
        let symbol = self.parser.extract_symbol_fast(bytes)?;
        let book = self.books.get(&symbol)?;
        let d = update.data.first()?;

        let mut orders: Vec<Order> = Vec::with_capacity(d.bids.len() + d.asks.len());
        let mut buy_rows = Vec::new();
        let mut sell_rows = Vec::new();
        let mut hist_rows = Vec::new();
        let mut modifies = 0usize;
        let mut deletes = 0usize;

        for (side, list) in [(SIDE_BUY, &d.bids), (SIDE_SELL, &d.asks)] {
            for o in list.iter() {
                let (event, hist_event, status, prev_price) = match o.event.as_str() {
                    EVENT_ADD => (EVENT_NEW, EVENT_NEW, STATUS_OPEN, None),
                    EVENT_MODIFY => { modifies += 1; (EVENT_MODIFY, EVENT_MODIFY, STATUS_PARTIALLY_FILLED, Some(&o.limit_price)) }
                    EVENT_DELETE => { deletes += 1; (EVENT_DELETE, EVENT_CANCEL, STATUS_FILLED, None) }
                    _ => continue,
                };
                orders.push(Order {
                    unique_id: o.order_id.clone(),
                    symbol: book.symbol.clone(),
                    exchange: book.exchange.clone(),
                    price_level: o.limit_price.to_f32().unwrap_or(0.0),
                    quantity: o.order_qty.to_f32().unwrap_or(0.0),
                    side: side.to_string(),
                    event: event.to_string(),
                });
                if o.event == EVENT_ADD {
                    if side == SIDE_BUY {
                        buy_rows.push(NewOpenBuyOrder::new(o.timestamp, &book.symbol, &book.exchange, book.security_id,
                            book.exchange_id, book.buy_order_book_id, &o.order_id, &o.limit_price, &o.order_qty));
                    } else {
                        sell_rows.push(NewOpenSellOrder::new(o.timestamp, &book.symbol, &book.exchange, book.security_id,
                            book.exchange_id, book.sell_order_book_id, &o.order_id, &o.limit_price, &o.order_qty));
                    }
                }
                hist_rows.push(NewHistoricalOrder::new(o.timestamp, &o.order_id, hist_event, side, &o.limit_price,
                    &o.order_qty, prev_price, None, status, &book.exchange, &book.symbol, book.exchange_id, book.security_id));
            }
        }

        let n = orders.len();
        let request = PublishRequest {
            topic: self.topic.clone(),
            payload: Some(publish_request::Payload::MarketPayload(MarketMessage {
                market_id: "kraken_level3_update".to_string(),
                payload: Some(market_message::Payload::OrdersPayload(Orders { orders })),
            })),
        };
        Some(Normalized {
            orders: n,
            db_rows: buy_rows.len() + sell_rows.len() + hist_rows.len() + modifies + deletes,
            bytes: prost::Message::encode_to_vec(&request),
        })
    }
}

/// Synthetic Kraken WS v2 level3 `update` frames: 1-4 order events per frame
/// (add/modify/delete mix), rotating over 2 symbols.
pub fn synthetic_frames(count: usize) -> Vec<String> {
    let syms = ["BTC/USD", "ETH/USD"];
    let events = ["add", "add", "modify", "delete"];
    (0..count)
        .map(|i| {
            let k = 1 + i % 4;
            let mk = |j: usize, side: &str| {
                let px = if side == "b" { 50_000.0 - (j as f64) * 0.1 } else { 50_001.0 + (j as f64) * 0.1 };
                format!(
                    r#"{{"event":"{}","order_id":"O{}{}-{}","limit_price":{:.1},"order_qty":{:.8},"timestamp":"2025-11-14T19:{:02}:{:02}.{:06}Z"}}"#,
                    events[(i + j) % 4], side, i, j, px, 0.001 + (j as f64) * 0.01, (i / 60) % 60, i % 60, i % 1_000_000
                )
            };
            let bids: Vec<String> = (0..k.div_ceil(2)).map(|j| mk(j, "b")).collect();
            let asks: Vec<String> = (0..k / 2).map(|j| mk(j, "a")).collect();
            format!(
                r#"{{"channel":"level3","type":"update","data":[{{"symbol":"{}","checksum":{},"bids":[{}],"asks":[{}]}}]}}"#,
                syms[i % 2], 1_000_000 + i, bids.join(","), asks.join(",")
            )
        })
        .collect()
}
