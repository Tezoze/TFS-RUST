use std::collections::HashMap;
use std::path::Path;
use tfs_rust_common::error::Result;
use tracing::info;

#[derive(Debug, Clone)]
pub struct Outfit {
    pub name: String,
}

pub struct OutfitDatabase {
    pub outfits: HashMap<String, Outfit>,
}

impl OutfitDatabase {
    pub fn load(path: &Path) -> Result<Self> {
        info!("Loading outfits from {:?}", path);
        // TODO: Load XML
        Ok(Self {
            outfits: HashMap::new(),
        })
    }
}
