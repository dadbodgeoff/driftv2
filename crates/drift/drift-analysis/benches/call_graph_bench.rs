//! Call graph benchmarks — build + BFS traversal.

use criterion::{criterion_group, criterion_main, Criterion};

fn call_graph_build_benchmark(c: &mut Criterion) {
    c.bench_function("call_graph_build_empty", |b| {
        b.iter(|| {
            // Placeholder — will be filled when call_graph module is complete
            std::hint::black_box(42)
        });
    });
}

fn call_graph_bfs_benchmark(c: &mut Criterion) {
    c.bench_function("call_graph_bfs_empty", |b| {
        b.iter(|| {
            std::hint::black_box(42)
        });
    });
}

criterion_group!(benches, call_graph_build_benchmark, call_graph_bfs_benchmark);
criterion_main!(benches);
