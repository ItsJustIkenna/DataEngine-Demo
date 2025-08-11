use anyhow::Result;
use serde::{
    Deserialize,
    Serialize
};

#[derive(Debug, Serialize, Deserialize)]
pub struct BalancesSubscribeAcknowledgement {
    method: String,
    result: BalancesSubscribeResonseData,
    success: bool,
    time_in: String,
    time_out: String,
}

impl BalancesSubscribeAcknowledgement {
    pub fn from_json(json: &serde_json::Value) -> Result<BalancesSubscribeAcknowledgement> {
        let subscribe_ack: BalancesSubscribeAcknowledgement = serde_json::from_value(json.clone())?;
        Ok(subscribe_ack)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BalancesSubscribeResonseData {
    channel: String,
    snapshot: bool,
}