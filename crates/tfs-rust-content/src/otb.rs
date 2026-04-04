use std::collections::HashMap;
use std::path::Path;
use tfs_rust_common::error::{Result, TfsRustError};

#[derive(Debug, Clone, Default)]
pub struct ItemType {
    pub id: u16,
    pub server_id: u16,
    pub client_id: u16,
    pub name: String,
    pub flags: u32,
    pub weight: u32,
    pub rotate_to: u16,
    pub description: String,
    /// All `<attribute key="..." value="..."/>` pairs from `items.xml` (merged; keys lowercased).
    // C++ reference: src/items.cpp Items::parseItemNode
    pub xml_attributes: HashMap<String, String>,
}

pub struct OtbLoader;

impl OtbLoader {
    pub fn load_from_file(path: &Path) -> Result<HashMap<u16, ItemType>> {
        let data = std::fs::read(path).map_err(|e| TfsRustError::Content {
            file: path.to_string_lossy().into(),
            message: e.to_string(),
        })?;

        let mut db = HashMap::new();
        let mut index = 0usize;
        while index < data.len() {
            if data[index] == NODE_START {
                parse_node(&data, &mut index, &mut db, path)?;
            } else {
                index += 1;
            }
        }
        Ok(db)
    }
}

const ESCAPE: u8 = 0xFD;
const NODE_START: u8 = 0xFE;
const NODE_END: u8 = 0xFF;

const ITEM_ATTR_SERVERID: u8 = 0x10;
const ITEM_ATTR_CLIENTID: u8 = 0x11;
const ITEM_ATTR_NAME: u8 = 0x12;
const ITEM_ATTR_DESCR: u8 = 0x13;
const ITEM_ATTR_WEIGHT: u8 = 0x16;
const ITEM_ATTR_ROTATETO: u8 = 0x1A;

fn parse_node(
    data: &[u8],
    index: &mut usize,
    db: &mut HashMap<u16, ItemType>,
    path: &Path,
) -> Result<()> {
    expect_raw(data, index, NODE_START, path)?;

    let _node_type = read_data_u8(data, index, path)?;
    let flags = read_data_u32(data, index, path)?;
    let mut item = ItemType {
        flags,
        ..ItemType::default()
    };

    while *index < data.len() {
        match data[*index] {
            NODE_START => {
                parse_node(data, index, db, path)?;
            }
            NODE_END => {
                *index += 1;
                break;
            }
            _ => {
                let attr_type = read_data_u8(data, index, path)?;
                let attr_size = read_data_u16(data, index, path)? as usize;
                let attr_data = read_data_bytes(data, index, attr_size, path)?;
                apply_attr(&mut item, attr_type, &attr_data);
            }
        }
    }

    if item.server_id != 0 {
        item.id = item.server_id;
        db.insert(item.server_id, item);
    }
    Ok(())
}

fn apply_attr(item: &mut ItemType, attr_type: u8, attr_data: &[u8]) {
    match attr_type {
        ITEM_ATTR_SERVERID if attr_data.len() >= 2 => {
            item.server_id = u16::from_le_bytes([attr_data[0], attr_data[1]]);
        }
        ITEM_ATTR_CLIENTID if attr_data.len() >= 2 => {
            item.client_id = u16::from_le_bytes([attr_data[0], attr_data[1]]);
        }
        ITEM_ATTR_NAME => {
            item.name = String::from_utf8_lossy(attr_data).to_string();
        }
        ITEM_ATTR_DESCR => {
            item.description = String::from_utf8_lossy(attr_data).to_string();
        }
        ITEM_ATTR_WEIGHT => {
            item.weight = match attr_data.len() {
                0 => 0,
                1 => attr_data[0] as u32,
                2 => u16::from_le_bytes([attr_data[0], attr_data[1]]) as u32,
                _ => u32::from_le_bytes([attr_data[0], attr_data[1], attr_data[2], attr_data[3]]),
            };
        }
        ITEM_ATTR_ROTATETO if attr_data.len() >= 2 => {
            item.rotate_to = u16::from_le_bytes([attr_data[0], attr_data[1]]);
        }
        _ => {}
    }
}

fn expect_raw(data: &[u8], index: &mut usize, expected: u8, path: &Path) -> Result<()> {
    if *index >= data.len() {
        return Err(TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: "unexpected end of OTB stream".to_string(),
        });
    }
    if data[*index] != expected {
        return Err(TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: format!(
                "invalid OTB token at offset {}: expected {expected:#04x}, got {:#04x}",
                *index, data[*index]
            ),
        });
    }
    *index += 1;
    Ok(())
}

fn read_data_u8(data: &[u8], index: &mut usize, path: &Path) -> Result<u8> {
    if *index >= data.len() {
        return Err(TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: "unexpected end of OTB stream".to_string(),
        });
    }
    let value = if data[*index] == ESCAPE {
        *index += 1;
        if *index >= data.len() {
            return Err(TfsRustError::Content {
                file: path.to_string_lossy().into_owned(),
                message: "dangling OTB escape byte".to_string(),
            });
        }
        data[*index]
    } else {
        data[*index]
    };
    *index += 1;
    Ok(value)
}

fn read_data_u16(data: &[u8], index: &mut usize, path: &Path) -> Result<u16> {
    let lo = read_data_u8(data, index, path)?;
    let hi = read_data_u8(data, index, path)?;
    Ok(u16::from_le_bytes([lo, hi]))
}

fn read_data_u32(data: &[u8], index: &mut usize, path: &Path) -> Result<u32> {
    let b0 = read_data_u8(data, index, path)?;
    let b1 = read_data_u8(data, index, path)?;
    let b2 = read_data_u8(data, index, path)?;
    let b3 = read_data_u8(data, index, path)?;
    Ok(u32::from_le_bytes([b0, b1, b2, b3]))
}

fn read_data_bytes(data: &[u8], index: &mut usize, count: usize, path: &Path) -> Result<Vec<u8>> {
    let mut bytes = Vec::with_capacity(count);
    for _ in 0..count {
        bytes.push(read_data_u8(data, index, path)?);
    }
    Ok(bytes)
}
