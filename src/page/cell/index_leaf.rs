use super::{parse_varint, Schema, SerialValue};
use crate::page::cell::build_serial_values;
use anyhow::{anyhow, Result};
use std::io::{Cursor, Read};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct IndexLeafCell {
    payload_size: u64,
    pub payload: Vec<u8>,
    overflow_page_num: Option<u32>,
}

impl TryFrom<&[u8]> for IndexLeafCell {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> Result<Self> {
        let mut cursor = Cursor::new(bytes);
        let (payload_size, data) = parse_varint(bytes)?;
        let payload = &data[0..payload_size as usize];

        cursor.set_position(payload_size);

        let mut buf = [0; 4];
        let overflow_page_num = match cursor.read_exact(&mut buf) {
            Ok(_) => Some(u32::from_be_bytes(buf)),
            Err(_) => None,
        };

        Ok(IndexLeafCell {
            payload_size,
            payload: payload.to_vec(),
            overflow_page_num,
        })
    }
}

impl IndexLeafCell {
    pub fn sqlite_schema(&self) -> Result<Schema> {
        let mut schema_vec = build_serial_values(&self.payload)?;

        let schema_type = match schema_vec.remove(0) {
            SerialValue::Text(value) => value,
            _ => return Err(anyhow!("Expected TEXT for schema type")),
        };

        let name = match schema_vec.remove(0) {
            SerialValue::Text(value) => value,
            _ => return Err(anyhow!("Expected TEXT for name")),
        };

        let table_name = match schema_vec.remove(0) {
            SerialValue::Text(value) => value,
            _ => return Err(anyhow!("Expected TEXT for table name")),
        };

        let root_page = match schema_vec.remove(0) {
            SerialValue::Integer(value) => value as usize,
            _ => return Err(anyhow!("Expected INTEGER for root page")),
        };

        let sql = match schema_vec.remove(0) {
            SerialValue::Text(value) => value,
            _ => return Err(anyhow!("Expected TEXT for SQL")),
        };

        Ok(Schema {
            schema_type,
            name,
            table_name,
            root_page,
            sql,
        })
    }
}
