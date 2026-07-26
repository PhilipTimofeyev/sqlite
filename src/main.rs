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
            // Lists number of tables and page size
            let page_size = u16::from_be_bytes(root_page.file_header.unwrap().page_size);

            println!("number of tables: {}", root_page.page_header.cell_count);
            println!("database page size: {page_size}");
        }
        ".tables" => {
            // Lists all table names in database, including sqlite_sequence table
            let table_names = btree::get_table_names(schemas);
            page::btree::display_string_vector(table_names)?;
        }
        cmd if cmd.to_lowercase().contains("select count(*)") => {
            // Example command: "SELECT COUNT(*) FROM apples"
            // Last argument is the table to get count from

            let mut split_command = command::split_command(cmd);
            let table_name = split_command.remove(split_command.len() - 1);
            let page_number = btree::find_table_page(schemas.as_slice(), &table_name)?;

            let mut rows = Vec::new();
            page::btree::full_table_scan(&mut file, page_size, page_number, &mut rows)?;

            println!("{}", rows.len())
        }
        cmd if cmd.to_lowercase().contains("select *") => {
            // Displays all rows from a table
            let split_command = command::split_command(cmd);
            let table_name = command::parse_command_table_name(&split_command)?;

            // Search root page cells for specific table, returning row id of table
            let page_number = btree::find_table_page(schemas.as_slice(), &table_name)? as usize;

            let mut rows = Vec::new();
            btree::full_table_scan(&mut file, page_size, page_number, &mut rows)?;

            let rows = btree::read_rows(rows);
            btree::display_columns(rows);

            // btree::display_columns(rows)
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

            // Get column index of each specified column
            let column_index = btree::column_index(&schemas, &where_column, &table_name)?;
            let column_indices =
                btree::indicies_of_columns(schemas.as_slice(), columns, &table_name)?;

            let index_page = btree::get_index_page(&schemas, &table_name, &where_column)?;

            where_command_search(
                index_page,
                column_indices.as_slice(),
                &mut file,
                page_size,
                page_number,
                &where_column_value,
                column_index,
            )?;
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
            //
            // // // Get column index of each specified column
            let column_indices =
                btree::indicies_of_columns(schemas.as_slice(), columns, &table_name)?;
            //
            let mut rows = Vec::new();
            page::btree::full_table_scan(&mut file, page_size, page_number, &mut rows)?;

            let columns = btree::read_columns(rows, column_indices.as_slice());
            btree::display_columns(columns);
        }
        _ => bail!("Missing or invalid command passed: {}", command),
    }

    Ok(())
}

fn where_command_search(
    index_page: Option<u32>,
    column_indices: &[usize],
    file: &mut File,
    page_size: u16,
    page_number: usize,
    where_column_value: &str,
    column_index: usize,
) -> Result<()> {
    match index_page {
        Some(index_page) => {
            //Sorted by index key greatest to least
            let mut row_ids = Vec::new();
            page::btree::search_index(
                file,
                page_size,
                index_page as usize,
                &mut row_ids,
                where_column_value,
            )?;

            let mut rows = Vec::new();

            // Sort by index key in order
            row_ids.reverse();
            for row_id in row_ids {
                page::btree::traverse_b_tree_table(
                    file,
                    page_size,
                    page_number,
                    &mut rows,
                    row_id,
                )?;
            }

            let columns = btree::read_columns(rows, column_indices);
            btree::display_columns(columns);
        }
        None => {
            let mut rows = Vec::new();
            page::btree::full_table_scan(file, page_size, page_number, &mut rows)?;

            let mut filtered_rows = Vec::new();
            for row in rows {
                let col_value = match &row.values[column_index] {
                    SerialValue::Text(value) => value.to_string(),
                    SerialValue::Integer(value) => value.to_string(),
                    SerialValue::Float(value) => value.to_string(),
                    SerialValue::Null => row.row_id.to_string(),
                    _ => todo!(),
                };

                if col_value == where_column_value {
                    filtered_rows.push(row)
                };
            }

            let columns = btree::read_columns(filtered_rows, column_indices);
            btree::display_columns(columns);
        }
    };

    Ok(())
}
