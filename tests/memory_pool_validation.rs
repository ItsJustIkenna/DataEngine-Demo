//! Quick validation test for memory pool fixes

use performance::memory_pool::{BufferPool, PoolConfig};

#[test]
fn test_memory_pool_basic_functionality() {
    println!("🧪 Testing memory pool basic functionality...");
    
    // Test pool creation with default config
    let pool = BufferPool::new();
    let initial_stats = pool.get_stats();
    
    println!("   Initial pool stats:");
    println!("   - Small pool size: {}", initial_stats.small_pool_size);
    println!("   - Medium pool size: {}", initial_stats.medium_pool_size);
    println!("   - Large pool size: {}", initial_stats.large_pool_size);
    
    // Should have pre-allocated buffers
    assert!(initial_stats.small_pool_size > 0, "Should have pre-allocated small buffers");
    assert!(initial_stats.medium_pool_size > 0, "Should have pre-allocated medium buffers");
    assert!(initial_stats.large_pool_size > 0, "Should have pre-allocated large buffers");
    
    // Test buffer allocation and return
    {
        let buffer = pool.get_small_buffer();
        assert!(buffer.capacity() >= 1024, "Small buffer should have at least 1KB capacity");
        
        let stats = pool.get_stats();
        assert_eq!(stats.small_allocated, initial_stats.small_allocated + 1, "Should track allocation");
    }
    // Buffer should be returned when dropped
    
    let final_stats = pool.get_stats();
    assert_eq!(final_stats.small_returned, initial_stats.small_returned + 1, "Should track buffer return");
    
    println!("✅ Memory pool basic functionality test passed");
}

#[test]
fn test_buffer_size_classes() {
    println!("📏 Testing buffer size class selection...");
    
    let pool = BufferPool::new();
    
    // Test size class selection
    let small = pool.get_buffer_for_size(512);
    assert!(small.capacity() >= 512, "Should handle small size requests");
    assert!(small.capacity() >= 1024, "Should allocate small buffer class (1KB)");
    
    let medium = pool.get_buffer_for_size(4096);
    assert!(medium.capacity() >= 4096, "Should handle medium size requests");
    assert!(medium.capacity() >= 8192, "Should allocate medium buffer class (8KB)");
    
    let large = pool.get_buffer_for_size(32768);
    assert!(large.capacity() >= 32768, "Should handle large size requests");
    assert!(large.capacity() >= 65536, "Should allocate large buffer class (64KB)");
    
    println!("✅ Buffer size class selection test passed");
}

#[test]
fn test_custom_pool_config() {
    println!("⚙️  Testing custom pool configuration...");
    
    let custom_config = PoolConfig {
        small_buffer_size: 512,
        medium_buffer_size: 4096,
        large_buffer_size: 32768,
        pool_capacity: 50,
        prealloc_count: 25,
    };
    
    let pool = BufferPool::with_config(custom_config);
    let stats = pool.get_stats();
    
    // Should respect custom preallocation count
    assert_eq!(stats.small_pool_size, 25, "Should pre-allocate 25 small buffers");
    assert_eq!(stats.medium_pool_size, 25, "Should pre-allocate 25 medium buffers");
    assert_eq!(stats.large_pool_size, 25, "Should pre-allocate 25 large buffers");
    
    println!("✅ Custom pool configuration test passed");
}

#[test]
fn test_buffer_operations() {
    println!("🔧 Testing buffer operations...");
    
    let pool = BufferPool::new();
    let mut buffer = pool.get_medium_buffer();
    
    // Test buffer operations
    assert_eq!(buffer.len(), 0, "New buffer should be empty");
    
    buffer.resize(100, 42);
    assert_eq!(buffer.len(), 100, "Buffer should resize correctly");
    assert_eq!(buffer[0], 42, "Buffer should be filled with specified value");
    
    // Test mutable access
    buffer.as_mut_slice()[0] = 123;
    assert_eq!(buffer[0], 123, "Buffer should allow mutable access");
    
    // Test clear
    buffer.clear();
    assert_eq!(buffer.len(), 0, "Buffer should clear correctly");
    assert!(buffer.capacity() > 0, "Buffer capacity should remain");
    
    println!("✅ Buffer operations test passed");
}

#[test]
fn test_global_pool() {
    println!("🌍 Testing global pool access...");
    
    use performance::memory_pool::{initialize_global_pools, global_pool};
    
    // Initialize global pools
    let init_result = initialize_global_pools();
    assert!(init_result.is_ok(), "Global pools should initialize successfully");
    
    // Access global pool
    let pool = global_pool();
    let buffer = pool.get_small_buffer();
    
    assert!(buffer.capacity() >= 1024, "Global pool should work correctly");
    
    println!("✅ Global pool test passed");
}

#[test]
fn test_high_frequency_allocation() {
    println!("⚡ Testing high-frequency buffer allocation...");
    
    let pool = BufferPool::new();
    let iterations = 1000;
    
    let start = std::time::Instant::now();
    
    // Simulate high-frequency allocations
    for _ in 0..iterations {
        let _buffer = pool.get_small_buffer();
        // Buffer automatically returns to pool when dropped
    }
    
    let duration = start.elapsed();
    let allocations_per_sec = iterations as f64 / duration.as_secs_f64();
    
    println!("   - {} allocations in {:?}", iterations, duration);
    println!("   - Rate: {:.0} allocations/sec", allocations_per_sec);
    
    // Should be very fast
    assert!(duration.as_millis() < 100, "Should complete quickly (under 100ms)");
    assert!(allocations_per_sec > 10_000.0, "Should handle >10k allocations/sec");
    
    // Check final stats
    let stats = pool.get_stats();
    assert_eq!(stats.small_allocated, iterations as usize, "Should track all allocations");
    assert_eq!(stats.small_returned, iterations as usize, "Should track all returns");
    
    println!("✅ High-frequency allocation test passed");
}

fn main() {
    println!("🚀 Running memory pool validation tests...");
    
    test_memory_pool_basic_functionality();
    test_buffer_size_classes();
    test_custom_pool_config();
    test_buffer_operations();
    test_global_pool();
    test_high_frequency_allocation();
    
    println!("\n🎉 All memory pool tests passed! The fixes are working correctly.");
}
