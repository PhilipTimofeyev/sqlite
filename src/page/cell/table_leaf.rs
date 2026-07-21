use anyhow::Result;
use std::io::{Cursor, Read};

// A Table Leaf Cell conceptually is like a row in a table
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct TableLeafCell {
    payload_size: u64,
    pub row_id: u64, // Primary Key
    payload: Vec<u8>,
}

#[allow(dead_code)]
#[derive(Debug)]
enum SerialType {
    Null,
    Integer(i64),
    Blob(usize),
    Text(usize),
}

impl SerialType {
    fn from_code(code: u64) -> Self {
        match code {
            0 => SerialType::Null,
            1..7 => SerialType::Integer(1),
            n if n >= 12 && n % 2 == 0 => SerialType::Blob(((n - 12) / 2) as usize),
            n if n >= 13 && n % 2 == 1 => SerialType::Text(((n - 13) / 2) as usize),
            _ => todo!(),
        }
    }
}

fn parse_varint(data: &[u8]) -> Option<(u64, &[u8])> {
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

        // let mut row_id = [0; 1];
        // let mut header_size = [0; 1];

        // let _ = data.read_exact(&mut row_id);
        // let _ = data.read_exact(&mut header_size);
        //
        // let header_size = header_size[0] as usize;

        // let mut header = vec![header_size as u8; header_size];
        // let _ = data.read_exact(&mut header[1..]);
        let payload = &data[0..payload_size as usize];
        // println!("Payload size {payload_size:?}");
        // println!("Payload size hmm {:?}", payload.len());
        // let mut payload = Vec::new();
        // let _ = data.read_to_end(&mut payload);
        //
        Ok(TableLeafCell {
            payload_size,
            row_id,
            payload: payload.to_vec(),
        })
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Schema {
    schema_type: Vec<u8>,
    name: Vec<u8>,
    pub table_name: Vec<u8>,
    root_page: Vec<u8>,
    pub sql: Vec<u8>,
}

impl Schema {
    pub fn sql_contains_str(&self, text: &str) -> bool {
        let text = text.as_bytes();
        self.sql.windows(text.len()).any(|window| window == text)
    }

    pub fn column_position(&self, column_name: &str) -> Result<usize> {
        let schema_sql = String::from_utf8(self.sql.clone())?;
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
    pub fn read_column(&self, column: &usize) -> Result<String> {
        let schema_vec = self.build_schema_vec()?;
        let column = String::from_utf8(schema_vec.clone().to_vec()[*column].clone())?;

        Ok(column)
    }

    pub fn search_value(&self, column: &usize, value: &str) -> bool {
        self.read_column(column).unwrap() == value
    }

    pub fn build_schema_vec(&self) -> Result<Vec<Vec<u8>>> {
        let mut serial_types = Vec::new();

        let (header_size, bytes) = parse_varint(&self.payload).unwrap();
        let varints = parse_header_varints(&bytes[0..header_size as usize - 1]);
        // println!("varints {:?}", varints);

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
                    let mut buf = vec![0; *bytes as usize];
                    cursor.read_exact(&mut buf)?;
                    schema_vec.push(buf);
                }
                SerialType::Text(bytes) => {
                    let mut buf = vec![0; *bytes];
                    cursor.read_exact(&mut buf)?;
                    schema_vec.push(buf);
                }
                SerialType::Null => {
                    schema_vec.push(vec![0]);
                }
                _ => todo!(),
            }
        }

        Ok(schema_vec)
    }

    pub fn sqlite_schema(&self) -> Result<Schema> {
        let mut schema_vec = self.build_schema_vec()?;
        let schema_type = schema_vec.remove(0);
        let name = schema_vec.remove(0);
        let table_name = schema_vec.remove(0);
        let root_page = schema_vec.remove(0);
        let sql: Vec<u8> = schema_vec
            .into_iter()
            .flatten()
            .collect::<Vec<u8>>()
            .drain(..)
            .collect();

        Ok(Schema {
            schema_type,
            name,
            table_name,
            root_page,
            sql,
        })
    }
}
