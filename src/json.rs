#[cfg(feature = "serde_json")]
pub(crate) use serde_json::from_reader as parse_json;
#[cfg(feature = "serde_json")]
pub(crate) type JsonError = serde_json::Error;

#[cfg(not(feature = "serde_json"))]
pub(crate) use serde_json5::from_reader as parse_json;
#[cfg(not(feature = "serde_json"))]
pub(crate) type JsonError = serde_json5::Error;

#[cfg(feature = "serde_json5")]
pub(crate) use serde_json5::from_reader as parse_json5;

#[cfg(not(feature = "serde_json5"))]
pub(crate) fn parse_json5<I, T>(_input: I) -> crate::Result<T> {
    Err(crate::Error::Json5Error(Json5Error))
}

#[cfg(feature = "serde_json5")]
pub(crate) type Json5Error = serde_json5::Error;
#[cfg(not(feature = "serde_json5"))]
#[derive(Debug)]
pub struct Json5Error;

#[cfg(not(feature = "serde_json5"))]
impl std::fmt::Display for Json5Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Json5 data was encountered, but json5 support is not enabled. Enable the `serde_json5` feature of the serde-sqlite-jsonb crate to enable support for json5 data.")
    }
}

#[cfg(not(feature = "serde_json5"))]
impl std::error::Error for Json5Error {}
