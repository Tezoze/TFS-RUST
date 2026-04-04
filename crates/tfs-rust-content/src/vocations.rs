use std::collections::HashMap;
use std::path::Path;
use tfs_rust_common::error::Result;
use tracing::info;

#[derive(Debug, Clone)]
pub struct Vocation {
    pub id: u16,
    pub name: String,
}

pub struct VocationDatabase {
    pub vocations: HashMap<u16, Vocation>,
}

impl VocationDatabase {
    pub fn load(path: &Path) -> Result<Self> {
        info!("Loading vocations from {:?}", path);
        // TODO: Load XML
        Ok(Self {
            vocations: HashMap::new(),
        })
    }
}
