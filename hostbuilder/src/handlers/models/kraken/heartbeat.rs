use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct HeartBeat {
    channel: String,
}

impl HeartBeat {
    pub fn from_json(json: &serde_json::Value) -> Result<HeartBeat> {
        let heartbeat: HeartBeat = serde_json::from_value(json.clone())?;
        Ok(heartbeat)
    }
}
