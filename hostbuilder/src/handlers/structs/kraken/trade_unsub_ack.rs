use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TradeUnsubscribeAcknowledgement {
    method: String,
    result: TradeUnsubscribeResonseData,
    success: bool,
    time_in: String,
    time_out: String,
}

impl TradeUnsubscribeAcknowledgement {
    pub fn from_json(json: &serde_json::Value) -> Result<TradeUnsubscribeAcknowledgement> {
        let unsubscribe_ack: TradeUnsubscribeAcknowledgement = serde_json::from_value(json.clone())?;
        Ok(unsubscribe_ack)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TradeUnsubscribeResonseData {
    channel: String,
    symbol: String,
}