use super::super::header::DatabaseHeader;
use super::header::{PageHeader, PageType};
use crate::page::cell::table_leaf::{self, parse_varint, TableInteriorCell, TableLeafCell};
use anyhow::{anyhow, bail, Result};
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom};

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

    // REFACTOR WITH ROOT PAGE
    pub fn build_page(file: &mut File, page_size: usize, page_num: usize) -> Result<BTreePage> {
        let page_offset = (page_num as u64 - 1) * page_size as u64;

        file.seek(SeekFrom::Start(page_offset))?;

        let mut page = vec![0; page_size];
        file.read_exact(&mut page)?;

        let page = BTreePage::new(page, None)?;
        Ok(page)
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

    // DELETE
    pub fn _old_cells(&self) -> Result<Vec<table_leaf::TableLeafCell>> {
        let mut file = Cursor::new(self.data.clone());
        println!("pointer array {:?}", self.cell_pointer_array);
        // println!("DATA {:?}", file);
        let mut cell_pointers_peek = self.cell_pointer_array.iter().rev().peekable();
        let mut cells = Vec::with_capacity(self.cell_pointer_array.len());

        // file.set_position(cell_pointers_peek.peek().unwrap().to_owned().to_owned() as u64);
        println!("HERE {:?}", self.page_header);
        while let Some(pointer) = cell_pointers_peek.next() {
            // Offset when root B Tree Page
            if self.file_header.is_some() {
                file.set_position(*pointer as u64 - 100);
            } else {
                file.set_position(*pointer as u64);
            }

            if let Some(next_pointer) = cell_pointers_peek.peek() {
                let num_bytes_to_read = *next_pointer - pointer;
                println!("next_pointer {}", next_pointer);
                println!("pointer {}", pointer);
                println!("bytes to read {}", num_bytes_to_read);
                let mut buf = vec![0; num_bytes_to_read as usize];
                // println!("First buf{:?}", buf);
                file.read_exact(&mut buf)?;
                let table_leaf_cell = table_leaf::TableLeafCell::try_from(&buf[..])?;
                // println!("{:?}", table_leaf_cell);
                cells.push(table_leaf_cell)
            } else {
                let mut buf = Vec::new();
                file.read_to_end(&mut buf)?;
                // println!("\n{:?}", buf);
                let table_leaf_cell = table_leaf::TableLeafCell::try_from(&buf[..])?;
                cells.push(table_leaf_cell);
            }
        }

        // println!("{:?}", cells);

        Ok(cells)
    }

    pub fn cells(&self) -> Result<Vec<table_leaf::TableLeafCell>> {
        let cell_pointers = &self.cell_pointer_array;

        let mut cells = Vec::with_capacity(self.cell_pointer_array.len());
        for pointer in cell_pointers {
            let offset = if self.file_header.is_some() {
                (pointer - 100) as usize
            } else {
                *pointer as usize
            };
            let cell = TableLeafCell::try_from(&self.data[offset..])?;
            cells.push(cell);
        }

        Ok(cells)
    }

    pub fn cells_int(&self) -> Result<Vec<table_leaf::TableInteriorCell>> {
        let cell_pointers = &self.cell_pointer_array;

        let mut cells = Vec::with_capacity(self.cell_pointer_array.len());
        for pointer in cell_pointers {
            let offset = if self.file_header.is_some() {
                (pointer - 100) as usize
            } else {
                *pointer as usize
            };
            let cell = TableInteriorCell::try_from(&self.data[offset..])?;
            cells.push(cell);
        }

        Ok(cells)
    }

    // pub fn find_cells(&self, column: &usize, value: &str) -> Vec<table_leaf::TableLeafCell> {
    //     self.cells()
    //         .unwrap()
    //         .iter()
    //         .filter(|cell| cell.search_value(*column, value))
    //         .cloned()
    //         .collect()
    // }

    // pub fn read_cell_columns(
    //     &self,
    //     columns: Vec<usize>,
    //     cells: Vec<table_leaf::TableLeafCell>,
    // ) -> Result<()> {
    //     for cell in cells {
    //         let row = columns
    //             .iter()
    //             .map(|i| cell.read_column(*i).unwrap())
    //             .collect::<Vec<String>>()
    //             .join("|");
    //         println!("{row}");
    //     }
    //
    //     Ok(())
    // }

    // pub fn indicies_of_columns(&self, columns: Vec<String>, table_name: String) -> Vec<usize> {
    //     columns
    //         .into_iter()
    //         .map(|column| {
    //             let schema = self.find_column(&table_name, &column).unwrap();
    //             schema.column_position(&column).unwrap()
    //         })
    //         .collect()
    // }

    // pub fn column_index(&self, column: &str, table_name: &str) -> usize {
    //     let schema = self.find_column(table_name, column).unwrap();
    //     schema.column_position(column).unwrap()
    // }

    pub fn table_names(&self) -> Result<Vec<String>> {
        let table_names: Vec<String> = self
            .cells()?
            .iter()
            .rev()
            .map(|cell| {
                let schema_definition = cell.sqlite_schema().unwrap();
                schema_definition.table_name
            })
            .collect();

        Ok(table_names)
    }

    pub fn find_table_page(&self, table: &str) -> Result<usize> {
        self.cells()?
            .iter()
            .find(|cell| {
                cell.sqlite_schema()
                    .is_ok_and(|schema| schema.table_name == table)
            })
            // .map(|cell| cell.row_id as usize)
            .map(|cell| cell.sqlite_schema().unwrap().root_page)
            .ok_or_else(|| anyhow!("Table `{}` not found", table))
    }

    // pub fn find_column(&self, table: &str, column: &str) -> Result<table_leaf::Schema> {
    //     for cell in self.cells()? {
    //         let schema = cell.sqlite_schema()?;
    //         if schema.sql_contains_str(column) && schema.table_name == table {
    //             return Ok(schema);
    //         }
    //     }
    //
    //     bail!("Column not found")
    // }

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

pub fn traverse_b_tree(file: &mut File, page_size: u16, page_number: usize) {
    let page = BTreePage::build_page(file, page_size as usize, page_number).unwrap();

    match page.page_header.page_type {
        PageType::TableInterior => {
            for child in page.cells_int().unwrap() {
                traverse_b_tree(file, page_size, child.left_child as usize);
            }
        }

        PageType::TableLeaf => {
            for cell in page.cells().unwrap() {
                println!("row id: {}, {:?}", cell.row_id, cell.build_schema_vec());
            }
        }
        _ => todo!(),
    }
}
