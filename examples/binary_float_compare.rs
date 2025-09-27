//! this example compares sqlite database size after insert
//!  lots of float numbers between whether using binary float
//!  or not.

use rusqlite::Connection;

pub fn create_table(conn: &Connection) {
    conn.execute_batch("create table float_data (data blob)")
        .unwrap();
}

pub fn insert_data(conn: &Connection, data: &Vec<f32>, binary_float: bool) {
    let options = serde_sqlite_jsonb::Options { binary_float };
    let blob = serde_sqlite_jsonb::to_vec_with_options(data, options).unwrap();
    conn.execute("INSERT INTO float_data (data) VALUES (?)", [blob])
        .unwrap();
}

pub fn get_database_size(conn: &Connection) -> i64 {
    let page_count: i64 = conn
        .query_row("PRAGMA page_count", [], |row| row.get(0))
        .unwrap();
    let page_size: i64 = conn
        .query_row("PRAGMA page_size", [], |row| row.get(0))
        .unwrap();
    let db_size = page_count * page_size;
    println!("database size: {} bytes", db_size);
    db_size
}

fn main() {
    let clock = std::time::Instant::now();
    println!("generating random data...");
    let v: Vec<Vec<f32>> = (0..20000)
        .map(|_| {
            (0..768)
                .map(|_| (rand::random::<f32>() - 0.5) * 2.0 * 1.0e-10) // float越小 textrepr越长，存储空间更大，对比效果更'突出'
                .collect()
        })
        .collect();
    println!("vectors[0]: {:?}", v[0]);
    println!(
        "done, total {} vectors, elpased {}ms",
        v.len(),
        clock.elapsed().as_millis()
    );

    let json_conn = Connection::open_in_memory().unwrap();
    create_table(&json_conn);

    let jsonb_conn = Connection::open_in_memory().unwrap();
    create_table(&jsonb_conn);

    for vec in v.iter() {
        insert_data(&json_conn, vec, false);
        insert_data(&jsonb_conn, vec, true);
    }

    println!(
        "inserting done, total elapsed {}ms",
        clock.elapsed().as_millis()
    );

    let json_db_size = get_database_size(&json_conn);
    let jsonb_db_size = get_database_size(&jsonb_conn);
    println!(
        "json db size: {:.2}mb (1.0x), jsonb db size: {:.2}mb ({:.2}x)",
        json_db_size as f64 / 1024.0 / 1024.0,
        jsonb_db_size as f64 / 1024.0 / 1024.0,
        jsonb_db_size as f64 / json_db_size as f64
    );
}
