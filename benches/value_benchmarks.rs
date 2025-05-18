use crabdis::storage::value::Value;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::collections::{HashMap, VecDeque};

fn bench_value_creation(c: &mut Criterion) {
    c.bench_function("create_string", |b| {
        b.iter(|| Value::String(black_box("test".to_string())))
    });

    c.bench_function("create_integer", |b| {
        b.iter(|| Value::Integer(black_box(42)))
    });

    c.bench_function("create_multi", |b| {
        b.iter(|| {
            let mut vd = VecDeque::new();
            vd.push_back(Value::String("test".to_string()));
            vd.push_back(Value::Integer(42));
            Value::Multi(black_box(vd))
        })
    });

    c.bench_function("create_map", |b| {
        b.iter(|| {
            let mut map = HashMap::new();
            map.insert(
                Value::String("key".to_string()),
                Value::String("value".to_string()),
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
