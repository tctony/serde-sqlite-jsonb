use rusqlite::{Connection, DatabaseName};
use serde_derive::Deserialize;

#[derive(Debug, PartialEq, Deserialize)]
struct Person {
    id: i32,
    name: String,
    phone_numbers: Vec<Option<String>>,
    is_champion: bool,
    data: Vec<u8>,
}

#[test]
fn test_fetch_json_object() -> rusqlite::Result<()> {
    let conn = Connection::open_in_memory()?;
    let blob: Vec<u8> = conn.query_row(
        r#"select jsonb('{
        "id": 1,
        "name": "John Doe",
        "phone_numbers": ["1234", null, "567"],
        "is_champion": true,
        "data": [1, 2, 3]
    }')"#,
        [],
        |row| row.get(0),
    )?;
    let person: Person = serde_sqlite_jsonb::from_bytes(&blob).unwrap();
    assert_eq!(
        person,
        Person {
            id: 1,
            name: "John Doe".to_string(),
            phone_numbers: vec![
                Some("1234".to_string()),
                None,
                Some("567".to_string())
            ],
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
    let parsed: serde_json::Value =
        serde_sqlite_jsonb::from_reader(my_blob).unwrap();
    assert_eq!(
        parsed,
        serde_json::json!({
            "my long string": "x".repeat(10_000_000)
        })
    );
    Ok(())
}
