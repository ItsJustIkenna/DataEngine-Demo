use anyhow::Result;
use serde::{
    Deserialize,
    Serialize
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Level3SubscribeAcknowledgement {
    method: String,
    result: Level3SubscribeResonseData,
    success: bool,
    time_in: String,
    time_out: String,
}

impl Level3SubscribeAcknowledgement {
    pub fn from_json(json: &serde_json::Value) -> Result<Level3SubscribeAcknowledgement> {
        let subscribe_ack: Level3SubscribeAcknowledgement = serde_json::from_value(json.clone())?;
        Ok(subscribe_ack)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Level3SubscribeResonseData {
    channel: String,
    snapshot: bool,
    symbol: String,
}