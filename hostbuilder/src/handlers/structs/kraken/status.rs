use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Status {
    channel: String,
    data: Vec<StatusData>,
    r#type: String,
}

impl Status {
    pub fn from_json(json: &serde_json::Value) -> Result<Status> {
        let status: Status = serde_json::from_value(json.clone())?;
        Ok(status)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusData {
    api_version: String,
    connection_id: u64,
    system: String,
    version: String,
}