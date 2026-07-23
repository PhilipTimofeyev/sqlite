use anyhow::{anyhow, Result};
use std::io::{Cursor, Read};

// A Table Leaf Cell conceptually is like a row in a table
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct TableLeafCell {
    payload_size: u64,
    pub row_id: u64, // Primary Key
    payload: Vec<u8>,
    overflow_page_num: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct TableInteriorCell {
    pub left_child: u32,
    pub row_id: u64, // Primary Key
}

impl TryFrom<&[u8]> for TableInteriorCell {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> Result<Self> {
        let left_child: u32 = u32::from_be_bytes(bytes[0..4].try_into().unwrap());
        let (row_id, _data) = parse_varint(&bytes[4..]).unwrap();

        Ok(TableInteriorCell { left_child, row_id })
    }
}

#[allow(dead_code)]
#[derive(Debug)]
enum SerialType {
    Null,
    Integer(usize),
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
            n @ 1..7 => SerialType::Integer(n as usize),
            // Need to check 8 and 9
            8 => SerialType::Null,
            9 => SerialType::Null,
            n if n >= 12 && n % 2 == 0 => SerialType::Blob(((n - 12) / 2) as usize),
            n if n >= 13 && n % 2 == 1 => SerialType::Text(((n - 13) / 2) as usize),
            0 => SerialType::Null,
            _ => todo!(),
        }
    }
}

pub fn parse_varint(data: &[u8]) -> Option<(u64, &[u8])> {
    // println!("{:?}", data);
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
            return Some((value, &data[i + 1..]));
        }
    }

    // More than 7 bytes is invalid.
    panic!("Too many bytes for varint");
}

fn parse_header_varints(mut data: &[u8]) -> Vec<u64> {
    let mut result = Vec::new();
    while !data.is_empty() {
        if let Some((value, consumed)) = parse_varint(data) {
            result.push(value);
            data = consumed;
        } else {
            break;
        }
    }

    result
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

#[allow(dead_code)]
#[derive(Debug)]
pub struct Schema {
    schema_type: String,
    pub name: String,
    pub table_name: String,
    pub root_page: usize,
    pub sql: String,
}

impl Schema {
    pub fn sql_contains_str(&self, text: &str) -> bool {
        self.sql.contains(text)
        // let text = text.as_bytes();
        // self.sql.windows(text.len()).any(|window| window == text)
    }

    pub fn column_position(&self, column_name: &str) -> Result<usize> {
        let schema_sql = self.sql.clone();
        let columns = schema_sql.split(&['(', ')'][..]).collect::<Vec<&str>>();
        let columns = columns[..columns.len() - 1]
            .last()
            .unwrap()
            .split(',')
            .collect::<Vec<&str>>();

        let column = columns
            .iter()
            .position(|column| column.contains(column_name))
            .unwrap();

        Ok(column)
    }
}

// May add
#[allow(dead_code)]
enum SchemaType {
    TableType,
    Name,
}

impl TableLeafCell {
    pub fn read_column(&self, column: usize) -> Result<String> {
        let mut serial_types_vec = self.build_serial_types()?;
        let column = serial_types_vec.remove(column);
        match column {
            SerialValue::Text(value) => Ok(value),
            // SerialValue::Integer()
            SerialValue::Null => Ok("Null".to_string()),
            _ => todo!(),
        }

        // Ok(())
        // Ok(column)
    }

    // pub fn search_value(&self, column: usize, value: &str) -> bool {
    //     self.read_column(column).unwrap() == value
    // }

    pub fn build_serial_types(&self) -> Result<Vec<SerialValue>> {
        let mut serial_types = Vec::new();

        let (header_size, bytes) = parse_varint(&self.payload).unwrap();
        let varints = parse_header_varints(&bytes[0..header_size as usize - 1]);
        // println!("varints {:?}", varints);

        for code in varints {
            let serial_type = SerialType::from_code(code);
            serial_types.push(serial_type);
        }

        // println!("serial types {:?}", serial_types);

        let mut cursor = Cursor::new(self.payload.clone());
        cursor.set_position(header_size);
        let mut schema_vec = Vec::new();

        for serial_type in &serial_types {
            match serial_type {
                // NEEDS TO HANDLE 0 to 8 bytes
                SerialType::Integer(bytes) => {
                    // NEED TO ACCEPT 1 THROUGH 8 Bytes
                    let mut buf = vec![0; *bytes];
                    cursor.read_exact(&mut buf)?;
                    let value = buf.remove(0) as u64;
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
                _ => {
                    println!("Other");
                    todo!()
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
