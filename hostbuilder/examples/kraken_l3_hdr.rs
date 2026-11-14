//! Per-frame latency histogram + single-thread throughput, Kraken level3 hot path.
//! cargo run --release -p hostbuilder --features bench-internals --example kraken_l3_hdr -- [frames]
use hdrhistogram::Histogram;
use hostbuilder::network::handlers::models::kraken::level3_update::bench_internals::{synthetic_frames, L3Pipeline};
use std::time::Instant;

fn main() {
    let n: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(1_000_000);
    let frames = synthetic_frames(n);
    let p = L3Pipeline::new(&["BTC/USD", "ETH/USD"]);
    let (mut orders, mut bytes) = (0usize, 0usize);
    for f in frames.iter().take(n / 10) { let r = p.process_frame(f).expect("frame"); bytes += r.bytes.len(); }
    let mut h = Histogram::<u64>::new_with_bounds(1, 10_000_000_000, 3).unwrap();
    let t0 = Instant::now();
    for f in &frames {
        let t = Instant::now();
        let r = p.process_frame(f).expect("frame");
        h.record(t.elapsed().as_nanos() as u64).unwrap();
        orders += r.orders; bytes += r.bytes.len();
    }
    let el = t0.elapsed().as_secs_f64();
    let t1 = Instant::now();
    for f in &frames { bytes += p.process_frame(f).map(|r| r.bytes.len()).unwrap_or(0); }
    let el2 = t1.elapsed().as_secs_f64();
    eprintln!(
        "RESULT kraken_l3 frames={} order_events={} p50_ns={} p90_ns={} p99_ns={} p999_ns={} max_ns={} mean_ns={:.0} timed_rate_frames_per_s={:.0} untimed_rate_frames_per_s={:.0} (bytes={})",
        n, orders, h.value_at_quantile(0.5), h.value_at_quantile(0.9), h.value_at_quantile(0.99), h.value_at_quantile(0.999),
        h.max(), h.mean(), n as f64 / el, n as f64 / el2, bytes
    );
}
