use anyhow::{anyhow, Result};
use std::io::Read;

#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct DatabaseHeader {
    header_string: [u8; 16],
    pub page_size: [u8; 2], // Must be a power of two between 512 and 32768 inclusive, or the value 1 representing a page size of 65536.
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
