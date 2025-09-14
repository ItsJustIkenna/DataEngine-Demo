use anyhow::Result;
use serde::{
    Deserialize,
    Serialize
};

#[derive(Debug, Serialize, Deserialize)]
pub struct BalancesSubscribe {
    method: String,
    params: BalancesSubscribeParams,
}

impl BalancesSubscribe {
    pub fn from_json(json: &serde_json::Value) -> Result<BalancesSubscribe> {
        let subscribe: BalancesSubscribe = serde_json::from_value(json.clone())?;
        Ok(subscribe)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BalancesSubscribeParams {
    channel: String,
    token: String,
}