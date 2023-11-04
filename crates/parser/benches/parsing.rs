use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use spanned_json_parser::parse;
use std::fs;

fn parse_benchmark(c: &mut Criterion) {
    let paths = [
        "./benches/data/twitter.json",
        "./benches/data/citm_catalog.json",
        "./benches/data/canada.json",
    ];
    let mut group = c.benchmark_group("Parser");

    group.sample_size(10);

    for path in paths {
        let json = fs::read_to_string(path).unwrap();

        group.throughput(Throughput::Bytes(json.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(path), &json, |b, data| {
            b.iter(|| {
                let _ = parse(black_box(data)).unwrap();
            })
        });
    }
}

criterion_group!(benches, parse_benchmark);
criterion_main!(benches);
