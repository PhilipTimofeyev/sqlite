use super::super::header::DatabaseHeader;
use super::header::{PageHeader, PageType};
use crate::page::cell::table_leaf::{
    self, IndexInteriorCell, IndexLeafCell, SerialValue, TableInteriorCell, TableLeafCell,
};
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
        let _unallocated_space_size = if file_header.is_some() {
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

    pub fn get_index_page(&self) -> Result<Option<u32>> {
        let cells = self.table_leaf_cells()?;

        let table_leaf_cell = cells.into_iter().find(|cell| cell.is_index_page().unwrap());

        match table_leaf_cell {
            Some(cell) => {
                let root_page_num = cell.sqlite_schema().unwrap().root_page;
                Ok(Some(root_page_num as u32))
            }
            None => Ok(None),
        }
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

    pub fn table_interior_cells(&self) -> Result<Vec<table_leaf::TableInteriorCell>> {
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

    pub fn index_interior_cells(&self) -> Result<Vec<table_leaf::IndexInteriorCell>> {
        let cell_pointers = &self.cell_pointer_array;

        let mut cells = Vec::with_capacity(self.cell_pointer_array.len());
        for pointer in cell_pointers {
            let offset = if self.file_header.is_some() {
                (pointer - 100) as usize
            } else {
                *pointer as usize
            };
            let cell = IndexInteriorCell::try_from(&self.data[offset..])?;
            cells.push(cell);
        }

        Ok(cells)
    }

    pub fn index_leaf_cells(&self) -> Result<Vec<table_leaf::IndexLeafCell>> {
        let cell_pointers = &self.cell_pointer_array;

        let mut cells = Vec::with_capacity(self.cell_pointer_array.len());
        for pointer in cell_pointers {
            let offset = if self.file_header.is_some() {
                (pointer - 100) as usize
            } else {
                *pointer as usize
            };
            let cell = IndexLeafCell::try_from(&self.data[offset..])?;
            cells.push(cell);
        }

        Ok(cells)
    }

    pub fn table_leaf_cells(&self) -> Result<Vec<table_leaf::TableLeafCell>> {
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

    pub fn indicies_of_columns(
        &self,
        columns: Vec<String>,
        table_name: String,
    ) -> Result<Vec<usize>> {
        columns
            .into_iter()
            .map(|column| {
                let schema = self.find_column(&table_name, &column)?;
                let position = schema.column_position(&column)?;
                Ok(position)
            })
            .collect()
    }

    pub fn column_index(&self, column: &str, table_name: &str) -> Result<usize> {
        let schema = self.find_column(table_name, column)?;
        let position = schema.column_position(column)?;

        Ok(position)
    }

    pub fn table_names(&self) -> Result<Vec<String>> {
        let mut table_names = Vec::new();

        for cell in self.table_leaf_cells()? {
            let schema = cell.sqlite_schema()?;
            table_names.push(schema.table_name);
        }

        Ok(table_names)
    }

    pub fn find_table_page(&self, table: &str) -> Result<usize> {
        let cells = self.table_leaf_cells()?;

        cells
            .into_iter()
            .find(|cell| {
                cell.sqlite_schema()
                    .is_ok_and(|schema| schema.table_name == table)
            })
            .map(|cell| cell.sqlite_schema().unwrap().root_page)
            .ok_or_else(|| anyhow!("Table `{}` not found", table))
    }

    pub fn find_column(&self, table: &str, column: &str) -> Result<table_leaf::Schema> {
        for cell in self.table_leaf_cells()? {
            let schema = cell.sqlite_schema()?;
            if schema.sql_contains_str(column) && schema.table_name == table {
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

pub fn get_row_ids(
    file: &mut File,
    page_size: u16,
    page_number: usize,
    rows: &mut Vec<u64>,
    value: &str,
) -> Result<()> {
    let page = BTreePage::build_page(file, page_size as usize, page_number).unwrap();

    match page.page_header.page_type {
        PageType::IndexInterior => {
            let cells = page.index_interior_cells()?;

            for cell in &cells {
                let mut info = cell.build_serial_types()?;

                let text = info.remove(0);
                let key = if let SerialValue::Text(key) = text {
                    Some(key)
                } else {
                    None
                };

                let row_id = info.remove(0);

                let row_id = if let SerialValue::Integer(row_id) = row_id {
                    Some(row_id)
                } else {
                    None
                };

                if let Some(key) = key {
                    if value < key.as_str() {
                        get_row_ids(file, page_size, cell.left_child as usize, rows, value)?;
                    }

                    if let Some(row_id) = row_id {
                        if value == key {
                            rows.push(row_id);
                            get_row_ids(file, page_size, cell.left_child as usize, rows, value)?;
                        }
                    }
                }
            }

            // Target is greater than all separators,
            // or equal to the last separator.
            get_row_ids(
                file,
                page_size,
                page.page_header.right_page_number.unwrap() as usize,
                rows,
                value,
            )?;
        }
        PageType::IndexLeaf => {
            for cell in page.index_leaf_cells().unwrap() {
                let mut info = cell.build_serial_types().unwrap();
                let text = info.remove(0);
                let key = if let SerialValue::Text(key) = text {
                    Some(key)
                } else {
                    None
                }
                .unwrap();

                let row_id = info.remove(0);

                let row_id = if let SerialValue::Integer(row_id) = row_id {
                    Some(row_id)
                } else {
                    None
                }
                .unwrap();

                // println!("{:?}. {}", key, row_id);
                if key == value {
                    rows.push(row_id);
                }
            }
        }
        _ => todo!(),
    }

    Ok(())
}

pub fn traverse_b_tree_table(
    file: &mut File,
    page_size: u16,
    page_number: usize,
    rows: &mut Vec<Row>,
    row_id: u64,
) -> Result<()> {
    let page = BTreePage::build_page(file, page_size as usize, page_number).unwrap();

    match page.page_header.page_type {
        PageType::TableInterior => {
            let cells = page.table_interior_cells().unwrap();

            let cell = cells.iter().find(|cell| row_id < cell.row_id);

            match cell {
                Some(cell) => {
                    traverse_b_tree_table(file, page_size, cell.left_child as usize, rows, row_id)?;
                }
                None => {
                    traverse_b_tree_table(
                        file,
                        page_size,
                        page.page_header.right_page_number.unwrap() as usize,
                        rows,
                        row_id,
                    )?;
                }
            }
        }

        PageType::TableLeaf => {
            for cell in page.table_leaf_cells().unwrap() {
                // println!("{:?}", cell.row_id);
                // println!("{:?}", row_id);
                if row_id == cell.row_id {
                    let values = cell.build_serial_types().unwrap();
                    let row = Row {
                        row_id: cell.row_id,
                        values,
                    };
                    rows.push(row);
                }
            }
        } // PageType::IndexInterior => {

        _ => todo!(),
    }

    Ok(())
}

pub fn traverse_b_tree(
    file: &mut File,
    page_size: u16,
    page_number: usize,
    rows: &mut Vec<Row>,
) -> Result<()> {
    let page = BTreePage::build_page(file, page_size as usize, page_number).unwrap();

    match page.page_header.page_type {
        PageType::TableInterior => {
            for child in page.table_interior_cells().unwrap() {
                traverse_b_tree(file, page_size, child.left_child as usize, rows)?;
            }
        }

        PageType::TableLeaf => {
            for cell in page.table_leaf_cells().unwrap() {
                let values = cell.build_serial_types().unwrap();
                let row = Row {
                    row_id: cell.row_id,
                    values,
                };
                rows.push(row);
            }
        }
        PageType::IndexInterior => {
            for child in page.index_interior_cells().unwrap() {
                traverse_b_tree(file, page_size, child.left_child as usize, rows)?;
            }
        }
        PageType::IndexLeaf => {
            for cell in page.index_leaf_cells().unwrap() {
                let values = cell.build_serial_types().unwrap();
                // let row = Row {
                //     row_id: cell.row_id,
                //     values,
                // };
                // rows.push(row);
            }
        }
    }

    Ok(())
}

#[derive(Debug)]
pub struct Row {
    pub row_id: u64,
    pub values: Vec<SerialValue>,
}
