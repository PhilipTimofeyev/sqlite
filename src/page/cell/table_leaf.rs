use crate::page::cell::build_serial_values;

use super::{parse_varint, Schema, SerialValue};
use anyhow::{anyhow, Result};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct TableLeafCell {
    payload_size: u64,
    pub row_id: u64,
    pub payload: Vec<u8>,
    overflow_page_num: Option<u32>,
}

impl TryFrom<&[u8]> for TableLeafCell {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> Result<Self> {
        let (payload_size, data) = parse_varint(bytes).unwrap();
        let (row_id, data) = parse_varint(data).unwrap();
        let payload = &data[0..payload_size as usize];

        Ok(TableLeafCell {
            payload_size,
            row_id,
            payload: payload.to_vec(),
            overflow_page_num: None,
        })
    }
}

impl TableLeafCell {
    pub fn is_index_page(&self) -> Result<bool> {
        let schema = self.sqlite_schema()?;
        Ok(schema.schema_type == "index")
    }
    pub fn read_column(&self, column: usize) -> Result<String> {
        let mut serial_types_vec = build_serial_values(&self.payload)?;
        let column = serial_types_vec.remove(column);
        match column {
            SerialValue::Text(value) => Ok(value),
            SerialValue::Null => Ok("Null".to_string()),
            _ => todo!(),
        }
    }

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
