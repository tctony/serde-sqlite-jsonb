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

struct JsonbWriter<'a> {
    pub buffer: &'a mut Vec<u8>,
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
        } else if payload_size <= 0xffffff {
            header[0] |= 0xe0;
            header[1..5].copy_from_slice(&(payload_size as u32).to_be_bytes());
            5
        } else {
            header[0] |= 0xf0;
            header[1..9].copy_from_slice(&payload_size.to_be_bytes());
            9
        };
        self.buffer
            .copy_within(data_start..data_end, self.header_start + head_len);
        self.buffer
            .truncate(self.header_start + head_len + payload_size);
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

impl ser::Serializer for &mut Serializer {
    type Ok = ();

    type Error = Error;

    type SerializeSeq = Self;

    type SerializeTuple = Self;

    type SerializeTupleStruct = Self;

    type SerializeTupleVariant = Self;

    type SerializeMap = Self;

    type SerializeStruct = Self;

    type SerializeStructVariant = Self;

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
        todo!()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _value: &T,
    ) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        todo!()
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        todo!()
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        todo!()
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        todo!()
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        todo!()
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

impl ser::SerializeSeq for &mut Serializer {
    type Ok = ();

    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(
        &mut self,
        value: &T,
    ) -> Result<()> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok> {
        todo!()
    }
}

impl ser::SerializeTuple for &mut Serializer {
    type Ok = ();

    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(
        &mut self,
        _value: &T,
    ) -> std::prelude::v1::Result<(), Self::Error> {
        todo!()
    }

    fn end(self) -> Result<Self::Ok> {
        todo!()
    }
}

impl ser::SerializeTupleStruct for &mut Serializer {
    type Ok = ();

    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        _value: &T,
    ) -> std::prelude::v1::Result<(), Self::Error> {
        todo!()
    }

    fn end(self) -> Result<Self::Ok> {
        todo!()
    }
}

impl ser::SerializeTupleVariant for &mut Serializer {
    type Ok = ();

    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        value: &T,
    ) -> Result<()> {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(())
    }
}

impl ser::SerializeMap for &mut Serializer {
    type Ok = ();

    type Error = Error;

    fn serialize_key<T: ?Sized + Serialize>(
        &mut self,
        _key: &T,
    ) -> std::prelude::v1::Result<(), Self::Error> {
        todo!()
    }

    fn serialize_value<T: ?Sized + Serialize>(
        &mut self,
        _value: &T,
    ) -> std::prelude::v1::Result<(), Self::Error> {
        todo!()
    }

    fn end(self) -> Result<Self::Ok> {
        todo!()
    }
}

impl ser::SerializeStruct for &mut Serializer {
    type Ok = ();

    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        _key: &'static str,
        _value: &T,
    ) -> Result<()> {
        todo!()
    }

    fn end(self) -> Result<Self::Ok> {
        todo!()
    }
}

impl ser::SerializeStructVariant for &mut Serializer {
    type Ok = ();

    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        _key: &'static str,
        _value: &T,
    ) -> Result<()> {
        todo!()
    }

    fn end(self) -> Result<Self::Ok> {
        todo!()
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
        assert_eq!(to_vec(&42i64).unwrap(), b"\x2342");
    }

    #[test]
    fn test_serialize_bool() {
        assert_eq!(to_vec(&true).unwrap(), b"\x01");
        assert_eq!(to_vec(&false).unwrap(), b"\x02");
    }
}
