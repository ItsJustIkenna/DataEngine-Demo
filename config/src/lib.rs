//! Centralized configuration management for DataEngine
//! 
//! Simple, focused configuration handling for the DataEngine with
//! environment variable support and basic validation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use anyhow::Result;

/// Environment types for configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Environment {
    Development,
    Testing,
    Production,
}

impl Default for Environment {
    fn default() -> Self {
        Environment::Development
    }
}

/// Core DataEngine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub environment: Environment,
    pub message_broker: MessageBrokerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub exchanges: Vec<ExchangeConfig>,
    pub performance: PerformanceConfig,
    pub security: SecurityConfig,
    pub topics: Vec<String>,
    pub topic_routing: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    pub endpoint: String,
    pub channel: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeConfig {
    pub name: String,
    pub enabled: bool,
    pub symbols: Vec<String>,
    pub websocket_url: String,
    pub api_url: String,
    pub max_depth: u32,
    pub websocket_token: String,
    pub websockets: Vec<WebSocketConfig>,
    pub depth: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageBrokerConfig {
    pub address: String,
    pub port: u16,
    pub buffer_size: usize,
    pub batch_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub postgres_url: String,
    pub redis_url: String,
    pub max_connections: u32,
    pub connection_timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub cluster_mode: bool,
    pub cluster_nodes: Vec<String>,
    pub db_index: u32,
    pub key_prefix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub worker_threads: u32,
    pub memory_limit_mb: u64,
    pub metrics_enabled: bool,
    pub cpu_affinity_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub api_key_required: bool,
    pub request_signing_required: bool,
    pub rate_limiting_enabled: bool,
    pub max_requests_per_minute: u32,
}

impl Default for Config {
    fn default() -> Self {
        let mut topic_routing = HashMap::new();
        topic_routing.insert("trades".to_string(), "market.data.kraken.trades".to_string());
        topic_routing.insert("balances".to_string(), "portfolio.updates.balances".to_string());
        topic_routing.insert("level3".to_string(), "market.data.kraken.level3".to_string());
        topic_routing.insert("level3_snapshot".to_string(), "market.data.kraken.snapshots".to_string());
        
        Self {
            environment: Environment::Development,
            message_broker: MessageBrokerConfig {
                address: "localhost".to_string(),
                port: 9092,
                buffer_size: 8192,
                batch_size: 1000,
            },
            database: DatabaseConfig {
                postgres_url: "postgresql://localhost:5432/dataengine".to_string(),
                redis_url: "redis://localhost:6379".to_string(),
                max_connections: 20,
                connection_timeout_secs: 30,
            },
            redis: RedisConfig {
                cluster_mode: false,
                cluster_nodes: vec!["localhost:6379".to_string()],
                db_index: 0,
                key_prefix: "dataengine:".to_string(),
            },
            exchanges: vec![
                ExchangeConfig {
                    name: "kraken".to_string(),
                    enabled: true,
                    symbols: vec!["XBTUSD".to_string(), "ETHUSD".to_string()],
                    websocket_url: "wss://ws.kraken.com".to_string(),
                    api_url: "https://api.kraken.com".to_string(),
                    max_depth: 100,
                    websocket_token: "".to_string(),
                    websockets: vec![
                        WebSocketConfig {
                            endpoint: "wss://ws.kraken.com".to_string(),
                            channel: "level3".to_string(),
                        },
                        WebSocketConfig {
                            endpoint: "wss://ws.kraken.com".to_string(),
                            channel: "trades".to_string(),
                        },
                    ],
                    depth: 100,
                }
            ],
            performance: PerformanceConfig {
                worker_threads: num_cpus::get() as u32,
                memory_limit_mb: 1024,
                metrics_enabled: true,
                cpu_affinity_enabled: false,
            },
            security: SecurityConfig {
                api_key_required: false,
                request_signing_required: false,
                rate_limiting_enabled: true,
                max_requests_per_minute: 100,
            },
            topics: vec![
                "market.data.trades".to_string(),
                "portfolio.updates".to_string(),
            ],
            topic_routing,
        }
    }
}

impl Config {
    /// Create a new config from file path (alias for from_file)
    pub fn new(path: &str) -> Result<Self> {
        Self::from_file(path)
    }

    /// Load configuration from YAML file with environment overrides
    pub fn from_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let mut config: Self = serde_yaml::from_str(&content)?;
        config.apply_env_overrides();
        config.validate()?;
        Ok(config)
    }

    /// Apply environment variable overrides
    fn apply_env_overrides(&mut self) {
        if let Ok(url) = std::env::var("DATABASE_URL") {
            self.database.postgres_url = url;
        }
        if let Ok(url) = std::env::var("REDIS_URL") {
            self.database.redis_url = url;
        }
        if let Ok(addr) = std::env::var("MESSAGE_BROKER_ADDRESS") {
            self.message_broker.address = addr;
        }
        if let Ok(port) = std::env::var("MESSAGE_BROKER_PORT") {
            if let Ok(port_num) = port.parse() {
                self.message_broker.port = port_num;
            }
        }
        if let Ok(threads) = std::env::var("WORKER_THREADS") {
            if let Ok(thread_count) = threads.parse() {
                self.performance.worker_threads = thread_count;
            }
        }
    }

    /// Basic validation
    fn validate(&self) -> Result<()> {
        if self.exchanges.is_empty() {
            return Err(anyhow::anyhow!("At least one exchange must be configured"));
        }
        for exchange in &self.exchanges {
            if exchange.name.is_empty() {
                return Err(anyhow::anyhow!("Exchange name cannot be empty"));
            }
            if exchange.symbols.is_empty() {
                return Err(anyhow::anyhow!("Exchange '{}' must have symbols", exchange.name));
            }
        }
        if self.message_broker.address.is_empty() {
            return Err(anyhow::anyhow!("Message broker address cannot be empty"));
        }
        if self.message_broker.port == 0 {
            return Err(anyhow::anyhow!("Message broker port must be greater than 0"));
        }
        Ok(())
    }

    /// Get enabled exchange by name
    pub fn get_exchange(&self, name: &str) -> Option<&ExchangeConfig> {
        self.exchanges.iter().find(|e| e.name == name && e.enabled)
    }

    /// Save configuration to YAML file
    pub fn save_to_file(&self, path: &str) -> Result<()> {
        let yaml = serde_yaml::to_string(self)?;
        std::fs::write(path, yaml)?;
        Ok(())
    }

    /// Get configuration as JSON for API responses (with sensitive data redacted)
    pub fn to_json(&self) -> Result<String> {
        let mut sanitized = self.clone();
        sanitized.database.postgres_url = "***REDACTED***".to_string();
        sanitized.database.redis_url = "***REDACTED***".to_string();
        
        serde_json::to_string_pretty(&sanitized)
            .map_err(|e| anyhow::anyhow!("Failed to serialize config to JSON: {}", e))
    }
}

/// Simple configuration manager for API compatibility
pub struct ConfigManager {
    config: std::sync::Arc<std::sync::RwLock<Config>>,
}

impl ConfigManager {
    pub fn new(config: Config) -> Self {
        Self {
            config: std::sync::Arc::new(std::sync::RwLock::new(config)),
        }
    }

    pub fn get_config(&self) -> std::sync::Arc<std::sync::RwLock<Config>> {
        std::sync::Arc::clone(&self.config)
    }

    /// Update configuration with a closure
    pub fn update_config<F>(&self, updater: F) -> Result<()>
    where
        F: FnOnce(&mut Config) -> Result<()>,
    {
        let mut config = self.config.write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire config lock: {}", e))?;
        
        updater(&mut config)?;
        config.validate()?;
        Ok(())
    }

    /// Reload configuration from file if changed (placeholder implementation)
    pub fn reload_if_changed(&self) -> Result<bool> {
        // In a real implementation, this would check file modification time
        // For now, just return false indicating no reload occurred
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.validate().is_ok());
        assert!(!config.exchanges.is_empty());
        assert_eq!(config.environment, Environment::Development);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let yaml = serde_yaml::to_string(&config).unwrap();
        let deserialized: Config = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(config.exchanges.len(), deserialized.exchanges.len());
    }

    #[test]
    fn test_config_validation() {
        let mut config = Config::default();
        
        // Valid config should pass
        assert!(config.validate().is_ok());
        
        // Invalid config should fail
        config.exchanges.clear();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_exchange_lookup() {
        let config = Config::default();
        assert!(config.get_exchange("kraken").is_some());
        assert!(config.get_exchange("nonexistent").is_none());
    }
}
