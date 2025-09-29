use crate::Error;

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
    /// Binary Float of IEEE 754 in little-endian
    BinaryFloat = 0xF,
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
            0xF => ElementType::BinaryFloat,
            _ => unreachable!("A four-bit number cannot be larger than 15"),
        }
    }
}

impl std::convert::From<ElementType> for u8 {
    fn from(value: ElementType) -> Self {
        value as u8
    }
}

pub fn is_jsonb(data: &[u8]) -> Result<Header, Error> {
    if data.len() == 0 {
        return Err(Error::Empty);
    }

    let first_byte = data[0];
    let upper_four_bits = first_byte >> 4;
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
        if data.len() < 1 + bytes_to_read {
            return Err(Error::Message(
                "not enough bytes to for header".to_string(),
            ));
        }

        let mut buf = [0u8; 8];
        let start = 8 - bytes_to_read;
        buf[start..].copy_from_slice(&data[1..1 + bytes_to_read]);
        u64::from_be_bytes(buf)
    };

    // then check length of rest bytes instead of checking recursively
    // which means we just do a naive checking here
    if data.len() != 1 + bytes_to_read + payload_size as usize {
        return Err(Error::Message(
            "data length does not match header payload size".to_string(),
        ));
    }

    Ok(Header {
        element_type: ElementType::from(first_byte),
        payload_size,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_jsonb_empty_data() {
        let data = &[];
        let result = is_jsonb(data);
        assert!(matches!(result, Err(Error::Empty)));
    }

    #[test]
    fn test_is_jsonb_small_payload_sizes() {
        // Test payload sizes 0-11 (encoded in upper 4 bits, no additional bytes)
        for payload_size in 0..=11u8 {
            let first_byte = (payload_size << 4) | 0x00; // ElementType::Null
                                                         // Create data with correct total length: 1 (header) + 0 (no extra bytes) + payload_size
            let mut data = vec![first_byte];
            data.extend(vec![0u8; payload_size as usize]); // Add payload data

            let result = is_jsonb(&data).unwrap();
            assert_eq!(result.element_type, ElementType::Null);
            assert_eq!(result.payload_size, payload_size as u64);
        }
    }

    #[test]
    fn test_is_jsonb_payload_size_12() {
        // Upper 4 bits = 12 means 1 additional byte for payload size
        let first_byte = (12 << 4) | 0x01; // ElementType::True
        let payload_byte = 42u8;
        // Create data with correct total length: 1 (header) + 1 (size byte) + 42 (payload)
        let mut data = vec![first_byte, payload_byte];
        data.extend(vec![0u8; 42]); // Add 42 bytes of payload data

        let result = is_jsonb(&data).unwrap();
        assert_eq!(result.element_type, ElementType::True);
        assert_eq!(result.payload_size, 42);
    }

    #[test]
    fn test_is_jsonb_payload_size_13() {
        // Upper 4 bits = 13 means 2 additional bytes for payload size
        let first_byte = (13 << 4) | 0x02; // ElementType::False
        let payload_bytes = [0x01, 0x00]; // 256 in big-endian
                                          // Create data with correct total length: 1 (header) + 2 (size bytes) + 256 (payload)
        let mut data = vec![first_byte, payload_bytes[0], payload_bytes[1]];
        data.extend(vec![0u8; 256]); // Add 256 bytes of payload data

        let result = is_jsonb(&data).unwrap();
        assert_eq!(result.element_type, ElementType::False);
        assert_eq!(result.payload_size, 256);
    }

    #[test]
    fn test_is_jsonb_payload_size_14() {
        // Upper 4 bits = 14 means 4 additional bytes for payload size
        let first_byte = (14 << 4) | 0x03; // ElementType::Int
                                           // Use smaller payload size for testing to avoid huge allocations
        let payload_bytes = [0x00, 0x00, 0x01, 0x00]; // 256 in big-endian
                                                      // Create data with correct total length: 1 (header) + 4 (size bytes) + 256 (payload)
        let mut data = vec![
            first_byte,
            payload_bytes[0],
            payload_bytes[1],
            payload_bytes[2],
            payload_bytes[3],
        ];
        data.extend(vec![0u8; 256]); // Add 256 bytes of payload data

        let result = is_jsonb(&data).unwrap();
        assert_eq!(result.element_type, ElementType::Int);
        assert_eq!(result.payload_size, 256);
    }

    #[test]
    fn test_is_jsonb_payload_size_15() {
        // Upper 4 bits = 15 means 8 additional bytes for payload size
        let first_byte = (15 << 4) | 0x04; // ElementType::Int5
                                           // Use smaller payload size for testing to avoid huge allocations
        let payload_bytes = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00]; // 256 in big-endian
                                                                              // Create data with correct total length: 1 (header) + 8 (size bytes) + 256 (payload)
        let mut data = vec![
            first_byte,
            payload_bytes[0],
            payload_bytes[1],
            payload_bytes[2],
            payload_bytes[3],
            payload_bytes[4],
            payload_bytes[5],
            payload_bytes[6],
            payload_bytes[7],
        ];
        data.extend(vec![0u8; 256]); // Add 256 bytes of payload data

        let result = is_jsonb(&data).unwrap();
        assert_eq!(result.element_type, ElementType::Int5);
        assert_eq!(result.payload_size, 256);
    }

    #[test]
    fn test_is_jsonb_all_element_types() {
        // Test all valid element types with small payload
        let element_types = [
            (0x00, ElementType::Null),
            (0x01, ElementType::True),
            (0x02, ElementType::False),
            (0x03, ElementType::Int),
            (0x04, ElementType::Int5),
            (0x05, ElementType::Float),
            (0x06, ElementType::Float5),
            (0x07, ElementType::Text),
            (0x08, ElementType::TextJ),
            (0x09, ElementType::Text5),
            (0x0A, ElementType::TextRaw),
            (0x0B, ElementType::Array),
            (0x0C, ElementType::Object),
            (0x0D, ElementType::Reserved13),
            (0x0E, ElementType::Reserved14),
            (0x0F, ElementType::BinaryFloat),
        ];

        for (type_code, expected_type) in element_types {
            let first_byte = (5 << 4) | type_code; // payload size 5
                                                   // Create data with correct total length: 1 (header) + 0 (no extra bytes) + 5 (payload)
            let mut data = vec![first_byte];
            data.extend(vec![0u8; 5]); // Add 5 bytes of payload data

            let result = is_jsonb(&data).unwrap();
            assert_eq!(result.element_type, expected_type);
            assert_eq!(result.payload_size, 5);
        }
    }

    #[test]
    fn test_is_jsonb_insufficient_data_for_payload_size_12() {
        let first_byte = (12 << 4) | 0x00; // Need 1 additional byte but none provided
        let data = &[first_byte];

        let result = is_jsonb(data);
        assert!(matches!(result, Err(Error::Message(_))));
        if let Err(Error::Message(msg)) = result {
            assert!(msg.contains("not enough bytes"));
        }
    }

    #[test]
    fn test_is_jsonb_insufficient_data_for_payload_size_13() {
        let first_byte = (13 << 4) | 0x00; // Need 2 additional bytes but only 1 provided
        let data = &[first_byte, 0x42];

        let result = is_jsonb(data);
        assert!(matches!(result, Err(Error::Message(_))));
        if let Err(Error::Message(msg)) = result {
            assert!(msg.contains("not enough bytes"));
        }
    }

    #[test]
    fn test_is_jsonb_insufficient_data_for_payload_size_14() {
        let first_byte = (14 << 4) | 0x00; // Need 4 additional bytes but only 2 provided
        let data = &[first_byte, 0x00, 0x01];

        let result = is_jsonb(data);
        assert!(matches!(result, Err(Error::Message(_))));
        if let Err(Error::Message(msg)) = result {
            assert!(msg.contains("not enough bytes"));
        }
    }

    #[test]
    fn test_is_jsonb_insufficient_data_for_payload_size_15() {
        let first_byte = (15 << 4) | 0x00; // Need 8 additional bytes but only 4 provided
        let data = &[first_byte, 0x00, 0x00, 0x00, 0x01];

        let result = is_jsonb(data);
        assert!(matches!(result, Err(Error::Message(_))));
        if let Err(Error::Message(msg)) = result {
            assert!(msg.contains("not enough bytes"));
        }
    }

    #[test]
    fn test_is_jsonb_real_world_examples() {
        // Test with a typical JSONB null value
        let null_data = &[0x00]; // payload size 0, ElementType::Null
        let result = is_jsonb(null_data).unwrap();
        assert_eq!(result.element_type, ElementType::Null);
        assert_eq!(result.payload_size, 0);

        // Test with a typical JSONB true value
        let true_data = &[0x01]; // payload size 0, ElementType::True
        let result = is_jsonb(true_data).unwrap();
        assert_eq!(result.element_type, ElementType::True);
        assert_eq!(result.payload_size, 0);

        // Test with a JSONB string that has a medium payload size
        // Create data with correct total length: 1 (header) + 1 (size byte) + 20 (payload)
        let mut string_data = vec![(12 << 4) | 0x07, 20]; // payload size 20, ElementType::Text
        string_data.extend(vec![b'a'; 20]); // Add 20 bytes of string data
        let result = is_jsonb(&string_data).unwrap();
        assert_eq!(result.element_type, ElementType::Text);
        assert_eq!(result.payload_size, 20);
    }

    #[test]
    fn test_is_jsonb_data_length_mismatch_too_short() {
        // Test when data is shorter than expected
        let first_byte = (5 << 4) | 0x00; // payload size 5, ElementType::Null
        let data = vec![first_byte, 0x00, 0x00]; // Only 2 payload bytes instead of 5

        let result = is_jsonb(&data);
        assert!(matches!(result, Err(Error::Message(_))));
        if let Err(Error::Message(msg)) = result {
            assert!(
                msg.contains("data length does not match header payload size")
            );
        }
    }

    #[test]
    fn test_is_jsonb_data_length_mismatch_too_long() {
        // Test when data is longer than expected
        let first_byte = (3 << 4) | 0x00; // payload size 3, ElementType::Null
        let mut data = vec![first_byte];
        data.extend(vec![0u8; 10]); // 10 payload bytes instead of 3

        let result = is_jsonb(&data);
        assert!(matches!(result, Err(Error::Message(_))));
        if let Err(Error::Message(msg)) = result {
            assert!(
                msg.contains("data length does not match header payload size")
            );
        }
    }

    #[test]
    fn test_is_jsonb_data_length_mismatch_with_size_bytes() {
        // Test length mismatch with additional size bytes
        let first_byte = (12 << 4) | 0x07; // ElementType::Text, size in next byte
        let payload_size_byte = 10u8;
        let mut data = vec![first_byte, payload_size_byte];
        data.extend(vec![b'x'; 5]); // Only 5 bytes instead of 10

        let result = is_jsonb(&data);
        assert!(matches!(result, Err(Error::Message(_))));
        if let Err(Error::Message(msg)) = result {
            assert!(
                msg.contains("data length does not match header payload size")
            );
        }
    }

    #[test]
    fn test_is_jsonb_zero_payload_size_correct_length() {
        // Test various element types with zero payload size
        let zero_payload_types =
            [ElementType::Null, ElementType::True, ElementType::False];

        for element_type in zero_payload_types {
            let first_byte = (0 << 4) | (element_type as u8); // payload size 0
            let data = vec![first_byte]; // Just the header, no payload

            let result = is_jsonb(&data).unwrap();
            assert_eq!(result.element_type, element_type);
            assert_eq!(result.payload_size, 0);
        }
    }
}
