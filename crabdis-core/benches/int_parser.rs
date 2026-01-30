use std::hint::black_box;
use std::io::Cursor;

use crabdis_core::parsers::resp::int::{deserialize, serialize};
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
                let mut cursor = Cursor::new(black_box(*data));
                let v = deserialize(&mut cursor).await.unwrap();
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
                    serialize(&mut writer, black_box(*data)).await.unwrap();
                    black_box(writer.into_inner())
                })
            },
        );
    }

    group.finish();
}

fn bench_int_parse_builtin(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("int_parse_builtin");

    for input in ["0", "1", "42", "1234", "1234567890", "-100"] {
        group.throughput(Throughput::Bytes(input.len() as u64));
        group.bench_with_input(BenchmarkId::new("cursor", input), &input, |b, data| {
            b.to_async(&rt).iter(|| async {
                let mut cursor = Cursor::new(black_box(data).as_bytes());
                let mut buf = String::new();
                tokio::io::AsyncReadExt::read_to_string(&mut cursor, &mut buf)
                    .await
                    .unwrap();
                let val: i64 = buf.parse().unwrap();
                black_box(val)
            })
        });
    }

    group.finish();
}

fn bench_int_to_string_builtin(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("int_to_string_builtin");

    for input in [0i64, 1, 42, 1234, 1234567890, -100] {
        group.throughput(Throughput::Bytes(input.to_string().len() as u64));
        group.bench_with_input(BenchmarkId::new("cursor", input), &input, |b, data| {
            b.to_async(&rt).iter(|| async {
                let s = black_box(data).to_string();
                let mut writer = Cursor::new(Vec::new());
                tokio::io::AsyncWriteExt::write_all(&mut writer, s.as_bytes())
                    .await
                    .unwrap();
                black_box(writer.into_inner())
            })
        });
    }

    group.finish();
}

fn bench_int_display_builtin(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("int_display_builtin");

    for input in [0i64, 1, 42, 1234, 1234567890, -100] {
        group.throughput(Throughput::Bytes(input.to_string().len() as u64));
        group.bench_with_input(BenchmarkId::new("cursor", input), &input, |b, data| {
            b.to_async(&rt).iter(|| async {
                let s = black_box(data).to_string();
                let mut writer = Cursor::new(Vec::new());
                tokio::io::AsyncWriteExt::write_all(&mut writer, s.as_bytes())
                    .await
                    .unwrap();
                black_box(writer.into_inner())
            })
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_int_deserialize_fast,
    bench_int_serialize_fast,
    bench_int_parse_builtin,
    bench_int_to_string_builtin,
    bench_int_display_builtin,
);
criterion_main!(benches);
