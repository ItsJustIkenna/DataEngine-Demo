use anyhow::Result;
use serde::{
    Deserialize,
    Serialize
};

#[derive(Debug, Serialize, Deserialize)]
pub struct BalancesUnsubscribeAcknowledgement {
    method: String,
    result: BalancesUnsubscribeResonseData,
    success: bool,
    time_in: String,
    time_out: String,
}

impl BalancesUnsubscribeAcknowledgement {
    pub fn from_json(json: &serde_json::Value) -> Result<BalancesUnsubscribeAcknowledgement> {
        let subscribe_ack: BalancesUnsubscribeAcknowledgement = serde_json::from_value(json.clone())?;
        Ok(subscribe_ack)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BalancesUnsubscribeResonseData {
    channel: String,
}