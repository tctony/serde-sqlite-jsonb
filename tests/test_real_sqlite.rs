extern crate fifthtry_serde_sqlite_jsonb as serde_sqlite_jsonb;

use std::collections::HashMap;

use rusqlite::{Connection, DatabaseName};
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Deserialize, Serialize)]
struct Person {
    id: i32,
    name: String,
    phone_numbers: Vec<PhoneNumber>,
    is_champion: bool,
    data: Vec<u8>,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
enum PhoneNumber {
    Internal(i32),
    National(String),
    International {
        country_code: Option<i32>,
        number: String,
    },
    Custom(Option<i32>, String),
}

#[test]
fn test_fetch_json_object() -> rusqlite::Result<()> {
    let conn = Connection::open_in_memory()?;
    let blob: Vec<u8> = conn.query_row(
        r#"select jsonb('{
        "id": 1,
        "name": "John Doe",
        "phone_numbers": [{"National": "1234"}],
        "is_champion": true,
        "data": [1, 2, 3]
    }')"#,
        [],
        |row| row.get(0),
    )?;
    let person: Person = serde_sqlite_jsonb::from_slice(&blob).unwrap();
    assert_eq!(
        person,
        Person {
            id: 1,
            name: "John Doe".to_string(),
            phone_numbers: vec![PhoneNumber::National("1234".to_string())],
            is_champion: true,
            data: vec![1, 2, 3]
        }
    );

    Ok(())
}

#[test]
fn test_large_object_as_blob() -> rusqlite::Result<()> {
    let conn = Connection::open_in_memory()?;
    // Store a large json string as a jsonb blob in a table
    conn.execute_batch(r#"
    create table bigdata (
        id integer primary key,
        data blob
    );
    insert into bigdata (id, data) values (
            42, -- the value of the integer primary key is the "rowid" in sqlite
            jsonb_object(
                'my long string', printf('%.*c', 10000000, 'x') -- 10Mb of 'x' characters
            )
    )"#,
    )?;
    let my_blob =
        conn.blob_open(DatabaseName::Main, "bigdata", "data", 42, true)?;
    let parsed: HashMap<String, String> =
        serde_sqlite_jsonb::from_reader(my_blob).unwrap();
    assert_eq!(
        parsed,
        [("my long string".into(), "x".repeat(10_000_000))]
            .into_iter()
            .collect()
    );
    Ok(())
}

#[test]
fn test_roadtrip() {
    // Let's go on a roadtrip. We'll
    // - first serialize an object to jsonb in rust,
    // - then decode it as json in sqlite,
    // - then encode it back to jsonb in sqlite,
    // - and finally decode it back to an object in rust.
    let my_obj: Vec<Person> = vec![
        Person {
            id: 1,
            name: "John Doe".to_string(),
            phone_numbers: vec![
                PhoneNumber::International {
                    country_code: Some(33),
                    number: "1234".to_string(),
                },
                PhoneNumber::Custom(None, "5678".to_string()),
            ],
            is_champion: true,
            data: vec![1, 2, 3],
        },
        Person {
            id: 2,
            name: "Mister Smiley Baily 😊".to_string(),
            phone_numbers: vec![],
            is_champion: false,
            data: (0..u8::MAX).collect(),
        },
    ];
    let encoded = serde_sqlite_jsonb::to_vec(&my_obj).unwrap();
    // start of the trip check: we can go back to the original object
    assert_eq!(
        my_obj,
        serde_sqlite_jsonb::from_slice::<Vec<Person>>(&encoded).unwrap(),
        "we can decode what we encoded"
    );
    // now we go to sqlite
    let conn = Connection::open_in_memory().unwrap();
    let went_through: Vec<u8> = conn
        .query_row(
            "SELECT jsonb(json(?))", // convert jsonb to json and back to jsonb
            [&encoded],
            |row| row.get(0),
        )
        .unwrap();
    // now we go back to rust
    let decoded: Vec<Person> =
        serde_sqlite_jsonb::from_slice(&went_through).unwrap();
    assert_eq!(my_obj, decoded, "went through sqlite and back");
}
