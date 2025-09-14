use anyhow::Result;
use serde::{
    Deserialize,
    Serialize
};

#[derive(Debug, Serialize, Deserialize)]
pub struct TradeSubscribe {
    method: String,
    params: TradeSubscribeParams,
}

impl TradeSubscribe {
    pub fn from_json(json: &serde_json::Value) -> Result<TradeSubscribe> {
        let subscribe: TradeSubscribe = serde_json::from_value(json.clone())?;
        Ok(subscribe)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TradeSubscribeParams {
    channel: String,
    symbol: Vec<String>,
    snapshot: bool,
}