use crate::{
    error::{Error, Result},
    header::ElementType,
};
use serde::ser::{self, Serialize};
use std::io::{Cursor, Seek, SeekFrom, Write};

pub struct Serializer<W: Write + Seek> {
    writer: W,
}

impl<'a> Serializer<Cursor<&'a mut Vec<u8>>> {
    pub fn from_bytes(vec: &'a mut Vec<u8>) -> Self {
        let cursor = std::io::Cursor::new(vec);
        Serializer { writer: cursor }
    }
}

pub fn to_vec<T>(value: &T) -> Result<Vec<u8>>
where
    T: Serialize,
{
    let mut writer = Vec::new();
    let mut serializer = Serializer::from_bytes(&mut writer);
    value.serialize(&mut serializer)?;
    Ok(writer)
}

impl<W: Write + Seek> Serializer<W> {
    fn write_header_nodata(&mut self, element_type: ElementType) -> Result<()> {
        crate::header::Header {
            element_type,
            payload_size: 0,
        }
        .write_minimal(&mut self.writer)?;
        Ok(())
    }
    fn write_displayable(
        &mut self,
        element_type: ElementType,
        data: impl std::fmt::Display,
    ) -> Result<()> {
        let data = data.to_string();
        let payload_size = data.len();
        crate::header::Header {
            element_type,
            payload_size,
        }
        .write_minimal(&mut self.writer)?;
        self.writer.write_all(data.as_bytes())?;
        Ok(())
    }
    fn write_displayable_nocopy(
        &mut self,
        element_type: ElementType,
        data: impl std::fmt::Display,
    ) -> Result<()> {
        let header_bytes_max = crate::header::Header {
            element_type,
            payload_size: 0,
        }
        .serialize();
        self.writer.write_all(&header_bytes_max)?;
        let data_start = self.writer.stream_position()?;
        write!(self.writer, "{}", data)?;
        let data_end = self.writer.stream_position()?;
        let payload_size = data_end - data_start;
        self.writer.seek(SeekFrom::Start(data_start - 8))?;
        self.writer.write_all(&payload_size.to_be_bytes())?;
        self.writer.seek(SeekFrom::Start(data_end))?;
        Ok(())
    }
}

impl<W: Write + Seek> ser::Serializer for &mut Serializer<W> {
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

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok>
    where
        T: Serialize,
    {
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

    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        _value: &T,
    ) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        todo!()
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        todo!()
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        todo!()
    }

    fn serialize_tuple(
        self,
        _len: usize,
    ) -> std::prelude::v1::Result<Self::SerializeTuple, Self::Error> {
        todo!()
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> std::prelude::v1::Result<Self::SerializeTupleStruct, Self::Error> {
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
    ) -> std::prelude::v1::Result<Self::SerializeStruct, Self::Error> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> std::prelude::v1::Result<Self::SerializeStructVariant, Self::Error>
    {
        todo!()
    }
}

impl<W: Write + Seek> ser::SerializeSeq for &mut Serializer<W> {
    type Ok = ();

    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok> {
        todo!()
    }
}

impl<W: Write + Seek> ser::SerializeTuple for &mut Serializer<W> {
    type Ok = ();

    type Error = Error;

    fn serialize_element<T: ?Sized>(
        &mut self,
        _value: &T,
    ) -> std::prelude::v1::Result<(), Self::Error>
    where
        T: Serialize,
    {
        todo!()
    }

    fn end(self) -> Result<Self::Ok> {
        todo!()
    }
}

impl<W: Write + Seek> ser::SerializeTupleStruct for &mut Serializer<W> {
    type Ok = ();

    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        _value: &T,
    ) -> std::prelude::v1::Result<(), Self::Error>
    where
        T: Serialize,
    {
        todo!()
    }

    fn end(self) -> Result<Self::Ok> {
        todo!()
    }
}

impl<W: Write + Seek> ser::SerializeTupleVariant for &mut Serializer<W> {
    type Ok = ();

    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(())
    }
}

impl<W: Write + Seek> ser::SerializeMap for &mut Serializer<W> {
    type Ok = ();

    type Error = Error;

    fn serialize_key<T: ?Sized>(
        &mut self,
        _key: &T,
    ) -> std::prelude::v1::Result<(), Self::Error>
    where
        T: Serialize,
    {
        todo!()
    }

    fn serialize_value<T: ?Sized>(
        &mut self,
        _value: &T,
    ) -> std::prelude::v1::Result<(), Self::Error>
    where
        T: Serialize,
    {
        todo!()
    }

    fn end(self) -> Result<Self::Ok> {
        todo!()
    }
}

impl<W: Write + Seek> ser::SerializeStruct for &mut Serializer<W> {
    type Ok = ();

    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        _key: &'static str,
        _value: &T,
    ) -> Result<()>
    where
        T: Serialize,
    {
        todo!()
    }

    fn end(self) -> Result<Self::Ok> {
        todo!()
    }
}

impl<W: Write + Seek> ser::SerializeStructVariant for &mut Serializer<W> {
    type Ok = ();

    type Error = Error;

    fn serialize_field<T: ?Sized>(
        &mut self,
        _key: &'static str,
        _value: &T,
    ) -> Result<()>
    where
        T: Serialize,
    {
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
        assert_eq!(to_vec(&42u8).unwrap(), b"\xf3\0\0\0\0\0\0\0\x0242");
    }

    #[test]
    fn test_serialize_i64() {
        assert_eq!(to_vec(&42i64).unwrap(), b"\xf3\0\0\0\0\0\0\0\x0242");
    }
}
