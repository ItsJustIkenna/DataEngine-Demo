use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use serde::{
    Deserialize,
    Serialize
};

#[derive(Debug, Serialize, Deserialize)]
pub struct TradeData {
    pub symbol: String,
    pub side: String,
    pub price: BigDecimal,
    pub qty: BigDecimal,
    pub ord_type: String,
    pub trade_id: u64,
    pub timestamp: DateTime<Utc>,
}