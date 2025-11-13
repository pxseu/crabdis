use crabdis::storage::value::Value;
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::collections::{HashMap, HashSet};
use std::hint::black_box;
use std::io::Cursor;
use std::sync::Arc;
use tokio::io::BufReader;
use tokio::runtime::Runtime;

// ============================================================================
// RESP2 Serialization Benchmarks
// ============================================================================

fn bench_resp2_serialize_primitives(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("resp2_serialize_primitives");

    // Simple string
    let value = Value::Simple("OK".into());
    group.bench_function("simple", |b| {
        b.to_async(&rt).iter(|| async {
            let mut buf = Vec::new();
            black_box(&value).to_resp2(&mut buf).await.unwrap();
            black_box(buf)
        })
    });

    // Integer
    let value = Value::Integer(42);
    group.bench_function("integer", |b| {
        b.to_async(&rt).iter(|| async {
            let mut buf = Vec::new();
            black_box(&value).to_resp2(&mut buf).await.unwrap();
            black_box(buf)
        })
    });

    // Nil
    let value = Value::Nil;
    group.bench_function("nil", |b| {
        b.to_async(&rt).iter(|| async {
            let mut buf = Vec::new();
            black_box(&value).to_resp2(&mut buf).await.unwrap();
            black_box(buf)
        })
    });

    // Error
    let value = Value::Error("ERR something went wrong".into());
    group.bench_function("error", |b| {
        b.to_async(&rt).iter(|| async {
            let mut buf = Vec::new();
            black_box(&value).to_resp2(&mut buf).await.unwrap();
            black_box(buf)
        })
    });

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
                black_box(value).to_resp2(&mut buf).await.unwrap();
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
                black_box(value).to_resp2(&mut buf).await.unwrap();
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
                black_box(value).to_resp2(&mut buf).await.unwrap();
                black_box(buf)
            })
        });
    }

    group.finish();
}

fn bench_resp2_serialize_complex(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("resp2_serialize_complex");

    // Nested arrays
    let nested = Value::Multi(
        vec![
            Value::Multi(
                vec![
                    Value::String("nested".into()),
                    Value::Integer(1),
                    Value::Nil,
                ]
                .into(),
            ),
            Value::Multi(
                vec![
                    Value::String("array".into()),
                    Value::Integer(2),
                    Value::Simple("OK".into()),
                ]
                .into(),
            ),
        ]
        .into(),
    );

    group.bench_function("nested_arrays", |b| {
        b.to_async(&rt).iter(|| async {
            let mut buf = Vec::new();
            black_box(&nested).to_resp2(&mut buf).await.unwrap();
            black_box(buf)
        })
    });

    // Expire value
    let expire = Value::Expire((
        Arc::new(Value::String("cached_value".into())),
        tokio::time::Instant::now() + tokio::time::Duration::from_secs(3600),
    ));

    group.bench_function("expire", |b| {
        b.to_async(&rt).iter(|| async {
            let mut buf = Vec::new();
            black_box(&expire).to_resp2(&mut buf).await.unwrap();
            black_box(buf)
        })
    });

    group.finish();
}

// ============================================================================
// RESP2 Deserialization Benchmarks
// ============================================================================

fn bench_resp2_deserialize_primitives(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("resp2_deserialize_primitives");

    // Simple string
    let data = b"+OK\r\n".to_vec();
    group.bench_function("simple", |b| {
        b.to_async(&rt).iter(|| async {
            let mut cursor = Cursor::new(black_box(data.clone()));
            let mut reader = BufReader::new(&mut cursor);
            black_box(Value::from_resp(&mut reader).await.unwrap())
        })
    });

    // Integer
    let data = b":42\r\n".to_vec();
    group.bench_function("integer", |b| {
        b.to_async(&rt).iter(|| async {
            let mut cursor = Cursor::new(black_box(data.clone()));
            let mut reader = BufReader::new(&mut cursor);
            black_box(Value::from_resp(&mut reader).await.unwrap())
        })
    });

    // Nil
    let data = b"$-1\r\n".to_vec();
    group.bench_function("nil", |b| {
        b.to_async(&rt).iter(|| async {
            let mut cursor = Cursor::new(black_box(data.clone()));
            let mut reader = BufReader::new(&mut cursor);
            black_box(Value::from_resp(&mut reader).await.unwrap())
        })
    });

    // Error
    let data = b"-ERR something went wrong\r\n".to_vec();
    group.bench_function("error", |b| {
        b.to_async(&rt).iter(|| async {
            let mut cursor = Cursor::new(black_box(data.clone()));
            let mut reader = BufReader::new(&mut cursor);
            black_box(Value::from_resp(&mut reader).await.unwrap())
        })
    });

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
                let mut cursor = Cursor::new(black_box(data.clone()));
                let mut reader = BufReader::new(&mut cursor);
                black_box(Value::from_resp(&mut reader).await.unwrap())
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
                let mut cursor = Cursor::new(black_box(data.clone()));
                let mut reader = BufReader::new(&mut cursor);
                black_box(Value::from_resp(&mut reader).await.unwrap())
            })
        });
    }

    group.finish();
}

fn bench_resp2_deserialize_complex(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("resp2_deserialize_complex");

    // Nested arrays
    let data =
        b"*2\r\n*3\r\n$6\r\nnested\r\n:1\r\n$-1\r\n*3\r\n$5\r\narray\r\n:2\r\n+OK\r\n".to_vec();
    group.bench_function("nested_arrays", |b| {
        b.to_async(&rt).iter(|| async {
            let mut cursor = Cursor::new(black_box(data.clone()));
            let mut reader = BufReader::new(&mut cursor);
            black_box(Value::from_resp(&mut reader).await.unwrap())
        })
    });

    // Mixed types array
    let data = b"*5\r\n$5\r\nHello\r\n:42\r\n$-1\r\n+OK\r\n-ERR error\r\n".to_vec();
    group.bench_function("mixed_types", |b| {
        b.to_async(&rt).iter(|| async {
            let mut cursor = Cursor::new(black_box(data.clone()));
            let mut reader = BufReader::new(&mut cursor);
            black_box(Value::from_resp(&mut reader).await.unwrap())
        })
    });

    group.finish();
}

// ============================================================================
// RESP3 Serialization Benchmarks
// ============================================================================

fn bench_resp3_serialize_specific(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("resp3_serialize_specific");

    // Set
    let mut set = HashSet::new();
    for i in 0..10 {
        set.insert(Value::String(format!("value_{}", i).into()));
    }
    let value = Value::Set(set);

    group.bench_function("set_10", |b| {
        b.to_async(&rt).iter(|| async {
            let mut buf = Vec::new();
            black_box(&value).to_resp3(&mut buf).await.unwrap();
            black_box(buf)
        })
    });

    // Map
    let mut map = HashMap::new();
    for i in 0..10 {
        map.insert(
            Value::String(format!("key_{}", i).into()),
            Value::String(format!("value_{}", i).into()),
        );
    }
    let value = Value::Map(map);

    group.bench_function("map_10", |b| {
        b.to_async(&rt).iter(|| async {
            let mut buf = Vec::new();
            black_box(&value).to_resp3(&mut buf).await.unwrap();
            black_box(buf)
        })
    });

    // Push message (pub/sub)
    let value = Value::Push(
        vec![
            Value::String("message".into()),
            Value::String("channel".into()),
            Value::String("Hello, World!".into()),
        ]
        .into(),
    );

    group.bench_function("push_message", |b| {
        b.to_async(&rt).iter(|| async {
            let mut buf = Vec::new();
            black_box(&value).to_resp3(&mut buf).await.unwrap();
            black_box(buf)
        })
    });

    // RESP3 Error (with length)
    let value = Value::Error("ERR this is a longer error message".into());
    group.bench_function("error", |b| {
        b.to_async(&rt).iter(|| async {
            let mut buf = Vec::new();
            black_box(&value).to_resp3(&mut buf).await.unwrap();
            black_box(buf)
        })
    });

    // Nil (different encoding)
    let value = Value::Nil;
    group.bench_function("nil", |b| {
        b.to_async(&rt).iter(|| async {
            let mut buf = Vec::new();
            black_box(&value).to_resp3(&mut buf).await.unwrap();
            black_box(buf)
        })
    });

    group.finish();
}

// ============================================================================
// RESP3 Deserialization Benchmarks
// ============================================================================

fn bench_resp3_deserialize_specific(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("resp3_deserialize_specific");

    // Set
    let mut data = "~10\r\n".to_string();
    for i in 0..10 {
        let s = format!("value_{}", i);
        data.push_str(&format!("${}\r\n{}\r\n", s.len(), s));
    }
    let data = data.into_bytes();

    group.bench_function("set_10", |b| {
        b.to_async(&rt).iter(|| async {
            let mut cursor = Cursor::new(black_box(data.clone()));
            let mut reader = BufReader::new(&mut cursor);
            black_box(Value::from_resp(&mut reader).await.unwrap())
        })
    });

    // Map
    let mut data = "%10\r\n".to_string();
    for i in 0..10 {
        let key = format!("key_{}", i);
        let val = format!("value_{}", i);
        data.push_str(&format!("${}\r\n{}\r\n", key.len(), key));
        data.push_str(&format!("${}\r\n{}\r\n", val.len(), val));
    }
    let data = data.into_bytes();

    group.bench_function("map_10", |b| {
        b.to_async(&rt).iter(|| async {
            let mut cursor = Cursor::new(black_box(data.clone()));
            let mut reader = BufReader::new(&mut cursor);
            black_box(Value::from_resp(&mut reader).await.unwrap())
        })
    });

    // Push message
    let data = b">3\r\n$7\r\nmessage\r\n$7\r\nchannel\r\n$13\r\nHello, World!\r\n".to_vec();
    group.bench_function("push_message", |b| {
        b.to_async(&rt).iter(|| async {
            let mut cursor = Cursor::new(black_box(data.clone()));
            let mut reader = BufReader::new(&mut cursor);
            black_box(Value::from_resp(&mut reader).await.unwrap())
        })
    });

    group.finish();
}

// ============================================================================
// Round-trip Benchmarks (serialize -> deserialize)
// ============================================================================

fn bench_roundtrip_resp2(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("roundtrip_resp2");

    // String
    let value = Value::String("Hello, World!".into());
    group.bench_function("string", |b| {
        b.to_async(&rt).iter(|| async {
            let mut buf = Vec::new();
            black_box(&value).to_resp2(&mut buf).await.unwrap();
            let mut cursor = Cursor::new(buf);
            let mut reader = BufReader::new(&mut cursor);
            black_box(Value::from_resp(&mut reader).await.unwrap())
        })
    });

    // Array
    let value = Value::Multi(
        vec![
            Value::String("Hello".into()),
            Value::Integer(42),
            Value::Nil,
            Value::Simple("OK".into()),
        ]
        .into(),
    );
    group.bench_function("array", |b| {
        b.to_async(&rt).iter(|| async {
            let mut buf = Vec::new();
            black_box(&value).to_resp2(&mut buf).await.unwrap();
            let mut cursor = Cursor::new(buf);
            let mut reader = BufReader::new(&mut cursor);
            black_box(Value::from_resp(&mut reader).await.unwrap())
        })
    });

    // Map (as array in RESP2)
    let mut map = HashMap::new();
    map.insert(Value::String("key1".into()), Value::String("value1".into()));
    map.insert(Value::String("key2".into()), Value::Integer(42));
    let value = Value::Map(map);

    group.bench_function("map", |b| {
        b.to_async(&rt).iter(|| async {
            let mut buf = Vec::new();
            black_box(&value).to_resp2(&mut buf).await.unwrap();
            let mut cursor = Cursor::new(buf);
            let mut reader = BufReader::new(&mut cursor);
            black_box(Value::from_resp(&mut reader).await.unwrap())
        })
    });

    group.finish();
}

fn bench_roundtrip_resp3(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("roundtrip_resp3");

    // Map
    let mut map = HashMap::new();
    map.insert(Value::String("key1".into()), Value::String("value1".into()));
    map.insert(Value::String("key2".into()), Value::Integer(42));
    let value = Value::Map(map);

    group.bench_function("map", |b| {
        b.to_async(&rt).iter(|| async {
            let mut buf = Vec::new();
            black_box(&value).to_resp3(&mut buf).await.unwrap();
            let mut cursor = Cursor::new(buf);
            let mut reader = BufReader::new(&mut cursor);
            black_box(Value::from_resp(&mut reader).await.unwrap())
        })
    });

    // Set
    let mut set = HashSet::new();
    set.insert(Value::String("value1".into()));
    set.insert(Value::String("value2".into()));
    set.insert(Value::Integer(42));
    let value = Value::Set(set);

    group.bench_function("set", |b| {
        b.to_async(&rt).iter(|| async {
            let mut buf = Vec::new();
            black_box(&value).to_resp3(&mut buf).await.unwrap();
            let mut cursor = Cursor::new(buf);
            let mut reader = BufReader::new(&mut cursor);
            black_box(Value::from_resp(&mut reader).await.unwrap())
        })
    });

    // Push
    let value = Value::Push(
        vec![
            Value::String("message".into()),
            Value::String("channel".into()),
            Value::String("data".into()),
        ]
        .into(),
    );

    group.bench_function("push", |b| {
        b.to_async(&rt).iter(|| async {
            let mut buf = Vec::new();
            black_box(&value).to_resp3(&mut buf).await.unwrap();
            let mut cursor = Cursor::new(buf);
            let mut reader = BufReader::new(&mut cursor);
            black_box(Value::from_resp(&mut reader).await.unwrap())
        })
    });

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
