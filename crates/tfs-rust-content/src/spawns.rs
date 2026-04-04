//! Spawn file loader (`*-spawn.xml`). C++ loads these via `IOMap::loadSpawns` → `Spawns::loadFromXml`.
// C++ reference: src/spawn.cpp Spawns::loadFromXml

use roxmltree::Document;
use std::path::Path;
use tfs_rust_common::error::{Result, TfsRustError};
use tfs_rust_common::Position;
use tracing::info;

/// One `<spawn>` block from the spawn XML file.
#[derive(Debug, Clone)]
pub struct SpawnZone {
    pub center: Position,
    pub radius: i32,
    pub entries: Vec<SpawnEntry>,
}

#[derive(Debug, Clone)]
pub enum SpawnEntry {
    Monster {
        name: String,
        position: Position,
        spawntime_ms: i32,
        direction: Option<u16>,
    },
    Monsters {
        position: Position,
        spawntime_ms: i32,
        monsters: Vec<MonsterWeight>,
    },
    Npc {
        name: String,
        position: Position,
        spawntime_ms: i32,
        direction: Option<u16>,
    },
}

#[derive(Debug, Clone)]
pub struct MonsterWeight {
    pub name: String,
    pub chance: u16,
}

pub fn load_spawn_xml(path: &Path) -> Result<Vec<SpawnZone>> {
    info!("Loading spawn XML from {:?}", path);
    let xml = std::fs::read_to_string(path).map_err(|e| TfsRustError::Content {
        file: path.to_string_lossy().into_owned(),
        message: e.to_string(),
    })?;

    let doc = Document::parse(&xml).map_err(|e| TfsRustError::Content {
        file: path.to_string_lossy().into_owned(),
        message: e.to_string(),
    })?;

    let root = doc.root_element();
    let spawns_el = if root.has_tag_name("spawns") {
        root
    } else {
        doc.descendants()
            .find(|n| n.has_tag_name("spawns"))
            .ok_or_else(|| TfsRustError::Content {
                file: path.to_string_lossy().into_owned(),
                message: "missing <spawns> root".to_string(),
            })?
    };

    let mut zones = Vec::new();
    for spawn in spawns_el
        .children()
        .filter(|n| n.is_element() && n.has_tag_name("spawn"))
    {
        let cx = spawn
            .attribute("centerx")
            .and_then(|a| a.parse().ok())
            .unwrap_or(0);
        let cy = spawn
            .attribute("centery")
            .and_then(|a| a.parse().ok())
            .unwrap_or(0);
        let cz = spawn
            .attribute("centerz")
            .and_then(|a| a.parse().ok())
            .unwrap_or(0);
        let center = Position::new(cx, cy, cz);
        let radius = spawn
            .attribute("radius")
            .and_then(|a| a.parse().ok())
            .unwrap_or(-1);

        let mut entries = Vec::new();
        for child in spawn.children().filter(|n| n.is_element()) {
            let tag = child.tag_name().name();
            if tag.eq_ignore_ascii_case("monster") {
                let name = child.attribute("name").unwrap_or("").to_string();
                let ox = child
                    .attribute("x")
                    .and_then(|a| a.parse().ok())
                    .unwrap_or(0i32);
                let oy = child
                    .attribute("y")
                    .and_then(|a| a.parse().ok())
                    .unwrap_or(0i32);
                let st = child
                    .attribute("spawntime")
                    .and_then(|a| a.parse().ok())
                    .unwrap_or(60i32);
                let dir = child.attribute("direction").and_then(|a| a.parse().ok());
                let pos = Position::new(
                    (center.x as i32 + ox).max(0) as u16,
                    (center.y as i32 + oy).max(0) as u16,
                    center.z,
                );
                entries.push(SpawnEntry::Monster {
                    name,
                    position: pos,
                    spawntime_ms: st * 1000,
                    direction: dir,
                });
            } else if tag.eq_ignore_ascii_case("monsters") {
                let ox = child
                    .attribute("x")
                    .and_then(|a| a.parse().ok())
                    .unwrap_or(0i32);
                let oy = child
                    .attribute("y")
                    .and_then(|a| a.parse().ok())
                    .unwrap_or(0i32);
                let st = child
                    .attribute("spawntime")
                    .and_then(|a| a.parse().ok())
                    .unwrap_or(60i32);
                let pos = Position::new(
                    (center.x as i32 + ox).max(0) as u16,
                    (center.y as i32 + oy).max(0) as u16,
                    center.z,
                );
                let mut monsters = Vec::new();
                for m in child
                    .children()
                    .filter(|n| n.is_element() && n.has_tag_name("monster"))
                {
                    let n = m.attribute("name").unwrap_or("").to_string();
                    let chance = m
                        .attribute("chance")
                        .and_then(|a| a.parse().ok())
                        .unwrap_or(0u16);
                    monsters.push(MonsterWeight { name: n, chance });
                }
                entries.push(SpawnEntry::Monsters {
                    position: pos,
                    spawntime_ms: st * 1000,
                    monsters,
                });
            } else if tag.eq_ignore_ascii_case("npc") {
                let name = child.attribute("name").unwrap_or("").to_string();
                let ox = child
                    .attribute("x")
                    .and_then(|a| a.parse().ok())
                    .unwrap_or(0i32);
                let oy = child
                    .attribute("y")
                    .and_then(|a| a.parse().ok())
                    .unwrap_or(0i32);
                let st = child
                    .attribute("spawntime")
                    .and_then(|a| a.parse().ok())
                    .unwrap_or(60i32);
                let dir = child.attribute("direction").and_then(|a| a.parse().ok());
                let pos = Position::new(
                    (center.x as i32 + ox).max(0) as u16,
                    (center.y as i32 + oy).max(0) as u16,
                    center.z,
                );
                entries.push(SpawnEntry::Npc {
                    name,
                    position: pos,
                    spawntime_ms: st * 1000,
                    direction: dir,
                });
            }
        }

        zones.push(SpawnZone {
            center,
            radius,
            entries,
        });
    }

    Ok(zones)
}
