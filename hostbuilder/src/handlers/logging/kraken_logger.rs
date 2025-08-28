use ultra_logger::{UltraLogger, LogLevel, LogValue, LogEntry};
use std::collections::HashMap;
use bigdecimal::BigDecimal;
use once_cell::sync::Lazy;

/// High-performance centralized logging service for Kraken operations
/// Optimized for minimal latency impact on critical trading paths
pub struct KrakenLogger {
    logger: UltraLogger,
    service_name: String,
}

impl KrakenLogger {
    pub fn new(service_name: &str) -> Self {
        Self {
            logger: UltraLogger::new(format!("DataEngine-Kraken-{}", service_name)),
            service_name: service_name.to_string(),
        }
    }

    /// Log WebSocket connection events
    pub async fn log_websocket_event(&self, event: WebSocketEvent) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut fields = HashMap::new();
        fields.insert("endpoint".to_string(), LogValue::String(event.endpoint));
        fields.insert("action".to_string(), LogValue::String(event.action));
        fields.insert("connection_id".to_string(), LogValue::String(event.connection_id.unwrap_or_default()));
        
        if let Some(latency) = event.latency_micros {
            fields.insert("latency_microseconds".to_string(), LogValue::Number(latency as f64));
        }

        let entry = LogEntry::new(
            event.level,
            self.service_name.clone(),
            event.message,
        );

        // Add fields to entry
        let entry_with_fields = fields.into_iter().fold(entry, |acc, (k, v)| {
            acc.with_field(k, v)
        });

        // Use structured logging output
        println!("{}", serde_json::to_string(&entry_with_fields)?);
        Ok(())
    }

    /// Log order book update processing with performance metrics
    pub async fn log_orderbook_update(&self, update: OrderBookUpdateLog) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut fields = HashMap::new();
        fields.insert("symbol".to_string(), LogValue::String(update.symbol.clone()));
        fields.insert("update_type".to_string(), LogValue::String(update.update_type.clone()));
        fields.insert("bids_count".to_string(), LogValue::Number(update.bids_count as f64));
        fields.insert("asks_count".to_string(), LogValue::Number(update.asks_count as f64));
        fields.insert("processing_time_micros".to_string(), LogValue::Number(update.processing_time_micros as f64));
        fields.insert("total_volume".to_string(), LogValue::String(update.total_volume.to_string()));
        
        if let Some(ref error) = update.error {
            fields.insert("error".to_string(), LogValue::String(error.clone()));
        }

        let level = if update.error.is_some() { LogLevel::Error } else { LogLevel::Info };
        
        let entry = LogEntry::new(
            level,
            self.service_name.clone(),
            format!("Order book update processed for {}", update.symbol),
        );

        let entry_with_fields = fields.into_iter().fold(entry, |acc, (k, v)| {
            acc.with_field(k, v)
        });

        println!("{}", serde_json::to_string(&entry_with_fields)?);
        Ok(())
    }

    /// Log trade processing events
    pub async fn log_trade_processing(&self, trade: TradeProcessingLog) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut fields = HashMap::new();
        fields.insert("symbol".to_string(), LogValue::String(trade.symbol.clone()));
        fields.insert("trade_id".to_string(), LogValue::String(trade.trade_id.clone()));
        fields.insert("side".to_string(), LogValue::String(trade.side.clone()));
        fields.insert("price".to_string(), LogValue::Number(trade.price as f64));
        fields.insert("quantity".to_string(), LogValue::Number(trade.quantity as f64));
        fields.insert("processing_time_micros".to_string(), LogValue::Number(trade.processing_time_micros as f64));
        
        if let Some(ref error) = trade.error {
            fields.insert("error".to_string(), LogValue::String(error.clone()));
        }

        let level = if trade.error.is_some() { LogLevel::Error } else { LogLevel::Debug };
        
        let entry = LogEntry::new(
            level,
            self.service_name.clone(),
            format!("Trade processed: {} @ {} for {}", trade.quantity, trade.price, trade.symbol),
        );

        let entry_with_fields = fields.into_iter().fold(entry, |acc, (k, v)| {
            acc.with_field(k, v)
        });

        println!("{}", serde_json::to_string(&entry_with_fields)?);
        Ok(())
    }

    /// Log API authentication events
    pub async fn log_auth_event(&self, event: AuthEvent) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut fields = HashMap::new();
        fields.insert("endpoint".to_string(), LogValue::String(event.endpoint.clone()));
        fields.insert("success".to_string(), LogValue::Bool(event.success));
        
        if let Some(ref error) = event.error {
            fields.insert("error".to_string(), LogValue::String(error.clone()));
        }
        
        if let Some(latency) = event.latency_micros {
            fields.insert("latency_microseconds".to_string(), LogValue::Number(latency as f64));
        }

        let level = if event.success { LogLevel::Info } else { LogLevel::Error };
        
        let entry = LogEntry::new(
            level,
            self.service_name.clone(),
            format!("Authentication {} for {}", if event.success { "succeeded" } else { "failed" }, event.endpoint),
        );

        let entry_with_fields = fields.into_iter().fold(entry, |acc, (k, v)| {
            acc.with_field(k, v)
        });

        println!("{}", serde_json::to_string(&entry_with_fields)?);
        Ok(())
    }

    /// Log performance metrics
    pub async fn log_performance_metrics(&self, metrics: PerformanceMetrics) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut fields = HashMap::new();
        fields.insert("component".to_string(), LogValue::String(metrics.component.clone()));
        fields.insert("operation".to_string(), LogValue::String(metrics.operation.clone()));
        fields.insert("execution_time_micros".to_string(), LogValue::Number(metrics.execution_time_micros as f64));
        fields.insert("throughput_ops_per_sec".to_string(), LogValue::Number(metrics.throughput_ops_per_sec));
        fields.insert("memory_usage_mb".to_string(), LogValue::Number(metrics.memory_usage_mb));
        fields.insert("cpu_usage_percent".to_string(), LogValue::Number(metrics.cpu_usage_percent));

        let entry = LogEntry::new(
            LogLevel::Info,
            self.service_name.clone(),
            format!("Performance metrics for {}-{}", metrics.component, metrics.operation),
        );

        let entry_with_fields = fields.into_iter().fold(entry, |acc, (k, v)| {
            acc.with_field(k, v)
        });

        println!("{}", serde_json::to_string(&entry_with_fields)?);
        Ok(())
    }

    /// Emergency error logging with minimal overhead
    pub async fn log_critical_error(&self, error: CriticalError) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut fields = HashMap::new();
        fields.insert("component".to_string(), LogValue::String(error.component.clone()));
        fields.insert("error_code".to_string(), LogValue::String(error.error_code.clone()));
        fields.insert("error_message".to_string(), LogValue::String(error.error_message.clone()));
        fields.insert("stack_trace".to_string(), LogValue::String(error.stack_trace.unwrap_or_default()));
        
        if let Some(context) = error.context {
            fields.insert("context".to_string(), LogValue::String(context));
        }

        let entry = LogEntry::new(
            LogLevel::Error,
            self.service_name.clone(),
            format!("CRITICAL ERROR in {}: {}", error.component, error.error_code),
        );

        let entry_with_fields = fields.into_iter().fold(entry, |acc, (k, v)| {
            acc.with_field(k, v)
        });

        println!("{}", serde_json::to_string(&entry_with_fields)?);
        Ok(())
    }

    /// Log snapshot processing (Level3, Trade, Balance snapshots)
    pub async fn log_snapshot_processing(&self, snapshot: SnapshotProcessingLog) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut fields = HashMap::new();
        fields.insert("symbol".to_string(), LogValue::String(snapshot.symbol.clone()));
        fields.insert("snapshot_type".to_string(), LogValue::String(snapshot.snapshot_type.clone()));
        fields.insert("data_count".to_string(), LogValue::Number(snapshot.data_count as f64));
        fields.insert("processing_time_micros".to_string(), LogValue::Number(snapshot.processing_time_micros as f64));
        fields.insert("data_size_bytes".to_string(), LogValue::Number(snapshot.data_size_bytes as f64));
        
        if let Some(ref error) = snapshot.error {
            fields.insert("error".to_string(), LogValue::String(error.clone()));
        }

        let level = if snapshot.error.is_some() { LogLevel::Error } else { LogLevel::Info };
        let entry = LogEntry::new(
            level,
            self.service_name.clone(),
            format!("{} snapshot processed for {} ({} records)", 
                   snapshot.snapshot_type, snapshot.symbol, snapshot.data_count),
        );

        let entry_with_fields = fields.into_iter().fold(entry, |acc, (k, v)| {
            acc.with_field(k, v)
        });

        println!("{}", serde_json::to_string(&entry_with_fields)?);
        Ok(())
    }

    /// Log balance updates
    pub async fn log_balance_update(&self, balance: BalanceUpdateLog) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut fields = HashMap::new();
        fields.insert("asset".to_string(), LogValue::String(balance.asset.clone()));
        fields.insert("balance".to_string(), LogValue::Number(balance.balance));
        fields.insert("balance_change".to_string(), LogValue::Number(balance.balance_change));
        fields.insert("ledger_id".to_string(), LogValue::String(balance.ledger_id.clone()));
        fields.insert("processing_time_micros".to_string(), LogValue::Number(balance.processing_time_micros as f64));
        
        if let Some(ref error) = balance.error {
            fields.insert("error".to_string(), LogValue::String(error.clone()));
        }

        let level = if balance.error.is_some() { LogLevel::Error } else { LogLevel::Info };
        let entry = LogEntry::new(
            level,
            self.service_name.clone(),
            format!("Balance update for {} - New: {:.8}, Change: {:.8}", 
                   balance.asset, balance.balance, balance.balance_change),
        );

        let entry_with_fields = fields.into_iter().fold(entry, |acc, (k, v)| {
            acc.with_field(k, v)
        });

        println!("{}", serde_json::to_string(&entry_with_fields)?);
        Ok(())
    }

    /// Log heartbeat events for connection monitoring
    pub async fn log_heartbeat(&self, heartbeat: HeartbeatLog) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut fields = HashMap::new();
        fields.insert("channel".to_string(), LogValue::String(heartbeat.channel.clone()));
        fields.insert("timestamp_micros".to_string(), LogValue::Number(heartbeat.timestamp_micros as f64));
        fields.insert("connection_status".to_string(), LogValue::String(heartbeat.connection_status.clone()));
        
        if let Some(latency) = heartbeat.latency_micros {
            fields.insert("latency_micros".to_string(), LogValue::Number(latency as f64));
        }

        let level = match heartbeat.connection_status.as_str() {
            "healthy" => LogLevel::Debug,
            "delayed" => LogLevel::Warn,
            "timeout" => LogLevel::Error,
            _ => LogLevel::Info,
        };

        let entry = LogEntry::new(
            level,
            self.service_name.clone(),
            format!("Heartbeat received on {} - Status: {}", 
                   heartbeat.channel, heartbeat.connection_status),
        );

        let entry_with_fields = fields.into_iter().fold(entry, |acc, (k, v)| {
            acc.with_field(k, v)
        });

        println!("{}", serde_json::to_string(&entry_with_fields)?);
        Ok(())
    }

    /// Log subscription management events
    pub async fn log_subscription(&self, subscription: SubscriptionLog) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut fields = HashMap::new();
        fields.insert("channel".to_string(), LogValue::String(subscription.channel.clone()));
        fields.insert("action".to_string(), LogValue::String(subscription.action.clone()));
        fields.insert("success".to_string(), LogValue::String(subscription.success.to_string()));
        fields.insert("processing_time_micros".to_string(), LogValue::Number(subscription.processing_time_micros as f64));
        
        if let Some(ref symbol) = subscription.symbol {
            fields.insert("symbol".to_string(), LogValue::String(symbol.clone()));
        }
        
        if let Some(ref error) = subscription.error {
            fields.insert("error".to_string(), LogValue::String(error.clone()));
        }

        let level = if subscription.success { LogLevel::Info } else { LogLevel::Error };
        let symbol_info = subscription.symbol.as_ref().map(|s| format!(" for {}", s)).unwrap_or_default();
        
        let entry = LogEntry::new(
            level,
            self.service_name.clone(),
            format!("{} {} on {}{}", 
                   subscription.action, 
                   if subscription.success { "successful" } else { "failed" },
                   subscription.channel,
                   symbol_info),
        );

        let entry_with_fields = fields.into_iter().fold(entry, |acc, (k, v)| {
            acc.with_field(k, v)
        });

        println!("{}", serde_json::to_string(&entry_with_fields)?);
        Ok(())
    }
}

// Global logger instances for different components - using Lazy for thread safety
pub static KRAKEN_WEBSOCKET_LOGGER: Lazy<KrakenLogger> = Lazy::new(|| KrakenLogger::new("WebSocket"));
pub static KRAKEN_REST_LOGGER: Lazy<KrakenLogger> = Lazy::new(|| KrakenLogger::new("REST"));
pub static KRAKEN_ORDERBOOK_LOGGER: Lazy<KrakenLogger> = Lazy::new(|| KrakenLogger::new("OrderBook"));
pub static KRAKEN_TRADE_LOGGER: Lazy<KrakenLogger> = Lazy::new(|| KrakenLogger::new("Trade"));
pub static KRAKEN_BALANCE_LOGGER: Lazy<KrakenLogger> = Lazy::new(|| KrakenLogger::new("Balance"));
pub static KRAKEN_SNAPSHOT_LOGGER: Lazy<KrakenLogger> = Lazy::new(|| KrakenLogger::new("Snapshot"));
pub static KRAKEN_HEARTBEAT_LOGGER: Lazy<KrakenLogger> = Lazy::new(|| KrakenLogger::new("Heartbeat"));
pub static KRAKEN_SUBSCRIPTION_LOGGER: Lazy<KrakenLogger> = Lazy::new(|| KrakenLogger::new("Subscription"));

// Logging data structures for structured logging
#[derive(Debug, Clone)]
pub struct WebSocketEvent {
    pub endpoint: String,
    pub action: String, // connect, disconnect, message_received, message_sent, error
    pub connection_id: Option<String>,
    pub message: String,
    pub level: LogLevel,
    pub latency_micros: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct OrderBookUpdateLog {
    pub symbol: String,
    pub update_type: String, // snapshot, update
    pub bids_count: usize,
    pub asks_count: usize,
    pub processing_time_micros: u64,
    pub total_volume: BigDecimal,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TradeProcessingLog {
    pub symbol: String,
    pub trade_id: String,
    pub side: String,
    pub price: f32,
    pub quantity: f32,
    pub processing_time_micros: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AuthEvent {
    pub endpoint: String,
    pub success: bool,
    pub error: Option<String>,
    pub latency_micros: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub component: String,
    pub operation: String,
    pub execution_time_micros: u64,
    pub throughput_ops_per_sec: f64,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
}

#[derive(Debug, Clone)]
pub struct CriticalError {
    pub component: String,
    pub error_code: String,
    pub error_message: String,
    pub stack_trace: Option<String>,
    pub context: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SnapshotProcessingLog {
    pub symbol: String,
    pub snapshot_type: String, // level3_snapshot, trade_snapshot, balance_snapshot
    pub data_count: usize,
    pub processing_time_micros: u64,
    pub data_size_bytes: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BalanceUpdateLog {
    pub asset: String,
    pub balance: f64,
    pub balance_change: f64,
    pub ledger_id: String,
    pub processing_time_micros: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct HeartbeatLog {
    pub channel: String,
    pub timestamp_micros: u64,
    pub latency_micros: Option<u64>,
    pub connection_status: String, // healthy, delayed, timeout
}

#[derive(Debug, Clone)]
pub struct SubscriptionLog {
    pub channel: String,
    pub action: String, // subscribe, unsubscribe, ack, error
    pub symbol: Option<String>,
    pub success: bool,
    pub processing_time_micros: u64,
    pub error: Option<String>,
}

// Convenience macros for common logging operations
#[macro_export]
macro_rules! log_kraken_websocket {
    ($action:expr, $endpoint:expr, $message:expr) => {
        {
            let event = WebSocketEvent {
                endpoint: $endpoint.to_string(),
                action: $action.to_string(),
                connection_id: None,
                message: $message.to_string(),
                level: LogLevel::Info,
                latency_micros: None,
            };
            let _ = KRAKEN_WEBSOCKET_LOGGER.log_websocket_event(event).await;
        }
    };
    ($action:expr, $endpoint:expr, $message:expr, $level:expr) => {
        {
            let event = WebSocketEvent {
                endpoint: $endpoint.to_string(),
                action: $action.to_string(),
                connection_id: None,
                message: $message.to_string(),
                level: $level,
                latency_micros: None,
            };
            let _ = KRAKEN_WEBSOCKET_LOGGER.log_websocket_event(event).await;
        }
    };
}

#[macro_export]
macro_rules! log_kraken_orderbook {
    ($symbol:expr, $update_type:expr, $bids:expr, $asks:expr, $processing_time:expr, $volume:expr) => {
        {
            let update = OrderBookUpdateLog {
                symbol: $symbol.to_string(),
                update_type: $update_type.to_string(),
                bids_count: $bids,
                asks_count: $asks,
                processing_time_micros: $processing_time,
                total_volume: $volume,
                error: None,
            };
            let _ = KRAKEN_ORDERBOOK_LOGGER.log_orderbook_update(update).await;
        }
    };
}

#[macro_export]
macro_rules! log_kraken_critical {
    ($component:expr, $code:expr, $message:expr) => {
        {
            let error = CriticalError {
                component: $component.to_string(),
                error_code: $code.to_string(),
                error_message: $message.to_string(),
                stack_trace: None,
                context: None,
            };
            let _ = KRAKEN_WEBSOCKET_LOGGER.log_critical_error(error).await;
        }
    };
}
