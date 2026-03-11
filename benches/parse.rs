#![cfg(not(miri))] // Miri is too slow for these benchmarks

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use locat::parse;

const DECIMAL_DEG: &str = "+40.7128-074.0060/";
const DEG_MIN: &str = "+4042.7700-07400.3600/";
const DMS: &str = "+404243.123000-0740002.456000/";
const DMS_INTEGER: &str = "+404243-0740002/";
const WITH_ALT: &str = "+27.5916+086.5640+8848/";
const WITH_ALT_CRS: &str = "+27.5916+086.5640+8848CRSepsg4326/";
const MINIMAL: &str = "+00+000/";

static INPUTS: &[(&str, &str)] = &[
  ("decimal_deg", DECIMAL_DEG),
  ("deg_min", DEG_MIN),
  ("dms", DMS),
  ("dms_integer", DMS_INTEGER),
  ("with_alt", WITH_ALT),
  ("with_alt_crs", WITH_ALT_CRS),
  ("minimal", MINIMAL),
];

fn bench_parse(c: &mut Criterion) {
  let mut group = c.benchmark_group("parse");

  for &(name, input) in INPUTS {
    group.throughput(Throughput::Bytes(input.len() as u64));
    group.bench_with_input(BenchmarkId::new("single", name), input, |b, s| {
      b.iter(|| parse(black_box(s)).unwrap());
    });
  }

  group.finish();
}

fn bench_roundtrip(c: &mut Criterion) {
  let mut group = c.benchmark_group("roundtrip");

  for &(name, input) in INPUTS {
    group.throughput(Throughput::Bytes(input.len() as u64));
    group.bench_with_input(BenchmarkId::new("parse+display", name), input, |b, s| {
      b.iter(|| {
        let coord = parse(black_box(s)).unwrap();
        let _ = black_box(format!("{coord}"));
      });
    });
  }

  group.finish();
}

fn bench_to_decimal_degrees(c: &mut Criterion) {
  let mut group = c.benchmark_group("to_decimal_degrees");

  for &(name, input) in INPUTS {
    let coord = parse(input).unwrap();
    group.bench_function(BenchmarkId::new("convert", name), |b| {
      b.iter(|| black_box(coord.to_decimal_degrees()));
    });
  }

  group.finish();
}

fn bench_batch(c: &mut Criterion) {
  // Parse many coordinates in sequence to measure throughput
  let batch: Vec<&str> = INPUTS.iter().map(|&(_, s)| s).cycle().take(1000).collect();
  let total_bytes: u64 = batch.iter().map(|s| s.len() as u64).sum();

  let mut group = c.benchmark_group("batch");
  group.throughput(Throughput::Bytes(total_bytes));
  group.bench_function("parse_1000", |b| {
    b.iter(|| {
      for &s in &batch {
        let _ = black_box(parse(black_box(s)));
      }
    });
  });
  group.finish();
}

criterion_group!(
  benches,
  bench_parse,
  bench_roundtrip,
  bench_to_decimal_degrees,
  bench_batch
);
criterion_main!(benches);
