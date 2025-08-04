use anyhow::{bail, Result};
use std::fs::File;
use std::io::prelude::*;
use std::io::Cursor;
use std::num;

fn main() -> Result<()> {
    // Parse arguments
    let args = std::env::args().collect::<Vec<_>>();
    match args.len() {
        0 | 1 => bail!("Missing <database path> and <command>"),
        2 => bail!("Missing <command>"),
        _ => {}
    }

    // Parse command and act accordingly
    let command = &args[2];
    match command.as_str() {
        ".dbinfo" => {
            let mut file = File::open(&args[1])?;
            let mut header = [0; 100];
            file.read_exact(&mut header)?;

            // The page size is stored at the 16th byte offset, using 2 bytes in big-endian order
            #[allow(unused_variables)]
            let page_size = u16::from_be_bytes([header[16], header[17]]);

            let mut page_type = [0u8; 3];
            let _ = file.read_exact(&mut page_type);
            // println!("page type bytes {page_type:?}");

            let mut num_of_cells = [0u8; 2];
            let _ = file.read_exact(&mut num_of_cells);
            let num_of_cells = u16::from_be_bytes(num_of_cells);

            print!("number of tables: {num_of_cells}");
            println!("database page size: {page_size}");
        }
        _ => bail!("Missing or invalid command passed: {}", command),
    }

    Ok(())
}


#[derive(Debug)]
enum PageType {
    TableLeaf,
}

impl PageType {
    fn from_byte(byte: &[u8]) -> PageType {
        match byte[0] {
            13 => PageType::TableLeaf,
            _ => todo!()
        }
    }
}
