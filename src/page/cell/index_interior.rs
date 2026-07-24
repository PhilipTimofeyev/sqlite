use super::{parse_header_varints, parse_varint, SerialType, SerialValue};
use anyhow::Result;
use std::io::{Cursor, Read};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct IndexInteriorCell {
    pub left_child: u32,
    payload_size: u64,
    payload: Vec<u8>,
    overflow_page_num: Option<u32>,
}

impl TryFrom<&[u8]> for IndexInteriorCell {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> Result<Self> {
        let left_child: u32 = u32::from_be_bytes(bytes[0..4].try_into()?);
        let (payload_size, data) = parse_varint(&bytes[4..]).unwrap();
        let payload = &data[0..payload_size as usize];

        Ok(IndexInteriorCell {
            left_child,
            payload_size,
            payload: payload.to_vec(),
            overflow_page_num: None,
        })
    }
}

impl IndexInteriorCell {
    pub fn build_serial_types(&self) -> Result<Vec<SerialValue>> {
        let mut serial_types = Vec::new();

        // println!("{:?}", self.payload);
        let (header_size, bytes) = parse_varint(&self.payload).unwrap();
        let varints = parse_header_varints(&bytes[0..header_size as usize - 1]);

        for code in varints {
            let serial_type = SerialType::from_code(code);
            serial_types.push(serial_type);
        }

        let mut cursor = Cursor::new(self.payload.clone());
        cursor.set_position(header_size);
        let mut schema_vec = Vec::new();

        for serial_type in &serial_types {
            match serial_type {
                // NEEDS TO HANDLE 0 to 8 bytes
                SerialType::Integer(bytes) => {
                    let mut buf = vec![0; *bytes];
                    cursor.read_exact(&mut buf)?;

                    let mut value = 0u64;

                    for byte in buf {
                        value = (value << 8) | byte as u64;
                    }

                    schema_vec.push(SerialValue::Integer(value));
                }
                SerialType::Text(bytes) => {
                    let mut buf = vec![0; *bytes];
                    cursor.read_exact(&mut buf)?;
                    // println!("HERE {:?}", buf);
                    let value = String::from_utf8(buf.clone()).unwrap();
                    schema_vec.push(SerialValue::Text(value));
                }
                SerialType::Blob(bytes) => {
                    let mut buf = vec![0; *bytes];
                    cursor.read_exact(&mut buf)?;
                    // let value = String::from_utf8(buf.clone()).unwrap();
                    schema_vec.push(SerialValue::Blob(buf));
                }
                SerialType::Null => {
                    schema_vec.push(SerialValue::Null);
                }
            }
        }

        Ok(schema_vec)
    }
}
