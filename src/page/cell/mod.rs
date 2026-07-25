pub mod index_interior;
pub mod index_leaf;
pub mod table_interior;
pub mod table_leaf;
use anyhow::{anyhow, Result};
use std::io::{Cursor, Read};

#[allow(dead_code)]
#[derive(Debug)]
pub enum SerialType {
    Null,
    Integer(usize),
    IntegerConstant(u64),
    Float(usize),
    Blob(usize),
    Text(usize),
}

#[derive(Debug)]
pub enum SerialValue {
    Null,
    Integer(u64),
    Float(f64),
    Text(String),
    Blob(Vec<u8>),
}

impl SerialType {
    fn from_code(code: u64) -> Self {
        match code {
            0 => SerialType::Null,
            n @ 1..=4 => SerialType::Integer(n as usize),
            5 => SerialType::Integer(6),
            6 => SerialType::Integer(8),
            7 => SerialType::Float(8),
            8 => SerialType::IntegerConstant(0),
            9 => SerialType::IntegerConstant(1),
            n if n >= 12 && n % 2 == 0 => SerialType::Blob(((n - 12) / 2) as usize),
            n if n >= 13 && n % 2 == 1 => SerialType::Text(((n - 13) / 2) as usize),
            _ => todo!(),
        }
    }
}

pub fn build_serial_types(header_size: u64, payload: &[u8]) -> Result<Vec<SerialType>> {
    let mut serial_types = Vec::new();
    let varints = parse_header_varints(&payload[0..header_size as usize - 1])?;

    for code in varints {
        let serial_type = SerialType::from_code(code);
        serial_types.push(serial_type);
    }

    Ok(serial_types)
}

pub fn build_serial_values(payload: &[u8]) -> Result<Vec<SerialValue>> {
    let (header_size, bytes) = parse_varint(payload)?;

    let serial_types = build_serial_types(header_size, bytes)?;

    let mut cursor = Cursor::new(payload);
    cursor.set_position(header_size);
    let mut serial_values = Vec::new();

    for serial_type in &serial_types {
        let serial_value = match serial_type {
            SerialType::Integer(n) => {
                let bytes = read_n_bytes(&mut cursor, n)?;
                let mut integer = 0u64;

                for byte in bytes {
                    integer = (integer << 8) | byte as u64;
                }
                SerialValue::Integer(integer)
            }
            SerialType::IntegerConstant(i) => SerialValue::Integer(*i),
            SerialType::Float(n) => {
                let bytes = read_n_bytes(&mut cursor, n)?;
                let array: [u8; 8] = bytes.try_into().expect("Float must have 8 bytes");
                let float = f64::from_be_bytes(array);
                SerialValue::Float(float)
            }
            SerialType::Text(n) => {
                let bytes = read_n_bytes(&mut cursor, n)?;
                let text = String::from_utf8(bytes)?;
                SerialValue::Text(text)
            }
            SerialType::Blob(n) => {
                let blob = read_n_bytes(&mut cursor, n)?;
                SerialValue::Blob(blob)
            }
            SerialType::Null => SerialValue::Null,
        };
        serial_values.push(serial_value);
    }

    Ok(serial_values)
}

fn read_n_bytes(data: &mut Cursor<&[u8]>, n: &usize) -> Result<Vec<u8>> {
    let mut buf = vec![0; *n];
    data.read_exact(&mut buf)?;
    Ok(buf)
}

pub fn parse_varint(data: &[u8]) -> Result<(u64, &[u8])> {
    for i in 0..9 {
        let Some(b) = data.get(i) else {
            panic!("Not enough bytes for varint");
        };
        if b & 0x80 == 0 {
            // Last byte of the VARINT
            let mut value = 0u64;
            for b in data[..=i].iter() {
                value = (value << 7) | (b & 0x7f) as u64;
            }
            return Ok((value, &data[i + 1..]));
        }
    }

    // More than 7 bytes is invalid.
    panic!("Too many bytes for varint");
}

fn parse_header_varints(mut data: &[u8]) -> Result<Vec<u64>> {
    let mut result = Vec::new();
    while !data.is_empty() {
        match parse_varint(data) {
            Ok((value, consumed)) => {
                result.push(value);
                data = consumed;
            }
            Err(_) => break,
        }
    }

    Ok(result)
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Schema {
    pub schema_type: String,
    pub name: String,
    pub table_name: String,
    pub root_page: usize,
    pub sql: String,
}

impl Schema {
    pub fn sql_contains_str(&self, text: &str) -> bool {
        self.sql.contains(text)
    }

    pub fn column_position(&self, column_name: &str) -> Result<usize> {
        let sql = self.sql.clone();

        let start = sql.find('(').ok_or_else(|| anyhow!("No opening paren"))?;
        let end = sql.rfind(')').ok_or_else(|| anyhow!("No closing paren"))?;

        let columns = &sql[start + 1..end];
        let columns: Vec<&str> = columns.split(',').map(str::trim).collect();

        let column = columns
            .iter()
            .position(|column| {
                column
                    .to_lowercase()
                    .as_str()
                    .contains(column_name.to_lowercase().as_str())
            })
            .unwrap();

        Ok(column)
    }
}

#[cfg(test)]
mod tests {
    use super::*; // Imports everything from the outer module

    #[test]
    fn test_varint_0() {
        let varint = vec![0x00];
        let (result, _) = parse_varint(&varint).unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_varint_1() {
        let varint = vec![0x01];
        let (result, _) = parse_varint(&varint).unwrap();
        assert_eq!(result, 1);
    }

    #[test]
    fn test_varint_127() {
        let varint = vec![0x7F];
        let (result, _) = parse_varint(&varint).unwrap();
        assert_eq!(result, 127);
    }

    #[test]
    fn test_varint_two_bytes() {
        let varint = vec![0x81, 0x00];
        let (result, _) = parse_varint(&varint).unwrap();
        assert_eq!(result, 128);
    }

    #[test]
    fn test_varint_three_bytes() {
        let varint = vec![0x81, 0x80, 0x00];
        let (result, _) = parse_varint(&varint).unwrap();
        assert_eq!(result, 16384);
    }

    #[test]
    fn test_varint_four_bytes() {
        let varint = vec![0x81, 0x80, 0x80, 0x00];
        let (result, _) = parse_varint(&varint).unwrap();
        assert_eq!(result, 2_097_152);
    }

    #[test]
    fn test_varint_nine_bytes() {
        let varint = vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F];
        let (result, _) = parse_varint(&varint).unwrap();
        assert_eq!(result, 9_223_372_036_854_775_807);
    }
}
