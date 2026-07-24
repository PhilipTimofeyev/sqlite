use super::{parse_header_varints, parse_varint, Schema, SerialType, SerialValue};
use anyhow::{anyhow, Result};
use std::io::{Cursor, Read};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct TableLeafCell {
    payload_size: u64,
    pub row_id: u64, // Primary Key
    payload: Vec<u8>,
    overflow_page_num: Option<u32>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct IndexInteriorCell {
    pub left_child: u32,
    payload_size: u64,
    payload: Vec<u8>,
    overflow_page_num: Option<u32>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct IndexLeafCell {
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

impl TryFrom<&[u8]> for IndexLeafCell {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> Result<Self> {
        let (payload_size, data) = parse_varint(bytes).unwrap();
        let payload = &data[0..payload_size as usize];

        Ok(IndexLeafCell {
            payload_size,
            payload: payload.to_vec(),
            overflow_page_num: None,
        })
    }
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

impl IndexLeafCell {
    pub fn sqlite_schema(&self) -> Result<Schema> {
        let mut schema_vec = self.build_serial_types()?;

        println!("{:?}", self);

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

    pub fn build_serial_types(&self) -> Result<Vec<SerialValue>> {
        let mut serial_types = Vec::new();

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

impl TableLeafCell {
    pub fn is_index_page(&self) -> Result<bool> {
        let schema = self.sqlite_schema()?;
        Ok(schema.schema_type == "index")
    }
    pub fn read_column(&self, column: usize) -> Result<String> {
        let mut serial_types_vec = self.build_serial_types()?;
        let column = serial_types_vec.remove(column);
        match column {
            SerialValue::Text(value) => Ok(value),
            // SerialValue::Integer()
            SerialValue::Null => Ok("Null".to_string()),
            _ => todo!(),
        }
    }

    pub fn build_serial_types(&self) -> Result<Vec<SerialValue>> {
        let mut serial_types = Vec::new();

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

    pub fn sqlite_schema(&self) -> Result<Schema> {
        let mut schema_vec = self.build_serial_types()?;

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
