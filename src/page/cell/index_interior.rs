use super::parse_varint;
use anyhow::Result;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct IndexInteriorCell {
    pub left_child: u32,
    payload_size: u64,
    pub payload: Vec<u8>,
    overflow_page_num: Option<u32>,
}

impl TryFrom<&[u8]> for IndexInteriorCell {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> Result<Self> {
        let left_child: u32 = u32::from_be_bytes(bytes[0..4].try_into()?);
        let (payload_size, data) = parse_varint(&bytes[4..])?;
        let payload = &data[0..payload_size as usize];

        Ok(IndexInteriorCell {
            left_child,
            payload_size,
            payload: payload.to_vec(),
            overflow_page_num: None,
        })
    }
}
