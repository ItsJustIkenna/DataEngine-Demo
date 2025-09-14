use anyhow::Result;
use serde::{
    Deserialize,
    Serialize
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Level3UnsubscribeAcknowledgement {
    method: String,
    result: UnsubscribeResonseData,
    success: bool,
    time_in: String,
    time_out: String,
}

impl Level3UnsubscribeAcknowledgement {
    pub fn from_json(json: &serde_json::Value) -> Result<Level3UnsubscribeAcknowledgement> {
        let unsubscribe_ack: Level3UnsubscribeAcknowledgement = serde_json::from_value(json.clone())?;
        Ok(unsubscribe_ack)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UnsubscribeResonseData {
    channel: String,
    symbol: String,
}