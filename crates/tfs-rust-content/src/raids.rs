//! TFS raid XML catalog — `data/raids/raids.xml` + per-raid files.
//! Pack surface: TFS `Raids::loadFromXml` / `Raid::loadFromXml` (`raids.cpp`).
//! Delays and lifetimes stay in **milliseconds** here; convert to rounds at queue time.

use std::collections::HashMap;
use std::path::Path;

use roxmltree::Document;
use tfs_rust_common::error::{Result, TfsRustError};
use tfs_rust_common::Position;
use tracing::info;

/// One `<monster>` under `<areaspawn>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaidMonsterAmount {
    pub name: String,
    pub min_amount: u16,
    pub max_amount: u16,
}

/// One announce / areaspawn / singlespawn from a raid file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RaidWave {
    Announce {
        delay_ms: u32,
        announce_type: String,
        message: String,
    },
    AreaSpawn {
        delay_ms: u32,
        lifetime_ms: u32,
        radius: u16,
        center: Position,
        monsters: Vec<RaidMonsterAmount>,
    },
    SingleSpawn {
        delay_ms: u32,
        name: String,
        position: Position,
    },
}

/// One `<raid>` catalog row plus parsed waves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaidDefinition {
    pub name: String,
    pub interval_secs: Option<u32>,
    pub date_unix: Option<i64>,
    pub log: bool,
    pub filename: String,
    pub waves: Vec<RaidWave>,
}

/// Loaded `data/raids` catalog. Empty when `raids.xml` is missing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RaidCatalog {
    /// Lowercased name → definition.
    pub by_name: HashMap<String, RaidDefinition>,
}

impl RaidCatalog {
    pub fn get(&self, name: &str) -> Option<&RaidDefinition> {
        self.by_name.get(&name.to_ascii_lowercase())
    }

    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }
}

fn content_err(file: &Path, message: impl Into<String>) -> TfsRustError {
    TfsRustError::Content {
        file: file.to_string_lossy().into_owned(),
        message: message.into(),
    }
}

fn parse_bool_attr(raw: Option<&str>) -> bool {
    matches!(
        raw.map(|s| s.trim().to_ascii_lowercase()).as_deref(),
        Some("true" | "1" | "yes")
    )
}

fn parse_u32_attr(node: roxmltree::Node, name: &str) -> Option<u32> {
    node.attribute(name).and_then(|s| s.parse().ok())
}

fn parse_i64_attr(node: roxmltree::Node, name: &str) -> Option<i64> {
    node.attribute(name).and_then(|s| s.parse().ok())
}

fn parse_u16_attr(node: roxmltree::Node, name: &str) -> Option<u16> {
    node.attribute(name).and_then(|s| s.parse().ok())
}

fn parse_u8_attr(node: roxmltree::Node, name: &str) -> Option<u8> {
    node.attribute(name).and_then(|s| s.parse().ok())
}

/// Parse TFS `raids.xml` + each `file=` raid. Missing catalog → empty.
pub fn load_raids(data_dir: &Path) -> Result<RaidCatalog> {
    let catalog_path = data_dir.join("raids").join("raids.xml");
    if !catalog_path.is_file() {
        tracing::info!(
            path = %catalog_path.display(),
            "no raids.xml — raid catalog empty"
        );
        return Ok(RaidCatalog::default());
    }
    let xml = std::fs::read_to_string(&catalog_path)
        .map_err(|e| content_err(&catalog_path, e.to_string()))?;
    load_raids_xml(&xml, data_dir, &catalog_path)
}

/// Parse catalog XML; `file=` paths are relative to `data/raids/`.
pub fn load_raids_xml(xml: &str, data_dir: &Path, catalog_path: &Path) -> Result<RaidCatalog> {
    let raids_dir = data_dir.join("raids");
    let doc = Document::parse(xml).map_err(|e| content_err(catalog_path, e.to_string()))?;
    let root = doc.root_element();
    let raids_el = if root.has_tag_name("raids") {
        root
    } else {
        doc.descendants()
            .find(|n| n.has_tag_name("raids"))
            .ok_or_else(|| content_err(catalog_path, "missing <raids> root"))?
    };

    let mut by_name = HashMap::new();
    for node in raids_el.children().filter(|n| n.has_tag_name("raid")) {
        let Some(name) = node
            .attribute("name")
            .map(str::trim)
            .filter(|s| !s.is_empty())
        else {
            tracing::warn!("raid catalog entry missing name — skipped");
            continue;
        };
        let filename = node
            .attribute("file")
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("")
            .to_string();
        if filename.is_empty() {
            tracing::warn!(name, "raid catalog entry missing file — skipped");
            continue;
        }
        let raid_path = raids_dir.join(&filename);
        let waves = if raid_path.is_file() {
            match std::fs::read_to_string(&raid_path) {
                Ok(body) => match parse_raid_file_xml(&body, &raid_path) {
                    Ok(w) => w,
                    Err(e) => {
                        tracing::warn!(name, error = %e, path = %raid_path.display(), "raid file parse failed");
                        continue;
                    }
                },
                Err(e) => {
                    tracing::warn!(name, error = %e, path = %raid_path.display(), "raid file unreadable");
                    continue;
                }
            }
        } else {
            tracing::warn!(name, path = %raid_path.display(), "raid file missing");
            continue;
        };
        let def = RaidDefinition {
            name: name.to_string(),
            interval_secs: parse_u32_attr(node, "interval"),
            date_unix: parse_i64_attr(node, "date"),
            log: parse_bool_attr(node.attribute("log")),
            filename,
            waves,
        };
        by_name.insert(name.to_ascii_lowercase(), def);
    }

    info!(count = by_name.len(), file = %catalog_path.display(), "loaded raid catalog");
    Ok(RaidCatalog { by_name })
}

/// Parse a single `<raid>` file from an in-memory string.
pub fn parse_raid_file_xml(xml: &str, path: &Path) -> Result<Vec<RaidWave>> {
    let doc = Document::parse(xml).map_err(|e| content_err(path, e.to_string()))?;
    let root = doc.root_element();
    let raid_el = if root.has_tag_name("raid") {
        root
    } else {
        doc.descendants()
            .find(|n| n.has_tag_name("raid"))
            .ok_or_else(|| content_err(path, "missing <raid> root"))?
    };

    let mut waves = Vec::new();
    for node in raid_el.children().filter(|n| n.is_element()) {
        if node.has_tag_name("announce") {
            let delay_ms = parse_u32_attr(node, "delay").unwrap_or(0);
            let announce_type = node.attribute("type").unwrap_or("event").to_string();
            let message = node.attribute("message").unwrap_or("").to_string();
            waves.push(RaidWave::Announce {
                delay_ms,
                announce_type,
                message,
            });
        } else if node.has_tag_name("areaspawn") {
            if let Some(wave) = parse_area_spawn(node) {
                waves.push(wave);
            }
        } else if node.has_tag_name("singlespawn") {
            let delay_ms = parse_u32_attr(node, "delay").unwrap_or(0);
            let Some(name) = node
                .attribute("name")
                .map(str::trim)
                .filter(|s| !s.is_empty())
            else {
                continue;
            };
            let x = parse_u16_attr(node, "x").unwrap_or(0);
            let y = parse_u16_attr(node, "y").unwrap_or(0);
            let z = parse_u8_attr(node, "z").unwrap_or(0);
            waves.push(RaidWave::SingleSpawn {
                delay_ms,
                name: name.to_string(),
                position: Position::new(x, y, z),
            });
        }
    }
    Ok(waves)
}

fn parse_area_spawn(node: roxmltree::Node) -> Option<RaidWave> {
    let delay_ms = parse_u32_attr(node, "delay").unwrap_or(0);
    let lifetime_ms = parse_u32_attr(node, "lifetime").unwrap_or(0);
    let (center, radius) = area_spawn_center_radius(node)?;
    let mut monsters = Vec::new();
    for child in node.children().filter(|n| n.has_tag_name("monster")) {
        let Some(name) = child
            .attribute("name")
            .map(str::trim)
            .filter(|s| !s.is_empty())
        else {
            continue;
        };
        let (min_amount, max_amount) = monster_amount_range(child);
        monsters.push(RaidMonsterAmount {
            name: name.to_string(),
            min_amount,
            max_amount,
        });
    }
    Some(RaidWave::AreaSpawn {
        delay_ms,
        lifetime_ms,
        radius,
        center,
        monsters,
    })
}

fn monster_amount_range(node: roxmltree::Node) -> (u16, u16) {
    if let Some(amount) = parse_u16_attr(node, "amount") {
        return (amount, amount);
    }
    let min = parse_u16_attr(node, "minamount").unwrap_or(1);
    let max = parse_u16_attr(node, "maxamount").unwrap_or(min).max(min);
    (min, max)
}

fn area_spawn_center_radius(node: roxmltree::Node) -> Option<(Position, u16)> {
    if let (Some(cx), Some(cy)) = (
        parse_u16_attr(node, "centerx"),
        parse_u16_attr(node, "centery"),
    ) {
        let cz = parse_u8_attr(node, "centerz").unwrap_or(7);
        let radius = parse_u16_attr(node, "radius").unwrap_or(0);
        return Some((Position::new(cx, cy, cz), radius));
    }
    let fromx = parse_u16_attr(node, "fromx")?;
    let fromy = parse_u16_attr(node, "fromy")?;
    let tox = parse_u16_attr(node, "tox").unwrap_or(fromx);
    let toy = parse_u16_attr(node, "toy").unwrap_or(fromy);
    let fromz = parse_u8_attr(node, "fromz")
        .or_else(|| parse_u8_attr(node, "toz"))
        .unwrap_or(7);
    let min_x = fromx.min(tox);
    let max_x = fromx.max(tox);
    let min_y = fromy.min(toy);
    let max_y = fromy.max(toy);
    let cx = min_x + (max_x - min_x) / 2;
    let cy = min_y + (max_y - min_y) / 2;
    let radius = ((max_x - min_x) / 2).max((max_y - min_y) / 2);
    Some((Position::new(cx, cy, fromz), radius))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn parses_in_memory_raid_catalog_and_waves() {
        let xml = r#"<?xml version="1.0"?>
<raid>
	<announce delay="1000" type="warning" message="Rats!" />
	<areaspawn delay="2000" lifetime="5000" radius="5" centerx="94" centery="126" centerz="7">
		<monster name="Rat" amount="3" />
	</areaspawn>
	<areaspawn delay="30000" fromx="89" fromy="122" fromz="7" tox="99" toy="130" toz="7">
		<monster name="Rat" minamount="4" maxamount="10" />
	</areaspawn>
	<singlespawn delay="15000" name="Cave Rat" x="93" y="123" z="7" />
</raid>
"#;
        let waves = parse_raid_file_xml(xml, Path::new("testraid.xml")).expect("parse");
        assert_eq!(waves.len(), 4);
        match &waves[0] {
            RaidWave::Announce {
                delay_ms,
                announce_type,
                message,
            } => {
                assert_eq!(*delay_ms, 1000);
                assert_eq!(announce_type, "warning");
                assert_eq!(message, "Rats!");
            }
            other => panic!("expected announce, got {other:?}"),
        }
        match &waves[1] {
            RaidWave::AreaSpawn {
                delay_ms,
                lifetime_ms,
                radius,
                center,
                monsters,
            } => {
                assert_eq!(*delay_ms, 2000);
                assert_eq!(*lifetime_ms, 5000);
                assert_eq!(*radius, 5);
                assert_eq!(*center, Position::new(94, 126, 7));
                assert_eq!(monsters.len(), 1);
                assert_eq!(monsters[0].name, "Rat");
                assert_eq!(monsters[0].min_amount, 3);
                assert_eq!(monsters[0].max_amount, 3);
            }
            other => panic!("expected areaspawn, got {other:?}"),
        }
        match &waves[2] {
            RaidWave::AreaSpawn {
                radius,
                center,
                monsters,
                ..
            } => {
                assert_eq!(*center, Position::new(94, 126, 7));
                assert_eq!(*radius, 5);
                assert_eq!(monsters[0].min_amount, 4);
                assert_eq!(monsters[0].max_amount, 10);
            }
            other => panic!("expected box areaspawn, got {other:?}"),
        }
        match &waves[3] {
            RaidWave::SingleSpawn {
                delay_ms,
                name,
                position,
            } => {
                assert_eq!(*delay_ms, 15000);
                assert_eq!(name, "Cave Rat");
                assert_eq!(*position, Position::new(93, 123, 7));
            }
            other => panic!("expected singlespawn, got {other:?}"),
        }
    }
}
