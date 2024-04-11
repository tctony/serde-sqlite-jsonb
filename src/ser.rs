use crate::{
    error::{Error, Result},
    header::ElementType,
};
use serde::ser::{self, Serialize};
use std::io::Write;

#[derive(Debug, Default)]
pub struct Serializer {
    buffer: Vec<u8>,
}

/// Serialize a value into a JSONB byte array
pub fn to_vec<T>(value: &T) -> Result<Vec<u8>>
where
    T: Serialize,
{
    let mut serializer = Serializer::default();
    value.serialize(&mut serializer)?;
    Ok(serializer.buffer)
}

/// Helper struct to write JSONB data, then finalize the header to its minimal size
pub struct JsonbWriter<'a> {
    buffer: &'a mut Vec<u8>,
    header_start: usize,
}

impl<'a> JsonbWriter<'a> {
    fn new(buffer: &'a mut Vec<u8>, element_type: ElementType) -> Self {
        let header_start = buffer.len();
        buffer.extend_from_slice(&[u8::from(element_type); 9]);
        Self {
            buffer,
            header_start,
        }
    }
    fn finalize(self) {
        let data_start = self.header_start + 9;
        let data_end = self.buffer.len();
        let payload_size = data_end - data_start;
        let header = &mut self.buffer[self.header_start..self.header_start + 9];
        let head_len = if payload_size <= 11 {
            header[0] |= (payload_size as u8) << 4;
            1
        } else if payload_size <= 0xff {
            header[0] |= 0xc0;
            header[1] = payload_size as u8;
            2
        } else if payload_size <= 0xffff {
            header[0] |= 0xd0;
            header[1..3].copy_from_slice(&(payload_size as u16).to_be_bytes());
            3
        } else if payload_size <= 0xffffffff {
            header[0] |= 0xe0;
            header[1..5].copy_from_slice(&(payload_size as u32).to_be_bytes());
            5
        } else {
            header[0] |= 0xf0;
            header[1..9].copy_from_slice(&payload_size.to_be_bytes());
            9
        };
        if head_len < 9 {
            self.buffer.copy_within(
                data_start..data_end,
                self.header_start + head_len,
            );
            self.buffer
                .truncate(self.header_start + head_len + payload_size);
        }
    }
}

impl Serializer {
    fn write_header_nodata(&mut self, element_type: ElementType) -> Result<()> {
        self.buffer.push(u8::from(element_type));
        Ok(())
    }

    fn write_displayable(
        &mut self,
        element_type: ElementType,
        data: impl std::fmt::Display,
    ) -> Result<()> {
        let mut w = JsonbWriter::new(&mut self.buffer, element_type);
        write!(&mut w.buffer, "{}", data)?;
        w.finalize();
        Ok(())
    }
}

impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = ();

    type Error = Error;

    type SerializeSeq = JsonbWriter<'a>;

    type SerializeTuple = JsonbWriter<'a>;

    type SerializeTupleStruct = JsonbWriter<'a>;

    type SerializeTupleVariant = TupleVariantSerializer<'a>;

    type SerializeMap = JsonbWriter<'a>;

    type SerializeStruct = JsonbWriter<'a>;

    type SerializeStructVariant = JsonbWriter<'a>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok> {
        self.write_header_nodata(if v {
            ElementType::True
        } else {
            ElementType::False
        })
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok> {
        self.write_displayable(ElementType::Int, v)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok> {
        self.write_displayable(ElementType::Int, v)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok> {
        self.write_displayable(ElementType::Int, v)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok> {
        self.write_displayable(ElementType::Int, v)
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok> {
        self.write_displayable(ElementType::Int, v)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok> {
        self.write_displayable(ElementType::Int, v)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok> {
        self.write_displayable(ElementType::Int, v)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok> {
        self.write_displayable(ElementType::Int, v)
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok> {
        self.write_displayable(ElementType::Float, v)
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok> {
        self.write_displayable(ElementType::Float, v)
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok> {
        self.write_displayable(ElementType::TextRaw, v)
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok> {
        self.write_displayable(ElementType::TextRaw, v)
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok> {
        use serde::ser::SerializeSeq;
        let mut s = self.serialize_seq(Some(v.len()))?;
        for byte in v {
            s.serialize_element(byte)?;
        }
        s.end()
    }

    fn serialize_none(self) -> Result<Self::Ok> {
        self.serialize_unit()
    }

    fn serialize_some<T: ?Sized + Serialize>(
        self,
        value: &T,
    ) -> Result<Self::Ok> {
        T::serialize(value, self)
    }

    fn serialize_unit(self) -> Result<Self::Ok> {
        self.write_header_nodata(ElementType::Null)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _value: &T,
    ) -> Result<Self::Ok> {
        self.serialize_unit()
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok> {
        let mut map = self.serialize_map(Some(1))?;
        serde::ser::SerializeMap::serialize_key(&mut map, variant)?;
        serde::ser::SerializeMap::serialize_value(&mut map, value)?;
        serde::ser::SerializeMap::end(map)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(JsonbWriter::new(&mut self.buffer, ElementType::Array))
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Ok(JsonbWriter::new(&mut self.buffer, ElementType::Array))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.serialize_tuple(len)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Ok(TupleVariantSerializer::new(&mut self.buffer, variant))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(JsonbWriter::new(&mut self.buffer, ElementType::Object))
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        todo!()
    }
}

impl<'a> ser::SerializeSeq for JsonbWriter<'a> {
    type Ok = ();

    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(
        &mut self,
        value: &T,
    ) -> Result<()> {
        let mut serializer = Serializer::default();
        std::mem::swap(self.buffer, &mut serializer.buffer);
        let r = value.serialize(&mut serializer);
        std::mem::swap(self.buffer, &mut serializer.buffer);
        r
    }

    fn end(self) -> Result<Self::Ok> {
        self.finalize();
        Ok(())
    }
}

impl<'a> ser::SerializeTuple for JsonbWriter<'a> {
    type Ok = ();

    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(
        &mut self,
        value: &T,
    ) -> Result<()> {
        <Self as ser::SerializeSeq>::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok> {
        <Self as ser::SerializeSeq>::end(self)
    }
}

impl<'a> ser::SerializeTupleStruct for JsonbWriter<'a> {
    type Ok = ();

    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        value: &T,
    ) -> std::prelude::v1::Result<(), Self::Error> {
        <Self as ser::SerializeTuple>::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok> {
        <Self as ser::SerializeTuple>::end(self)
    }
}

/// Serializes an enum tuple variant as an object with a single key for the variant name
/// and an array of the tuple fields as the value.
/// MyEnum::Variant(1, 2) -> {"Variant": [1, 2]}
/// We need to keep track of two jsonb headers, one for the map and one for the array.
pub struct TupleVariantSerializer<'a> {
    map_header_start: usize,
    seq_jsonb_writer: JsonbWriter<'a>,
}

impl<'a> TupleVariantSerializer<'a> {
    fn new(buffer: &'a mut Vec<u8>, variant: &'static str) -> Self {
        let mut map_jsonb_writer =
            JsonbWriter::new(buffer, ElementType::Object);
        ser::SerializeMap::serialize_key(&mut map_jsonb_writer, variant)
            .unwrap();
        let map_header_start = map_jsonb_writer.header_start;
        let seq_jsonb_writer = JsonbWriter::new(buffer, ElementType::Array);
        Self {
            map_header_start,
            seq_jsonb_writer,
        }
    }
}

impl<'a> ser::SerializeTupleVariant for TupleVariantSerializer<'a> {
    type Ok = ();

    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        value: &T,
    ) -> Result<()> {
        ser::SerializeTuple::serialize_element(
            &mut self.seq_jsonb_writer,
            value,
        )
    }

    fn end(self) -> Result<Self::Ok> {
        ser::SerializeTuple::end(JsonbWriter {
            buffer: self.seq_jsonb_writer.buffer,
            header_start: self.seq_jsonb_writer.header_start,
        })?;
        ser::SerializeMap::end(JsonbWriter {
            buffer: self.seq_jsonb_writer.buffer,
            header_start: self.map_header_start,
        })
    }
}

impl<'a> ser::SerializeMap for JsonbWriter<'a> {
    type Ok = ();

    type Error = Error;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<()> {
        <Self as ser::SerializeSeq>::serialize_element(self, key)
    }

    fn serialize_value<T: ?Sized + Serialize>(
        &mut self,
        value: &T,
    ) -> Result<()> {
        <Self as ser::SerializeSeq>::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(self.finalize())
    }
}

impl<'a> ser::SerializeStruct for JsonbWriter<'a> {
    type Ok = ();

    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()> {
        <Self as ser::SerializeMap>::serialize_key(self, key)?;
        <Self as ser::SerializeMap>::serialize_value(self, value)
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(self.finalize())
    }
}

impl<'a> ser::SerializeStructVariant for JsonbWriter<'a> {
    type Ok = ();

    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()> {
        <Self as ser::SerializeStruct>::serialize_field(self, key, value)
    }

    fn end(self) -> Result<Self::Ok> {
        <Self as ser::SerializeStruct>::end(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_u8() {
        assert_eq!(to_vec(&42u8).unwrap(), b"\x2342");
    }

    #[test]
    fn test_serialize_i64() {
        assert_eq!(
            to_vec(&1234567890123456789i64).unwrap(),
            b"\xc3\x131234567890123456789"
        );
    }

    #[test]
    fn test_serialize_bool() {
        assert_eq!(to_vec(&true).unwrap(), b"\x01");
        assert_eq!(to_vec(&false).unwrap(), b"\x02");
    }

    #[test]
    fn test_serialize_sring() {
        assert_eq!(to_vec(&"hello").unwrap(), b"\x5ahello");
    }

    fn assert_long_str(repeats: usize, expected_header: &[u8]) {
        let long_str = "x".repeat(repeats);
        assert_eq!(
            to_vec(&long_str).unwrap(),
            [&expected_header[..], &long_str.as_bytes()].concat()
        );
    }

    #[test]
    fn test_serialize_various_string_lengths() {
        assert_long_str(0x0, b"\x0a");
        assert_long_str(0x1, b"\x1a");
        assert_long_str(0xb, b"\xba");
        assert_long_str(0xc, b"\xca\x0c");
        assert_long_str(0xf, b"\xca\x0f");
        assert_long_str(0x100, b"\xda\x01\x00");
        assert_long_str(0xffff, b"\xda\xff\xff");
        assert_long_str(0x01_23_45_67, b"\xea\x01\x23\x45\x67");
        // disabled for test performance:
        // assert_long_str(0x01_0000_0000, b"\xfa\x00\x00\x00\x01\x00\x00\x00\x00");
    }

    #[test]
    fn test_serialize_array() {
        assert_eq!(
            to_vec(&Vec::<String>::new()).unwrap(),
            b"\x0b",
            "empty array"
        );
        assert_eq!(to_vec(&vec![true, false]).unwrap(), b"\x2b\x01\x02");
    }

    #[test]
    fn test_serialize_tuple() {
        assert_eq!(to_vec(&(true, 1, 2)).unwrap(), b"\x5b\x01\x131\x132");
    }

    #[test]
    fn test_serialize_tuple_struct() {
        #[derive(serde_derive::Serialize)]
        struct TupleStruct(String, f32);

        assert_eq!(
            to_vec(&TupleStruct("hello".to_string(), 3.14)).unwrap(),
            b"\xbb\x5ahello\x453.14"
        );
    }

    #[test]
    fn test_serialize_struct() {
        #[derive(serde_derive::Serialize)]
        struct TestStruct {
            smol: char,
            long_long_long_long: usize,
        }
        let test_struct = TestStruct {
            smol: 'X',
            long_long_long_long: 42,
        };
        assert_eq!(
            to_vec(&test_struct).unwrap(),
            b"\xcc\x1f\x4asmol\x1aX\xca\x13long_long_long_long\x2342"
        );
    }

    #[test]
    fn test_serialize_map() {
        let mut test_map = std::collections::HashMap::new();
        test_map.insert("k".to_string(), false);
        assert_eq!(to_vec(&test_map).unwrap(), b"\x3c\x1ak\x02",);
    }

    #[test]
    fn test_serialize_empty_map() {
        let test_map = std::collections::HashMap::<String, ()>::new();
        assert_eq!(to_vec(&test_map).unwrap(), b"\x0c",);
    }

    #[test]
    fn test_serialize_option() {
        assert_eq!(to_vec(&Some(42)).unwrap(), b"\x2342");
        assert_eq!(to_vec(&Option::<i32>::None).unwrap(), b"\x00");
    }

    #[test]
    fn test_serialize_unit() {
        assert_eq!(to_vec(&()).unwrap(), b"\x00");
    }

    #[test]
    fn test_serialize_unit_struct() {
        #[derive(serde_derive::Serialize)]
        struct UnitStruct;

        assert_eq!(to_vec(&UnitStruct).unwrap(), b"\x00");
    }

    #[test]
    fn test_serialize_enum_unit_variants() {
        #[derive(serde_derive::Serialize)]
        enum Enum {
            A,
            B,
        }

        assert_eq!(to_vec(&Enum::A).unwrap(), b"\x1aA");
        assert_eq!(to_vec(&Enum::B).unwrap(), b"\x1aB");
    }

    #[test]
    fn test_serialize_enum_newtype_variant() {
        #[derive(serde_derive::Serialize)]
        enum Enum {
            A(i32),
        }

        assert_eq!(to_vec(&Enum::A(42)).unwrap(), b"\x5c\x1aA\x2342");
    }

    #[test]
    fn test_serialize_enum_tuple_variant() {
        #[derive(serde_derive::Serialize)]
        enum Enum {
            A(i32, i32),
        }

        assert_eq!(to_vec(&Enum::A(1, 2)).unwrap(), b"\x7c\x1aA\x4b\x131\x132");
    }
}
