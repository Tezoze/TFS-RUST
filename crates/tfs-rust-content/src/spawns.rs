//! Spawn file loader (`*-spawn.xml`). C++ loads these via `IOMap::loadSpawns` → `Spawns::loadFromXml`.
// C++ reference: `src/spawn.cpp` `Spawns::loadFromXml` (1098 nested `<spawn>`)
// C++ reference: `gameserver/src/spawn.cpp` `Spawns::loadFromXml` (`amount` / TVP flat spawns)

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
    for spawn in spawns_el.children().filter(|n| n.is_element() && is_spawn_element(n)) {
        if spawn.attribute("amount").is_some() {
            if let Some(zone) = parse_tvp_spawn_element(spawn) {
                zones.push(zone);
            }
            continue;
        }
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

fn is_spawn_element(node: &roxmltree::Node<'_, '_>) -> bool {
    let tag = node.tag_name().name();
    tag.eq_ignore_ascii_case("spawn") || tag.eq_ignore_ascii_case("tvpspawn")
}

/// TVP / 7.72 flat spawn row (`<tvpspawn … amount="N" monstername="…"/>`).
/// C++ reference: `gameserver/src/spawn.cpp` `Spawns::loadFromXml` (`amountAttribute` branch).
fn parse_tvp_spawn_element(spawn: roxmltree::Node<'_, '_>) -> Option<SpawnZone> {
    let cx = spawn.attribute("centerx")?.parse().ok()?;
    let cy = spawn.attribute("centery")?.parse().ok()?;
    let cz = spawn.attribute("centerz")?.parse().ok()?;
    let center = Position::new(cx, cy, cz);
    let radius = spawn
        .attribute("radius")
        .and_then(|a| a.parse().ok())
        .unwrap_or(-1);
    let direction = spawn.attribute("direction").and_then(|a| a.parse().ok());
    let amount = spawn
        .attribute("amount")
        .and_then(|a| a.parse().ok())
        .unwrap_or(1)
        .max(1);
    let spawntime_secs = spawn
        .attribute("spawntime")
        .and_then(|a| a.parse().ok())
        .unwrap_or(60i32);
    let spawntime_ms = spawntime_secs * 1000;

    let mut entries = Vec::new();
    if let Some(name) = spawn.attribute("monstername") {
        for _ in 0..amount {
            entries.push(SpawnEntry::Monster {
                name: name.to_string(),
                position: center,
                spawntime_ms,
                direction,
            });
        }
    } else if let Some(name) = spawn.attribute("npcname") {
        entries.push(SpawnEntry::Npc {
            name: name.to_string(),
            position: center,
            spawntime_ms,
            direction,
        });
    }

    if entries.is_empty() {
        return None;
    }

    Some(SpawnZone {
        center,
        radius,
        entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;

    fn entry_spawntime_ms(entry: &SpawnEntry) -> i32 {
        match entry {
            SpawnEntry::Monster { spawntime_ms, .. }
            | SpawnEntry::Monsters { spawntime_ms, .. }
            | SpawnEntry::Npc { spawntime_ms, .. } => *spawntime_ms,
        }
    }

    fn write_temp_spawn_xml(label: &str, contents: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "tfs-spawn-test-{}-{}.xml",
            std::process::id(),
            label
        ));
        let mut file = std::fs::File::create(&path).expect("temp spawn xml");
        file.write_all(contents.as_bytes()).expect("write spawn xml");
        path
    }

    #[test]
    fn load_tvp_flat_spawns() {
        let path = write_temp_spawn_xml(
            "tvp",
            r#"<?xml version="1.0"?>
<spawns>
  <tvpspawn centerx="100" centery="200" centerz="7" monstername="Rat" spawntime="60" amount="2" radius="5" />
  <tvpspawn centerx="101" centery="201" centerz="7" npcname="Tom" spawntime="60" amount="1" direction="2" radius="2" />
</spawns>"#,
        );
        let zones = load_spawn_xml(&path).expect("parse tvp spawns");
        let _ = std::fs::remove_file(&path);

        assert_eq!(zones.len(), 2);
        assert_eq!(zones[0].entries.len(), 2);
        assert!(matches!(
            &zones[0].entries[0],
            SpawnEntry::Monster { name, .. } if name == "Rat"
        ));
        assert_eq!(entry_spawntime_ms(&zones[0].entries[0]), 60_000);
        assert_eq!(zones[1].entries.len(), 1);
        assert!(matches!(
            &zones[1].entries[0],
            SpawnEntry::Npc { name, .. } if name == "Tom"
        ));
    }

    #[test]
    fn load_repo_forgotten_tvpspawns() {
        let path = Path::new("data/world/forgotten-spawn.xml");
        if !path.exists() {
            return;
        }
        let zones = load_spawn_xml(path).expect("parse repo TVP spawn file");
        let entries: usize = zones.iter().map(|z| z.entries.len()).sum();
        assert!(zones.len() > 9_000, "zones={}", zones.len());
        assert!(entries > 15_000, "entries={entries}");
    }

    #[test]
    fn load_tfs_nested_spawns() {
        let path = write_temp_spawn_xml(
            "tfs",
            r#"<?xml version="1.0"?>
<spawns>
  <spawn centerx="10" centery="20" centerz="7" radius="3">
    <monster name="Dragon" x="0" y="0" spawntime="30" />
  </spawn>
</spawns>"#,
        );
        let zones = load_spawn_xml(&path).expect("parse tfs spawns");
        let _ = std::fs::remove_file(&path);

        assert_eq!(zones.len(), 1);
        assert_eq!(zones[0].entries.len(), 1);
        assert_eq!(entry_spawntime_ms(&zones[0].entries[0]), 30_000);
    }
}
