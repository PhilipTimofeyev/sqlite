use anyhow::{anyhow, bail, Result};
use std::convert::TryFrom;
use std::fs::File;
use std::io::prelude::*;
use std::io::Cursor;
use std::io::{self, BufReader};

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
            let mut header = [0; 100];
            file.read_exact(&mut header)?;

            // The page size is stored at the 16th byte offset, using 2 bytes in big-endian order
            let page_size = u16::from_be_bytes([header[16], header[17]]);

            let mut page_type = [0u8; 3];
            let _ = file.read_exact(&mut page_type);

            let mut num_of_cells = [0u8; 2];
            let _ = file.read_exact(&mut num_of_cells);
            let num_of_cells = u16::from_be_bytes(num_of_cells);

            print!("number of tables: {num_of_cells}");
            println!("database page size: {page_size}");
        }
        ".tables" => {
            let mut file = File::open(&args[1])?;
            let mut pages = BTreePage::build_pages(&mut file)?;

            let root_page = pages.remove(0);

            let _ = root_page.table_names();
        }
        cmd if cmd.to_lowercase().contains("select count(*)") => {
            let mut file = File::open(&args[1])?;
            let mut pages = BTreePage::build_pages(&mut file)?;

            let table_name_to_find: Vec<&str> = args.last().unwrap().split_whitespace().collect();
            let table_name_to_find = table_name_to_find.last().unwrap();
            let root_page = pages.remove(0);
            let page = root_page.find_table_page(table_name_to_find.to_string());

            if let Some(table_page) = page {
                let a = pages.remove(table_page as usize - 1);
                println!("{}", a.cell_pointer_array.len())
            } else {
                bail!("Table not found")
            };
        }
        cmd if cmd.to_lowercase().contains("select") => {
            let mut file = File::open(&args[1])?;
            let mut pages = BTreePage::build_pages(&mut file)?;

            let stuff: Vec<&str> = args.last().unwrap().split_whitespace().collect();
            let table_name_to_find = stuff.last().unwrap();
            let selection = stuff[1];
            // println!("TABLE {:?}", table_name_to_find);
            // CHANGE TO REF
            let root_page = pages.remove(0);
            let page = root_page.find_table_page(table_name_to_find.to_string());
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

#[derive(Debug)]
enum PageType {
    TableLeaf,
    TableInterior,
    IndexLeaf,
    IndexInterior,
}

impl PageType {
    fn from_bytes(bytes: &[u8]) -> PageType {
        match bytes[0] {
            13 => PageType::TableLeaf,
            _ => todo!(),
        }
    }
}

#[derive(Debug, Default)]
#[allow(dead_code)]
struct DatabaseHeader {
    header_string: [u8; 16],
    page_size: [u8; 2], // Must be a power of two between 512 and 32768 inclusive, or the value 1 representing a page size of 65536.
    file_format_write_version: [u8; 1], //1 for legacy; 2 for WAL.
    file_format_read_version: [u8; 1],
    unused_space: [u8; 1],         // Usually 0
    max_embedded_payload: [u8; 1], // Must be 64
    min_embedded_payload: [u8; 1], // Must be 32
    leaf_payload: [u8; 1],         // Must be 32
    file_change_counter: [u8; 4],
    in_header_database_size: [u8; 4],
    page_num_first_freelist_trunk_page: [u8; 4],
    total_freelist_pages: [u8; 4],
    schema_cookie: [u8; 4],
    schema_format_num: [u8; 4], // The schema format number. Supported schema formats are 1, 2, 3, and 4.
    default_page_cache_size: [u8; 4],
    largest_root_b_tree_page: [u8; 4], // when in auto-vacuum or incremental-vacuum modes, or zero otherwise.
    text_encoding: [u8; 4], // A value of 1 means UTF-8. A value of 2 means UTF-16le. A value of 3 means UTF-16be.
    user_version: [u8; 4],
    incremental_vacuum_mode: [u8; 4], // True (non-zero) for incremental-vacuum mode. False (zero) otherwise.
    app_id: [u8; 4],
    reserved_expansion: [u8; 20], // Must be zero.
    version_valid_for_number: [u8; 4],
    sqlite_version: [u8; 4],
}

impl TryFrom<&[u8]> for DatabaseHeader {
    type Error = anyhow::Error;

    fn try_from(mut bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 100 {
            return Err(anyhow!("Header is too short"));
        }

        let mut header_string = [0; 16];
        let mut page_size = [0; 2]; // Must be a power of two between 512 and 32768 inclusive, or the value 1 representing a page size of 65536.
        let mut file_format_write_version = [0; 1]; //1 for legacy; 2 for WAL.
        let mut file_format_read_version = [0; 1];
        let mut unused_space = [0; 1]; // Usually 0
        let mut max_embedded_payload = [0; 1]; // Must be 64
        let mut min_embedded_payload = [0; 1]; // Must be 32
        let mut leaf_payload = [0; 1]; // Must be 32
        let mut file_change_counter = [0; 4];
        let mut in_header_database_size = [0; 4];
        let mut page_num_first_freelist_trunk_page = [0; 4];
        let mut total_freelist_pages = [0; 4];
        let mut schema_cookie = [0; 4];
        let mut schema_format_num = [0; 4]; // The schema format number. Supported schema formats are 1, 2, 3, and 4.
        let mut default_page_cache_size = [0; 4];
        let mut largest_root_b_tree_page = [0; 4]; // when in auto-vacuum or incremental-vacuum modes, or zero otherwise.
        let mut text_encoding = [0; 4]; // A value of 1 means UTF-8. A value of 2 means UTF-16le. A value of 3 means UTF-16be.
        let mut user_version = [0; 4];
        let mut incremental_vacuum_mode = [0; 4]; // True (non-zero) for incremental-vacuum mode. False (zero) otherwise.
        let mut app_id = [0; 4];
        let mut reserved_expansion = [0; 20]; // Must be zero.
        let mut version_valid_for_number = [0; 4];
        let mut sqlite_version = [0; 4];

        let _ = bytes.read_exact(&mut header_string);
        let _ = bytes.read_exact(&mut page_size);
        let _ = bytes.read_exact(&mut file_format_write_version);
        let _ = bytes.read_exact(&mut file_format_read_version);
        let _ = bytes.read_exact(&mut unused_space);
        let _ = bytes.read_exact(&mut max_embedded_payload);
        let _ = bytes.read_exact(&mut min_embedded_payload);
        let _ = bytes.read_exact(&mut leaf_payload);
        let _ = bytes.read_exact(&mut file_change_counter);
        let _ = bytes.read_exact(&mut in_header_database_size);
        let _ = bytes.read_exact(&mut page_num_first_freelist_trunk_page);
        let _ = bytes.read_exact(&mut total_freelist_pages);
        let _ = bytes.read_exact(&mut schema_cookie);
        let _ = bytes.read_exact(&mut schema_format_num);
        let _ = bytes.read_exact(&mut default_page_cache_size);
        let _ = bytes.read_exact(&mut largest_root_b_tree_page);
        let _ = bytes.read_exact(&mut text_encoding);
        let _ = bytes.read_exact(&mut user_version);
        let _ = bytes.read_exact(&mut incremental_vacuum_mode);
        let _ = bytes.read_exact(&mut app_id);
        let _ = bytes.read_exact(&mut reserved_expansion);
        let _ = bytes.read_exact(&mut version_valid_for_number);
        let _ = bytes.read_exact(&mut sqlite_version);

        Ok(DatabaseHeader {
            header_string,
            page_size,
            file_format_write_version,
            file_format_read_version,
            unused_space,
            max_embedded_payload,
            min_embedded_payload,
            leaf_payload,
            file_change_counter,
            in_header_database_size,
            page_num_first_freelist_trunk_page,
            total_freelist_pages,
            schema_cookie,
            schema_format_num,
            default_page_cache_size,
            largest_root_b_tree_page,
            text_encoding,
            user_version,
            incremental_vacuum_mode,
            app_id,
            reserved_expansion,
            version_valid_for_number,
            sqlite_version,
        })
    }
}

#[derive(Debug)]
struct BTreePage {
    file_header: Option<DatabaseHeader>,
    page_header: PageHeader,
    cell_pointer_array: Vec<u16>,
    unallocated_space: Vec<u8>,
    cell_content_area: Vec<u8>,
    reserved_region: Vec<u8>,
}

impl BTreePage {
    fn new(bytes: Vec<u8>, file_header: Option<DatabaseHeader>) -> BTreePage {
        let mut cursor = Cursor::new(bytes);
        let page_header = PageHeader::new(&mut cursor).unwrap();
        let cell_pointer_array = Cell::pointers(&mut cursor, &page_header);
        let unallocated_space_size = if file_header.is_some() {
            page_header.cell_content_area_start as u64 - 100 - cursor.position()
        } else {
            page_header.cell_content_area_start as u64 - cursor.position()
        };
        let mut unallocated_space = vec![0; unallocated_space_size as usize];
        let _ = cursor.read_exact(&mut unallocated_space);
        let mut cell_content_area = Vec::new();
        let _ = cursor.read_to_end(&mut cell_content_area);
        let reserved_region = Vec::default();

        BTreePage {
            file_header: None,
            page_header,
            cell_pointer_array,
            unallocated_space,
            cell_content_area,
            reserved_region,
        }
    }

    fn build_pages(file: &mut File) -> Result<Vec<BTreePage>> {
        let mut header = [0; 100];

        file.read_exact(&mut header)?;
        let database_header = DatabaseHeader::try_from(&header[..])?;

        let page_size = u16::from_be_bytes(database_header.page_size) as usize;
        let mut root_page = vec![0; page_size - 100];
        file.read_exact(&mut root_page)?;

        let root_page = BTreePage::new(root_page, Some(database_header));
        let mut pages = vec![root_page];

        loop {
            let mut buf = vec![0; page_size];
            if file.read_exact(&mut buf).is_err() {
                return Ok(pages);
            };
            let b_tree = BTreePage::new(buf, None);
            pages.push(b_tree);
        }
    }

    fn cells(&self) -> Result<Vec<TableLeafCell>> {
        let mut file = Cursor::new(self.cell_content_area.clone());
        let mut cell_pointers_peek = self.cell_pointer_array.iter().rev().peekable();
        let mut cells = Vec::new();

        // println!("{}", self.cell_pointer_array.len());

        while let Some(pointer) = cell_pointers_peek.next() {
            if let Some(next_pointer) = cell_pointers_peek.peek() {
                let num_bytes_to_read = *next_pointer - pointer;
                let mut buf = vec![0; num_bytes_to_read as usize];
                let _ = file.read_exact(&mut buf);
                let table_leaf_cell = TableLeafCell::try_from(&buf[..])?;
                cells.push(table_leaf_cell)
            } else {
                let mut buf = Vec::new();
                let _ = file.read_to_end(&mut buf);
                let table_leaf_cell = TableLeafCell::try_from(&buf[..])?;
                cells.push(table_leaf_cell);
            }
        }

        Ok(cells)
    }

    fn table_names(&self) -> Result<()> {
        for cell in self.cells()?.iter().rev() {
            let table_name_bytes = cell.schema().table_name;
            println!("{}", String::from_utf8(table_name_bytes)?);
        }

        Ok(())
    }

    fn find_table_page(&self, table: String) -> Option<u8> {
        let cells = &self.cells().unwrap();
        // println!("CELLS: {:?}", cells);
        let table_cell = cells
            .iter()
            .find(|cell| String::from_utf8(cell.schema().table_name) == Ok(table.clone()));
        table_cell.map(|cell| cell.row_id)
    }
}

#[derive(Debug)]
struct PageHeader {
    page_type: PageType,
    first_free_block: u16,
    cell_count: u16,
    cell_content_area_start: u16,
    fragment_free_bytes: u8,
    page_number: Option<[u8; 4]>,
}

impl PageHeader {
    fn new(file: &mut Cursor<Vec<u8>>) -> Result<PageHeader> {
        // Change to dynamic vector for headers
        let mut leaf_header = [0; 8];
        let mut interior_header = [0; 12];
        let mut page_type_buf = [0u8; 1];
        let _ = file.read_exact(&mut page_type_buf);

        let page_type = PageType::from_bytes(&page_type_buf);
        match page_type {
            PageType::TableLeaf | PageType::IndexLeaf => {
                leaf_header[0..1].copy_from_slice(&page_type_buf);
                file.read_exact(&mut leaf_header[1..])?;
                PageHeader::try_from(&leaf_header[..])
            }
            PageType::TableInterior | PageType::IndexInterior => {
                interior_header[0..1].copy_from_slice(&page_type_buf);
                let _ = file.read_exact(&mut interior_header[1..]);
                todo!()
            }
        }
    }
}

impl TryFrom<&[u8]> for PageHeader {
    type Error = anyhow::Error;

    fn try_from(mut bytes: &[u8]) -> Result<Self> {
        if bytes.len() < 8 {
            return Err(anyhow!("Header is too short"));
        }

        let mut page_type = [0; 1];
        let mut first_free_block = [0; 2];
        let mut cell_count = [0; 2];
        let mut cell_content_area = [0; 2];
        let mut fragment_free_bytes = [0; 1];

        let _ = bytes.read_exact(&mut page_type);
        let _ = bytes.read_exact(&mut first_free_block);
        let _ = bytes.read_exact(&mut cell_count);
        let _ = bytes.read_exact(&mut cell_content_area);
        let _ = bytes.read_exact(&mut fragment_free_bytes);

        let page_type = PageType::from_bytes(&page_type);
        let first_free_block = u16::from_be_bytes(first_free_block);
        let cell_count = u16::from_be_bytes(cell_count);
        let cell_content_area_start = u16::from_be_bytes(cell_content_area);
        let fragment_free_bytes = u8::from_be_bytes(fragment_free_bytes);

        let page_header = PageHeader {
            page_type,
            first_free_block,
            cell_count,
            cell_content_area_start,
            fragment_free_bytes,
            page_number: None,
        };

        Ok(page_header)
    }
}

struct Cell {}

impl Cell {
    fn pointers(file: &mut Cursor<Vec<u8>>, page_header: &PageHeader) -> Vec<u16> {
        let mut cell_buf = [0; 2];
        let mut cell_pointers = Vec::new();
        for _ in 0..page_header.cell_count {
            let _ = file.read_exact(&mut cell_buf);
            let pointer = u16::from_be_bytes(cell_buf);
            cell_pointers.push(pointer);
        }

        cell_pointers
    }
}
#[derive(Debug)]
struct TableLeafCell {
    payload_size: u8,
    row_id: u8, // Primary Key
    header: Vec<u8>,
    payload: Vec<u8>,
}

#[derive(Debug)]
enum SerialType {
    Null,
    Integer(i64),
    Blob(usize),
    Text(usize),
}

impl SerialType {
    fn from_code(code: u8) -> Self {
        match code {
            0 => SerialType::Null,
            1..7 => SerialType::Integer(1),
            n if n >= 12 && n % 2 == 0 => SerialType::Blob(((n - 12) / 2) as usize),
            n if n >= 13 && n % 2 == 1 => SerialType::Text(((n - 13) / 2) as usize),
            _ => todo!(),
        }
    }
}

fn parse_varint(data: &[u8]) -> (u64, &[u8]) {
    for i in 0..9 {
        let Some(b) = data.get(i) else {
            panic!("Not enough bytes for varint");
        };
        if b & 0x80 == 0 {
            // This is the last byte of the VARINT, so convert it to
            // a u64 and return it.
            let mut value = 0u64;
            for b in data[..=i].iter().rev() {
                value = (value << 7) | (b & 0x7f) as u64;
            }
            return (value, &data[i + 1..]);
        }
    }

    // More than 7 bytes is invalid.
    panic!("Too many bytes for varint");
}

impl TryFrom<&[u8]> for TableLeafCell {
    type Error = anyhow::Error;

    fn try_from(mut bytes: &[u8]) -> Result<Self> {
        // println!("BYTES {:?}", bytes);

        let (varint,mut data) = parse_varint(bytes);

        // println!("varint{}", varint);

        let mut payload_size = [0; 1];
        let mut row_id = [0; 1];
        let mut header_size = [0; 1];

        // let _ = data.read_exact(&mut payload_size);
        let _ = data.read_exact(&mut row_id);
        let _ = data.read_exact(&mut header_size);

        let header_size = header_size[0] as usize;

        let mut header = vec![header_size as u8; header_size];
        let _ = data.read_exact(&mut header[1..]);
        let mut payload = Vec::new();
        let _ = data.read_to_end(&mut payload);

        // let payload = Schema::new(header.clone(), payload);

        Ok(TableLeafCell {
            payload_size: payload_size[0],
            row_id: row_id[0],
            header,
            payload,
        })
    }
}

#[derive(Debug)]
struct Schema {
    schema_type: Vec<u8>,
    name: Vec<u8>,
    table_name: Vec<u8>,
    root_page: Vec<u8>,
    sql: Vec<u8>,
}

// May add
enum SchemaType {
    TableType,
    Name,
}

impl TableLeafCell {
    fn decode(&self, column: usize) {
        let mut serial_types = Vec::new();

        for code in &self.header[1..] {
            let a = SerialType::from_code(*code);
            serial_types.push(a);
        }

        // println!("{:?}", serial_types);

        let mut cursor = Cursor::new(self.payload.clone());
        let mut schema_vec = Vec::new();

        for serial_type in &serial_types {
            match serial_type {
                SerialType::Integer(bytes) => {
                    let mut buf = vec![0; *bytes as usize];
                    let _ = cursor.read_exact(&mut buf);
                    schema_vec.push(buf);
                }
                SerialType::Text(bytes) => {
                    let mut buf = vec![0; *bytes];
                    let _ = cursor.read_exact(&mut buf);
                    schema_vec.push(buf);
                }
                SerialType::Null => {
                    schema_vec.push(vec![0]);
                }
                _ => todo!(),
            }
        }

        // println!("{:?}", serial_types);

        // for row in &schema_vec {
            let a = String::from_utf8(schema_vec.clone().to_vec()[column].clone()).unwrap();
            println!("{a}");
        // }
    }

    fn schema(&self) -> Schema {
        let mut serial_types = Vec::new();

        for code in &self.header[1..] {
            let a = SerialType::from_code(*code);
            serial_types.push(a);
        }

        // println!("{:?}", serial_types);

        let mut cursor = Cursor::new(self.payload.clone());
        let mut schema_vec = Vec::new();

        for serial_type in &serial_types {
            match serial_type {
                SerialType::Integer(bytes) => {
                    let mut buf = vec![0; *bytes as usize];
                    let _ = cursor.read_exact(&mut buf);
                    schema_vec.push(buf);
                }
                SerialType::Text(bytes) => {
                    let mut buf = vec![0; *bytes];
                    let _ = cursor.read_exact(&mut buf);
                    schema_vec.push(buf);
                }
                SerialType::Null => {
                    schema_vec.push(vec![0]);
                }
                _ => todo!(),
            }
        }

        for row in &schema_vec {
            let a = String::from_utf8(row.to_vec()).unwrap();
            // println!("WeLL {a}");
        }

        // println!("VECTOR {:?}", schema_vec);
        let schema_type = schema_vec.remove(0);
        let name = schema_vec.remove(0);
        let table_name = schema_vec.remove(0);
        let root_page = schema_vec.remove(0);
        let sql: Vec<u8> = schema_vec
            .into_iter()
            .flatten()
            .collect::<Vec<u8>>()
            .drain(..)
            .collect();

        // println!("Hmm {}", String::from_utf8(sql.clone()).unwrap());

        Schema {
            schema_type,
            name,
            table_name,
            root_page,
            sql,
        }
    }
}
