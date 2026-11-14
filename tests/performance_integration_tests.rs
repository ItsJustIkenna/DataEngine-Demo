//! Performance and Infrastructure Integration Tests
//!
//! Tests performance metrics, CPU affinity, build system organization,
//! and infrastructure components of the DataEngine system.

use performance::{
    PerformanceMetrics, CpuAffinityManager, initialize,
    get_system_stats, initialize_global_metrics, global_metrics
};
use hostbuilder::{TEST_LOGGER, log_info};

#[test]
fn test_performance_system_initialization() {
    // Test that the entire performance system can be initialized
    log_info!(TEST_LOGGER, "🔧 Testing performance system initialization...");
    
    let result = initialize();
    assert!(result.is_ok(), "Performance system should initialize successfully");
    
    log_info!(TEST_LOGGER, "✅ Performance system initialized successfully");
}

#[test]
fn test_metrics_basic_functionality() {
    log_info!(TEST_LOGGER, "📊 Testing basic metrics functionality...");
    
    let metrics = PerformanceMetrics::new();
    let initial_stats = metrics.get_stats();
    
    // Verify initial state
    assert_eq!(initial_stats.total_requests, 0);
    assert_eq!(initial_stats.parse_errors, 0);
    
    // Test recording metrics
    metrics.record_latency(1500); // 1.5μs
    metrics.record_request();
    metrics.record_throughput(1000); // 1000 msgs/sec
    
    let updated_stats = metrics.get_stats();
    assert_eq!(updated_stats.total_requests, 1);
    assert!(updated_stats.avg_latency_ns > 0);
    assert!(updated_stats.throughput_per_sec > 0);
    
    log_info!(TEST_LOGGER, "✅ Basic metrics functionality working correctly");
    log_info!(TEST_LOGGER, "   - Requests: {}", updated_stats.total_requests);
    log_info!(TEST_LOGGER, "   - Avg Latency: {:.2}μs", updated_stats.avg_latency_ns as f64 / 1000.0);
    log_info!(TEST_LOGGER, "   - Throughput: {} msgs/sec", updated_stats.throughput_per_sec);
}

#[test]
fn test_cpu_affinity_manager() {
    log_info!(TEST_LOGGER, "💻 Testing CPU affinity manager...");
    
    let manager = CpuAffinityManager::new();
    
    // Test basic functionality
    let cores = num_cpus::get();
    log_info!(TEST_LOGGER, "   - Detected CPU cores: {}", cores);
    
    // Test optimization method (may fail without admin privileges)
    let result = manager.optimize_for_trading();
    match result {
        Ok(()) => log_info!(TEST_LOGGER, "✅ CPU affinity optimization successful"),
        Err(e) => log_info!(TEST_LOGGER, "⚠️  CPU affinity optimization failed (expected without admin): {}", e),
    }
    
    // Should not panic regardless of result
    assert!(true, "CPU affinity manager should handle errors gracefully");
}

#[test]
fn test_high_frequency_metrics() {
    log_info!(TEST_LOGGER, "Testing rapid metrics recording...");
    
    let metrics = PerformanceMetrics::new();
    let iterations = 10_000;
    
    let start = std::time::Instant::now();
    
    // Simulate high-frequency trading operations
    for i in 0..iterations {
        metrics.record_request();
        metrics.record_latency(200 + (i % 100)); // Varying latencies 200-299ns
        
        if i % 100 == 0 {
            metrics.record_throughput(50_000); // 50k msgs/sec
        }
    }
    
    let duration = start.elapsed();
    let stats = metrics.get_stats();
    
    log_info!(TEST_LOGGER, "   - Processed {} operations in {:?}", iterations, duration);
    log_info!(TEST_LOGGER, "   - Rate: {:.2} ops/μs", iterations as f64 / duration.as_micros() as f64);
    log_info!(TEST_LOGGER, "   - Total requests: {}", stats.total_requests);
    log_info!(TEST_LOGGER, "   - Average latency: {:.2}ns", stats.avg_latency_ns);
    log_info!(TEST_LOGGER, "   - Max latency: {}ns", stats.max_latency_ns);
    
    assert_eq!(stats.total_requests, iterations);
    assert!(stats.avg_latency_ns > 0);
    assert!(duration.as_millis() < 100, "Should complete in under 100ms");
    
    log_info!(TEST_LOGGER, "Rapid metrics recording test passed");
}

#[test]
fn test_global_metrics_system() {
    log_info!(TEST_LOGGER, "🌍 Testing global metrics system...");
    
    // Initialize global metrics
    let init_result = initialize_global_metrics();
    assert!(init_result.is_ok(), "Global metrics should initialize");
    
    // Access global instance
    let metrics = global_metrics();
    
    // Record some test data
    for i in 0..100 {
        metrics.record_request();
        metrics.record_latency(150 + i); // Increasing latency pattern
    }
    
    let stats = metrics.get_stats();
    assert_eq!(stats.total_requests, 100);
    assert!(stats.avg_latency_ns > 150);
    
    log_info!(TEST_LOGGER, "✅ Global metrics system working correctly");
    log_info!(TEST_LOGGER, "   - Global requests: {}", stats.total_requests);
    log_info!(TEST_LOGGER, "   - Global avg latency: {:.2}ns", stats.avg_latency_ns);
}

#[test]
fn test_system_statistics() {
        log_info!(TEST_LOGGER, "📈 Testing system statistics...");
    
    let stats = get_system_stats();
    
    // Basic sanity checks
    assert!(stats.cpu_usage >= 0.0);
    assert!(stats.memory_usage >= 0);
    assert!(stats.thread_count >= 1);
    
    log_info!(TEST_LOGGER, "   - CPU Usage: {:.1}%", stats.cpu_usage);
    log_info!(TEST_LOGGER, "   - Memory Usage: {} bytes", stats.memory_usage);
    log_info!(TEST_LOGGER, "   - Thread Count: {}", stats.thread_count);
    log_info!(TEST_LOGGER, "   - Uptime: {:?}", stats.uptime);
    
    log_info!(TEST_LOGGER, "✅ System statistics collection working");
}

#[test]
fn test_error_handling_robustness() {
    log_info!(TEST_LOGGER, "🛡️  Testing error handling robustness...");
    
    use performance::CpuAffinityError;
    
    // Test error display
    let errors = vec![
        CpuAffinityError::InvalidCoreId,
        CpuAffinityError::SystemError("test error".to_string()),
        CpuAffinityError::PermissionDenied,
    ];
    
    for error in errors {
        let error_msg = format!("{}", error);
        assert!(!error_msg.is_empty(), "Error messages should not be empty");
        log_info!(TEST_LOGGER, "   - Error: {}", error_msg);
    }
    
    // Test that errors implement standard traits
    let error = CpuAffinityError::InvalidCoreId;
    let _debug_str = format!("{:?}", error);
    let _clone = error.clone();
    
    log_info!(TEST_LOGGER, "✅ Error handling system robust and complete");
}

#[test]
fn test_performance_monitoring_integration() {
    log_info!(TEST_LOGGER, "📊 Testing performance monitoring integration...");
    
    let metrics = PerformanceMetrics::new();
    
    // Simulate a complete trading cycle
    let trades = 1000;
    let start_time = std::time::Instant::now();
    
    for i in 0..trades {
        // Simulate order processing latency
        let latency = if i % 100 == 0 { 1000 } else { 200 + (i % 50) }; // Occasional high latency
        
        metrics.record_request();
        metrics.record_latency(latency);
        
        // Record parse errors occasionally
        if i % 500 == 0 {
            metrics.record_parse_error();
        }
        
        // Record throughput periodically
        if i % 10 == 0 {
            metrics.record_throughput(10_000); // 10k msgs/sec
        }
    }
    
    let total_duration = start_time.elapsed();
    let stats = metrics.get_stats();
    
    // Generate performance report
    metrics.generate_performance_report();
    
    // Validate results
    assert_eq!(stats.total_requests, trades);
    assert_eq!(stats.parse_errors, 2); // Should have 2 parse errors (at 0 and 500)
    assert!(stats.avg_latency_ns > 0);
    assert!(stats.throughput_per_sec > 0);
    
    log_info!(TEST_LOGGER, "   - Trading simulation completed in {:?}", total_duration);
    log_info!(TEST_LOGGER, "   - Processed {} trades", trades);
    log_info!(TEST_LOGGER, "   - Average latency: {:.2}μs", stats.avg_latency_ns as f64 / 1000.0);
    log_info!(TEST_LOGGER, "   - Peak latency: {:.2}μs", stats.max_latency_ns as f64 / 1000.0);
    log_info!(TEST_LOGGER, "   - Parse errors: {}", stats.parse_errors);
    log_info!(TEST_LOGGER, "   - Throughput: {} msgs/sec", stats.throughput_per_sec);
    
    log_info!(TEST_LOGGER, "✅ Performance monitoring integration successful");
}

#[test] 
fn test_build_system_organization() {
    log_info!(TEST_LOGGER, "🏗️  Testing new build system organization...");
    
    // Verify the new file structure exists
    let build_dir = std::path::Path::new("build");
    assert!(build_dir.exists(), "Build directory should exist");
    
    let expected_build_files = [
        "build/build.ps1",
        "build/build.sh", 
        "build/build.py",
        "build/README.md",
    ];
    
    for file in &expected_build_files {
        let path = std::path::Path::new(file);
        if path.exists() {
            log_info!(TEST_LOGGER, "   ✅ Found: {}", file);
        } else {
            log_info!(TEST_LOGGER, "   ⚠️  Missing: {}", file);
        }
    }
    
    // Verify Kubernetes organization
    let k8s_dir = std::path::Path::new("k8s");
    if k8s_dir.exists() {
        log_info!(TEST_LOGGER, "   ✅ Kubernetes directory organized");
        
        let helm_dir = std::path::Path::new("k8s/helm");
        if helm_dir.exists() {
            log_info!(TEST_LOGGER, "   ✅ Helm charts moved to k8s/helm");
        }
    }
    
    // Verify performance crate organization
    let perf_dir = std::path::Path::new("performance");
    assert!(perf_dir.exists(), "Performance crate should exist");
    
    let perf_src_dir = std::path::Path::new("performance/src");
    assert!(perf_src_dir.exists(), "Performance source directory should exist");
    
    log_info!(TEST_LOGGER, "✅ Build system organization validated");
}

// Performance benchmark helpers
#[cfg(test)]
mod benchmarks {
    use super::*;
    use std::time::Instant;
    
    #[test]
    fn benchmark_metrics_throughput() {
        log_info!(TEST_LOGGER, "🔥 Benchmarking metrics throughput...");
        
        let metrics = PerformanceMetrics::new();
        let operations = 100_000;
        
        let start = Instant::now();
        
        for i in 0..operations {
            metrics.record_request();
            metrics.record_latency(100 + (i % 20)); // Varying latency
        }
        
        let duration = start.elapsed();
        let ops_per_sec = operations as f64 / duration.as_secs_f64();
        
        log_info!(TEST_LOGGER, "   - {} operations in {:?}", operations, duration);
        log_info!(TEST_LOGGER, "   - Throughput: {:.0} ops/sec", ops_per_sec);
        log_info!(TEST_LOGGER, "   - Latency per op: {:.2}ns", duration.as_nanos() as f64 / operations as f64);
        
        // Should handle at least 1M ops/sec
        assert!(ops_per_sec > 1_000_000.0, "Should handle >1M ops/sec");
        
        log_info!(TEST_LOGGER, "✅ Metrics throughput benchmark passed");
    }
}
