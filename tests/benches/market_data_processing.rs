//! Market Data Processing Benchmarks
//! Measures performance of critical market data processing paths

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use hostbuilder::data::caching::{MarketDataCache, CachedOrderBook, CachedTrade};
use std::time::Duration;
use fake::{Fake, Faker};
use chrono::Utc;

fn generate_test_orderbook() -> CachedOrderBook {
    CachedOrderBook {
        symbol: "BTC/USD".to_string(),
        timestamp: Utc::now(),
        bids: (0..100).map(|i| (45000.0 - i as f64, (i as f64) * 0.1)).collect(),
        asks: (0..100).map(|i| (45001.0 + i as f64, (i as f64) * 0.1)).collect(),
        last_updated: Utc::now(),
    }
}

fn generate_test_trade() -> CachedTrade {
    CachedTrade {
        symbol: "BTC/USD".to_string(),
        timestamp: Utc::now(),
        price: 45000.5,
        volume: 1.5,
        side: "buy".to_string(),
        trade_id: Faker.fake::<String>(),
    }
}

fn bench_orderbook_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("orderbook_processing");
    
    for depth in [10, 50, 100, 500, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("update_orderbook", depth),
            depth,
            |b, &depth| {
                let cache = MarketDataCache::new(1000);
                let orderbook = generate_test_orderbook();
                
                b.iter(|| {
                    // Benchmark orderbook update performance
                    black_box(cache.update_orderbook(black_box(orderbook.clone())));
                });
            },
        );
    }
    
    group.finish();
}

fn bench_trade_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("trade_processing");
    
    for batch_size in [1, 10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("process_trades", batch_size),
            batch_size,
            |b, &batch_size| {
                let cache = MarketDataCache::new(10000);
                let trades: Vec<CachedTrade> = (0..*batch_size)
                    .map(|_| generate_test_trade())
                    .collect();
                
                b.iter(|| {
                    for trade in &trades {
                        black_box(cache.add_trade(black_box(trade.clone())));
                    }
                });
            },
        );
    }
    
    group.finish();
}

fn bench_cache_lookup_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_lookup");
    
    // Pre-populate cache
    let cache = MarketDataCache::new(10000);
    for i in 0..1000 {
        let orderbook = generate_test_orderbook();
        cache.update_orderbook(orderbook);
    }
    
    group.bench_function("get_latest_orderbook", |b| {
        b.iter(|| {
            black_box(cache.get_latest_orderbook(black_box("BTC/USD")));
        });
    });
    
    group.bench_function("get_recent_trades", |b| {
        b.iter(|| {
            black_box(cache.get_recent_trades(black_box("BTC/USD"), black_box(100)));
        });
    });
    
    group.finish();
}

fn bench_concurrent_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_access");
    
    group.bench_function("concurrent_updates", |b| {
        use std::sync::Arc;
        use tokio::runtime::Runtime;
        
        let rt = Runtime::new().unwrap();
        let cache = Arc::new(MarketDataCache::new(10000));
        
        b.to_async(&rt).iter(|| {
            let cache = cache.clone();
            async move {
                let tasks: Vec<_> = (0..10).map(|_| {
                    let cache = cache.clone();
                    tokio::spawn(async move {
                        let orderbook = generate_test_orderbook();
                        cache.update_orderbook(orderbook);
                    })
                }).collect();
                
                for task in tasks {
                    task.await.unwrap();
                }
            }
        });
    });
    
    group.finish();
}

criterion_group!(
    benches,
    bench_orderbook_processing,
    bench_trade_processing,
    bench_cache_lookup_performance,
    bench_concurrent_access
);
criterion_main!(benches);