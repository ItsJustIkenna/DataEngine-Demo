use anyhow::Result;
use serde::{
    Deserialize,
    Serialize
};
use std::time::Instant;

use crate::network::handlers::logging::{KRAKEN_WEBSOCKET_LOGGER, KRAKEN_SUBSCRIPTION_LOGGER, WebSocketLog, SubscriptionLog};

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

    /// Process subscription request and log it
    pub async fn process_subscription(&self) -> Result<()> {
        let start = Instant::now();
        
        // Log each symbol subscription separately
        for symbol in &self.params.symbol {
            let log_entry = SubscriptionLog {
                channel: self.params.channel.clone(),
                action: "subscribe".to_string(),
                symbol: Some(symbol.clone()),
                success: true, // Assume success for now, could be enhanced with actual result
                processing_time_micros: start.elapsed().as_micros() as u64,
                error: None,
            };
            
            let _ = KRAKEN_SUBSCRIPTION_LOGGER.log_subscription(log_entry).await;
        }
        
        Ok(())
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