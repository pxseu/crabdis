use std::collections::{HashMap, HashSet};
use std::hint::black_box;
use std::io::Cursor;
use std::sync::Arc;

use crabdis_core::parsers::resp::Resp;
use crabdis_core::value::Value;
use crabdis_core::value_multi;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use tokio::io::BufReader;
use tokio::runtime::Runtime;

fn bench_resp2_serialize_primitives(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("resp2_serialize_primitives");

    let primitives: &[(&str, Value)] = &[
        ("simple", Value::Simple("OK".into())),
        ("integer", Value::Integer(42)),
        ("nil", Value::Nil),
        ("error", Value::Error("ERR something went wrong".into())),
    ];

    for (name, value) in primitives {
        group.bench_with_input(BenchmarkId::from_parameter(name), value, |b, value| {
            b.to_async(&rt).iter(|| async {
                let mut buf = Vec::new();
                Resp::to2(black_box(value), &mut buf).await.unwrap();
                black_box(buf)
            })
        });
    }

    group.finish();
}

fn bench_resp2_serialize_strings(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("resp2_serialize_strings");

    for size in [10, 100, 1000, 10000] {
        let data = "x".repeat(size);
        let value = Value::String(data.into());

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &value, |b, value| {
            b.to_async(&rt).iter(|| async {
                let mut buf = Vec::new();
                Resp::to2(black_box(value), &mut buf).await.unwrap();
                black_box(buf)
            })
        });
    }

    group.finish();
}

fn bench_resp2_serialize_arrays(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("resp2_serialize_arrays");

    for size in [1, 10, 100, 1000] {
        let values: Vec<Value> = (0..size)
            .map(|i| Value::String(format!("value_{}", i).into()))
            .collect();
        let value = Value::Multi(values.into());

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &value, |b, value| {
            b.to_async(&rt).iter(|| async {
                let mut buf = Vec::new();
                Resp::to2(black_box(value), &mut buf).await.unwrap();
                black_box(buf)
            })
        });
    }

    group.finish();
}

fn bench_resp2_serialize_maps(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("resp2_serialize_maps");

    for size in [1, 10, 100] {
        let mut map = HashMap::new();
        for i in 0..size {
            map.insert(
                Value::String(format!("key_{}", i).into()),
                Value::String(format!("value_{}", i).into()),
            );
        }
        let value = Value::Map(map);

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &value, |b, value| {
            b.to_async(&rt).iter(|| async {
                let mut buf = Vec::new();
                Resp::to2(black_box(value), &mut buf).await.unwrap();
                black_box(buf)
            })
        });
    }

    group.finish();
}

fn bench_resp2_serialize_complex(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("resp2_serialize_complex");

    let complex_values: &[(&str, Value)] = &[
        (
            "nested_arrays",
            value_multi![
                value_multi![
                    Value::String("nested".into()),
                    Value::Integer(1),
                    Value::Nil,
                ],
                value_multi![
                    Value::String("array".into()),
                    Value::Integer(2),
                    Value::Simple("OK".into()),
                ],
            ],
        ),
        (
            "expire",
            Value::Expire((
                Arc::new(Value::String("cached_value".into())),
                tokio::time::Instant::now() + tokio::time::Duration::from_secs(3600),
            )),
        ),
    ];

    for (name, value) in complex_values {
        group.bench_with_input(BenchmarkId::from_parameter(name), value, |b, value| {
            b.to_async(&rt).iter(|| async {
                let mut buf = Vec::new();
                Resp::to2(black_box(value), &mut buf).await.unwrap();
                black_box(buf)
            })
        });
    }

    group.finish();
}

fn bench_resp2_deserialize_primitives(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("resp2_deserialize_primitives");

    let primitives: &[(&str, &[u8])] = &[
        ("simple", b"+OK\r\n"),
        ("integer", b":42\r\n"),
        ("nil", b"$-1\r\n"),
        ("error", b"-ERR something went wrong\r\n"),
    ];

    for (name, data) in primitives {
        group.bench_with_input(BenchmarkId::from_parameter(name), data, |b, data| {
            b.to_async(&rt).iter(|| async {
                let mut cursor = Cursor::new(black_box(*data));
                let mut reader = BufReader::new(&mut cursor);
                black_box(Resp::from2(&mut reader).await.unwrap())
            })
        });
    }

    group.finish();
}

fn bench_resp2_deserialize_strings(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("resp2_deserialize_strings");

    for size in [10, 100, 1000, 10000] {
        let s = "x".repeat(size);
        let data = format!("${}\r\n{}\r\n", size, s).into_bytes();

        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, data| {
            b.to_async(&rt).iter(|| async {
                let mut cursor = Cursor::new(black_box(data.as_slice()));
                let mut reader = BufReader::new(&mut cursor);
                black_box(Resp::from2(&mut reader).await.unwrap())
            })
        });
    }

    group.finish();
}

fn bench_resp2_deserialize_arrays(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("resp2_deserialize_arrays");

    for size in [1, 10, 100, 1000] {
        let mut data = format!("*{}\r\n", size);
        for i in 0..size {
            let s = format!("value_{}", i);
            data.push_str(&format!("${}\r\n{}\r\n", s.len(), s));
        }
        let data = data.into_bytes();

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, data| {
            b.to_async(&rt).iter(|| async {
                let mut cursor = Cursor::new(black_box(data.as_slice()));
                let mut reader = BufReader::new(&mut cursor);
                black_box(Resp::from2(&mut reader).await.unwrap())
            })
        });
    }

    group.finish();
}

fn bench_resp2_deserialize_complex(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("resp2_deserialize_complex");

    let complex_data: &[(&str, &[u8])] = &[
        (
            "nested_arrays",
            b"*2\r\n*3\r\n$6\r\nnested\r\n:1\r\n$-1\r\n*3\r\n$5\r\narray\r\n:2\r\n+OK\r\n",
        ),
        (
            "mixed_types",
            b"*5\r\n$5\r\nHello\r\n:42\r\n$-1\r\n+OK\r\n-ERR error\r\n",
        ),
    ];

    for (name, data) in complex_data {
        group.bench_with_input(BenchmarkId::from_parameter(name), data, |b, data| {
            b.to_async(&rt).iter(|| async {
                let mut cursor = Cursor::new(black_box(*data));
                let mut reader = BufReader::new(&mut cursor);
                black_box(Resp::from2(&mut reader).await.unwrap())
            })
        });
    }

    group.finish();
}

fn bench_resp3_serialize_specific(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("resp3_serialize_specific");

    let mut set = HashSet::new();
    for i in 0..10 {
        set.insert(Value::String(format!("value_{}", i).into()));
    }
    let set_value = Value::Set(set);

    let mut map = HashMap::new();
    for i in 0..10 {
        map.insert(
            Value::String(format!("key_{}", i).into()),
            Value::String(format!("value_{}", i).into()),
        );
    }
    let map_value = Value::Map(map);

    let push_value = Value::Push(
        vec![
            Value::String("message".into()),
            Value::String("channel".into()),
            Value::String("Hello, World!".into()),
        ]
        .into(),
    );

    let error_value = Value::Error("ERR this is a longer error message".into());

    let nil_value = Value::Nil;

    let resp3_values: &[(&str, &Value)] = &[
        ("set_10", &set_value),
        ("map_10", &map_value),
        ("push_message", &push_value),
        ("error", &error_value),
        ("nil", &nil_value),
    ];

    for (name, value) in resp3_values {
        group.bench_with_input(BenchmarkId::from_parameter(name), value, |b, value| {
            b.to_async(&rt).iter(|| async {
                let mut buf = Vec::new();
                Resp::to3(black_box(*value), &mut buf).await.unwrap();
                black_box(buf)
            })
        });
    }

    group.finish();
}

fn bench_resp3_deserialize_specific(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("resp3_deserialize_specific");

    let mut set_data = "~10\r\n".to_string();
    for i in 0..10 {
        let s = format!("value_{}", i);
        set_data.push_str(&format!("${}\r\n{}\r\n", s.len(), s));
    }
    let set_data = set_data.into_bytes();

    let mut map_data = "%10\r\n".to_string();
    for i in 0..10 {
        let key = format!("key_{}", i);
        let val = format!("value_{}", i);
        map_data.push_str(&format!("${}\r\n{}\r\n", key.len(), key));
        map_data.push_str(&format!("${}\r\n{}\r\n", val.len(), val));
    }
    let map_data = map_data.into_bytes();

    let push_data = b">3\r\n$7\r\nmessage\r\n$7\r\nchannel\r\n$13\r\nHello, World!\r\n".to_vec();

    let resp3_data: Vec<(&str, Vec<u8>)> = vec![
        ("set_10", set_data),
        ("map_10", map_data),
        ("push_message", push_data),
    ];

    for (name, data) in &resp3_data {
        group.bench_with_input(BenchmarkId::from_parameter(name), data, |b, data| {
            b.to_async(&rt).iter(|| async {
                let mut cursor = Cursor::new(black_box(data.as_slice()));
                let mut reader = BufReader::new(&mut cursor);
                black_box(Resp::from2(&mut reader).await.unwrap())
            })
        });
    }

    group.finish();
}

fn bench_roundtrip_resp2(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("roundtrip_resp2");

    let string_value = Value::String("Hello, World!".into());

    let array_value = Value::Multi(
        vec![
            Value::String("Hello".into()),
            Value::Integer(42),
            Value::Nil,
            Value::Simple("OK".into()),
        ]
        .into(),
    );

    let mut map = HashMap::new();
    map.insert(Value::String("key1".into()), Value::String("value1".into()));
    map.insert(Value::String("key2".into()), Value::Integer(42));
    let map_value = Value::Map(map);

    let roundtrip_values: &[(&str, &Value)] = &[
        ("string", &string_value),
        ("array", &array_value),
        ("map", &map_value),
    ];

    for (name, value) in roundtrip_values {
        group.bench_with_input(BenchmarkId::from_parameter(name), value, |b, value| {
            b.to_async(&rt).iter(|| async {
                let mut buf = Vec::new();
                Resp::to2(black_box(*value), &mut buf).await.unwrap();
                let mut cursor = Cursor::new(black_box(buf));
                let mut reader = BufReader::new(&mut cursor);
                black_box(Resp::from2(&mut reader).await.unwrap())
            })
        });
    }

    group.finish();
}

fn bench_roundtrip_resp3(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("roundtrip_resp3");

    let mut map = HashMap::new();
    map.insert(Value::String("key1".into()), Value::String("value1".into()));
    map.insert(Value::String("key2".into()), Value::Integer(42));
    let map_value = Value::Map(map);

    let mut set = HashSet::new();
    set.insert(Value::String("value1".into()));
    set.insert(Value::String("value2".into()));
    set.insert(Value::Integer(42));
    let set_value = Value::Set(set);

    // Push
    let push_value = Value::Push(
        vec![
            Value::String("message".into()),
            Value::String("channel".into()),
            Value::String("data".into()),
        ]
        .into(),
    );

    let roundtrip_values: &[(&str, &Value)] = &[
        ("map", &map_value),
        ("set", &set_value),
        ("push", &push_value),
    ];

    for (name, value) in roundtrip_values {
        group.bench_with_input(BenchmarkId::from_parameter(name), value, |b, value| {
            b.to_async(&rt).iter(|| async {
                let mut buf = Vec::new();
                Resp::to3(black_box(*value), &mut buf).await.unwrap();
                let mut cursor = Cursor::new(black_box(buf));
                let mut reader = BufReader::new(&mut cursor);
                black_box(Resp::from2(&mut reader).await.unwrap())
            })
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    // RESP2 Serialization
    bench_resp2_serialize_primitives,
    bench_resp2_serialize_strings,
    bench_resp2_serialize_arrays,
    bench_resp2_serialize_maps,
    bench_resp2_serialize_complex,
    // RESP2 Deserialization
    bench_resp2_deserialize_primitives,
    bench_resp2_deserialize_strings,
    bench_resp2_deserialize_arrays,
    bench_resp2_deserialize_complex,
    // RESP3 Serialization
    bench_resp3_serialize_specific,
    // RESP3 Deserialization
    bench_resp3_deserialize_specific,
    // Round-trips
    bench_roundtrip_resp2,
    bench_roundtrip_resp3
);

criterion_main!(benches);
