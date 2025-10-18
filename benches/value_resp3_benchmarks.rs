use crabdis::storage::value::Value;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::collections::{HashMap, HashSet};
use std::io::Cursor;
use std::sync::Arc;
use tokio::io::BufReader;
use tokio::runtime::Runtime;

fn bench_resp3_serialization(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // Map
    let mut map = HashMap::new();
    map.insert(
        Value::String(black_box("key1".to_string())),
        Value::String(black_box("value1".to_string())),
    );
    map.insert(
        Value::String(black_box("key2".to_string())),
        Value::Integer(black_box(42)),
    );
    let value = Value::Map(map);
    c.bench_function("resp3_serialize_map", |b| {
        b.to_async(&rt).iter(|| async {
            let mut buf = Vec::new();
            black_box(&value).to_resp3(&mut buf).await.unwrap();
            black_box(buf)
        })
    });

    // Set
    let mut set = HashSet::new();
    set.insert(Value::String(black_box("value1".to_string())));
    set.insert(Value::Integer(black_box(42)));
    let value = Value::Set(set);
    c.bench_function("resp3_serialize_set", |b| {
        b.to_async(&rt).iter(|| async {
            let mut buf = Vec::new();
            black_box(&value).to_resp3(&mut buf).await.unwrap();
            black_box(buf)
        })
    });

    // Push
    let value = Value::Push(Arc::new(
        vec![
            Value::String(black_box("channel".to_string())),
            Value::String(black_box("message".to_string())),
        ]
        .into_boxed_slice(),
    ));
    c.bench_function("resp3_serialize_push", |b| {
        b.to_async(&rt).iter(|| async {
            let mut buf = Vec::new();
            black_box(&value).to_resp3(&mut buf).await.unwrap();
            black_box(buf)
        })
    });

    // Error
    let value = Value::Error(black_box("ERR something went wrong".to_string()));
    c.bench_function("resp3_serialize_error", |b| {
        b.to_async(&rt).iter(|| async {
            let mut buf = Vec::new();
            black_box(&value).to_resp3(&mut buf).await.unwrap();
            black_box(buf)
        })
    });
}

fn bench_resp3_deserialization(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // Map
    let data = b"%2\r\n$4\r\nkey1\r\n$6\r\nvalue1\r\n$4\r\nkey2\r\n:42\r\n".to_vec();
    c.bench_function("resp3_deserialize_map", |b| {
        b.to_async(&rt).iter(|| async {
            let mut cursor = Cursor::new(black_box(data.clone()));
            let mut reader = BufReader::new(&mut cursor);
            black_box(Value::from_resp(&mut reader).await.unwrap())
        })
    });

    // Set
    let data = b"~2\r\n$6\r\nvalue1\r\n:42\r\n".to_vec();
    c.bench_function("resp3_deserialize_set", |b| {
        b.to_async(&rt).iter(|| async {
            let mut cursor = Cursor::new(black_box(data.clone()));
            let mut reader = BufReader::new(&mut cursor);
            black_box(Value::from_resp(&mut reader).await.unwrap())
        })
    });

    // Push
    let data = b">2\r\n$7\r\nchannel\r\n$7\r\nmessage\r\n".to_vec();
    c.bench_function("resp3_deserialize_push", |b| {
        b.to_async(&rt).iter(|| async {
            let mut cursor = Cursor::new(black_box(data.clone()));
            let mut reader = BufReader::new(&mut cursor);
            black_box(Value::from_resp(&mut reader).await.unwrap())
        })
    });

    // Error
    let data = b"!24\r\nERR something went wrong\r\n".to_vec();
    c.bench_function("resp3_deserialize_error", |b| {
        b.to_async(&rt).iter(|| async {
            let mut cursor = Cursor::new(black_box(data.clone()));
            let mut reader = BufReader::new(&mut cursor);
            black_box(Value::from_resp(&mut reader).await.unwrap())
        })
    });
}

criterion_group!(
    benches,
    bench_resp3_serialization,
    bench_resp3_deserialization
);
criterion_main!(benches);
