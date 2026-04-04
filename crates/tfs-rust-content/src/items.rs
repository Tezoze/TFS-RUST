use crate::otb::{ItemType, OtbLoader};
use std::collections::HashMap;
use std::path::Path;
use tfs_rust_common::error::{Result, TfsRustError};
use tracing::info;

pub struct ItemDatabase {
    pub items: HashMap<u16, ItemType>,
}

impl ItemDatabase {
    pub fn load(otb_path: &Path, xml_path: &Path) -> Result<Self> {
        info!("Loading OTB from {:?}", otb_path);
        let mut items = OtbLoader::load_from_file(otb_path)?;

        info!("Merging items.xml from {:?}", xml_path);
        Self::merge_xml(&mut items, xml_path)?;

        Ok(Self { items })
    }

    fn merge_xml(_items: &mut HashMap<u16, ItemType>, xml_path: &Path) -> Result<()> {
        let _xml_str = std::fs::read_to_string(xml_path).map_err(|e| TfsRustError::Content {
            file: xml_path.to_string_lossy().into(),
            message: e.to_string(),
        })?;

        // TODO: Full quick-xml parser for `<item id="...">` fields
        // E.g., apply descriptions, weight overrides, formulas

        Ok(())
    }
}
