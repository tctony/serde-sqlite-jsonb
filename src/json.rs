#[cfg(feature = "serde_json5")]
type Error = serde_json5::Error;
#[cfg(not(feature = "serde_json5"))]
pub(crate) type Error = serde_json::Error;

#[cfg(feature = "serde_json")]
pub(crate) use serde_json::from_reader as parse_json;

#[cfg(not(feature = "serde_json"))]
pub(crate) use serde_json5::from_reader as parse_json;

#[cfg(feature = "serde_json5")]
pub(crate) use serde_json5::from_reader as parse_json5;

#[cfg(not(feature = "serde_json5"))]
pub(crate) fn parse_json5<I, T>(_input: I) -> crate::Result<T> {
    Err(crate::Error::NeedsJson5)
}
