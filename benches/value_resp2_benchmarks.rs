use crabdis::storage::value::Value;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::collections::{HashMap, VecDeque};
use std::io::Cursor;
use tokio::io::BufReader;
use tokio::runtime::Runtime;

fn bench_resp2_serialization(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // Simple String
    let value = Value::String("Hello, World!".to_string());
    c.bench_function("resp2_serialize_string", |b| {
        b.to_async(&rt).iter(|| async {
            let mut buf = Vec::new();
            value.to_resp2(&mut buf).await.unwrap();
            black_box(buf)
        })
    });

    // Integer
    let value = Value::Integer(42);
    c.bench_function("resp2_serialize_integer", |b| {
        b.to_async(&rt).iter(|| async {
            let mut buf = Vec::new();
            value.to_resp2(&mut buf).await.unwrap();
            black_box(buf)
        })
    });

    // Nil
    let value = Value::Nil;
    c.bench_function("resp2_serialize_nil", |b| {
        b.to_async(&rt).iter(|| async {
            let mut buf = Vec::new();
            value.to_resp2(&mut buf).await.unwrap();
            black_box(buf)
        })
    });

    // Multi with mixed types
    let value = Value::Multi(VecDeque::from([
        Value::String("Hello".to_string()),
        Value::Integer(42),
        Value::Nil,
        Value::Simple("OK".to_string()),
    ]));
    c.bench_function("resp2_serialize_multi", |b| {
        b.to_async(&rt).iter(|| async {
            let mut buf = Vec::new();
            value.to_resp2(&mut buf).await.unwrap();
            black_box(buf)
        })
    });

    // Map
    let mut map = HashMap::new();
    map.insert(
        Value::String("key1".to_string()),
        Value::String("value1".to_string()),
    );
    map.insert(Value::String("key2".to_string()), Value::Integer(42));
    let value = Value::Map(map);
    c.bench_function("resp2_serialize_map", |b| {
        b.to_async(&rt).iter(|| async {
            let mut buf = Vec::new();
            value.to_resp2(&mut buf).await.unwrap();
            black_box(buf)
        })
    });

    // Expire
    let value = Value::Expire((
        Box::new(Value::String("test".to_string())),
        tokio::time::Instant::now(),
    ));
    c.bench_function("resp2_serialize_expire", |b| {
        b.to_async(&rt).iter(|| async {
            let mut buf = Vec::new();
            value.to_resp2(&mut buf).await.unwrap();
            black_box(buf)
        })
    });
}

fn bench_resp2_deserialization(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // String
    let data = b"$13\r\nHello, World!\r\n".to_vec();
    c.bench_function("resp2_deserialize_string", |b| {
        b.to_async(&rt).iter(|| async {
            let mut cursor = Cursor::new(data.clone());
            let mut reader = BufReader::new(&mut cursor);
            Value::from_resp(&mut reader).await.unwrap()
        })
    });

    // Integer
    let data = b":42\r\n".to_vec();
    c.bench_function("resp2_deserialize_integer", |b| {
        b.to_async(&rt).iter(|| async {
            let mut cursor = Cursor::new(data.clone());
            let mut reader = BufReader::new(&mut cursor);
            Value::from_resp(&mut reader).await.unwrap()
        })
    });

    // Nil
    let data = b"$-1\r\n".to_vec();
    c.bench_function("resp2_deserialize_nil", |b| {
        b.to_async(&rt).iter(|| async {
            let mut cursor = Cursor::new(data.clone());
            let mut reader = BufReader::new(&mut cursor);
            Value::from_resp(&mut reader).await.unwrap()
        })
    });

    // Multi
    let data = b"*3\r\n$5\r\nHello\r\n:42\r\n$-1\r\n".to_vec();
    c.bench_function("resp2_deserialize_multi", |b| {
        b.to_async(&rt).iter(|| async {
            let mut cursor = Cursor::new(data.clone());
            let mut reader = BufReader::new(&mut cursor);
            Value::from_resp(&mut reader).await.unwrap()
        })
    });
}

criterion_group!(
    benches,
    bench_resp2_serialization,
    bench_resp2_deserialization
);
criterion_main!(benches);
