use anyhow::Result;
use std::env;
use hostbuilder::{HostedObject, HostedObjectTrait, infrastructure::logging_facade::MAIN_LOGGER, log_info, log_warn, log_error};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize Data Engine with logging
    log_info!(MAIN_LOGGER, "🚀 Starting Data Engine...");

    // Get configuration path from environment or use default
    let config_path = env::var("CONFIG_PATH").unwrap_or_else(|_| {
        log_warn!(MAIN_LOGGER, "CONFIG_PATH not set, using default config path");
        "./config/default.toml".to_string()
    });

    log_info!(MAIN_LOGGER, "📁 Using configuration from: {}", config_path);

    // Set the CONFIG_PATH environment variable for the hostbuilder
    env::set_var("CONFIG_PATH", &config_path);

    // Create the hosted object
    log_info!(MAIN_LOGGER, "🏗️  Initializing Data Engine components...");
    
    let mut engine = match HostedObject::new().await {
        Ok(engine) => {
            log_info!(MAIN_LOGGER, "✅ Data Engine initialized successfully");
            engine
        }
        Err(e) => {
            log_error!(MAIN_LOGGER, "❌ Failed to initialize Data Engine: {}", e);
            return Err(e);
        }
    };

    // Run the hosted object and handle any errors
    log_info!(MAIN_LOGGER, "🎯 Starting Data Engine main loop...");
    
    match engine.run().await {
        Ok(_) => {
            log_info!(MAIN_LOGGER, "✅ Data Engine completed successfully");
        }
        Err(e) => {
            log_error!(MAIN_LOGGER, "❌ Data Engine error: {}", e);
            return Err(e);
        }
    }

    log_info!(MAIN_LOGGER, "🛑 Data Engine shutdown complete");
    Ok(())
}
