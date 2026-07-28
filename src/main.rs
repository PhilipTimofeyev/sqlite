use crate::page::cell::SerialValue;
use anyhow::{Result, bail};
use sqlite::command;
use sqlite::page::btree;
use sqlite::page::{self};
use std::fs::File;

fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    match args.len() {
        0 | 1 => bail!("Missing <database path> and <command>"),
        2 => bail!("Missing <command>"),
        _ => {}
    }

    let command = &args[2];
    let mut file = File::open(&args[1])?;

    let root_page = page::btree::BTreePage::build_root_page(&mut file)?;
    let page_size = u16::from_be_bytes(root_page.file_header.as_ref().unwrap().page_size);

    let schemas = btree::schemas(&mut file, page_size, &root_page)?;

    match command.as_str() {
        ".dbinfo" => {
            println!("number of tables: {}", root_page.page_header.cell_count);
            println!("database page size: {page_size}");
        }
        ".tables" => {
            // Lists all table names in database, including sqlite_sequence table
            let table_names = btree::get_table_names(schemas);
            page::btree::display_string_vector(table_names)?;
        }
        ".indexes" => {
            btree::indexes(&schemas);
        }
        ".schema" => {
            let table = &args.get(3);

            match table {
                Some(table) => {
                    let schema = btree::find_table_schema(&schemas, table)?;
                    println!("{}", schema.sql);
                }
                None => {
                    for schema in schemas {
                        println!("{}\n", schema.sql)
                    }
                }
            };
        }
        cmd if cmd.to_lowercase().contains("select count(*)") => {
            // Example command: "SELECT COUNT(*) FROM apples"
            // Last argument is the table to get count from

            let mut split_command = command::split_command(cmd);
            let table_name = split_command.remove(split_command.len() - 1);
            let page_number = btree::find_table_page(schemas.as_slice(), &table_name)?;

            let mut page_location = btree::PageLocation {
                page_size,
                page_number,
            };

            let mut rows = Vec::new();
            page::btree::full_table_scan(&mut file, &mut page_location, &mut rows)?;

            println!("{}", rows.len())
        }
        cmd if cmd.to_lowercase().contains("select *") => {
            // Displays all rows from a table
            let split_command = command::split_command(cmd);
            let table_name = command::parse_command_table_name(&split_command)?;

            // Search root page cells for specific table, returning row id of table
            let page_number = btree::find_table_page(schemas.as_slice(), &table_name)? as usize;

            let mut page_location = btree::PageLocation {
                page_size,
                page_number,
            };

            let mut rows = Vec::new();
            page::btree::full_table_scan(&mut file, &mut page_location, &mut rows)?;

            let rows = btree::read_rows(rows);
            btree::display_columns(rows);
        }
        cmd if cmd.to_lowercase().contains("where") => {
            // Example command: "SELECT name, color FROM apples WHERE color = 'Yellow'"
            // Everything between SELECT and FROM are the columns, retaining order
            // Everything between FROM and WHERE are the tables
            // After WHERE is the specific column

            let split_command = command::split_command(cmd);
            let table_name = command::parse_command_table_name(&split_command)?;
            let columns = command::parse_command_columns(&split_command);
            let (where_column, where_column_value) = command::parse_command_where(&split_command)?;

            // Search root page cells for specific table, returning row id of table
            let page_number = btree::find_table_page(schemas.as_slice(), &table_name)? as usize;

            let mut page_location = btree::PageLocation {
                page_size,
                page_number,
            };

            // Get column index of each specified column
            let column_index = btree::column_index(&schemas, &where_column, &table_name)?;
            let column_indices =
                btree::indicies_of_columns(schemas.as_slice(), columns, &table_name)?;

            let index_page = btree::get_index_page(&schemas, &table_name, &where_column)?;

            let rows = where_command_search(
                &mut file,
                &mut page_location,
                index_page,
                &where_column_value,
                column_index,
            )?;

            let columns = btree::read_columns(&rows, &column_indices);

            btree::display_columns(columns);
        }
        cmd if cmd.to_lowercase().contains("select") => {
            // Example command: "SELECT name, color FROM oranges"
            // Everything between SELECT and FROM are the columns, retaining order
            // Last word is table name

            let split_command = command::split_command(cmd);
            let table_name = command::parse_command_table_name(&split_command)?;
            let columns = command::parse_command_columns(&split_command);

            // Search root page cells for specific table, returning row id of table
            let page_number = btree::find_table_page(schemas.as_slice(), &table_name)? as usize;

            let mut page_location = btree::PageLocation {
                page_size,
                page_number,
            };

            // Get column index of each specified column
            let column_indices =
                btree::indicies_of_columns(schemas.as_slice(), columns, &table_name)?;
            //
            let mut rows = Vec::new();
            page::btree::full_table_scan(&mut file, &mut page_location, &mut rows)?;

            let columns = btree::read_columns(&rows, column_indices.as_slice());
            btree::display_columns(columns);
        }
        _ => bail!("Missing or invalid command passed: {}", command),
    }

    Ok(())
}

fn where_command_search(
    file: &mut File,
    page_location: &mut btree::PageLocation,
    index_page: Option<u32>,
    where_column_value: &str,
    column_index: usize,
) -> Result<Vec<btree::Row>> {
    let mut rows = Vec::new();

    // If index page for column exists then search via index
    // If not then do a full table scan
    match index_page {
        Some(index_page) => {
            //Sorted by index key greatest to least
            let mut row_ids = Vec::new();
            let table_page_number = page_location.page_number;
            page_location.page_number = index_page as usize;

            page::btree::search_index(file, page_location, &mut row_ids, where_column_value)?;

            page_location.page_number = table_page_number;

            // Sort by index key in order
            for row_id in row_ids.iter().rev() {
                page::btree::search_table(file, page_location, &mut rows, *row_id)?;
                page_location.page_number = table_page_number;
            }

            Ok(rows)
        }
        None => {
            page::btree::full_table_scan(file, page_location, &mut rows)?;

            let mut filtered_rows = Vec::new();
            for row in rows.as_slice() {
                let col_value = match &row.values[column_index] {
                    SerialValue::Text(value) => value.to_string(),
                    SerialValue::Integer(value) => value.to_string(),
                    SerialValue::Float(value) => value.to_string(),
                    SerialValue::Null => row.row_id.to_string(),
                    _ => todo!(),
                };

                if col_value == where_column_value {
                    filtered_rows.push(row.clone())
                };
            }
            Ok(filtered_rows)
        }
    }
}
