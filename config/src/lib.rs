//! Configuration management for DataEngine
//! 
//! Provides centralized configuration management with environment-specific
//! settings, validation, and runtime configuration updates for the DataEngine.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use anyhow::{Result, anyhow, Context};

// Simple sync logging for config - we'll use println! for simplicity since config doesn't need ultra performance
macro_rules! log_info {
    ($msg:expr) => {
        println!("[CONFIG-INFO] {}", $msg);
    };
    ($fmt:expr, $($arg:tt)*) => {
        println!("[CONFIG-INFO] {}", format!($fmt, $($arg)*));
    };
}

macro_rules! log_warn {
    ($msg:expr) => {
        println!("[CONFIG-WARN] {}", $msg);
    };
    ($fmt:expr, $($arg:tt)*) => {
        println!("[CONFIG-WARN] {}", format!($fmt, $($arg)*));
    };
}

macro_rules! log_error {
    ($msg:expr) => {
        eprintln!("[CONFIG-ERROR] {}", $msg);
    };
    ($fmt:expr, $($arg:tt)*) => {
        eprintln!("[CONFIG-ERROR] {}", format!($fmt, $($arg)*));
    };
}
use serde_yaml;

/// Environment types for configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Environment {
    Development,
    Testing,
    Staging,
    Production,
}

impl Default for Environment {
    fn default() -> Self {
        Environment::Development
    }
}

/// Enhanced configuration with validation and environment support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedConfig {
    pub environment: Environment,
    pub exchanges: Vec<ExchangeConfig>,
    pub message_broker: MessageBrokerConfig,
    pub topics: Vec<String>,
    pub topic_routing: HashMap<String, String>,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub performance: PerformanceConfig,
    pub security: SecurityConfigSection,
    pub monitoring: MonitoringConfig,
    pub logging: LoggingConfig,
    
    #[serde(skip)]
    pub last_modified: Option<Instant>,
    #[serde(skip)]
    pub config_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeConfig {
    pub name: String,
    pub enabled: bool,
    pub symbols: Vec<String>,
    pub depth: u32,
    pub websocket_token: String,
    pub websockets: Vec<WebSocketConfig>,
    pub rate_limits: RateLimitConfig,
    pub timeout: Duration,
    pub retry_config: RetryConfigSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    pub channel: String,
    pub endpoint: String,
    pub compression: bool,
    pub heartbeat_interval: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageBrokerConfig {
    pub address: String,
    pub port: u16,
    pub max_connections: u32,
    pub buffer_size: usize,
    pub compression_enabled: bool,
    pub batch_size: usize,
    pub flush_interval: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub postgres_url: String,
    pub redis_url: String,
    pub max_connections: u32,
    pub connection_timeout: Duration,
    pub query_timeout: Duration,
    pub retry_attempts: u32,
    pub backup_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    pub cluster_mode: bool,
    pub cluster_nodes: Vec<String>,
    pub password: Option<String>,
    pub db_index: u32,
    pub key_prefix: String,
    pub ttl_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub worker_threads: u32,
    pub cpu_affinity_enabled: bool,
    pub cpu_cores: Vec<u32>,
    pub memory_limit_mb: u64,
    pub gc_threshold_mb: u64,
    pub metrics_enabled: bool,
    pub metrics_interval: Duration,
    pub simd_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfigSection {
    pub api_key_required: bool,
    pub request_signing_required: bool,
    pub rate_limiting_enabled: bool,
    pub max_requests_per_minute: u32,
    pub blocked_ips: Vec<String>,
    pub allowed_ips: Vec<String>,
    pub encryption_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub health_check_enabled: bool,
    pub health_check_interval: Duration,
    pub metrics_retention_days: u32,
    pub alerting_enabled: bool,
    pub alert_thresholds: AlertThresholds,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub latency_ms: f64,
    pub error_rate_percent: f64,
    pub connection_failures: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String, // "json" or "text"
    pub output: Vec<String>, // ["stdout", "file", "syslog"]
    pub file_path: Option<String>,
    pub max_file_size_mb: u64,
    pub max_files: u32,
    pub structured_logging: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_second: u32,
    pub burst_size: u32,
    pub window_size: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfigSection {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
    pub jitter_enabled: bool,
}

impl Default for EnhancedConfig {
    fn default() -> Self {
        let mut topic_routing = HashMap::new();
        topic_routing.insert("trades".to_string(), "market.data.kraken.trades".to_string());
        topic_routing.insert("balances".to_string(), "portfolio.updates.balances".to_string());
        topic_routing.insert("balances_snapshot".to_string(), "portfolio.updates.snapshots".to_string());
        topic_routing.insert("level3".to_string(), "market.data.kraken.level3".to_string());
        topic_routing.insert("level3_snapshot".to_string(), "market.data.kraken.snapshots".to_string());
        topic_routing.insert("trade_snapshot".to_string(), "market.data.kraken.trade_snapshots".to_string());
        
        Self {
            environment: Environment::Development,
            exchanges: vec![
                ExchangeConfig {
                    name: "kraken".to_string(),
                    enabled: true,
                    symbols: vec!["XBTUSD".to_string(), "ETHUSD".to_string()],
                    depth: 100,
                    websocket_token: "default_token".to_string(),
                    websockets: vec![
                        WebSocketConfig {
                            channel: "trade".to_string(),
                            endpoint: "wss://ws.kraken.com".to_string(),
                            compression: false,
                            heartbeat_interval: Duration::from_secs(30),
                        }
                    ],
                    rate_limits: RateLimitConfig {
                        requests_per_second: 10,
                        burst_size: 20,
                        window_size: Duration::from_secs(60),
                    },
                    timeout: Duration::from_secs(30),
                    retry_config: RetryConfigSection {
                        max_attempts: 3,
                        initial_delay: Duration::from_millis(100),
                        max_delay: Duration::from_secs(10),
                        backoff_multiplier: 2.0,
                        jitter_enabled: true,
                    },
                },
            ],
            message_broker: MessageBrokerConfig {
                address: "localhost".to_string(),
                port: 9092,
                max_connections: 10,
                buffer_size: 8192,
                compression_enabled: true,
                batch_size: 1000,
                flush_interval: Duration::from_millis(100),
            },
            topics: vec![
                "market.data.trades".to_string(),
                "portfolio.updates".to_string(),
            ],
            topic_routing,
            database: DatabaseConfig {
                postgres_url: "postgresql://localhost:5432/dataengine".to_string(),
                redis_url: "redis://localhost:6379".to_string(),
                max_connections: 20,
                connection_timeout: Duration::from_secs(30),
                query_timeout: Duration::from_secs(60),
                retry_attempts: 3,
                backup_enabled: false,
            },
            redis: RedisConfig {
                cluster_mode: false,
                cluster_nodes: vec!["localhost:6379".to_string()],
                password: None,
                db_index: 0,
                key_prefix: "dataengine:".to_string(),
                ttl_seconds: 3600,
            },
            performance: PerformanceConfig {
                worker_threads: 4,
                cpu_affinity_enabled: false,
                cpu_cores: vec![],
                memory_limit_mb: 1024,
                gc_threshold_mb: 512,
                metrics_enabled: true,
                metrics_interval: Duration::from_secs(60),
                simd_enabled: true,
            },
            security: SecurityConfigSection {
                api_key_required: false,
                request_signing_required: false,
                rate_limiting_enabled: true,
                max_requests_per_minute: 1000,
                blocked_ips: vec![],
                allowed_ips: vec![],
                encryption_enabled: true,
            },
            monitoring: MonitoringConfig {
                health_check_enabled: true,
                health_check_interval: Duration::from_secs(30),
                metrics_retention_days: 7,
                alerting_enabled: false,
                alert_thresholds: AlertThresholds {
                    cpu_usage_percent: 80.0,
                    memory_usage_percent: 85.0,
                    latency_ms: 1000.0,
                    error_rate_percent: 5.0,
                    connection_failures: 10,
                },
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "json".to_string(),
                output: vec!["stdout".to_string()],
                file_path: None,
                max_file_size_mb: 100,
                max_files: 5,
                structured_logging: true,
            },
            last_modified: None,
            config_path: None,
        }
    }
}

impl EnhancedConfig {
    /// Load configuration from file with environment override
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .map_err(|e| anyhow!("Failed to read config file '{}': {}", path.display(), e))?;
        
        let mut config: EnhancedConfig = serde_yaml::from_str(&content)
            .map_err(|e| anyhow!("Failed to parse config file: {}", e))?;
        
        config.config_path = Some(path.to_string_lossy().to_string());
        config.last_modified = Some(Instant::now());
        
        // Override with environment variables
        config.apply_environment_overrides();
        
        // Validate configuration
        config.validate()?;
        
        log_info!("Configuration loaded successfully from {}", path.display());
        Ok(config)
    }

    /// Apply environment variable overrides
    fn apply_environment_overrides(&mut self) {
        // Database URLs
        if let Ok(postgres_url) = std::env::var("DATABASE_URL") {
            self.database.postgres_url = postgres_url;
        }
        if let Ok(redis_url) = std::env::var("REDIS_URL") {
            self.database.redis_url = redis_url;
        }
        
        // Environment detection
        if let Ok(env) = std::env::var("ENVIRONMENT") {
            match env.to_lowercase().as_str() {
                "development" | "dev" => self.environment = Environment::Development,
                "testing" | "test" => self.environment = Environment::Testing,
                "staging" | "stage" => self.environment = Environment::Staging,
                "production" | "prod" => self.environment = Environment::Production,
                _ => log_warn!("Unknown environment '{}', using default", env),
            }
        }
        
        // Message broker
        if let Ok(mb_address) = std::env::var("MESSAGE_BROKER_ADDRESS") {
            self.message_broker.address = mb_address;
        }
        if let Ok(mb_port) = std::env::var("MESSAGE_BROKER_PORT") {
            if let Ok(port) = mb_port.parse::<u16>() {
                self.message_broker.port = port;
            }
        }
        
        // Performance
        if let Ok(threads) = std::env::var("WORKER_THREADS") {
            if let Ok(thread_count) = threads.parse::<u32>() {
                self.performance.worker_threads = thread_count;
            }
        }
        
        // Logging
        if let Ok(log_level) = std::env::var("LOG_LEVEL") {
            self.logging.level = log_level;
        }
    }

    /// Validate configuration settings
    pub fn validate(&self) -> Result<()> {
        // Validate exchanges
        if self.exchanges.is_empty() {
            return Err(anyhow!("At least one exchange must be configured"));
        }

        for exchange in &self.exchanges {
            if exchange.name.is_empty() {
                return Err(anyhow!("Exchange name cannot be empty"));
            }
            if exchange.symbols.is_empty() {
                return Err(anyhow!("Exchange '{}' must have at least one symbol", exchange.name));
            }
            if exchange.websockets.is_empty() {
                return Err(anyhow!("Exchange '{}' must have at least one websocket", exchange.name));
            }
        }

        // Validate message broker
        if self.message_broker.address.is_empty() {
            return Err(anyhow!("Message broker address cannot be empty"));
        }
        if self.message_broker.port == 0 {
            return Err(anyhow!("Message broker port must be greater than 0"));
        }

        // Validate performance settings
        if self.performance.worker_threads == 0 {
            return Err(anyhow!("Worker threads must be greater than 0"));
        }
        if self.performance.memory_limit_mb == 0 {
            return Err(anyhow!("Memory limit must be greater than 0"));
        }

        // Validate database URLs
        if self.database.postgres_url.is_empty() {
            return Err(anyhow!("PostgreSQL URL cannot be empty"));
        }
        if self.database.redis_url.is_empty() {
            return Err(anyhow!("Redis URL cannot be empty"));
        }

        log_info!("Configuration validation passed");
        Ok(())
    }

    /// Get environment-specific settings
    pub fn get_environment_overrides(&self) -> HashMap<String, serde_json::Value> {
        let mut overrides = HashMap::new();
        
        match self.environment {
            Environment::Development => {
                overrides.insert("logging.level".to_string(), serde_json::json!("debug"));
                overrides.insert("monitoring.health_check_interval".to_string(), 
                               serde_json::json!("10s"));
            }
            Environment::Testing => {
                overrides.insert("logging.level".to_string(), serde_json::json!("warn"));
                overrides.insert("performance.metrics_enabled".to_string(), serde_json::json!(false));
            }
            Environment::Staging => {
                overrides.insert("logging.level".to_string(), serde_json::json!("info"));
                overrides.insert("monitoring.alerting_enabled".to_string(), serde_json::json!(true));
            }
            Environment::Production => {
                overrides.insert("logging.level".to_string(), serde_json::json!("warn"));
                overrides.insert("security.api_key_required".to_string(), serde_json::json!(true));
                overrides.insert("performance.cpu_affinity_enabled".to_string(), serde_json::json!(true));
            }
        }
        
        overrides
    }

    /// Save configuration to file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let yaml = serde_yaml::to_string(self)
            .map_err(|e| anyhow!("Failed to serialize config: {}", e))?;
        
        fs::write(&path, yaml)
            .map_err(|e| anyhow!("Failed to write config file: {}", e))?;
        
        log_info!("Configuration saved to {}", path.as_ref().display());
        Ok(())
    }

    /// Check if configuration file has been modified
    pub fn has_file_changed(&self) -> bool {
        if let Some(ref path) = self.config_path {
            if let Ok(metadata) = fs::metadata(path) {
                if let Ok(modified) = metadata.modified() {
                    if let Some(last_modified) = self.last_modified {
                        let file_time = modified.duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default();
                        let last_check = last_modified.elapsed();
                        return file_time.as_secs() > last_check.as_secs();
                    }
                }
            }
        }
        false
    }

    /// Reload configuration from file if changed
    pub fn reload_if_changed(&mut self) -> Result<bool> {
        if self.has_file_changed() {
            if let Some(ref path) = self.config_path.clone() {
                let new_config = Self::load_from_file(path)?;
                *self = new_config;
                log_info!("Configuration reloaded from file");
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Get configuration for specific exchange
    pub fn get_exchange_config(&self, name: &str) -> Option<&ExchangeConfig> {
        self.exchanges.iter().find(|e| e.name == name && e.enabled)
    }

    /// Update exchange configuration
    pub fn update_exchange_config(&mut self, name: &str, config: ExchangeConfig) -> Result<()> {
        if let Some(exchange) = self.exchanges.iter_mut().find(|e| e.name == name) {
            *exchange = config;
            Ok(())
        } else {
            Err(anyhow!("Exchange '{}' not found", name))
        }
    }

    /// Get effective configuration as JSON for API responses
    pub fn to_json(&self) -> Result<String> {
        // Create sanitized version without sensitive data
        let mut sanitized = self.clone();
        sanitized.database.postgres_url = "***REDACTED***".to_string();
        sanitized.database.redis_url = "***REDACTED***".to_string();
        sanitized.redis.password = Some("***REDACTED***".to_string());
        
        serde_json::to_string_pretty(&sanitized)
            .map_err(|e| anyhow!("Failed to serialize config to JSON: {}", e))
    }
}

/// Configuration manager with hot-reloading support
pub struct ConfigManager {
    config: Arc<RwLock<EnhancedConfig>>,
    reload_interval: Duration,
}

impl ConfigManager {
    pub fn new(config: EnhancedConfig, reload_interval: Duration) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            reload_interval,
        }
    }

    pub fn get_config(&self) -> Arc<RwLock<EnhancedConfig>> {
        Arc::clone(&self.config)
    }

    /// Start hot-reloading service
    pub async fn start_hot_reload(&self) {
        let config = Arc::clone(&self.config);
        let interval = self.reload_interval;
        
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            
            loop {
                interval_timer.tick().await;
                
                if let Ok(mut config_guard) = config.write() {
                    match config_guard.reload_if_changed() {
                        Ok(true) => log_info!("Configuration hot-reloaded successfully"),
                        Ok(false) => {}, // No change
                        Err(e) => log_error!("Failed to reload configuration: {}", e),
                    }
                }
            }
        });
    }

    /// Update configuration at runtime
    pub fn update_config<F>(&self, updater: F) -> Result<()>
    where
        F: FnOnce(&mut EnhancedConfig) -> Result<()>,
    {
        let mut config = self.config.write()
            .map_err(|e| anyhow!("Failed to acquire config lock: {}", e))?;
        
        updater(&mut config)?;
        config.validate()?;
        
        log_info!("Configuration updated at runtime");
        Ok(())
    }
}

/// Legacy configuration structure (maintained for backward compatibility)
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Config {
    pub message_broker: MessageBroker,
    pub topics: Vec<String>,
    pub topic_routing: HashMap<String, String>,
    pub exchanges: Vec<Exchange>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct MessageBroker {
    pub address: String,
    pub port: u16,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Exchange {
    pub name: String,
    pub websocket_token: String,
    pub symbols: Vec<String>,
    pub depth: u64,
    pub websockets: Vec<Websocket>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Websocket {
    pub channel: String,
    pub endpoint: String,
}

impl Config {
    pub fn new(file_path: &str) -> Result<Self> {
        let config_data = fs::read_to_string(file_path)
            .with_context(|| format!("Unable to read file: {}", file_path))?;
        let config: Config = serde_yaml::from_str(&config_data)
            .context("YAML was not well-formatted")?;
        Ok(config)
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut topic_routing = HashMap::new();
        topic_routing.insert("trades".to_string(), "market.data.kraken.trades".to_string());
        topic_routing.insert("balances".to_string(), "portfolio.updates.balances".to_string());
        topic_routing.insert("balances_snapshot".to_string(), "portfolio.updates.snapshots".to_string());
        topic_routing.insert("level3".to_string(), "market.data.kraken.level3".to_string());
        topic_routing.insert("level3_snapshot".to_string(), "market.data.kraken.snapshots".to_string());
        topic_routing.insert("trade_snapshot".to_string(), "market.data.kraken.trade_snapshots".to_string());
        
        Self {
            message_broker: MessageBroker {
                address: "localhost".to_string(),
                port: 8080,
            },
            topics: vec!["market.data.kraken".to_string()],
            topic_routing,
            exchanges: vec![
                Exchange {
                    name: "Kraken".to_string(),
                    websocket_token: "https://api.kraken.com/0/private/GetWebSocketsToken".to_string(),
                    symbols: vec!["BTC/USD".to_string(), "ETH/USD".to_string()],
                    depth: 100,
                    websockets: vec![
                        Websocket {
                            channel: "level3".to_string(),
                            endpoint: "wss://ws.kraken.com".to_string(),
                        },
                        Websocket {
                            channel: "trade".to_string(),
                            endpoint: "wss://ws.kraken.com".to_string(),
                        },
                    ],
                }
            ],
        }
    }
}

/// Convert from enhanced config to legacy config
impl From<EnhancedConfig> for Config {
    fn from(enhanced: EnhancedConfig) -> Self {
        Self {
            message_broker: MessageBroker {
                address: enhanced.message_broker.address,
                port: enhanced.message_broker.port,
            },
            topics: enhanced.topics,
            topic_routing: enhanced.topic_routing,
            exchanges: enhanced.exchanges.into_iter().map(|e| Exchange {
                name: e.name,
                websocket_token: e.websocket_token,
                symbols: e.symbols,
                depth: e.depth as u64,
                websockets: e.websockets.into_iter().map(|w| Websocket {
                    channel: w.channel,
                    endpoint: w.endpoint,
                }).collect(),
            }).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = EnhancedConfig::default();
        assert!(config.validate().is_ok());
        assert!(!config.exchanges.is_empty());
        assert_eq!(config.environment, Environment::Development);
    }

    #[test]
    fn test_config_serialization() {
        let config = EnhancedConfig::default();
        let yaml = serde_yaml::to_string(&config).unwrap();
        let deserialized: EnhancedConfig = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(config.exchanges.len(), deserialized.exchanges.len());
    }

    #[test]
    fn test_config_validation() {
        let mut config = EnhancedConfig::default();
        
        // Valid config should pass
        assert!(config.validate().is_ok());
        
        // Invalid config should fail
        config.exchanges.clear();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_file_operations() {
        let config = EnhancedConfig::default();
        let temp_file = NamedTempFile::new().unwrap();
        
        // Save config
        config.save_to_file(temp_file.path()).unwrap();
        
        // Load config
        let loaded_config = EnhancedConfig::load_from_file(temp_file.path()).unwrap();
        assert_eq!(config.exchanges.len(), loaded_config.exchanges.len());
    }

    #[tokio::test]
    async fn test_config_manager() {
        let config = EnhancedConfig::default();
        let manager = ConfigManager::new(config, Duration::from_millis(100));
        
        // Test configuration access
        {
            let config_ref = manager.get_config();
            let config_guard = config_ref.read().unwrap();
            assert!(!config_guard.exchanges.is_empty());
        }
        
        // Test configuration update
        manager.update_config(|config| {
            config.message_broker.port = 9090;
            Ok(())
        }).unwrap();
        
        {
            let config_ref = manager.get_config();
            let config_guard = config_ref.read().unwrap();
            assert_eq!(config_guard.message_broker.port, 9090);
        }
    }

    #[test]
    fn test_legacy_config() {
        let config = Config::default();
        assert!(!config.exchanges.is_empty());
        assert_eq!(config.message_broker.port, 8080);
    }

    #[test]
    fn test_enhanced_to_legacy_conversion() {
        let enhanced = EnhancedConfig::default();
        let legacy: Config = enhanced.into();
        assert!(!legacy.exchanges.is_empty());
        assert_eq!(legacy.message_broker.address, "localhost");
    }
}
