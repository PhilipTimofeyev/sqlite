use super::super::header::DatabaseHeader;
use super::header::PageHeader;
use crate::page::cell::table_leaf;
use anyhow::Result;
use std::fs::File;
use std::io::{Cursor, Read};

#[derive(Debug)]
pub struct BTreePage {
    pub file_header: Option<DatabaseHeader>,
    pub page_header: PageHeader,
    pub cell_pointer_array: Vec<u16>,
    unallocated_space: Vec<u8>,
    cell_content_area: Vec<u8>,
    reserved_region: Vec<u8>,
}

impl BTreePage {
    fn new(bytes: Vec<u8>, file_header: Option<DatabaseHeader>) -> BTreePage {
        let mut cursor = Cursor::new(bytes);
        let page_header = PageHeader::new(&mut cursor).unwrap();
        let cell_pointer_array = BTreePage::pointers(&mut cursor, &page_header);
        let unallocated_space_size = if file_header.is_some() {
            page_header.cell_content_area_start as u64 - 100 - cursor.position()
        } else {
            page_header.cell_content_area_start as u64 - cursor.position()
        };
        let mut unallocated_space = vec![0; unallocated_space_size as usize];
        let _ = cursor.read_exact(&mut unallocated_space);
        let mut cell_content_area = Vec::new();
        let _ = cursor.read_to_end(&mut cell_content_area);
        let reserved_region = Vec::default();

        BTreePage {
            file_header,
            page_header,
            cell_pointer_array,
            unallocated_space,
            cell_content_area,
            reserved_region,
        }
    }

    pub fn build_pages(file: &mut File) -> Result<Vec<BTreePage>> {
        let mut header = [0; 100];

        file.read_exact(&mut header)?;
        let database_header = DatabaseHeader::try_from(&header[..])?;

        let page_size = u16::from_be_bytes(database_header.page_size) as usize;
        let mut root_page = vec![0; page_size - 100];
        file.read_exact(&mut root_page)?;

        let root_page = BTreePage::new(root_page, Some(database_header));
        let mut pages = vec![root_page];

        loop {
            let mut buf = vec![0; page_size];
            if file.read_exact(&mut buf).is_err() {
                return Ok(pages);
            };
            let b_tree = BTreePage::new(buf, None);
            pages.push(b_tree);
        }
    }

    pub fn cells(&self) -> Result<Vec<table_leaf::TableLeafCell>> {
        let mut file = Cursor::new(self.cell_content_area.clone());
        let mut cell_pointers_peek = self.cell_pointer_array.iter().rev().peekable();
        let mut cells = Vec::new();

        while let Some(pointer) = cell_pointers_peek.next() {
            if let Some(next_pointer) = cell_pointers_peek.peek() {
                let num_bytes_to_read = *next_pointer - pointer;
                let mut buf = vec![0; num_bytes_to_read as usize];
                let _ = file.read_exact(&mut buf);
                let table_leaf_cell = table_leaf::TableLeafCell::try_from(&buf[..])?;
                cells.push(table_leaf_cell)
            } else {
                let mut buf = Vec::new();
                let _ = file.read_to_end(&mut buf);
                let table_leaf_cell = table_leaf::TableLeafCell::try_from(&buf[..])?;
                cells.push(table_leaf_cell);
            }
        }

        Ok(cells)
    }

    pub fn table_names(&self) -> Result<Vec<String>> {
        let table_names: Result<Vec<String>,_> = self.cells()?.iter().rev().map(|cell| {
               let table_name_bytes = cell.schema().table_name;
           String::from_utf8(table_name_bytes)   
        }).collect();

        Ok(table_names?)
    }



    pub fn find_table_page(&self, table: String) -> Option<u8> {
        let cells = &self.cells().unwrap();
        let table_cell = cells
            .iter()
            .find(|cell| String::from_utf8(cell.schema().table_name) == Ok(table.clone()));
        table_cell.map(|cell| cell.row_id)
    }

    fn pointers(file: &mut Cursor<Vec<u8>>, page_header: &PageHeader) -> Vec<u16> {
        let mut cell_buf = [0; 2];
        let mut cell_pointers = Vec::new();
        for _ in 0..page_header.cell_count {
            let _ = file.read_exact(&mut cell_buf);
            let pointer = u16::from_be_bytes(cell_buf);
            cell_pointers.push(pointer);
        }

        cell_pointers
    }
}

    pub fn display_string_vector(table_names: Vec<String>) -> Result<()> {
        for name in table_names {
            println!("{name}");
        }

        Ok(())
    }
