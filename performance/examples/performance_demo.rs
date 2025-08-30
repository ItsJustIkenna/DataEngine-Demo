use performance::{
    initialize, create_latency_timer, 
    trading_perf::{measure_order_processing, warn_if_slow},
    memory_utils::TradingBufferPool,
    resource_monitor::{ResourceMonitor, PerformanceAlerter}
};
use hostbuilder::logging_facade::{PERFORMANCE_LOGGER, log_info, log_warn, log_error};
use std::thread;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log_info!(PERFORMANCE_LOGGER, "🚀 DataEngine Performance Demo");
    
    // Initialize performance systems
    if let Err(e) = initialize() {
        log_warn!(PERFORMANCE_LOGGER, "⚠️  Performance initialization warning: {}", e);
    } else {
        log_info!(PERFORMANCE_LOGGER, "✅ Performance systems initialized");
    }
    
    // Demo 1: High-precision timing
    log_info!(PERFORMANCE_LOGGER, "\n📊 High-Precision Timing Demo");
    let timer = create_latency_timer();
    
    // Simulate some work
    thread::sleep(Duration::from_micros(100));
    
    log_info!(PERFORMANCE_LOGGER, "Operation completed in {} nanoseconds", timer.elapsed_nanos());
    log_info!(PERFORMANCE_LOGGER, "Operation completed in {} microseconds", timer.elapsed_micros());
    
    // Demo 2: Trading-specific performance measurement
    log_info!(PERFORMANCE_LOGGER, "\n📈 Trading Performance Measurement");
    let (result, latency_nanos) = measure_order_processing(|| {
        // Simulate order processing
        thread::sleep(Duration::from_micros(50));
        "Order processed successfully"
    });
    
    log_info!(PERFORMANCE_LOGGER, "Order processing result: {}", result);
    log_info!(PERFORMANCE_LOGGER, "Order processing latency: {} nanoseconds", latency_nanos);
    
    // Demo 3: Performance warning system
    log_info!(PERFORMANCE_LOGGER, "\n⚠️  Performance Warning System");
    let slow_result = warn_if_slow(|| {
        // Simulate slow operation
        thread::sleep(Duration::from_millis(2));
        "Slow operation completed"
    }, 1000, "slow_operation"); // 1ms threshold
    
    log_info!(PERFORMANCE_LOGGER, "Slow operation result: {}", slow_result);
    
    // Demo 4: Memory buffer pool
    log_info!(PERFORMANCE_LOGGER, "\n💾 Memory Buffer Pool Demo");
    let mut buffer_pool = TradingBufferPool::new(10, 5);
    
    if let Some(mut buffer) = buffer_pool.get_order_buffer() {
        buffer.extend_from_slice(b"Sample order data");
        log_info!(PERFORMANCE_LOGGER, "Used order buffer with {} bytes", buffer.len());
        buffer_pool.return_order_buffer(buffer);
        log_info!(PERFORMANCE_LOGGER, "Buffer returned to pool");
    }
    
    // Demo 5: Resource monitoring
    log_info!(PERFORMANCE_LOGGER, "\n📊 Resource Monitoring Demo");
    let mut resource_monitor = ResourceMonitor::new(100);
    let mut alerter = PerformanceAlerter::new(80.0, 90.0, 1000, 5); // 5-second cooldown
    
    // Take a few snapshots
    for i in 0..3 {
        let snapshot = resource_monitor.take_snapshot();
        log_info!(PERFORMANCE_LOGGER, "Snapshot {}: CPU: {:.2}%, Memory: {:.2}%", 
                 i + 1, 
                 snapshot.cpu_usage_percent, 
                 snapshot.memory_utilization_percent());
        
        // Check for alerts (this would normally use real system data)
        if alerter.should_alert_latency(2000) { // 2ms latency
            log_warn!(PERFORMANCE_LOGGER, "🚨 High latency alert triggered!");
        }
        
        thread::sleep(Duration::from_millis(100));
    }
    
    log_info!(PERFORMANCE_LOGGER, "Average CPU usage: {:.2}%", resource_monitor.avg_cpu_usage(3));
    
    if resource_monitor.is_system_under_pressure(50.0, 80.0) {
        log_warn!(PERFORMANCE_LOGGER, "⚠️  System is under pressure!");
    } else {
        log_info!(PERFORMANCE_LOGGER, "✅ System performance is healthy");
    }
    
    log_info!(PERFORMANCE_LOGGER, "\n🎉 Performance demo completed successfully!");
    
    // Ensure logging is flushed before exit
    PERFORMANCE_LOGGER.flush().await?;
    
    Ok(())
}
