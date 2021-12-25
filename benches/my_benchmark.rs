use bagextract::parse_num::Postcodes;
use bagextract::parse_vbo::Verblijfsobjecten;
use criterion::{criterion_group, criterion_main, Criterion};

const NUM_INPUT: &str = include_str!("/home/folkertdev/Downloads/inspire/num_01.xml");
const VBO_INPUT: &str = include_str!("/home/folkertdev/Downloads/inspire/vbo_01.xml");

fn parse_num_input_new() -> Postcodes {
    bagextract::parse_num::parse_manual_str(NUM_INPUT).unwrap()
}

fn parse_vbo_input_new() -> Verblijfsobjecten {
    bagextract::parse_vbo::parse_manual_str(VBO_INPUT).unwrap()
}

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("new_num", |b| b.iter(parse_num_input_new));
    c.bench_function("new_vbo", |b| b.iter(parse_vbo_input_new));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
