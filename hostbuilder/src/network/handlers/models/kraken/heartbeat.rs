use anyhow::Result;
use serde::{
    Deserialize,
    Serialize
};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::network::handlers::logging::{KRAKEN_WEBSOCKET_LOGGER, KRAKEN_HEARTBEAT_LOGGER, HeartbeatLog};

#[derive(Debug, Serialize, Deserialize)]
pub struct HeartBeat {
    channel: String,
}

impl HeartBeat {
    pub fn from_json(json: &serde_json::Value) -> Result<HeartBeat> {
        let heartbeat: HeartBeat = serde_json::from_value(json.clone())?;
        Ok(heartbeat)
    }

    /// Process heartbeat and log for connection monitoring
    pub async fn process_heartbeat(&self) -> Result<()> {
        let timestamp_micros = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64;

        let log_entry = HeartbeatLog {
            channel: self.channel.clone(),
            timestamp_micros,
            latency_micros: None, // Could be calculated if we track request times
            connection_status: "healthy".to_string(),
        };

        let _ = KRAKEN_HEARTBEAT_LOGGER.log_heartbeat(log_entry).await;
        Ok(())
    }
}
