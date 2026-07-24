use crate::page::cell::table_leaf::SerialValue;
use anyhow::{bail, Result};
use codecrafters_sqlite::command;
use codecrafters_sqlite::page::{self, btree};
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

    match command.as_str() {
        ".dbinfo" => {
            // Lists number of tables and page size

            let page_size = u16::from_be_bytes(root_page.file_header.unwrap().page_size);

            print!("number of tables: {}", root_page.page_header.cell_count);
            println!("database page size: {page_size}");
        }
        ".tables" => {
            // Lists all table names in database

            let table_names = root_page.table_names()?;
            page::btree::display_string_vector(table_names)?;
        }
        cmd if cmd.to_lowercase().contains("select count(*)") => {
            // Example command: "SELECT COUNT(*) FROM apples"
            // Last argument is the table to get count from

            let mut split_command = command::split_command(cmd);
            let table_name = split_command.remove(split_command.len() - 1);
            let page_number = root_page.find_table_page(&table_name)?;
            let page_size = u16::from_be_bytes(root_page.file_header.as_ref().unwrap().page_size);

            let mut rows = Vec::new();
            page::btree::traverse_b_tree(&mut file, page_size, page_number, &mut rows)?;

            println!("{}", rows.len())
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
            let page_number = root_page.find_table_page(&table_name)? as usize;
            let page_size = u16::from_be_bytes(root_page.file_header.as_ref().unwrap().page_size);

            // Get column index of each specified column
            let column_index = root_page.column_index(&where_column, &table_name)?;
            let column_indices = root_page.indicies_of_columns(columns, table_name)?;

            let index_page = root_page.get_index_page()?;

            match index_page {
                Some(index_page) => {
                    let mut row_ids = Vec::new();
                    page::btree::get_row_ids(
                        &mut file,
                        page_size,
                        index_page as usize,
                        &mut row_ids,
                        where_column_value.as_ref(),
                    )?;

                    let mut rows = Vec::new();

                    for row_id in row_ids {
                        page::btree::traverse_b_tree_table(
                            &mut file,
                            page_size,
                            page_number,
                            &mut rows,
                            row_id,
                        )?;
                    }

                    let columns = read_columns(rows, column_indices);
                    display_columns(columns);
                    //
                    // println!("{:?}", rows);
                }
                None => {
                    let mut rows = Vec::new();
                    page::btree::traverse_b_tree(&mut file, page_size, page_number, &mut rows)?;

                    let mut filtered_rows = Vec::new();
                    for row in rows {
                        let col_value = match &row.values[column_index] {
                            SerialValue::Text(value) => value.to_string(),
                            SerialValue::Null => row.row_id.to_string(),
                            _ => todo!(),
                        };

                        if col_value == where_column_value {
                            filtered_rows.push(row)
                        };
                    }

                    let columns = read_columns(filtered_rows, column_indices);
                    display_columns(columns);
                }
            }
        }
        cmd if cmd.to_lowercase().contains("select") => {
            // Example command: "SELECT name, color FROM oranges"
            // Everything between SELECT and FROM are the columns, retaining order
            // Last word is table name

            let split_command = command::split_command(cmd);
            let table_name = command::parse_command_table_name(&split_command)?;
            let columns = command::parse_command_columns(&split_command);

            // Search root page cells for specific table, returning row id of table
            let page_number = root_page.find_table_page(&table_name)? as usize;
            let page_size = u16::from_be_bytes(root_page.file_header.as_ref().unwrap().page_size);

            // // Get column index of each specified column
            let column_indices = root_page.indicies_of_columns(columns, table_name)?;

            let mut rows = Vec::new();
            page::btree::traverse_b_tree(&mut file, page_size, page_number, &mut rows)?;

            let columns = read_columns(rows, column_indices);
            display_columns(columns);
        }
        _ => bail!("Missing or invalid command passed: {}", command),
    }

    Ok(())
}

fn read_columns(rows: Vec<btree::Row>, column_indices: Vec<usize>) -> Vec<Vec<String>> {
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

fn display_columns(columns: Vec<Vec<String>>) {
    for row in columns {
        let columns = row.join("|");
        println!("{columns}");
    }
}
