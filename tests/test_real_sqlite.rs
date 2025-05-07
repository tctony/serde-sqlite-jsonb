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

#[derive(Debug, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type")]
enum Shape {
    Circle { radius: f64 },
    Rectangle { width: f64, height: f64 },
    Square { size: f64 },
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
struct Drawing {
    shapes: Vec<Shape>,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
struct Point {
    x: i32,
    y: i32,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
enum Color {
    Red,
    Green,
    Blue,
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
enum Animal {
    Dog { name: String },
    Cat { name: String },
    Bird { species: String },
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

#[test]
fn test_internally_tagged_enum() -> rusqlite::Result<()> {
    let conn = Connection::open_in_memory()?;
    let blob: Vec<u8> = conn.query_row(
        r#"select jsonb('{
            "shapes": [
                {"type": "Circle", "radius": 5.0},
                {"type": "Rectangle", "width": 10.0, "height": 20.0},
                {"type": "Square", "size": 15.0}
            ]
        }')"#,
        [],
        |row| row.get(0),
    )?;
    let drawing: Drawing = serde_sqlite_jsonb::from_slice(&blob).unwrap();
    assert_eq!(
        drawing,
        Drawing {
            shapes: vec![
                Shape::Circle { radius: 5.0 },
                Shape::Rectangle {
                    width: 10.0,
                    height: 20.0
                },
                Shape::Square { size: 15.0 }
            ]
        }
    );

    Ok(())
}

#[test]
fn test_internally_tagged_enum_single_shape() -> rusqlite::Result<()> {
    let conn = Connection::open_in_memory()?;
    let blob: Vec<u8> = conn.query_row(
        r#"select jsonb('{
            "shapes": [
                {"type": "Circle", "radius": 5.0}
            ]
        }')"#,
        [],
        |row| row.get(0),
    )?;
    let drawing: Drawing = serde_sqlite_jsonb::from_slice(&blob).unwrap();
    assert_eq!(
        drawing,
        Drawing {
            shapes: vec![Shape::Circle { radius: 5.0 },]
        }
    );

    Ok(())
}

#[test]
fn test_internally_tagged_enum_two_shapes() -> rusqlite::Result<()> {
    let conn = Connection::open_in_memory()?;
    let blob: Vec<u8> = conn.query_row(
        r#"select jsonb('{
            "shapes": [
                {"type": "Circle", "radius": 5.0},
                {"type": "Rectangle", "width": 10.0, "height": 20.0}
            ]
        }')"#,
        [],
        |row| row.get(0),
    )?;
    let drawing: Drawing = serde_sqlite_jsonb::from_slice(&blob).unwrap();
    assert_eq!(
        drawing,
        Drawing {
            shapes: vec![
                Shape::Circle { radius: 5.0 },
                Shape::Rectangle {
                    width: 10.0,
                    height: 20.0
                },
            ]
        }
    );

    Ok(())
}

#[test]
fn test_shapes_array_directly() -> rusqlite::Result<()> {
    let conn = Connection::open_in_memory()?;
    let blob: Vec<u8> = conn.query_row(
        r#"select jsonb('[
            {"type": "Circle", "radius": 5.0},
            {"type": "Rectangle", "width": 10.0, "height": 20.0},
            {"type": "Square", "size": 15.0}
        ]')"#,
        [],
        |row| row.get(0),
    )?;
    let shapes: Vec<Shape> = serde_sqlite_jsonb::from_slice(&blob).unwrap();
    assert_eq!(
        shapes,
        vec![
            Shape::Circle { radius: 5.0 },
            Shape::Rectangle {
                width: 10.0,
                height: 20.0
            },
            Shape::Square { size: 15.0 }
        ]
    );

    Ok(())
}

#[test]
fn test_simple_int_array() -> rusqlite::Result<()> {
    let conn = Connection::open_in_memory()?;
    let blob: Vec<u8> =
        conn.query_row(r#"select jsonb('[1, 2, 3]')"#, [], |row| row.get(0))?;
    let values: Vec<i32> = serde_sqlite_jsonb::from_slice(&blob).unwrap();
    assert_eq!(values, vec![1, 2, 3]);

    Ok(())
}

#[test]
fn test_string_array() -> rusqlite::Result<()> {
    let conn = Connection::open_in_memory()?;
    let blob: Vec<u8> = conn.query_row(
        r#"select jsonb('["one", "two", "three"]')"#,
        [],
        |row| row.get(0),
    )?;
    let values: Vec<String> = serde_sqlite_jsonb::from_slice(&blob).unwrap();
    assert_eq!(
        values,
        vec!["one".to_string(), "two".to_string(), "three".to_string()]
    );

    Ok(())
}

#[test]
fn test_object_array() -> rusqlite::Result<()> {
    let conn = Connection::open_in_memory()?;
    let blob: Vec<u8> = conn.query_row(
        r#"select jsonb('[
            {"x": 1, "y": 2},
            {"x": 3, "y": 4},
            {"x": 5, "y": 6}
        ]')"#,
        [],
        |row| row.get(0),
    )?;
    let values: Vec<Point> = serde_sqlite_jsonb::from_slice(&blob).unwrap();
    assert_eq!(
        values,
        vec![
            Point { x: 1, y: 2 },
            Point { x: 3, y: 4 },
            Point { x: 5, y: 6 },
        ]
    );

    Ok(())
}

#[test]
fn test_normal_enum_array() -> rusqlite::Result<()> {
    let conn = Connection::open_in_memory()?;
    let blob: Vec<u8> = conn.query_row(
        r#"select jsonb('["Red", "Green", "Blue"]')"#,
        [],
        |row| row.get(0),
    )?;
    let values: Vec<Color> = serde_sqlite_jsonb::from_slice(&blob).unwrap();
    assert_eq!(values, vec![Color::Red, Color::Green, Color::Blue,]);

    Ok(())
}

#[test]
fn test_externally_tagged_enum_array() -> rusqlite::Result<()> {
    let conn = Connection::open_in_memory()?;
    let blob: Vec<u8> = conn.query_row(
        r#"select jsonb('[
            {"Dog": {"name": "Fido"}},
            {"Cat": {"name": "Whiskers"}},
            {"Bird": {"species": "Parrot"}}
        ]')"#,
        [],
        |row| row.get(0),
    )?;
    let values: Vec<Animal> = serde_sqlite_jsonb::from_slice(&blob).unwrap();
    assert_eq!(
        values,
        vec![
            Animal::Dog {
                name: "Fido".to_string()
            },
            Animal::Cat {
                name: "Whiskers".to_string()
            },
            Animal::Bird {
                species: "Parrot".to_string()
            },
        ]
    );

    Ok(())
}

#[test]
fn test_print_test() -> rusqlite::Result<()> {
    let conn = Connection::open_in_memory()?;
    let blob: Vec<u8> = conn.query_row(
        r#"select jsonb('[{"t": "A"}, {"t": "B"}]')"#,
        [],
        |row| row.get(0),
    )?;
    assert_eq!(
        blob, b"\xab\x4c\x17\x74\x17\x41\x4c\x17\x74\x17\x42",
        "{:x?}",
        &blob
    );

    Ok(())
}
