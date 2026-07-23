use anyhow::{bail, Result};
use codecrafters_sqlite::command;
use codecrafters_sqlite::page::btree::BTreePage;
use codecrafters_sqlite::page::{self};
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
    // let mut pages = page::btree::BTreePage::build_pages(&mut file)?;
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
        // cmd if cmd.to_lowercase().contains("where") => {
        //     // Example command: "SELECT name, color FROM apples WHERE color = 'Yellow'"
        //     // Everything between SELECT and FROM are the columns, retaining order
        //     // Everything between FROM and WHERE are the tables
        //     // After WHERE is the specific column
        //
        //     let split_command = command::split_command(cmd);
        //     let table_name = command::parse_command_table_name(&split_command)?;
        //     let columns = command::parse_command_columns(&split_command);
        //     let (where_column, where_column_value) = command::parse_command_where(&split_command)?;
        //
        //     // Search root page cells for specific table, returning row id of table
        //     let page_index = root_page.find_table_page(&table_name)?;
        //     let page = &pages[page_index - 1];
        //
        //     // Get column index of each specified column
        //     let column_index = root_page.column_index(&where_column, &table_name);
        //     let column_indexes = root_page.indicies_of_columns(columns.clone(), table_name);
        //
        //     let cells = page.find_cells(&column_index, &where_column_value);
        //     page.read_cell_columns(column_indexes, cells)?;
        // }
        //
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
            let column_indexes = Some(root_page.indicies_of_columns(columns, table_name));

            let mut rows = Vec::new();
            page::btree::traverse_b_tree(&mut file, page_size, page_number, &mut rows)?;
        }
        _ => bail!("Missing or invalid command passed: {}", command),
    }

    Ok(())
}
