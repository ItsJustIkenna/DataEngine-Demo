use ultra_logger::{UltraLogger, LogLevel, LogValue, LogEntry};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::collections::HashMap;
use once_cell::sync::Lazy;

/// Global logging facade for the DataEngine
/// Provides unified access to UltraLogger across all components
pub struct DataEngineLogger {
    logger: UltraLogger,
    service_name: String,
}

impl DataEngineLogger {
    pub fn new(service_name: &str) -> Self {
        Self {
            logger: UltraLogger::new(format!("DataEngine-{}", service_name)),
            service_name: service_name.to_string(),
        }
    }

    /// Log debug information
    pub async fn debug(&self, message: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.logger.debug(message.to_string()).await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    /// Log informational messages
    pub async fn info(&self, message: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.logger.info(message.to_string()).await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    /// Log warning messages
    pub async fn warn(&self, message: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.logger.warn(message.to_string()).await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    /// Log error messages
    pub async fn error(&self, message: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.logger.error(message.to_string()).await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    /// Log with custom level
    pub async fn log(&self, level: LogLevel, message: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.logger.log(level, message.to_string()).await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    /// Log performance metrics
    pub async fn log_performance(
        &self,
        operation: &str,
        duration_nanos: u64,
        success: bool
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let message = format!("Performance: {} took {}ns, success: {}", operation, duration_nanos, success);
        self.info(&message).await
    }

    /// Log Redis operations
    pub async fn log_redis_operation(
        &self,
        operation: &str,
        key: &str,
        success: bool,
        error_message: Option<String>
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let message = if success {
            format!("Redis {}: {}", operation, key)
        } else {
            format!("Redis {} failed: {} - error: {:?}", operation, key, error_message)
        };
        
        if success { 
            self.debug(&message).await 
        } else { 
            self.error(&message).await 
        }
    }

    /// Log database operations
    pub async fn log_database_operation(
        &self,
        operation: &str,
        table: &str,
        affected_rows: Option<usize>,
        success: bool,
        error_message: Option<String>
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let message = if success {
            format!("Database {}: {} (rows: {:?})", operation, table, affected_rows)
        } else {
            format!("Database {} failed: {} - error: {:?}", operation, table, error_message)
        };
        
        if success { 
            self.debug(&message).await 
        } else { 
            self.error(&message).await 
        }
    }

    /// Flush logs synchronously
    pub async fn flush(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.logger.flush().await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    /// Shutdown the logger
    pub async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.logger.shutdown().await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }
}

/// Global logger instances for different DataEngine components
pub static MAIN_LOGGER: Lazy<Arc<DataEngineLogger>> = Lazy::new(|| {
    Arc::new(DataEngineLogger::new("Main"))
});

pub static KRAKEN_LOGGER: Lazy<Arc<DataEngineLogger>> = Lazy::new(|| {
    Arc::new(DataEngineLogger::new("Kraken"))
});

pub static ORDERBOOK_LOGGER: Lazy<Arc<DataEngineLogger>> = Lazy::new(|| {
    Arc::new(DataEngineLogger::new("OrderBook"))
});

pub static PERFORMANCE_LOGGER: Lazy<Arc<DataEngineLogger>> = Lazy::new(|| {
    Arc::new(DataEngineLogger::new("Performance"))
});

pub static DATA_PIPELINE_LOGGER: Lazy<Arc<DataEngineLogger>> = Lazy::new(|| {
    Arc::new(DataEngineLogger::new("DataPipeline"))
});

/// Convenience macros for logging throughout the DataEngine
#[macro_export]
macro_rules! log_debug {
    ($logger:expr, $($arg:tt)*) => {
        {
            let logger = $logger.clone();
            let message = format!($($arg)*);
            tokio::spawn(async move {
                let _ = logger.debug(&message).await;
            });
        }
    };
}

#[macro_export]
macro_rules! log_info {
    ($logger:expr, $($arg:tt)*) => {
        {
            let logger = $logger.clone();
            let message = format!($($arg)*);
            tokio::spawn(async move {
                let _ = logger.info(&message).await;
            });
        }
    };
}

#[macro_export]
macro_rules! log_warn {
    ($logger:expr, $($arg:tt)*) => {
        {
            let logger = $logger.clone();
            let message = format!($($arg)*);
            tokio::spawn(async move {
                let _ = logger.warn(&message).await;
            });
        }
    };
}

#[macro_export]
macro_rules! log_error {
    ($logger:expr, $($arg:tt)*) => {
        {
            let logger = $logger.clone();
            let message = format!($($arg)*);
            tokio::spawn(async move {
                let _ = logger.error(&message).await;
            });
        }
    };
}
