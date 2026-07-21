use super::super::header::DatabaseHeader;
use super::header::PageHeader;
use crate::page::cell::table_leaf;
use anyhow::{anyhow, bail, Result};
use std::fs::File;
use std::io::{Cursor, Read, Seek};

#[derive(Debug)]
pub struct BTreePage {
    pub file_header: Option<DatabaseHeader>,
    pub page_header: PageHeader,
    pub cell_pointer_array: Vec<u16>,
    data: Vec<u8>,
}

impl BTreePage {
    pub fn new(bytes: Vec<u8>, file_header: Option<DatabaseHeader>) -> Result<BTreePage> {
        let mut cursor = Cursor::new(bytes.clone());
        let page_header = PageHeader::new(&mut cursor)?;
        let cell_pointer_array = BTreePage::pointers(&mut cursor, &page_header)?;
        let unallocated_space_size = if file_header.is_some() {
            page_header.cell_content_area_start as u64 - 100 - cursor.position()
        } else {
            page_header.cell_content_area_start as u64 - cursor.position()
        };

        let data = bytes;
        // cursor.read_to_end(&mut data)?;

        Ok(BTreePage {
            file_header,
            page_header,
            cell_pointer_array,
            data,
        })
    }

    pub fn build_root_page(file: &mut File) -> Result<BTreePage> {
        // Build root page
        let mut header = [0; 100];

        file.read_exact(&mut header)?;
        let database_header = DatabaseHeader::try_from(&header[..])?;

        let page_size = u16::from_be_bytes(database_header.page_size) as usize;
        let mut root_page = vec![0; page_size - 100];
        file.read_exact(&mut root_page)?;

        let root_page = BTreePage::new(root_page, Some(database_header))?;
        Ok(root_page)
    }

    pub fn build_pages(file: &mut File) -> Result<Vec<BTreePage>> {
        // Build root page
        let mut header = [0; 100];

        file.read_exact(&mut header)?;
        let database_header = DatabaseHeader::try_from(&header[..])?;

        let page_size = u16::from_be_bytes(database_header.page_size) as usize;
        let mut root_page = vec![0; page_size - 100];
        file.read_exact(&mut root_page)?;

        let root_page = BTreePage::new(root_page, Some(database_header))?;

        // Build rest of pages
        let mut pages = vec![root_page];

        loop {
            let mut buf = vec![0; page_size];
            if file.read_exact(&mut buf).is_err() {
                return Ok(pages);
            };
            // println!("{buf:?}");
            let b_tree = BTreePage::new(buf, None)?;
            pages.push(b_tree);
        }
    }

    pub fn cells(&self) -> Result<Vec<table_leaf::TableLeafCell>> {
        let mut file = Cursor::new(self.data.clone());
        // println!("page header {:?}", self.page_header);
        // println!("DATA {:?}", file);
        // println!("{:?}", self.cell_pointer_array);
        let mut cell_pointers_peek = self.cell_pointer_array.iter().rev().peekable();
        let mut cells = Vec::with_capacity(self.cell_pointer_array.len());

        // file.set_position(cell_pointers_peek.peek().unwrap().to_owned().to_owned() as u64);
        // println!("HERE {:?}", self.page_header);
        while let Some(pointer) = cell_pointers_peek.next() {
            file.set_position(*pointer as u64 - 100);

            if let Some(next_pointer) = cell_pointers_peek.peek() {
                let num_bytes_to_read = *next_pointer - pointer;
                // println!("bytes to read {}", num_bytes_to_read);
                let mut buf = vec![0; num_bytes_to_read as usize];
                file.read_exact(&mut buf)?;
                // println!("First buf{:?}", buf);
                let table_leaf_cell = table_leaf::TableLeafCell::try_from(&buf[..])?;
                cells.push(table_leaf_cell)
            } else {
                let mut buf = Vec::new();
                file.read_to_end(&mut buf)?;
                let table_leaf_cell = table_leaf::TableLeafCell::try_from(&buf[..])?;
                cells.push(table_leaf_cell);
            }
        }

        Ok(cells)
    }

    //    pub fn cell(&self) {
    //        // THIS NEEDS TO LOOP AND CREATE CELLS, PARSING VARINTS AND BUILDING CELLS UNTIL ALL BYTES
    //        // HAVE BEEN READ
    //        let mut file = Cursor::new(self.data.clone());
    //        // println!("{:?}", self.data);
    //        let mut cell_pointers_peek = self.cell_pointer_array.iter().peekable();
    //        // let mut cells = Vec::with_capacity(self.cell_pointer_array.len());
    //
    //        // println!("HERE {:?}", self.data);
    //        let first_pointer = self.cell_pointer_array[0];
    //        println!("POINTER ARRAY{:?}", self.cell_pointer_array);
    //
    //        for pointer in self.cell_pointer_array.iter().as_slice() {
    //            // println!("first pointer {:?}", first_pointer);
    //            // println!("pointer {:?}", pointer);
    //                // let hmm = first_pointer - pointer;
    //                // println!("well {hmm:?}");
    //                let table_leaf_cell =
    //                    table_leaf::TableLeafCell::try_from(&self.data[*pointer as usize..]);
    //                // println!("{:?}", table_leaf_cell);
    //                // let well = table_leaf_cell.unwrap().build_schema_vec();
    //                // // println!("First buf{:?}", table_leaf_cell);
    //                // for a in well.unwrap() {
    //                //     println!("{:?}", String::from_utf8(a))
    //                // }
    //
    //        }
    //        // let pointer = cell_pointers_peek.next().unwrap();
    //        // println!("cell content {:?}", self.cell_content_area);
    //        // // file.set_position(*pointer as u64);
    //        // // let (payload, bytes) = parse_varint(&self.cell_content_area).unwrap();
    //        // // let (row_id, bytes) = parse_varint(bytes).unwrap();
    //        //
    //        // for pointer in self.cell_pointer_array.clone() {
    //        //     println!("pointer {:?}", pointer);
    //        // }
    //
    //        // println!("payload {:?}", payload);
    //        // println!("row_id {:?}", row_id);
    //        // println!("CUrrent {pointer:?}");
    //        // let next_pointer = cell_pointers_peek.peek().unwrap();
    //        // println!("NExt {next_pointer:?}");
    //        // let num_bytes_to_read = *next_pointer - pointer;
    //        // let mut buf = vec![0; num_bytes_to_read as usize];
    //        // println!("{num_bytes_to_read:?}");
    //        // file.read_exact(&mut buf)?;
    //        // let hmm = parse_varint(&buf);
    //        // // println!("varint{:?}", hmm);
    //        // // println!("First buf{:?}", buf);
    //        // let table_leaf_cell = table_leaf::TableLeafCell::try_from(&self.cell_content_area[..]);
    //        // let well = table_leaf_cell.unwrap().build_schema_vec();
    //        // // println!("First buf{:?}", table_leaf_cell);
    //        // for a in well.unwrap() {
    //        //     println!("{:?}", String::from_utf8(a))
    //        // }
    //        // println!("{:?}", table_leaf_cell.unwrap());
    // ; // // todo!();
    //         // Ok(table_leaf_cell)
    //    }

    pub fn find_cells(&self, column: &usize, value: &str) -> Vec<table_leaf::TableLeafCell> {
        self.cells()
            .unwrap()
            .iter()
            .filter(|cell| cell.search_value(column, value))
            .cloned()
            .collect()
    }

    pub fn read_cell_columns(
        &self,
        columns: Vec<usize>,
        cells: Vec<table_leaf::TableLeafCell>,
    ) -> Result<()> {
        for cell in cells {
            let row = columns
                .iter()
                .map(|i| cell.read_column(i).unwrap())
                .collect::<Vec<String>>()
                .join("|");
            // println!("{row}");
        }

        Ok(())
    }

    pub fn indicies_of_columns(&self, columns: Vec<String>, table_name: String) -> Vec<usize> {
        columns
            .into_iter()
            .map(|column| {
                let schema = self.find_column(&table_name, &column).unwrap();
                schema.column_position(&column).unwrap()
            })
            .collect()
    }

    pub fn column_index(&self, column: &str, table_name: &str) -> usize {
        let schema = self.find_column(table_name, column).unwrap();
        schema.column_position(column).unwrap()
    }

    pub fn table_names(&self) -> Result<Vec<String>> {
        let table_names: Result<Vec<String>, _> = self
            .cells()?
            .iter()
            .rev()
            .map(|cell| {
                let schema_definition = cell.sqlite_schema()?;
                String::from_utf8(schema_definition.table_name).map_err(anyhow::Error::from)
            })
            .collect();

        table_names
    }

    pub fn find_table_page(&self, table: &str) -> Result<usize> {
        self.cells()?
            .iter()
            .find(|cell| {
                cell.sqlite_schema()
                    .is_ok_and(|schema| schema.table_name == table.as_bytes())
            })
            .map(|cell| cell.row_id as usize)
            .ok_or_else(|| anyhow!("Table `{}` not found", table))
    }

    pub fn find_column(&self, table: &str, column: &str) -> Result<table_leaf::Schema> {
        for cell in self.cells()? {
            let schema = cell.sqlite_schema()?;
            if schema.sql_contains_str(column) && schema.table_name == table.as_bytes() {
                return Ok(schema);
            }
        }

        bail!("Column not found")
    }

    fn pointers(file: &mut Cursor<Vec<u8>>, page_header: &PageHeader) -> Result<Vec<u16>> {
        let mut cell_buf = [0; 2];
        let mut cell_pointers = Vec::new();
        for _ in 0..page_header.cell_count {
            file.read_exact(&mut cell_buf)?;
            let pointer = u16::from_be_bytes(cell_buf);
            cell_pointers.push(pointer);
        }

        Ok(cell_pointers)
    }
}

pub fn display_string_vector(vector: Vec<String>) -> Result<()> {
    for string in vector {
        println!("{string}");
    }

    Ok(())
}

fn parse_varint(data: &[u8]) -> Option<(u64, &[u8])> {
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
