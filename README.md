# serde-sqlite-jsonb

This crate provides a custom Serde deserializer for SQLite JSONB columns.

## Crate features

The binary format can contain raw json data, so this crate depends on the `serde_json` crate to parse the JSON data.
Since SQLite also supports json5, the `serde-json5` feature can be used if json5 support is needed.

By default, the (faster) `serde_json` feature is enabled and this crate returns an error when trying to parse json5 data.
To enable json5 support, disable the default features and enable the `serde-json5` feature:

```toml
[dependencies]
serde-sqlite-jsonb = { version = "0.1", features = ["serde-json5"], default-features = false }
```

## Format

The format of the JSONB column is described in the SQLite documentation:
https://sqlite.org/draft/jsonb.html

The data format is a binary format with a header and payload. The header contains information about the type of the element and the size of the payload. The payload contains the actual data.

Here's a rough ASCII representation:

```
bits:  0  1  2  3  4  5  6  7  8
    +-------------+-------------+
    |  size(4)    | type(4)     | first header byte
    +-------------+-------------+
    |   payload size (0 - 64)   | header bytes number 2 to 9
    +---------------------------+
    |   payload data            | payload bytes (JSON strings or numbers in text format)
    +---------------------------+
```

### Header

#### Payload size

 - If the payload data is between 0 and 11 bytes (inclusive), the size is encoded in the first 4 bits of the header.
 - If the payload data is between 12 and 2^8 - 1 bytes (inclusive), the size is encoded in the next byte and the first 4 bits of the header are set to 12.
 - If the payload data is between 2^8 and 2^16 - 1 bytes (inclusive), the size is encoded in the next two bytes and the first 4 bits of the header are set to 13.
 - If the payload data is between 2^16 and 2^32 - 1 bytes (inclusive), the size is encoded in the next four bytes and the first 4 bits of the header are set to 14.
 - If the payload data is between 2^32 and 2^64 - 1 bytes (inclusive), the size is encoded in the next eight bytes and the first 4 bits of the header are set to 15.

#### Type

- `Null` (0x0): The element is a JSON "null".
- `True` (0x1): The element is a JSON "true".
- `False` (0x2): The element is a JSON "false".
- `Int` (0x3): The element is a JSON integer value in the canonical RFC 8259 format.
- `Int5` (0x4): The element is a JSON integer value that is not in the canonical format.
- `Float` (0x5): The element is a JSON floating-point value in the canonical RFC 8259 format.
- `Float5` (0x6): The element is a JSON floating-point value that is not in the canonical format.
- `Text` (0x7): The element is a JSON string value that does not contain any escapes.
- `TextJ` (0x8): The element is a JSON string value that contains RFC 8259 character escapes.
- `Text5` (0x9): The element is a JSON string value that contains character escapes, including some from JSON5.
- `TextRaw` (0xA): The element is a JSON string value that contains UTF8 characters that need to be escaped.
- `Array` (0xB): The element is a JSON array.
- `Object` (0xC): The element is a JSON object.
- `Reserved13` (0xD): Reserved for future expansion.
- `Reserved14` (0xE): Reserved for future expansion.
- `Reserved15` (0xF): Reserved for future expansion.

#### Example

The following JSON object:
```json
{"a": false, "b":true}
```
is encoded as the following 7 bytes of binary data:

```
6c 17 61 02 17 62 01
```

byte | value | description
-----|-------|-------------
0    | 0x6c  | header: payload size = 6, type = Object (0xC)
1    | 0x17  | header: payload size = 1, type = Text (0x7)
2    | 0x61  | payload: 'a'
3    | 0x02  | header: payload size = 0, type = False (0x2)
4    | 0x17  | header: payload size = 1, type = Text (0x7)
5    | 0x62  | payload: 'b'
6    | 0x01  | header: payload size = 0, type = True (0x1)


## MSRV

Requires rust >= 1.63 (debian stable)