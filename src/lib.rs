#![warn(clippy::pedantic)]

mod de;
mod error;
mod header;
mod json;
mod ser;

pub use crate::de::{from_reader, from_slice, Deserializer};
pub use crate::error::{Error, Result};
pub use crate::ser::{to_vec, to_vec_with_options, Options, Serializer};
