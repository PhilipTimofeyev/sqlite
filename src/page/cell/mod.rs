pub mod index_interior;
pub mod index_leaf;
pub mod table_interior;
pub mod table_leaf;
use anyhow::Result;

#[allow(dead_code)]
#[derive(Debug)]
pub enum SerialType {
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
