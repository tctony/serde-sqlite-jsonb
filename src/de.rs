// Copyright 2018 Serde Developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::error::{Error, Result};
use crate::header::{ElementType, Header};
use serde::de::{self, Deserialize, IntoDeserializer, SeqAccess, Visitor};
use std::convert::Infallible;
use std::io::Read;

/// A structure that deserializes SQLite JSONB data into Rust values.
pub struct Deserializer<R: Read> {
    /// The reader that the deserializer reads from.
    reader: R,
}

impl<'a> Deserializer<&'a [u8]> {
    /// Deserialize an instance of type `T` from a byte slice of SQLite JSONB data.
    #[allow(clippy::should_implement_trait)]
    pub fn from_bytes(input: &'a [u8]) -> Self {
        Deserializer { reader: input }
    }
}

/// Deserialize an instance of type `T` from a byte slice of SQLite JSONB data.
pub fn from_slice<'a, T>(s: &'a [u8]) -> Result<T>
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

/// Deserialize an instance of type `T` from a byte slice of SQLite JSONB data.
pub fn from_reader<'a, R: Read, T>(reader: R) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer { reader };
    let t = T::deserialize(&mut deserializer)?;
    let Deserializer { mut reader } = deserializer;
    if reader.read(&mut [0])? == 0 {
        Ok(t)
    } else {
        Err(Error::TrailingCharacters)
    }
}

impl<R: Read> Deserializer<R> {
    fn with_header(&mut self, header: Header) -> Deserializer<impl Read + '_> {
        // a little bit of a hack to "unread" a header that was already read
        let header_bytes = std::io::Cursor::new(header.serialize());
        let reader = header_bytes.chain(&mut self.reader);
        Deserializer { reader }
    }

    fn read_header(&mut self) -> Result<Header> {
        /*  The upper four bits of the first byte of the header determine
          - size of the header
          - and possibly also the size of the payload.
        */
        let mut header_0 = [0u8; 1];
        if self.reader.read(&mut header_0)? == 0 {
            return Err(Error::Empty);
        }
        let first_byte = header_0[0];
        let upper_four_bits = first_byte >> 4;
        /*
         If the upper four bits have a value between 0 and 11,
        then the header is exactly one byte in size and the payload size is determined by those upper four bits.

        If the upper four bits have a value between 12 and 15,
        that means that the total header size is 2, 3, 5, or 9 bytes and the payload size is unsigned big-endian integer that is contained in the subsequent bytes.

        The size integer is
          - the one byte that following the initial header byte if the upper four bits are 12,
          - two bytes if the upper bits are 13,
          - four bytes if the upper bits are 14,
          - and eight bytes if the upper bits are 15.
        */
        let bytes_to_read = match upper_four_bits {
            0..=11 => 0,
            12 => 1,
            13 => 2,
            14 => 4,
            15 => 8,
            n => unreachable!("{n} does not fit in four bits"),
        };
        let payload_size: u64 = if bytes_to_read == 0 {
            u64::from(upper_four_bits)
        } else {
            let mut buf = [0u8; 8];
            let start = 8 - bytes_to_read;
            self.reader.read_exact(&mut buf[start..8])?;
            u64::from_be_bytes(buf)
        };
        Ok(Header {
            element_type: ElementType::from(first_byte),
            payload_size,
        })
    }

    fn read_payload_string(&mut self, header: Header) -> Result<String> {
        let mut str = String::with_capacity(header.payload_size as usize);
        let read = self.reader_with_limit(header)?.read_to_string(&mut str)?;
        assert_eq!(read, header.payload_size as usize);
        Ok(str)
    }

    fn drop_payload(&mut self, header: Header) -> Result<ElementType> {
        let mut remaining = header.payload_size;
        while remaining > 0 {
            let mut buf = [0u8; 256];
            let len = buf.len().min(remaining as usize);
            self.reader.read_exact(&mut buf[..len])?;
            remaining -= len as u64;
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

    fn reader_with_limit(&mut self, header: Header) -> Result<impl Read + '_> {
        let limit =
            u64::try_from(header.payload_size).map_err(u64_conversion)?;
        Ok((&mut self.reader).take(limit))
    }

    fn read_json_compatible<T>(&mut self, header: Header) -> Result<T>
    where
        for<'a> T: Deserialize<'a>,
    {
        if header.payload_size <= 8 {
            // micro-optimization: read small payloads into a stack buffer
            let mut buf = [0u8; 8];
            let smallbuf = &mut buf[..header.payload_size as usize];
            self.reader.read_exact(smallbuf)?;
            Ok(crate::json::parse_json_slice(smallbuf)?)
        } else {
            let mut reader = self.reader_with_limit(header)?;
            Ok(crate::json::parse_json(&mut reader)?)
        }
    }

    fn read_json5_compatible<T>(&mut self, header: Header) -> Result<T>
    where
        for<'a> T: Deserialize<'a>,
    {
        let mut reader = self.reader_with_limit(header)?;
        Ok(crate::json::parse_json5(&mut reader)?)
    }

    fn read_json_compatible_string(
        &mut self,
        header: Header,
    ) -> Result<String> {
        let mut reader = read_with_quotes(self.reader_with_limit(header)?);
        Ok(crate::json::parse_json(&mut reader)?)
    }

    fn read_json5_compatible_string(
        &mut self,
        header: Header,
    ) -> Result<String> {
        let mut reader = read_with_quotes(self.reader_with_limit(header)?);
        Ok(crate::json::parse_json5(&mut reader)?)
    }

    fn read_integer<T>(&mut self, header: Header) -> Result<T>
    where
        for<'a> T: Deserialize<'a>,
    {
        match header.element_type {
            ElementType::Int => self.read_json_compatible(header),
            ElementType::Int5 => self.read_json5_compatible(header),
            t => Err(Error::UnexpectedType(t)),
        }
    }

    fn read_string(&mut self, header: Header) -> Result<String> {
        match header.element_type {
            ElementType::Text | ElementType::TextRaw => {
                self.read_payload_string(header)
            }
            ElementType::TextJ => self.read_json_compatible_string(header),
            ElementType::Text5 => self.read_json5_compatible_string(header),
            t => Err(Error::UnexpectedType(t)),
        }
    }

    fn read_float<T>(&mut self, header: Header) -> Result<T>
    where
        for<'a> T: Deserialize<'a>,
    {
        match header.element_type {
            ElementType::Int => self.read_json_compatible(header),
            ElementType::Int5 => self.read_json5_compatible(header),
            ElementType::Float => self.read_json_compatible(header),
            ElementType::Float5 => self.read_json5_compatible(header),
            t => Err(Error::UnexpectedType(t)),
        }
    }

    fn deserialize_any_with_header<'de, V>(
        &mut self,
        header: Header,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match header.element_type {
            ElementType::Null => {
                self.read_null(header)?;
                visitor.visit_unit()
            }
            ElementType::True | ElementType::False => {
                visitor.visit_bool(self.read_bool(header)?)
            }
            ElementType::Float | ElementType::Float5 => {
                visitor.visit_f64(self.read_float(header)?)
            }
            ElementType::Int | ElementType::Int5 => {
                let i: i64 = self.read_integer(header)?;
                if let Ok(x) = u8::try_from(i) {
                    visitor.visit_u8(x)
                } else if let Ok(x) = i8::try_from(i) {
                    visitor.visit_i8(x)
                } else if let Ok(x) = u16::try_from(i) {
                    visitor.visit_u16(x)
                } else if let Ok(x) = i16::try_from(i) {
                    visitor.visit_i16(x)
                } else if let Ok(x) = u32::try_from(i) {
                    visitor.visit_u32(x)
                } else if let Ok(x) = i32::try_from(i) {
                    visitor.visit_i32(x)
                } else if let Ok(x) = u64::try_from(i) {
                    visitor.visit_u64(x)
                } else {
                    visitor.visit_i64(i)
                }
            }
            ElementType::Array => visitor.visit_seq(self),
            ElementType::Object => visitor.visit_map(self),
            ElementType::Text
            | ElementType::TextJ
            | ElementType::Text5
            | ElementType::TextRaw => {
                visitor.visit_string(self.read_string(header)?)
            }
            ElementType::Reserved13
            | ElementType::Reserved14
            | ElementType::Reserved15 => {
                Err(Error::UnexpectedType(header.element_type))
            }
        }
    }
}

fn read_with_quotes(r: impl Read) -> impl Read {
    b"\"".chain(r).chain(&b"\""[..])
}

fn u64_conversion(e: Infallible) -> Error {
    Error::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

impl<'de, 'a, R: Read> de::Deserializer<'de> for &'a mut Deserializer<R> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let header = self.read_header()?;
        self.deserialize_any_with_header(header, visitor)
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let header = self.read_header()?;
        visitor.visit_bool(self.read_bool(header)?)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let header = self.read_header()?;
        visitor.visit_i8(self.read_integer(header)?)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let header = self.read_header()?;
        visitor.visit_i16(self.read_integer(header)?)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let header = self.read_header()?;
        visitor.visit_i32(self.read_integer(header)?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let header = self.read_header()?;
        visitor.visit_i64(self.read_integer(header)?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let header = self.read_header()?;
        visitor.visit_u8(self.read_integer(header)?)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let header = self.read_header()?;
        visitor.visit_u16(self.read_integer(header)?)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let header = self.read_header()?;
        visitor.visit_u32(self.read_integer(header)?)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let header = self.read_header()?;
        visitor.visit_u64(self.read_integer(header)?)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let header = self.read_header()?;
        if header.element_type == ElementType::Null {
            visitor.visit_none()
        } else {
            let mut deser = self.with_header(header);
            visitor.visit_some(&mut deser)
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let header = self.read_header()?;
        self.read_null(header)?;
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let head = self.read_header()?;
        let reader = self.reader_with_limit(head)?;
        let mut seq_deser = Deserializer { reader };
        visitor.visit_seq(&mut seq_deser)
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let head = self.read_header()?;
        let reader = self.reader_with_limit(head)?;
        let mut seq_deser = Deserializer { reader };
        visitor.visit_map(&mut seq_deser)
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let header = self.read_header()?;
        match header.element_type {
            ElementType::Text
            | ElementType::TextJ
            | ElementType::Text5
            | ElementType::TextRaw => {
                let s = self.read_string(header)?;
                visitor.visit_enum(s.into_deserializer())
            }
            ElementType::Object => {
                let reader = self.reader_with_limit(header)?;
                let mut de = Deserializer { reader };
                let r = visitor.visit_enum(&mut de);
                if de.reader.read(&mut [0])? == 0 {
                    r
                } else {
                    Err(Error::TrailingCharacters)
                }
            }
            other => Err(Error::UnexpectedType(other)),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let header = self.read_header()?;
        self.drop_payload(header)?;
        visitor.visit_unit()
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let header = self.read_header()?;
        visitor.visit_f32(self.read_float(header)?)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let header = self.read_header()?;
        visitor.visit_f64(self.read_float(header)?)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let header = self.read_header()?;
        let s = self.read_string(header)?;
        if s.len() != 1 {
            return Err(Error::Message(
                "invalid string length for char".into(),
            ));
        }
        visitor.visit_char(s.chars().next().unwrap())
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // Borrowed string deserialization is not supported
        self.deserialize_string(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let header = self.read_header()?;
        visitor.visit_string(self.read_string(header)?)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }
}

impl<'de, 'a, R: Read> de::SeqAccess<'de> for &'a mut Deserializer<R> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: de::DeserializeSeed<'de>,
    {
        match seed.deserialize(&mut **self) {
            Ok(v) => Ok(Some(v)),
            Err(Error::Empty) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

impl<'de, 'a, R: Read> de::MapAccess<'de> for &'a mut Deserializer<R> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: de::DeserializeSeed<'de>,
    {
        self.next_element_seed(seed)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: de::DeserializeSeed<'de>,
    {
        self.next_element_seed(seed)
            .and_then(|opt| opt.ok_or_else(|| Error::Empty))
    }
}

impl<'de, 'a, R: Read> de::EnumAccess<'de> for &'a mut Deserializer<R> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: de::DeserializeSeed<'de>,
    {
        let val = seed.deserialize(&mut *self)?;
        Ok((val, self))
    }
}

impl<'de, 'a, R: Read> de::VariantAccess<'de> for &'a mut Deserializer<R> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: de::DeserializeSeed<'de>,
    {
        seed.deserialize(self)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_seq(self, visitor)
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        de::Deserializer::deserialize_map(self, visitor)
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn assert_header(bytes: &[u8], expected: Header) {
        let mut de = Deserializer::from_bytes(bytes);
        let header = de.read_header().unwrap();
        assert_eq!(header, expected);
    }

    #[test]
    fn test_read_header() {
        assert_header(
            &[0b_0000_0000],
            Header {
                element_type: ElementType::Null,
                payload_size: 0,
            },
        );
        assert_header(
            &[0b_0000_0001],
            Header {
                element_type: ElementType::True,
                payload_size: 0,
            },
        );
        assert_header(
            &[0b_0000_0010],
            Header {
                element_type: ElementType::False,
                payload_size: 0,
            },
        );
        assert_header(
            &[0b_1100_0011, 0xFA],
            Header {
                element_type: ElementType::Int,
                payload_size: 0xFA,
            },
        );
        assert_header(
            b"\xf3\x00\x00\x00\x00\x00\x00\x00\x01",
            Header {
                element_type: ElementType::Int,
                payload_size: 1,
            },
        );
        assert_header(
            b"\xbb",
            Header {
                element_type: ElementType::Array,
                payload_size: 11,
            },
        );
    }

    fn assert_all_int_types_eq(encoded: &[u8], expected: i64) {
        // unsigned
        assert_eq!(
            from_slice::<i8>(encoded).unwrap(),
            expected as i8,
            "parsing {encoded:?} as i8"
        );
        assert_eq!(from_slice::<i16>(encoded).unwrap(), expected as i16);
        assert_eq!(from_slice::<i32>(encoded).unwrap(), expected as i32);
        assert_eq!(from_slice::<i64>(encoded).unwrap(), expected);
        // signed
        assert_eq!(from_slice::<u8>(encoded).unwrap(), expected as u8);
        assert_eq!(from_slice::<u16>(encoded).unwrap(), expected as u16);
        assert_eq!(from_slice::<u32>(encoded).unwrap(), expected as u32);
        assert_eq!(from_slice::<u64>(encoded).unwrap(), expected as u64);
    }

    #[test]
    fn test_decoding_1() {
        /* From the spec:
        The header for an element does not need to be in its simplest form. For example, consider the JSON numeric value "1". That element can be encode in five different ways:
           0x13 0x31
           0xc3 0x01 0x31
           0xd3 0x00 0x01 0x31
           0xe3 0x00 0x00 0x00 0x01 0x31
           0xf3 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x01 0x31
        */
        assert_all_int_types_eq(b"\x13\x31", 1);
        assert_all_int_types_eq(b"\xc3\x01\x31", 1);
        assert_all_int_types_eq(b"\xd3\x00\x01\x31", 1);
        assert_all_int_types_eq(b"\xe3\x00\x00\x00\x01\x31", 1);
        assert_all_int_types_eq(b"\xf3\x00\x00\x00\x00\x00\x00\x00\x01\x31", 1);
        assert_all_int_types_eq(b"\xc3\x03127", 127);
    }

    #[test]
    fn test_decoding_large_int() {
        assert_eq!(
            from_slice::<u64>(b"\xc3\xf418446744073709551615").unwrap(),
            18446744073709551615
        );
        // large negative i64
        assert_eq!(
            from_slice::<i64>(b"\xc3\xf5-9223372036854775808").unwrap(),
            -9223372036854775808
        );
    }

    #[test]
    fn test_decoding_large_float() {
        // large negative i64
        assert_eq!(
            from_slice::<f64>(b"\xc5\x0c-0.123456789").unwrap(),
            -0.123456789
        );
    }

    #[test]
    fn test_decoding_int_as_float() {
        // large negative i64
        assert_eq!(from_slice::<f32>(b"\xc3\x0512345").unwrap(), 12345.);
    }

    #[test]
    fn test_null() {
        from_slice::<()>(b"\x00").unwrap();
    }

    #[test]
    fn test_option() {
        assert_eq!(from_slice::<Option<u64>>(b"\x00").unwrap(), None);
        assert_eq!(from_slice::<Option<Vec<u8>>>(b"\x00").unwrap(), None);
        assert_eq!(from_slice::<Option<u8>>(b"\x2342").unwrap(), Some(42));
    }

    #[test]
    fn test_string_noescape() {
        assert_eq!(from_slice::<String>(b"\x57hello").unwrap(), "hello");
    }

    #[test]
    fn test_string_json_escape() {
        assert_eq!(from_slice::<String>(b"\x28\\n").unwrap(), "\n");
    }

    #[test]
    #[cfg(feature = "serde_json5")]
    fn test_string_json5_escape() {
        assert_eq!(from_slice::<String>(b"\x49\\x0A").unwrap(), "\n");
    }

    #[test]
    fn test_tuple() {
        assert_eq!(
            from_slice::<(u8, i64, char)>(b"\x6b\x131\x132\x18x").unwrap(),
            (1, 2, 'x')
        );
    }

    #[test]
    fn test_tuple_struct() {
        #[derive(Debug, PartialEq, serde_derive::Deserialize)]
        struct Test(Option<String>, bool, bool);
        assert_eq!(
            from_slice::<Test>(b"\x3b\x00\x01\x02").unwrap(),
            Test(None, true, false)
        );
    }

    #[test]
    fn test_vec() {
        assert_eq!(from_slice::<Vec<()>>(b"\x0b").unwrap(), vec![]);
        assert_eq!(
            from_slice::<Vec<u8>>(b"\x4b\x131\x132").unwrap(),
            vec![1, 2]
        );
    }

    #[test]
    fn test_vec_opts() {
        assert_eq!(
            from_slice::<Vec<Option<String>>>(b"\xbb\x471234\x00\x475678")
                .unwrap(),
            vec![Some("1234".to_string()), None, Some("5678".to_string())]
        );
    }

    #[test]
    fn test_vec_with_reader() {
        assert_eq!(from_reader::<_, Vec<()>>(&b"\x0b"[..]).unwrap(), vec![]);
    }

    #[test]
    fn test_vec_of_vecs() {
        assert_eq!(
            from_slice::<Vec<Vec<i16>>>(
                b"\xcb\x0a\x4b\x131\x132\x4b\x133\x134"
            )
            .unwrap(),
            vec![vec![1, 2], vec![3, 4]]
        );
    }

    #[test]
    fn test_hashmap() {
        use std::collections::HashMap;
        let actual =
            from_slice::<HashMap<String, bool>>(b"\x6c\x17a\x02\x17b\x01")
                .unwrap();
        let expected = [("a".into(), false), ("b".into(), true)]
            .into_iter()
            .collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_struct() {
        #[derive(Debug, PartialEq, serde_derive::Deserialize)]
        struct Test {
            a: bool,
            b: bool,
        }
        let actual = from_slice::<Test>(b"\x6c\x17a\x02\x17b\x01").unwrap();
        let expected = Test { a: false, b: true };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_complex_struct() {
        let bytes = b"\xcc\x3a\x27id\x131\x47name\x87John Doe\xc7\x0dphone_numbers\xbb\x471234\x00\x475678\x47data\x6b\x131\x132\x133";
        let mut deser = Deserializer::from_bytes(bytes);
        #[derive(Debug, PartialEq, serde_derive::Deserialize)]
        struct Person {
            id: i32,
            name: String,
            phone_numbers: Vec<Option<String>>,
            data: Vec<u8>,
        }
        let person: Person = Person::deserialize(&mut deser).unwrap();
        assert_eq!(
            person,
            Person {
                id: 1,
                name: "John Doe".to_string(),
                phone_numbers: vec![
                    Some("1234".to_string()),
                    None,
                    Some("5678".to_string())
                ],
                data: vec![1, 2, 3]
            }
        );
    }

    #[test]
    fn test_basic_enum() {
        #[derive(Debug, PartialEq, serde_derive::Deserialize)]
        enum Test {
            X,
            Y,
        }
        let actual: Vec<Test> = from_slice(b"\x4b\x18X\x18Y").unwrap();
        let expected = vec![Test::X, Test::Y];
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_externally_tagged_enum() {
        #[derive(Debug, PartialEq, serde_derive::Deserialize)]
        enum Test {
            X(String),
            Y(bool),
        }
        // {"X": "Y"}
        let actual: Test = from_slice(b"\x4c\x18X\x18Y").unwrap();
        let expected = Test::X("Y".to_string());
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_externally_tagged_enum_too_large() {
        #[derive(Debug, PartialEq, serde_derive::Deserialize)]
        enum Test {
            X(char),
            Y(char),
        }
        assert_eq!(
            from_slice::<Vec<Test>>(b"\x9b\x8c\x18X\x18Y\x18Y\x18A")
                .unwrap_err()
                .to_string(),
            Error::TrailingCharacters.to_string()
        );
    }
}
