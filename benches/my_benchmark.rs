use bagextract::parse_num::Postcodes;
use criterion::{criterion_group, criterion_main, Criterion};

const INPUT: &str = include_str!("/home/folkertdev/Downloads/inspire/num_01.xml");

fn parse_input_new() -> Postcodes {
    bagextract::parse_num::parse_manual_str(INPUT).unwrap()
}

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("new", |b| b.iter(parse_input_new));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
