use anyhow::{anyhow, bail, Result};
use std::io::Cursor;
use std::io::Read;

#[derive(Debug)]
#[allow(dead_code)]
pub struct PageHeader {
    page_type: PageType,
    first_free_block: u16,
    pub cell_count: u16,
    pub cell_content_area_start: u16,
    fragment_free_bytes: u8,
    page_number: Option<u32>, // Only for interior b-tree page
}

impl PageHeader {
    pub fn new(file: &mut Cursor<Vec<u8>>) -> Result<PageHeader> {
        let mut page_type_buf = [0; 1];
        file.read_exact(&mut page_type_buf)?;

        let page_type = PageType::from_bytes(&page_type_buf)?;

        let header_size = match page_type {
            PageType::TableLeaf | PageType::IndexLeaf => 8,
            PageType::TableInterior | PageType::IndexInterior => 12,
        };

        let mut buf = vec![0; header_size];
        buf[0] = page_type_buf[0];
        file.read_exact(&mut buf[1..])?;

        PageHeader::try_from(buf.as_slice())
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
        let mut page_number = [0; 4];

        bytes.read_exact(&mut page_type)?;
        bytes.read_exact(&mut first_free_block)?;
        bytes.read_exact(&mut cell_count)?;
        bytes.read_exact(&mut cell_content_area)?;
        bytes.read_exact(&mut fragment_free_bytes)?;

        let _ = bytes.read_exact(&mut page_number);
        let page_type = PageType::from_bytes(&page_type);
        let first_free_block = u16::from_be_bytes(first_free_block);
        let cell_count = u16::from_be_bytes(cell_count);
        let cell_content_area_start = u16::from_be_bytes(cell_content_area);
        let fragment_free_bytes = u8::from_be_bytes(fragment_free_bytes);

        let page_number = Some(u32::from_be_bytes(page_number));

        let page_header = PageHeader {
            page_type: page_type?,
            first_free_block,
            cell_count,
            cell_content_area_start,
            fragment_free_bytes,
            page_number,
        };

        Ok(page_header)
    }
}

#[allow(dead_code)]
#[derive(Debug)]
enum PageType {
    TableLeaf,
    TableInterior,
    IndexLeaf,
    IndexInterior,
}

impl PageType {
    fn from_bytes(bytes: &[u8]) -> Result<PageType> {
        // println!("{bytes:?}");
        match bytes[0] {
            2 => Ok(PageType::IndexInterior),
            5 => Ok(PageType::TableInterior),
            10 => Ok(PageType::IndexLeaf),
            13 => Ok(PageType::TableLeaf),
            _ => bail!("Invalid page type byte: {}", bytes[0]),
        }
    }
}
