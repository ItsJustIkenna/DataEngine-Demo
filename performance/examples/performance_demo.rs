use performance::{
    initialize, create_latency_timer, 
    trading_perf::{measure_order_processing, warn_if_slow},
    memory_utils::TradingBufferPool,
    resource_monitor::{ResourceMonitor, PerformanceAlerter}
};
use std::thread;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 DataEngine Performance Demo");
    
    // Initialize performance systems
    if let Err(e) = initialize() {
        eprintln!("⚠️  Performance initialization warning: {}", e);
    } else {
        println!("✅ Performance systems initialized");
    }
    
    // Demo 1: High-precision timing
    println!("\n📊 High-Precision Timing Demo");
    let timer = create_latency_timer();
    
    // Simulate some work
    thread::sleep(Duration::from_micros(100));
    
    println!("Operation completed in {} nanoseconds", timer.elapsed_nanos());
    println!("Operation completed in {} microseconds", timer.elapsed_micros());
    
    // Demo 2: Trading-specific performance measurement
    println!("\n📈 Trading Performance Measurement");
    let (result, latency_nanos) = measure_order_processing(|| {
        // Simulate order processing
        thread::sleep(Duration::from_micros(50));
        "Order processed successfully"
    });
    
    println!("Order processing result: {}", result);
    println!("Order processing latency: {} nanoseconds", latency_nanos);
    
    // Demo 3: Performance warning system
    println!("\n⚠️  Performance Warning System");
    let slow_result = warn_if_slow(|| {
        // Simulate slow operation
        thread::sleep(Duration::from_millis(2));
        "Slow operation completed"
    }, 1000, "slow_operation"); // 1ms threshold
    
    println!("Slow operation result: {}", slow_result);
    
    // Demo 4: Memory buffer pool
    println!("\n💾 Memory Buffer Pool Demo");
    let mut buffer_pool = TradingBufferPool::new(10, 5);
    
    if let Some(mut buffer) = buffer_pool.get_order_buffer() {
        buffer.extend_from_slice(b"Sample order data");
        println!("Used order buffer with {} bytes", buffer.len());
        buffer_pool.return_order_buffer(buffer);
        println!("Buffer returned to pool");
    }
    
    // Demo 5: Resource monitoring
    println!("\n📊 Resource Monitoring Demo");
    let mut resource_monitor = ResourceMonitor::new(100);
    let mut alerter = PerformanceAlerter::new(80.0, 90.0, 1000, 5); // 5-second cooldown
    
    // Take a few snapshots
    for i in 0..3 {
        let snapshot = resource_monitor.take_snapshot();
        println!("Snapshot {}: CPU: {:.2}%, Memory: {:.2}%", 
                 i + 1, 
                 snapshot.cpu_usage_percent, 
                 snapshot.memory_utilization_percent());
        
        // Check for alerts (this would normally use real system data)
        if alerter.should_alert_latency(2000) { // 2ms latency
            println!("🚨 High latency alert triggered!");
        }
        
        thread::sleep(Duration::from_millis(100));
    }
    
    println!("Average CPU usage: {:.2}%", resource_monitor.avg_cpu_usage(3));
    
    if resource_monitor.is_system_under_pressure(50.0, 80.0) {
        println!("⚠️  System is under pressure!");
    } else {
        println!("✅ System performance is healthy");
    }
    
    println!("\n🎉 Performance demo completed successfully!");
    Ok(())
}
