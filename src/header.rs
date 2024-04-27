#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
/// Represents the different element types in the JSONB format.
pub enum ElementType {
    /// The element is a JSON "null".
    Null = 0,
    /// The element is a JSON "true".
    True = 1,
    /// The element is a JSON "false".
    False = 2,
    /// The element is a JSON integer value in the canonical RFC 8259 format.
    Int = 3,
    /// The element is a JSON integer value that is not in the canonical format.
    Int5 = 4,
    /// The element is a JSON floating-point value in the canonical RFC 8259 format.
    Float = 5,
    /// The element is a JSON floating-point value that is not in the canonical format.
    Float5 = 6,
    /// The element is a JSON string value that does not contain any escapes.
    Text = 7,
    /// The element is a JSON string value that contains RFC 8259 character escapes.
    TextJ = 8,
    /// The element is a JSON string value that contains character escapes, including some from JSON5.
    Text5 = 9,
    /// The element is a JSON string value that contains UTF8 characters that need to be escaped.
    TextRaw = 0xA,
    /// The element is a JSON array.
    Array = 0xB,
    /// The element is a JSON object.
    Object = 0xC,
    /// Reserved for future expansion.
    Reserved13 = 0xD,
    /// Reserved for future expansion.
    Reserved14 = 0xE,
    /// Reserved for future expansion.
    Reserved15 = 0xF,
}

/// Represents the header of a JSONB element (size and type).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Header {
    pub element_type: ElementType,
    pub payload_size: u64,
}

impl Header {
    /// Serialize the header into a byte array.
    pub fn serialize(self) -> [u8; 9] {
        let mut s = [0u8; 9];
        s[0] = u8::from(self.element_type) | 0xF0;
        let payload_size = self.payload_size.to_be_bytes();
        s[1..].copy_from_slice(&payload_size);
        s
    }
}

impl std::convert::From<u8> for ElementType {
    fn from(value: u8) -> Self {
        match value & 0x0F {
            // Element types are stored in the lower 4 bits
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
            0xA => ElementType::TextRaw,
            0xB => ElementType::Array,
            0xC => ElementType::Object,
            0xD => ElementType::Reserved13,
            0xE => ElementType::Reserved14,
            0xF => ElementType::Reserved15,
            _ => unreachable!("A four-bit number cannot be larger than 15"),
        }
    }
}

impl std::convert::From<ElementType> for u8 {
    fn from(value: ElementType) -> Self {
        value as u8
    }
}
