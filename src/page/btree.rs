use super::super::header::DatabaseHeader;
use super::header::{PageHeader, PageType};
use crate::page::cell::index_interior::IndexInteriorCell;
use crate::page::cell::index_leaf::IndexLeafCell;
use crate::page::cell::table_interior::TableInteriorCell;
use crate::page::cell::table_leaf::TableLeafCell;
use crate::page::cell::{build_serial_values, Schema, SerialValue};
use anyhow::{bail, Result};
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

        Ok(BTreePage {
            file_header,
            page_header,
            cell_pointer_array,
            data,
        })
    }

    pub fn build_root_page(file: &mut File) -> Result<BTreePage> {
        let mut header = [0; 100];

        file.read_exact(&mut header)?;
        let database_header = DatabaseHeader::try_from(&header[..])?;

        let page_size = u16::from_be_bytes(database_header.page_size) as usize;
        let mut root_page = vec![0; page_size - 100];
        file.read_exact(&mut root_page)?;

        let root_page = BTreePage::new(root_page, Some(database_header))?;
        Ok(root_page)
    }

    // Takes a File, uses the page number and page size to figure out where in the file the page is,
    // builds the page
    pub fn build_page(file: &mut File, page_size: usize, page_num: usize) -> Result<BTreePage> {
        let page_offset = (page_num as u64 - 1) * page_size as u64;

        file.seek(SeekFrom::Start(page_offset))?;

        let mut page = vec![0; page_size];
        file.read_exact(&mut page)?;

        let page = BTreePage::new(page, None)?;
        Ok(page)
    }

    pub fn get_index_page_old(&self) -> Result<Option<u32>> {
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

    pub fn table_interior_cells(&self) -> Result<Vec<TableInteriorCell>> {
        self.build_cells(|data| TableInteriorCell::try_from(data))
    }

    pub fn table_leaf_cells(&self) -> Result<Vec<TableLeafCell>> {
        self.build_cells(|data| TableLeafCell::try_from(data))
    }

    pub fn index_leaf_cells(&self) -> Result<Vec<IndexLeafCell>> {
        self.build_cells(|data| IndexLeafCell::try_from(data))
    }

    pub fn index_interior_cells(&self) -> Result<Vec<IndexInteriorCell>> {
        self.build_cells(|data| IndexInteriorCell::try_from(data))
    }

    fn build_cells<T, F>(&self, parser: F) -> Result<Vec<T>>
    where
        F: Fn(&[u8]) -> Result<T>,
    {
        let mut cells = Vec::with_capacity(self.cell_pointer_array.len());

        for pointer in &self.cell_pointer_array {
            let offset = if self.file_header.is_some() {
                (*pointer - 100) as usize
            } else {
                *pointer as usize
            };

            let cell = parser(&self.data[offset..])?;
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

    pub fn find_column(&self, table: &str, column: &str) -> Result<Schema> {
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
    let page = BTreePage::build_page(file, page_size as usize, page_number)?;
    // println!("{:?}", value);

    match page.page_header.page_type {
        PageType::IndexInterior => {
            let cells = page.index_interior_cells()?;

            for cell in &cells {
                let mut info = build_serial_values(&cell.payload)?;

                let key = info.remove(0);
                let row_id = info.remove(0);

                let row_id = if let SerialValue::Integer(row_id) = row_id {
                    Some(row_id)
                } else {
                    None
                };

                match key {
                    SerialValue::Text(key) => {
                        if value < key.as_str() {
                            get_row_ids(file, page_size, cell.left_child as usize, rows, value)?;
                        }

                        if let Some(row_id) = row_id {
                            if value == key.as_str() {
                                rows.push(row_id);
                                get_row_ids(file, page_size, cell.left_child as usize, rows, value)?
                            }
                        };
                    }
                    SerialValue::Integer(key) => {
                        let value: u64 = value.parse().unwrap();
                        let string_value = value.to_string();
                        if value < key {
                            get_row_ids(
                                file,
                                page_size,
                                cell.left_child as usize,
                                rows,
                                &string_value,
                            )?;
                        }

                        if let Some(row_id) = row_id {
                            if value == key {
                                rows.push(row_id);
                                get_row_ids(
                                    file,
                                    page_size,
                                    cell.left_child as usize,
                                    rows,
                                    &string_value,
                                )?;
                            }
                        };
                    }

                    SerialValue::Null => {}
                    _ => {}
                };
            }

            get_row_ids(
                file,
                page_size,
                page.page_header.right_page_number.unwrap() as usize,
                rows,
                value,
            )?;
        }
        PageType::IndexLeaf => {
            for cell in page.index_leaf_cells()? {
                let mut info = build_serial_values(&cell.payload)?;
                let key = info.remove(0);

                let row_id = info.remove(0);

                let row_id = if let SerialValue::Integer(row_id) = row_id {
                    Some(row_id)
                } else {
                    None
                };

                match key {
                    SerialValue::Text(key) => {
                        if value == key.as_str() {
                            if let Some(row_id) = row_id {
                                rows.push(row_id);
                            };
                        }
                    }
                    SerialValue::Integer(key) => {
                        let value: u64 = value.parse().unwrap();
                        if value == key {
                            if let Some(row_id) = row_id {
                                rows.push(row_id);
                            };
                        }
                    }
                    _ => todo!(),
                }

                // println!("{:?}. {}", key, row_id);
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
            let cells = page.table_interior_cells()?;

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
                if row_id == cell.row_id {
                    let values = build_serial_values(&cell.payload)?;
                    let row = Row {
                        row_id: cell.row_id,
                        values,
                    };
                    rows.push(row);
                }
            }
        }
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
                let values = build_serial_values(&cell.payload)?;
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
        _ => unreachable!("Invalid page type for sqlite_schema"),
    }

    Ok(())
}

#[derive(Debug)]
pub struct Row {
    pub row_id: u64,
    pub values: Vec<SerialValue>,
}

pub fn traverse_schema_btree(
    file: &mut File,
    page_size: u16,
    page_number: usize,
    schemas: &mut Vec<Schema>,
) -> Result<()> {
    let page = BTreePage::build_page(file, page_size as usize, page_number)?;

    match page.page_header.page_type {
        PageType::TableLeaf => {
            for cell in page.table_leaf_cells()? {
                let schema = cell.sqlite_schema()?;

                schemas.push(schema);
            }
        }

        PageType::TableInterior => {
            for cell in page.table_interior_cells()? {
                traverse_schema_btree(file, page_size, cell.left_child as usize, schemas)?;
            }

            // traverse_schema_btree(
            //     file,
            //     page_size,
            //     page.page_header.right_page_number.unwrap() as usize,
            //     table_names,
            // )?;
        }

        _ => unreachable!("Invalid page type for sqlite_schema"),
    }

    Ok(())
}

pub fn schemas(file: &mut File, page_size: u16, page: &BTreePage) -> Result<Vec<Schema>> {
    let mut schemas = Vec::new();

    match page.page_header.page_type {
        PageType::TableInterior => {
            let cells = page.table_interior_cells()?;
            for cell in cells {
                traverse_schema_btree(file, page_size, cell.left_child as usize, &mut schemas)?;
            }
        }
        PageType::TableLeaf => {
            for cell in page.table_leaf_cells()? {
                let schema = cell.sqlite_schema()?;

                schemas.push(schema);
            }
        }
        _ => unreachable!("Invalid page type for sqlite_schema"),
    }

    Ok(schemas)
}

pub fn get_table_names(schemas: Vec<Schema>) -> Vec<String> {
    schemas
        .iter()
        .filter_map(|schema| {
            if schema.schema_type == "table" {
                Some(schema.table_name.clone())
            } else {
                None
            }
        })
        .collect()
}

pub fn find_table_page(schemas: &[Schema], table: &str) -> Result<usize> {
    let schema = schemas
        .iter()
        .find(|schema| schema.table_name == table)
        .unwrap();

    Ok(schema.root_page)
}

pub fn find_column(schemas: &[Schema], table: &str, column: &str) -> Result<Schema> {
    let schema = schemas.iter().find(|schema| {
        schema
            .sql
            .to_lowercase()
            .as_str()
            .contains(column.to_lowercase().as_str())
            && schema.table_name == table
    });

    match schema {
        Some(schema) => Ok(schema.clone()),
        None => bail!("Column not found"),
    }
}

pub fn column_index(schemas: &[Schema], column: &str, table_name: &str) -> Result<usize> {
    let schema = find_column(schemas, table_name, column)?;
    let position = schema.column_position(column)?;

    Ok(position)
}

pub fn indicies_of_columns(
    schemas: &[Schema],
    columns: Vec<String>,
    table_name: &str,
) -> Result<Vec<usize>> {
    columns
        .into_iter()
        .map(|column| column_index(schemas, &column, table_name))
        .collect()
}

pub fn read_columns(rows: Vec<Row>, column_indices: Vec<usize>) -> Vec<Vec<String>> {
    let mut column_values = Vec::new();
    for row in rows {
        let mut row_cols = Vec::new();
        if column_indices.contains(&0) {
            row_cols.push(row.row_id.to_string())
        }
        for column_index in column_indices.as_slice() {
            match &row.values[*column_index] {
                SerialValue::Text(value) => row_cols.push(value.clone()),
                SerialValue::Null => (),
                _ => todo!(),
            };
        }
        column_values.push(row_cols);
    }

    column_values
}

pub fn display_columns(columns: Vec<Vec<String>>) {
    for row in columns {
        let columns = row.join("|");
        println!("{columns}");
    }
}

pub fn get_index_page(schemas: &[Schema], table: &str, column: &str) -> Result<Option<u32>> {
    let index_schemas: Vec<Schema> = schemas
        .iter()
        .filter(|schema| schema.schema_type == "index")
        .cloned()
        .collect();

    // println!("{:?}", index_schemas);

    let table_index = index_schemas.iter().find(|schema| {
        schema.table_name == table
            && schema
                .sql
                .to_lowercase()
                .as_str()
                .contains(column.to_lowercase().as_str())
    });

    match table_index {
        Some(schema) => {
            let root_page_num = schema.root_page;
            Ok(Some(root_page_num as u32))
        }
        None => Ok(None),
    }
}

pub fn indexes(schemas: &[Schema]) {
    for schema in schemas {
        if schema.schema_type == "index" {
            println!("{:?}", schema)
        };
    }
}
