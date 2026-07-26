use super::parse_varint;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct TableInteriorCell {
    pub left_child: u32,
    pub row_id: u64, // Primary Key
}

impl TryFrom<&[u8]> for TableInteriorCell {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> Result<Self> {
        let left_child: u32 = u32::from_be_bytes(bytes[0..4].try_into()?);
        let (row_id, _data) = parse_varint(&bytes[4..])?;

        Ok(TableInteriorCell { left_child, row_id })
    }
}
