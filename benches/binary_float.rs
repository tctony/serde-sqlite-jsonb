use criterion::{criterion_group, criterion_main, Criterion};

fn bench_serde_float_as_binary_vs_text(c: &mut Criterion) {
    let mut group = c.benchmark_group("serde float");
    for vector_size in [50, 100, 200, 500, 1000, 2000].iter() {
        // create random vector
        let v: Vec<f64> =
            (0..*vector_size).map(|_| rand::random::<f64>()).collect();

        group.bench_function(format!("as binary {}", vector_size), |b| {
            b.iter(|| {
                let blob = serde_sqlite_jsonb::to_vec_with_options(
                    &v,
                    serde_sqlite_jsonb::Options { binary_float: true },
                )
                .unwrap();

                let _: Vec<f64> =
                    serde_sqlite_jsonb::from_slice(&blob).unwrap();
            })
        });

        group.bench_function(format!("as text {}", vector_size), |b| {
            b.iter(|| {
                let text = serde_sqlite_jsonb::to_vec(&v).unwrap();

                let _: Vec<f64> =
                    serde_sqlite_jsonb::from_slice(&text).unwrap();
            })
        });
    }
}

criterion_group!(benches, bench_serde_float_as_binary_vs_text);
criterion_main!(benches);
