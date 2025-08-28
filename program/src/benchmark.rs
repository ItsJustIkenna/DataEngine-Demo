use anyhow::Result;
use chrono::{DateTime, Utc};
use fastrand;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use tracing::{info, warn, Level};
use tracing_subscriber;

/// Comprehensive benchmark suite for DataEngine
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    info!("🚀 DataEngine Performance Benchmark Suite v0.1.0");
    info!("==================================================");

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let mut messages = 100_000_u64;
    let mut clients = 10_u32;
    let mut duration_secs = 60_u64;
    let mut latency_test = false;
    let mut platform_test = false;

    // Simple argument parsing
    for (i, arg) in args.iter().enumerate() {
        match arg.as_str() {
            "--messages" => {
                if i + 1 < args.len() {
                    messages = args[i + 1].parse().unwrap_or(100_000);
                }
            }
            "--clients" => {
                if i + 1 < args.len() {
                    clients = args[i + 1].parse().unwrap_or(10);
                }
            }
            "--duration" => {
                if i + 1 < args.len() {
                    let duration_str = &args[i + 1];
                    if duration_str.ends_with('s') {
                        duration_secs = duration_str.trim_end_matches('s').parse().unwrap_or(60);
                    } else {
                        duration_secs = duration_str.parse().unwrap_or(60);
                    }
                }
            }
            "--latency-test" => latency_test = true,
            "--platform-test" => platform_test = true,
            _ => {}
        }
    }

    info!("Benchmark Configuration:");
    info!("  Messages: {}", messages);
    info!("  Clients: {}", clients);
    info!("  Duration: {}s", duration_secs);
    info!("  Latency Test: {}", latency_test);
    info!("  Platform Test: {}", platform_test);
    info!("");

    // Run benchmarks
    if platform_test {
        run_platform_test().await?;
    } else if latency_test {
        run_latency_benchmark(duration_secs).await?;
    } else {
        run_throughput_benchmark(messages, clients).await?;
    }

    Ok(())
}

/// Platform compatibility and performance test
async fn run_platform_test() -> Result<()> {
    info!("🏁 Platform Performance Test");
    info!("============================");

    // System information
    let num_cpus = num_cpus::get();
    info!("CPU Cores: {}", num_cpus);
    
    // Memory allocation test
    let start = Instant::now();
    let mut buffers = Vec::new();
    for _ in 0..1000 {
        let buffer = vec![0u8; 1024 * 1024]; // 1MB buffers
        buffers.push(buffer);
    }
    let alloc_time = start.elapsed();
    info!("Memory Allocation (1GB): {:?}", alloc_time);

    // Buffer clearing
    buffers.clear();

    // CPU-intensive computation
    let start = Instant::now();
    let mut result = 0u64;
    for i in 0..10_000_000 {
        result = result.wrapping_add(i * 7 + 13);
    }
    let cpu_time = start.elapsed();
    info!("CPU Computation (10M ops): {:?} (result: {})", cpu_time, result);

    // Async task spawning
    let start = Instant::now();
    let mut handles = Vec::new();
    for i in 0..1000 {
        let handle = tokio::spawn(async move {
            sleep(Duration::from_millis(1)).await;
            i * 2
        });
        handles.push(handle);
    }
    
    let mut results = Vec::new();
    for handle in handles {
        results.push(handle.await?);
    }
    let async_time = start.elapsed();
    info!("Async Tasks (1000): {:?}", async_time);

    info!("✅ Platform test completed successfully");
    Ok(())
}

/// Latency-focused benchmark
async fn run_latency_benchmark(duration_secs: u64) -> Result<()> {
    info!("⚡ Ultra-Low Latency Benchmark");
    info!("==============================");

    let mut latencies = Vec::new();
    let end_time = Instant::now() + Duration::from_secs(duration_secs);
    let mut operations = 0u64;

    while Instant::now() < end_time {
        let start = Instant::now();
        
        // Simulate message processing
        let _data = simulate_message_processing().await;
        
        let latency = start.elapsed();
        latencies.push(latency);
        operations += 1;

        if operations % 10000 == 0 {
            info!("Processed {} operations...", operations);
        }
    }

    // Calculate statistics
    latencies.sort();
    let len = latencies.len();
    
    let min = latencies[0];
    let p50 = latencies[len / 2];
    let p90 = latencies[(len * 90) / 100];
    let p99 = latencies[(len * 99) / 100];
    let p999 = latencies[(len * 999) / 1000];
    let max = latencies[len - 1];
    
    let avg_nanos: u128 = latencies.iter().map(|d| d.as_nanos()).sum::<u128>() / len as u128;

    info!("");
    info!("🔥 LATENCY BENCHMARK RESULTS");
    info!("===========================");
    info!("Total Operations: {}", operations);
    info!("Operations/sec: {:.2}", operations as f64 / duration_secs as f64);
    info!("");
    info!("Latency Distribution:");
    info!("  Min:    {:>8.0} ns", min.as_nanos());
    info!("  P50:    {:>8.0} ns", p50.as_nanos());
    info!("  P90:    {:>8.0} ns", p90.as_nanos());
    info!("  P99:    {:>8.0} ns", p99.as_nanos());
    info!("  P99.9:  {:>8.0} ns", p999.as_nanos());
    info!("  Max:    {:>8.0} ns", max.as_nanos());
    info!("  Avg:    {:>8.0} ns", avg_nanos);
    info!("");

    // Performance score
    let score = calculate_performance_score(avg_nanos);
    info!("🏆 Performance Score: {:.1}/10.0 - {}", score.0, score.1);

    Ok(())
}

/// Throughput-focused benchmark
async fn run_throughput_benchmark(total_messages: u64, num_clients: u32) -> Result<()> {
    info!("🚀 High-Throughput Benchmark");
    info!("============================");

    let messages_per_client = total_messages / num_clients as u64;
    let start_time = Instant::now();

    // Spawn client tasks
    let mut handles = Vec::new();
    for client_id in 0..num_clients {
        let handle = tokio::spawn(async move {
            let mut processed = 0u64;
            for i in 0..messages_per_client {
                let _result = simulate_message_processing().await;
                processed += 1;

                if processed % 10000 == 0 {
                    info!("Client {}: Processed {} messages", client_id, processed);
                }
            }
            processed
        });
        handles.push(handle);
    }

    // Wait for all clients to complete
    let mut total_processed = 0u64;
    for handle in handles {
        total_processed += handle.await?;
    }

    let total_time = start_time.elapsed();
    let throughput = total_processed as f64 / total_time.as_secs_f64();

    info!("");
    info!("📊 THROUGHPUT BENCHMARK RESULTS");
    info!("===============================");
    info!("Total Messages: {}", total_processed);
    info!("Total Time: {:.2} seconds", total_time.as_secs_f64());
    info!("Throughput: {:.0} messages/second", throughput);
    info!("Average Latency: {:.2} μs/message", 1_000_000.0 / throughput);
    info!("");

    // Performance classification
    let performance_class = if throughput > 1_000_000.0 {
        "🏆 ULTRA-HIGH PERFORMANCE"
    } else if throughput > 100_000.0 {
        "🚀 HIGH PERFORMANCE"  
    } else if throughput > 10_000.0 {
        "⚡ GOOD PERFORMANCE"
    } else {
        "📈 BASELINE PERFORMANCE"
    };

    info!("Performance Classification: {}", performance_class);

    Ok(())
}

/// Simulate message processing workload
async fn simulate_message_processing() -> String {
    // Simulate JSON parsing and processing
    let data = serde_json::json!({
        "symbol": "BTC/USD",
        "price": fastrand::f64() * 50000.0 + 40000.0,
        "quantity": fastrand::f64() * 10.0,
        "timestamp": Utc::now().timestamp_millis(),
        "side": if fastrand::bool() { "buy" } else { "sell" }
    });

    // Simulate some CPU work
    let mut hash = 0u64;
    let s = data.to_string();
    for byte in s.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
    }

    format!("processed:{}", hash)
}

/// Calculate performance score based on average latency
fn calculate_performance_score(avg_latency_nanos: u128) -> (f64, &'static str) {
    match avg_latency_nanos {
        0..=1000 => (10.0, "EXCEPTIONAL"),      // <1μs
        1001..=5000 => (9.5, "OUTSTANDING"),   // 1-5μs  
        5001..=10000 => (9.0, "EXCELLENT"),    // 5-10μs
        10001..=50000 => (8.5, "VERY GOOD"),   // 10-50μs
        50001..=100000 => (8.0, "GOOD"),       // 50-100μs
        100001..=500000 => (7.0, "ACCEPTABLE"), // 100-500μs
        500001..=1000000 => (6.0, "FAIR"),     // 0.5-1ms
        _ => (5.0, "NEEDS IMPROVEMENT"),       // >1ms
    }
}
