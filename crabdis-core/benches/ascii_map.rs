use std::collections::HashMap;
use std::hint::black_box;

use crabdis_core::ascii_map::AsciiMap;
use criterion::{Criterion, Throughput, criterion_group, criterion_main};

type NormalMap = HashMap<String, usize>;

fn make_command(index: usize) -> String {
    let verb = match index % 16 {
        0 => "acl",
        1 => "append",
        2 => "client",
        3 => "command",
        4 => "config",
        5 => "del",
        6 => "exists",
        7 => "expire",
        8 => "get",
        9 => "hgetall",
        10 => "msetnx",
        11 => "psetex",
        12 => "publish",
        13 => "renamenx",
        14 => "subscribe",
        _ => "unsubscribe",
    };

    format!("{verb}:{index:05}:region-{}", index % 97)
}

fn mixed_case(input: &str, salt: usize) -> String {
    input
        .bytes()
        .enumerate()
        .map(|(index, byte)| {
            if byte.is_ascii_alphabetic() && !(index + salt).is_multiple_of(3) {
                byte.to_ascii_uppercase() as char
            } else {
                byte as char
            }
        })
        .collect()
}

fn data(size: usize) -> (Vec<String>, Vec<String>, AsciiMap<usize>, NormalMap) {
    let lowercase_keys: Vec<String> = (0..size).map(make_command).collect();
    let mixed_case_keys: Vec<String> = lowercase_keys
        .iter()
        .enumerate()
        .map(|(index, key)| mixed_case(key, index))
        .collect();

    let mut ascii_map = AsciiMap::new();
    for (index, key) in lowercase_keys.iter().enumerate() {
        ascii_map.insert(key.clone(), index);
    }

    let normal_map = lowercase_keys
        .iter()
        .enumerate()
        .map(|(index, key)| (key.clone(), index))
        .collect();

    (lowercase_keys, mixed_case_keys, ascii_map, normal_map)
}

fn bench_ascii_map(c: &mut Criterion) {
    let size = 20_000;
    let (lowercase_keys, mixed_case_keys, ascii_map, normal_map) = data(size);
    let mut group = c.benchmark_group("ascii_map_lookup");

    group.throughput(Throughput::Elements(size as u64));

    group.bench_function("ascii_map_mixed_case", |bench| {
        bench.iter(|| {
            let mut sum = 0usize;

            for key in &mixed_case_keys {
                sum = sum.wrapping_add(*ascii_map.get(black_box(key.as_str())).unwrap());
            }

            black_box(sum)
        });
    });

    group.bench_function("ascii_map_already_lowercase", |bench| {
        bench.iter(|| {
            let mut sum = 0usize;

            for key in &lowercase_keys {
                sum = sum.wrapping_add(*ascii_map.get(black_box(key.as_str())).unwrap());
            }

            black_box(sum)
        });
    });

    group.bench_function("normal_map_to_ascii_lowercase", |bench| {
        bench.iter(|| {
            let mut sum = 0usize;

            for key in &mixed_case_keys {
                let normalized = black_box(key.as_str()).to_ascii_lowercase();
                black_box(&normalized);
                sum = sum.wrapping_add(*normal_map.get(normalized.as_str()).unwrap());
            }

            black_box(sum)
        });
    });

    group.bench_function("normal_map_already_lowercase", |bench| {
        bench.iter(|| {
            let mut sum = 0usize;

            for key in &lowercase_keys {
                sum = sum.wrapping_add(*normal_map.get(black_box(key.as_str())).unwrap());
            }

            black_box(sum)
        });
    });

    group.finish();
}

criterion_group!(benches, bench_ascii_map);
criterion_main!(benches);
