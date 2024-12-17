use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Level3Unsubscribe {
    method: String,
    params: Level3UnsubscribeParams,
}

impl Level3Unsubscribe {
    pub fn from_json(json: &serde_json::Value) -> Result<Level3Unsubscribe> {
        let unsubscribe: Level3Unsubscribe = serde_json::from_value(json.clone())?;
        Ok(unsubscribe)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Level3UnsubscribeParams {
    channel: String,
    symbol: Vec<String>,
    token: String,
}