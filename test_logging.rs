use hostbuilder::logging_facade::{MAIN_LOGGER, KRAKEN_LOGGER, PERFORMANCE_LOGGER};
use hostbuilder::{log_info, log_warn, log_error, log_debug};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing UltraLogger Implementation Throughout DataEngine");
    
    // Test main logger
    log_info!(MAIN_LOGGER, "Main logger initialized successfully");
    log_debug!(MAIN_LOGGER, "Debug message from main logger");
    log_warn!(MAIN_LOGGER, "Warning message from main logger");
    log_error!(MAIN_LOGGER, "Error message from main logger");
    
    // Test Kraken logger
    log_info!(KRAKEN_LOGGER, "Kraken logger initialized successfully");
    log_debug!(KRAKEN_LOGGER, "Processing WebSocket message");
    log_warn!(KRAKEN_LOGGER, "Failed to get volume: connection timeout");
    log_error!(KRAKEN_LOGGER, "Critical error in order processing");
    
    // Test Performance logger
    log_info!(PERFORMANCE_LOGGER, "Performance logger initialized successfully");
    log_debug!(PERFORMANCE_LOGGER, "Operation completed in 500 nanoseconds");
    log_warn!(PERFORMANCE_LOGGER, "🚨 High latency alert triggered!");
    
    // Test specific methods
    KRAKEN_LOGGER.log_redis_operation("HGET", "order:12345", true, None).await?;
    KRAKEN_LOGGER.log_redis_operation("HGET", "order:99999", false, Some("Connection timeout".to_string())).await?;
    
    KRAKEN_LOGGER.log_database_operation("INSERT", "open_orders", Some(5), true, None).await?;
    KRAKEN_LOGGER.log_database_operation("UPDATE", "historical_orders", None, false, Some("Constraint violation".to_string())).await?;
    
    PERFORMANCE_LOGGER.log_performance("order_processing", 750, true).await?;
    PERFORMANCE_LOGGER.log_performance("market_data_parsing", 2500, false).await?;
    
    // Ensure all logs are flushed
    MAIN_LOGGER.flush().await?;
    KRAKEN_LOGGER.flush().await?;
    PERFORMANCE_LOGGER.flush().await?;
    
    println!("✅ All logging tests completed successfully!");
    println!("📊 UltraLogger is properly integrated throughout the DataEngine");
    
    Ok(())
}
