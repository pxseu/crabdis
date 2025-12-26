use std::hint::black_box;
use std::io::Cursor;

use crabdis::storage::parser::{deserialize_integer, serialize_integer};
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use tokio::runtime::Runtime;

fn bench_int_deserialize_fast(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("int_deserialize_fast");

    for input in [
        b"0\r\n".as_ref(),
        b"1\r\n".as_ref(),
        b"42\r\n".as_ref(),
        b"1234\r\n".as_ref(),
        b"1234567890\r\n".as_ref(),
        b"-100\r\n".as_ref(),
    ] {
        let label = std::str::from_utf8(&input[..input.len() - 2]).unwrap();
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(BenchmarkId::new("cursor", label), &input, |b, data| {
            b.to_async(&rt).iter(|| async {
                let mut cursor = Cursor::new(*data);
                let v = deserialize_integer(&mut cursor).await.unwrap();
                black_box(v)
            })
        });
    }

    group.finish();
}

fn bench_int_serialize_fast(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("int_serialize_fast");

    for input in [0, 1, 42, 1234, 1234567890, -100] {
        group.throughput(Throughput::Bytes(input.to_string().len() as u64));
        group.bench_with_input(
            BenchmarkId::new("cursor", input.to_string()),
            &input,
            |b, data| {
                b.to_async(&rt).iter(|| async {
                    let mut writer = Cursor::new(Vec::new());
                    serialize_integer(&mut writer, *data).await.unwrap();
                    black_box(writer.into_inner())
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_int_deserialize_fast,
    bench_int_serialize_fast,
);
criterion_main!(benches);
