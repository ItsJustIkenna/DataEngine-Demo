use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TradeUnsubscribe {
    method: String,
    params: TradeUnsubscribeParams,
}

impl TradeUnsubscribe {
    pub fn from_json(json: &serde_json::Value) -> Result<TradeUnsubscribe> {
        let unsubscribe: TradeUnsubscribe = serde_json::from_value(json.clone())?;
        Ok(unsubscribe)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TradeUnsubscribeParams {
    channel: String,
    symbol: Vec<String>,
    snapshot: bool,
}