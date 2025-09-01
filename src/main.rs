use anyhow::{bail, Result};
use codecrafters_sqlite::command;
use codecrafters_sqlite::page::{self};
use std::fs::File;

fn main() -> Result<()> {
    // Parse arguments
    let args = std::env::args().collect::<Vec<_>>();
    match args.len() {
        0 | 1 => bail!("Missing <database path> and <command>"),
        2 => bail!("Missing <command>"),
        _ => {}
    }

    let command = &args[2];
    match command.as_str() {
        ".dbinfo" => {
            let mut file = File::open(&args[1])?;
            let mut pages = page::btree::BTreePage::build_pages(&mut file)?;

            let root_page = pages.remove(0);
            let page_size = u16::from_be_bytes(root_page.file_header.unwrap().page_size);

            print!("number of tables: {}", root_page.page_header.cell_count);
            println!("database page size: {page_size}");
        }
        ".tables" => {
            // Lists all table names in database

            let mut file = File::open(&args[1])?;
            let mut pages = page::btree::BTreePage::build_pages(&mut file)?;

            let root_page = pages.remove(0);

            let table_names = root_page.table_names()?;
            let _ = page::btree::display_string_vector(table_names);
        }
        cmd if cmd.to_lowercase().contains("select count(*)") => {
            // Example command: "SELECT COUNT(*) FROM apples"
            // Last argument is the table to get count from

            let mut file = File::open(&args[1])?;
            let mut pages = page::btree::BTreePage::build_pages(&mut file)?;

            let root_page = pages.remove(0);
            let mut split_command = command::split_command(cmd);
            let table_name = split_command.remove(split_command.len() - 1);
            let page_index = root_page.find_table_page(&table_name)?;

            if let Some(page_index) = page_index {
                let page = pages.remove(page_index - 1);
                println!("{}", page.cell_pointer_array.len())
            } else {
                bail!("Table not found")
            };
        }
        cmd if cmd.to_lowercase().contains("where") => {
            // Example command: "SELECT name, color FROM apples WHERE color = 'Yellow'"
            // Everything between SELECT and FROM are the columns, retaining order
            // Everything between FROM and WHERE are the tables
            // After WHERE is the specific column

            let mut file = File::open(&args[1])?;
            let mut pages = page::btree::BTreePage::build_pages(&mut file)?;

            let root_page = pages.remove(0);

            let split_command = command::split_command(cmd);
            let table_name = command::parse_command_table_name(split_command.clone());
            let specific_columns = command::parse_command_columns(split_command.clone());
            let mut all_columns = specific_columns.clone();
            let (column, column_value) = command::parse_command_where(split_command);

            if !all_columns.contains(&column) {
                all_columns.push(column.clone())
            }

            // println!("Columns {:?}", columns);
            // println!("Table {:?}", table_name);
            // println!("Column {column}, column value: {column_value}");

            // Search root page cells for specific table, returning row id of table
            let page_index = root_page.find_table_page(&table_name)?;
            let page = &pages[page_index.unwrap() - 1];
            //
            // // Get column index of each specified column
            let column_indexes = root_page.indicies_of_columns(all_columns, table_name);
            //
            page.read_cell_columns_where(column_indexes, column, column_value, specific_columns)?;
        }

        cmd if cmd.to_lowercase().contains("select") => {
            // Example command: "SELECT name, color FROM oranges"
            // Everything between SELECT and FROM are the columns, retaining order
            // Last word is table name

            let mut file = File::open(&args[1])?;
            let mut pages = page::btree::BTreePage::build_pages(&mut file)?;

            let root_page = pages.remove(0);

            let mut split_command = command::split_command(cmd);
            let table_name = split_command.remove(split_command.len() - 1);
            let columns = command::parse_command_columns(split_command);

            // Search root page cells for specific table, returning row id of table
            let page_index = root_page.find_table_page(&table_name)?;
            let page = &pages[page_index.unwrap() - 1];

            // Get column index of each specified column
            let column_indexes = root_page.indicies_of_columns(columns, table_name);

            page.read_cell_columns(column_indexes)?;
        }
        _ => bail!("Missing or invalid command passed: {}", command),
    }

    Ok(())
}
