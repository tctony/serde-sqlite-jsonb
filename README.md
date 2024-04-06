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