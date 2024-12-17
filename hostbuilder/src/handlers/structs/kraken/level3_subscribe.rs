use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Level3Subscribe {
    method: String,
    params: Level3SubscribeParams,
}

impl Level3Subscribe {
    pub fn from_json(json: &serde_json::Value) -> Result<Level3Subscribe> {
        let subscribe: Level3Subscribe = serde_json::from_value(json.clone())?;
        Ok(subscribe)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Level3SubscribeParams {
    channel: String,
    symbol: Vec<String>,
    depth: u64,
    snapshot: bool,
    token: String,
}