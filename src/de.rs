// Copyright 2018 Serde Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::error::{Error, Result};
use serde::de::{
    self, Deserialize, DeserializeSeed, EnumAccess, IntoDeserializer,
    MapAccess, SeqAccess, VariantAccess, Visitor,
};
use std::{
    io::Read,
    ops::{AddAssign, MulAssign, Neg},
};

pub struct Deserializer<R: Read> {
    // This string starts with the input data and characters are truncated off
    // the beginning as data is parsed.
    reader: R,
}

impl<'a> Deserializer<&'a [u8]> {
    // By convention, `Deserializer` constructors are named like `from_xyz`.
    // That way basic use cases are satisfied by something like
    // `serde_json::from_str(...)` while advanced use cases that require a
    // deserializer can make one with `serde_json::Deserializer::from_str(...)`.
    #[allow(clippy::should_implement_trait)]
    pub fn from_bytes(input: &'a [u8]) -> Self {
        Deserializer { reader: input }
    }
}

// By convention, the public API of a Serde deserializer is one or more
// `from_xyz` methods such as `from_str`, `from_bytes`, or `from_reader`
// depending on what Rust types the deserializer is able to consume as input.
//
// This basic deserializer supports only `from_str`.
pub fn from_bytes<'a, T>(s: &'a [u8]) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_bytes(s);
    let t = T::deserialize(&mut deserializer)?;
    if deserializer.reader.is_empty() {
        Ok(t)
    } else {
        Err(Error::TrailingCharacters)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementType {
    Null,
    True,
    False,
    Int,
    Int5,
    Float,
    Float5,
    Text,
    TextJ,
    Text5,
    TextRaw,
    Array,
    Object,
    Reserved13,
    Reserved14,
    Reserved15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    element_type: ElementType,
    payload_size: usize,
}

impl<R: Read> Deserializer<R> {
    fn read_header(&mut self) -> Result<Header> {
        let mut buf = [0; 1];
        self.reader.read_exact(&mut buf)?;
        let byte = buf[0];
        let element_type = match byte & 0x0f {
            0 => ElementType::Null,
            1 => ElementType::True,
            2 => ElementType::False,
            3 => ElementType::Int,
            4 => ElementType::Int5,
            5 => ElementType::Float,
            6 => ElementType::Float5,
            7 => ElementType::Text,
            8 => ElementType::TextJ,
            9 => ElementType::Text5,
            10 => ElementType::TextRaw,
            11 => ElementType::Array,
            12 => ElementType::Object,
            n => return Err(Error::InvalidElementType(n)),
        };
        let payload_size = (byte >> 4) as usize;
        Ok(Header {
            element_type,
            payload_size,
        })
    }

    fn read_header_with_payload(&mut self) -> Result<(ElementType, Vec<u8>)> {
        let header = self.read_header()?;
        let mut buf = vec![0; header.payload_size];
        self.reader.read_exact(&mut buf)?;
        Ok((header.element_type, buf))
    }

    fn drop_payload(&mut self, header: Header) -> Result<ElementType> {
        let mut remaining = header.payload_size;
        while remaining > 0 {
            let mut buf = [0u8; 256];
            let len = buf.len().min(remaining);
            self.reader.read_exact(&mut buf[..len])?;
            remaining -= len;
        }
        Ok(header.element_type)
    }

    fn read_bool(&mut self, header: Header) -> Result<bool> {
        self.drop_payload(header)?;
        match header.element_type {
            ElementType::True => Ok(true),
            ElementType::False => Ok(false),
            t => Err(Error::UnexpectedType(t)),
        }
    }

    fn read_null(&mut self, header: Header) -> Result<()> {
        self.drop_payload(header)?;
        match header.element_type {
            ElementType::Null => Ok(()),
            t => Err(Error::UnexpectedType(t)),
        }
    }

    fn read_json_compatible<T>(self, header: Header) -> Result<T> where for<'a> T: Deserialize<'a> {
        let limit = u64::try_from(header.payload_size).map_err(|_| Error::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, "payload size too large")))?;
        let mut reader = self.reader.take(limit);
        crate::json::parse_json(&mut reader)
    }

    fn read_integer<T>(self, header: Header) -> Result<T> where for<'a> T: Deserialize<'a> {
        match header.element_type {
            ElementType::Int => {},
            ElementType::Int5 => crate::json::assert_json5_supported()?,
            t => return Err(Error::UnexpectedType(t)),
        };
        self.read_json_compatible(header)
    }
}

impl<'de, 'a, R: Read> de::Deserializer<'de> for &'a mut Deserializer<R> {
    type Error = Error;

    fn deserialize_any<V>(
        self,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let header = self.read_header()?;
        match header.element_type {
            ElementType::Null => {
                self.read_null(header)?;
                visitor.visit_unit()
            },
            ElementType::True | ElementType::False => {
                let b = self.read_bool(header)?;
                visitor.visit_bool(b)
            },
            e => todo!("deserialize any for {:?}", e),
        }
    }

    fn deserialize_bool<V>(
        self,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let header = self.read_header()?;
        visitor.visit_bool(self.read_bool(header)?)
    }

    fn deserialize_i8<V>(
        self,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let header = self.read_header()?;
        let i = self.read_integer(header);
        visitor.visit_i8(i?)
    }

    fn deserialize_i16<V>(
        self,
        visitor: V,
    ) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_i32<V>(
        self,
        visitor: V,
    ) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_i64<V>(
        self,
        visitor: V,
    ) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_u8<V>(
        self,
        visitor: V,
    ) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_u16<V>(
        self,
        visitor: V,
    ) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_u32<V>(
        self,
        visitor: V,
    ) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_u64<V>(
        self,
        visitor: V,
    ) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_f32<V>(
        self,
        visitor: V,
    ) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_f64<V>(
        self,
        visitor: V,
    ) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_char<V>(
        self,
        visitor: V,
    ) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_str<V>(
        self,
        visitor: V,
    ) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_string<V>(
        self,
        visitor: V,
    ) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_bytes<V>(
        self,
        visitor: V,
    ) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_byte_buf<V>(
        self,
        visitor: V,
    ) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_option<V>(
        self,
        visitor: V,
    ) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_unit<V>(
        self,
        visitor: V,
    ) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_seq<V>(
        self,
        visitor: V,
    ) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_tuple<V>(
        self,
        len: usize,
        visitor: V,
    ) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_map<V>(
        self,
        visitor: V,
    ) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_identifier<V>(
        self,
        visitor: V,
    ) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }

    fn deserialize_ignored_any<V>(
        self,
        visitor: V,
    ) -> std::prelude::v1::Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decoding_1() {
        assert_eq!(from_bytes::<u8>(b"\x13\x31").unwrap(), 1);
    }
}
