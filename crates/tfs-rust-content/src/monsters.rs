use std::collections::HashMap;
use std::path::Path;
use tfs_rust_common::error::Result;
use tracing::info;

#[derive(Debug, Clone)]
pub struct MonsterType {
    pub name: String,
    // Add additional properties as needed: health, loot, spells, etc.
}

pub struct MonsterDatabase {
    pub monsters: HashMap<String, MonsterType>,
}

impl MonsterDatabase {
    pub fn load_dir(dir: &Path) -> Result<Self> {
        info!("Loading monsters from {:?}", dir);
        let mut db = HashMap::new();
        // TODO: parse monsters.xml index, then traverse individual xml files

        // Dummy entry
        db.insert(
            "rat".to_string(),
            MonsterType {
                name: "Rat".to_string(),
            },
        );

        Ok(Self { monsters: db })
    }
}
