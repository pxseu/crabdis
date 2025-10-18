use crabdis::storage::value::Value;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::collections::HashMap;
use std::sync::Arc;

fn bench_value_creation(c: &mut Criterion) {
    c.bench_function("create_string", |b| {
        b.iter(|| Value::String(black_box("test".to_string())))
    });

    c.bench_function("create_integer", |b| {
        b.iter(|| Value::Integer(black_box(42)))
    });

    c.bench_function("create_multi", |b| {
        b.iter(|| {
            let values = vec![
                Value::String(black_box("test".to_string())),
                Value::Integer(black_box(42)),
            ];
            Value::Multi(Arc::new(values.into_boxed_slice()))
        })
    });

    c.bench_function("create_map", |b| {
        b.iter(|| {
            let mut map = HashMap::new();
            map.insert(
                Value::String(black_box("key".to_string())),
                Value::String(black_box("value".to_string())),
            );
            Value::Map(black_box(map))
        })
    });
}

fn bench_value_operations(c: &mut Criterion) {
    let value = Value::String("test".to_string());
    c.bench_function("is_some", |b| b.iter(|| black_box(&value).is_some()));

    let value = Value::Nil;
    c.bench_function("is_none", |b| b.iter(|| black_box(&value).is_none()));

    let value = Value::String("test".to_string());
    c.bench_function("inner", |b| b.iter(|| black_box(&value).inner()));
}

criterion_group!(benches, bench_value_creation, bench_value_operations);
criterion_main!(benches);
