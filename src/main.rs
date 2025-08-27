use anyhow::{bail, Result};
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
            let table_name_to_find = args.last().unwrap().split_whitespace().last().unwrap();
            let page_index = root_page.find_table_page(table_name_to_find)?;

            if let Some(page_index) = page_index {
                let page = pages.remove(page_index - 1);
                println!("{}", page.cell_pointer_array.len())
            } else {
                bail!("Table not found")
            };
        }

        cmd if cmd.to_lowercase().contains("select") => {
            // Example command: "SELECT color FROM oranges"
            // Second word is column and last word is table

            let mut file = File::open(&args[1])?;
            let mut pages = page::btree::BTreePage::build_pages(&mut file)?;

            let root_page = pages.remove(0);

            let command: Vec<&str> = args.last().unwrap().split_whitespace().collect();
            let table_name_to_find = command.last().unwrap();
            let column_name = command[1];

            // Find page containing table
            let page_index = root_page.find_table_page(table_name_to_find)?;

            // Find schema containing column
            let schema = root_page.find_column(column_name)?;

            // Find column position
            let column_index = schema.column_position(column_name)?;
            
            let page = &pages[page_index.unwrap() - 1];
            page.read_cell_columns(column_index)?;
        }
        _ => bail!("Missing or invalid command passed: {}", command),
    }

    Ok(())
}
