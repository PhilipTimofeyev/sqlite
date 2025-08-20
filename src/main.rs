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
                let page = pages.remove(page_index as usize - 1);
                println!("{}", page.cell_pointer_array.len())
            } else {
                bail!("Table not found")
            };
        }

        cmd if cmd.to_lowercase().contains("select") => {
            let mut file = File::open(&args[1])?;
            let mut pages = page::btree::BTreePage::build_pages(&mut file)?;

            let stuff: Vec<&str> = args.last().unwrap().split_whitespace().collect();
            let table_name_to_find = stuff.last().unwrap();
            let selection = stuff[1];
            let root_page = pages.remove(0);
            let page = root_page.find_table_page(table_name_to_find)?;
            let hmm = root_page.cells()?;
            let schema = hmm[0].schema();
            let columns = String::from_utf8(schema.sql)?;
            let columns = columns.split(&['(', ')'][..]).collect::<Vec<&str>>();
            let columns = columns.last().unwrap().split(',').collect::<Vec<&str>>();
            let column = columns
                .iter()
                .position(|&column| column.contains(selection));
            // println!("LENGTH {:?}", columns);
            // println!("LENGTH {:?}", column);

            // println!("CELL {:?}", pages[page.unwrap() as usize].cells()?[0]);
            // pages[page.unwrap() as usize].cells()?[0].decode()

            // for cell in pages[page.unwrap() as usize - 1].cells()? {
            //     cell.decode()
            //     // cell.schema();
            // }
            // println!("{:?}", pages[0].cells());

            for cell in pages[page.unwrap() as usize - 1].cells()? {
                cell.decode(column.unwrap())
            }

            // if let Some(table_page) = page {
            //     let a = pages.remove(table_page as usize - 1);
            //     for cell in a.cells()? {
            //         println!("{:?}", cell.schema().name)
            //     }
            // } else {
            //     bail!("Table not found")
            // };
        }
        _ => bail!("Missing or invalid command passed: {}", command),
    }

    Ok(())
}
