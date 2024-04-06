use serde::{de, ser};
use std::fmt::{self, Display};

use crate::de::ElementType;

pub type Result<T> = std::result::Result<T, Error>;

/// An error that can be produced during parsing.
#[derive(Debug)]
pub enum Error {
    Message(String),
    JsonError(crate::json::JsonError),
    Json5Error(crate::json::Json5Error),
    InvalidElementType(u8),
    UnexpectedType(ElementType),
    Io(std::io::Error),
    TrailingCharacters,
    Utf8(std::string::FromUtf8Error),
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Message(m) => write!(f, "{}", m),
            Error::JsonError(_) => write!(f, "json error"),
            Error::Json5Error(_) => write!(f, "json5 error"),
            Error::InvalidElementType(t) => {
                write!(f, "{t} is not a valid jsonb element type code")
            }
            Error::UnexpectedType(t) => write!(f, "unexpected type: {t:?}"),
            Error::Io(_) => write!(f, "io error"),
            Error::TrailingCharacters => {
                write!(f, "trailing data after the end of the jsonb value")
            }
            Error::Utf8(_) => write!(f, "invalid utf8 in string"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::JsonError(e) => Some(e),
            Error::Json5Error(e) => Some(e),
            Error::Io(e) => Some(e),
            Error::Utf8(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::Io(err)
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(err: std::string::FromUtf8Error) -> Self {
        Error::Utf8(err)
    }
}

#[cfg(feature = "serde_json")]
impl From<crate::json::JsonError> for Error {
    fn from(err: crate::json::JsonError) -> Error {
        Error::JsonError(err)
    }
}

impl From<crate::json::Json5Error> for Error {
    fn from(err: crate::json::Json5Error) -> Error {
        Error::Json5Error(err)
    }
}
