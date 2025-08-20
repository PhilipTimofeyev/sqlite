use anyhow::{anyhow, Result};
use std::io::Cursor;
use std::io::Read;

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

#[derive(Debug)]
pub struct PageHeader {
    page_type: PageType,
    first_free_block: u16,
    pub cell_count: u16,
    pub cell_content_area_start: u16,
    fragment_free_bytes: u8,
    page_number: Option<[u8; 4]>, // Only for interior b-tree page
}

impl PageHeader {
    pub fn new(file: &mut Cursor<Vec<u8>>) -> Result<PageHeader> {
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
