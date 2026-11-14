//! Criterion: Kraken level3 update frame -> detect -> parse -> normalize ->
//! protobuf encode. No network/DB/broker. p50/p99: examples/kraken_l3_hdr.rs.
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use hostbuilder::network::handlers::models::kraken::level3_update::bench_internals::{synthetic_frames, L3Pipeline};

fn bench(c: &mut Criterion) {
    let frames = synthetic_frames(100_000);
    let p = L3Pipeline::new(&["BTC/USD", "ETH/USD"]);
    let mut i = 0usize;
    let mut g = c.benchmark_group("kraken_l3_pipeline");
    g.throughput(Throughput::Elements(1));
    g.bench_function("frame_to_encoded_publish", |b| {
        b.iter(|| { let f = &frames[i % frames.len()]; i += 1; black_box(p.process_frame(f)) })
    });
    g.finish();
}
criterion_group!(benches, bench);
criterion_main!(benches);
