use anyhow::Result;
use serde::{
    Deserialize,
    Serialize
};

#[derive(Debug, Serialize, Deserialize)]
pub struct BalancesUnsubscribe {
    method: String,
    params: BalancesUnsubscribeParams,
}

impl BalancesUnsubscribe {
    pub fn from_json(json: &serde_json::Value) -> Result<BalancesUnsubscribe> {
        let unsubscribe: BalancesUnsubscribe = serde_json::from_value(json.clone())?;
        Ok(unsubscribe)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BalancesUnsubscribeParams {
    channel: String,
    token: String,
}