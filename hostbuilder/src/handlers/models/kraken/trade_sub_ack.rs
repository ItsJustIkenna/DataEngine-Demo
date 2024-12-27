use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TradeSubscribeAcknowledgement {
    method: String,
    result: TradeSubscribeResonseData,
    success: bool,
    time_in: String,
    time_out: String,
}

impl TradeSubscribeAcknowledgement {
    pub fn from_json(json: &serde_json::Value) -> Result<TradeSubscribeAcknowledgement> {
        let subscribe_ack: TradeSubscribeAcknowledgement = serde_json::from_value(json.clone())?;
        Ok(subscribe_ack)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TradeSubscribeResonseData {
    channel: String,
    snapshot: bool,
    symbol: String,
}