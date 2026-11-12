extern crate htmd;

mod common;

use std::{hint::black_box, time::Duration};

use common::{generate_deep_nested_html, generate_synthetic_html};
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use htmd::convert;

fn benchmark(c: &mut Criterion) {
    c.bench_function("convert()", |bencher| {
        let path = "examples/page-to-markdown/html/Elon Musk - Wikipedia.html";
        let html = std::fs::read_to_string(path).unwrap();
        bencher.iter(|| convert(black_box(&html)).unwrap())
    });
}

fn benchmark_synthetic(c: &mut Criterion) {
    const KB: usize = 1024;
    const MB: usize = 1024 * 1024;

    let cases = [
        ("50KB", generate_synthetic_html(50 * KB)),
        ("100KB", generate_synthetic_html(100 * KB)),
        ("500KB", generate_synthetic_html(500 * KB)),
        ("1MB", generate_synthetic_html(MB)),
        ("2MB", generate_synthetic_html(2 * MB)),
        ("5MB", generate_synthetic_html(5 * MB)),
        ("deep_nested", generate_deep_nested_html(500)),
    ];

    let mut group = c.benchmark_group("convert_synthetic");
    for (name, html) in &cases {
        group.throughput(Throughput::Bytes(html.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(name), html, |bencher, html| {
            bencher.iter(|| convert(black_box(html)).unwrap())
        });
    }
    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(10))
        .measurement_time(Duration::from_secs(30))
        .sample_size(200);
    targets = benchmark
);
criterion_group!(
    name = synthetic_benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_secs(3))
        .measurement_time(Duration::from_secs(10))
        .sample_size(50);
    targets = benchmark_synthetic
);
criterion_main!(benches, synthetic_benches);
