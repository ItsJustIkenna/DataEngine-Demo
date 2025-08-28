use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time::interval;
use sysinfo::{System};
use std::collections::HashMap;

use crate::handlers::logging::{KRAKEN_WEBSOCKET_LOGGER, PerformanceMetrics};

/// High-performance monitoring service for Kraken components
/// Tracks latency, throughput, memory usage, and system metrics
pub struct KrakenPerformanceMonitor {
    system: Arc<RwLock<System>>,
    component_metrics: Arc<RwLock<HashMap<String, ComponentMetrics>>>,
    monitoring_interval: Duration,
}

#[derive(Debug, Clone)]
pub struct ComponentMetrics {
    pub operation_count: u64,
    pub total_execution_time_micros: u64,
    pub peak_memory_mb: f64,
    pub error_count: u64,
    pub last_update: Instant,
}

impl ComponentMetrics {
    pub fn new() -> Self {
        Self {
            operation_count: 0,
            total_execution_time_micros: 0,
            peak_memory_mb: 0.0,
            error_count: 0,
            last_update: Instant::now(),
        }
    }

    pub fn record_operation(&mut self, execution_time_micros: u64) {
        self.operation_count += 1;
        self.total_execution_time_micros += execution_time_micros;
        self.last_update = Instant::now();
    }

    pub fn record_error(&mut self) {
        self.error_count += 1;
        self.last_update = Instant::now();
    }

    pub fn update_memory(&mut self, memory_mb: f64) {
        if memory_mb > self.peak_memory_mb {
            self.peak_memory_mb = memory_mb;
        }
        self.last_update = Instant::now();
    }

    pub fn get_average_execution_time(&self) -> f64 {
        if self.operation_count > 0 {
            self.total_execution_time_micros as f64 / self.operation_count as f64
        } else {
            0.0
        }
    }

    pub fn get_throughput_ops_per_sec(&self) -> f64 {
        let elapsed_seconds = self.last_update.elapsed().as_secs_f64();
        if elapsed_seconds > 0.0 {
            self.operation_count as f64 / elapsed_seconds
        } else {
            0.0
        }
    }

    pub fn get_error_rate(&self) -> f64 {
        if self.operation_count > 0 {
            self.error_count as f64 / self.operation_count as f64
        } else {
            0.0
        }
    }
}

impl KrakenPerformanceMonitor {
    pub fn new(monitoring_interval_secs: u64) -> Self {
        let mut system = System::new_all();
        system.refresh_all();
        
        Self {
            system: Arc::new(RwLock::new(system)),
            component_metrics: Arc::new(RwLock::new(HashMap::new())),
            monitoring_interval: Duration::from_secs(monitoring_interval_secs),
        }
    }

    /// Record operation performance for a specific component
    pub async fn record_operation(&self, component: &str, operation: &str, execution_time_micros: u64) {
        let key = format!("{}-{}", component, operation);
        let mut metrics = self.component_metrics.write().await;
        
        let component_metrics = metrics.entry(key).or_insert_with(ComponentMetrics::new);
        component_metrics.record_operation(execution_time_micros);
    }

    /// Record error for a specific component
    pub async fn record_error(&self, component: &str, operation: &str) {
        let key = format!("{}-{}", component, operation);
        let mut metrics = self.component_metrics.write().await;
        
        let component_metrics = metrics.entry(key).or_insert_with(ComponentMetrics::new);
        component_metrics.record_error();
    }

    /// Get current system CPU usage
    pub async fn get_cpu_usage(&self) -> f64 {
        let mut system = self.system.write().await;
        system.refresh_cpu_usage();
        
        // Wait a bit to get accurate CPU usage
        tokio::time::sleep(Duration::from_millis(100)).await;
        system.refresh_cpu_usage();
        
        let cpu_usage: f32 = system.cpus().iter().map(|cpu| cpu.cpu_usage()).sum();
        cpu_usage as f64 / system.cpus().len() as f64
    }

    /// Get current memory usage in MB
    pub async fn get_memory_usage_mb(&self) -> f64 {
        let mut system = self.system.write().await;
        system.refresh_memory();
        
        let used_memory = system.used_memory() as f64;
        used_memory / (1024.0 * 1024.0) // Convert from bytes to MB
    }

    /// Start background monitoring that logs performance metrics periodically
    pub async fn start_monitoring(self: Arc<Self>) {
        let monitor = Arc::clone(&self);
        
        tokio::spawn(async move {
            let mut interval = interval(monitor.monitoring_interval);
            
            loop {
                interval.tick().await;
                monitor.log_performance_metrics().await;
            }
        });
    }

    /// Log performance metrics for all components
    async fn log_performance_metrics(&self) {
        let cpu_usage = self.get_cpu_usage().await;
        let memory_usage_mb = self.get_memory_usage_mb().await;
        
        let metrics = self.component_metrics.read().await;
        
        for (component_operation, component_metrics) in metrics.iter() {
            let parts: Vec<&str> = component_operation.split('-').collect();
            if parts.len() >= 2 {
                let component = parts[0];
                let operation = parts[1..].join("-");
                
                let performance_metrics = PerformanceMetrics {
                    component: component.to_string(),
                    operation: operation,
                    execution_time_micros: component_metrics.get_average_execution_time() as u64,
                    throughput_ops_per_sec: component_metrics.get_throughput_ops_per_sec(),
                    memory_usage_mb,
                    cpu_usage_percent: cpu_usage,
                };

                let _ = KRAKEN_WEBSOCKET_LOGGER.log_performance_metrics(performance_metrics).await;
            }
        }
    }

    /// Get performance summary for a specific component
    pub async fn get_component_summary(&self, component: &str) -> Option<HashMap<String, ComponentMetrics>> {
        let metrics = self.component_metrics.read().await;
        let mut component_summary = HashMap::new();
        
        for (key, metrics) in metrics.iter() {
            if key.starts_with(component) {
                component_summary.insert(key.clone(), metrics.clone());
            }
        }
        
        if component_summary.is_empty() {
            None
        } else {
            Some(component_summary)
        }
    }

    /// Reset metrics for a specific component (useful for testing)
    pub async fn reset_component_metrics(&self, component: &str) {
        let mut metrics = self.component_metrics.write().await;
        metrics.retain(|key, _| !key.starts_with(component));
    }
}

// Convenience macros for performance monitoring
#[macro_export]
macro_rules! monitor_kraken_operation {
    ($monitor:expr, $component:expr, $operation:expr, $code:block) => {
        {
            let start = std::time::Instant::now();
            let result = $code;
            let execution_time = start.elapsed().as_micros() as u64;
            $monitor.record_operation($component, $operation, execution_time).await;
            result
        }
    };
}

#[macro_export]
macro_rules! monitor_kraken_async_operation {
    ($monitor:expr, $component:expr, $operation:expr, $future:expr) => {
        {
            let start = std::time::Instant::now();
            let result = $future.await;
            let execution_time = start.elapsed().as_micros() as u64;
            
            if result.is_err() {
                $monitor.record_error($component, $operation).await;
            } else {
                $monitor.record_operation($component, $operation, execution_time).await;
            }
            
            result
        }
    };
}

// Global performance monitor instance
use once_cell::sync::Lazy;

pub static KRAKEN_PERFORMANCE_MONITOR: Lazy<Arc<KrakenPerformanceMonitor>> = 
    Lazy::new(|| Arc::new(KrakenPerformanceMonitor::new(30))); // Monitor every 30 seconds
