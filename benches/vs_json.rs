use std::io::BufReader;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use rusqlite::{Connection, DatabaseName};
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Deserialize, Serialize)]
struct Person {
    id: usize,
    name: String,
    phone_numbers: Vec<String>,
    active: bool,
    data: String,
}

fn convert_to_json_then_deserialize(conn: &Connection) -> Person {
    let json_str: String = conn
        .query_row(
            "SELECT json(data) from bigdata where id=?", // convert jsonb to json
            [42],
            |row| row.get(0),
        )
        .unwrap();
    serde_json::from_str(&json_str).unwrap()
}

fn deserialize_jsonb_directly_from_blob(conn: &Connection) -> Person {
    let my_blob = conn
        .blob_open(DatabaseName::Main, "bigdata", "data", 42, true)
        .unwrap();
    let buffered = BufReader::new(my_blob);
    serde_sqlite_jsonb::from_reader(buffered).unwrap()
}

fn insert_big_data(conn: &Connection, data_size: usize) {
    let person = Person {
        id: 123,
        name: "John Doe".to_string(),
        phone_numbers: vec!["1234".to_string()],
        active: true,
        data: "x".repeat(data_size),
    };
    conn.execute(
        "INSERT OR REPLACE INTO bigdata (id, data) VALUES (42, ?)",
        [serde_sqlite_jsonb::to_vec(&person).unwrap()],
    )
    .unwrap();
}

fn bench_deserialize_json_vs_jsonb(c: &mut Criterion) {
    let conn = Connection::open_in_memory().unwrap();
    // Store a large json string as a jsonb blob in a table
    conn.execute_batch(
        "create table bigdata (id integer primary key, data blob)",
    )
    .unwrap();

    let mut group = c.benchmark_group("reading a stored jsonb blob");
    for data_size in [50, 100, 500, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("convert to json then deserialize", data_size),
            data_size,
            |b, data_size| {
                insert_big_data(&conn, *data_size);
                b.iter(|| convert_to_json_then_deserialize(&conn))
            },
        );
        group.bench_with_input(
            BenchmarkId::new("deserialize jsonb directly from blob", data_size),
            data_size,
            |b, data_size| {
                insert_big_data(&conn, *data_size);
                b.iter(|| deserialize_jsonb_directly_from_blob(&conn))
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_deserialize_json_vs_jsonb);
criterion_main!(benches);
