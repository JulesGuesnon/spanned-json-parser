use criterion::{black_box, criterion_group, criterion_main, Criterion};
use spanned_json_parser::parse;
use std::fs;

fn parse_benchmark(c: &mut Criterion) {
    let json = fs::read_to_string("./benches/data/twitter.json").unwrap();

    c.bench_function("parse twitter.json", |b| b.iter(|| parse(black_box(&json))));

    drop(json);

    let json = fs::read_to_string("./benches/data/citm_catalog.json").unwrap();

    c.bench_function("parse citm_catalog.json", |b| {
        b.iter(|| parse(black_box(&json)))
    });
}

criterion_group!(benches, parse_benchmark);
criterion_main!(benches);
