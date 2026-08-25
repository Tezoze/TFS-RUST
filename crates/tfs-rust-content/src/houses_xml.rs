//! House definitions from `{map}-houses.xml`.
//! Pack surface: TFS `Houses::loadHousesXML` — `house.cpp` (~586).
//! Corpus rent formula is `SQMPrice * Size + RentOffset` (`houses.cc` ~1681);
//! this pack ships explicit per-house `rent`, so XML wins.

use std::path::Path;

use roxmltree::Document;
use tfs_rust_common::Position;
use tfs_rust_common::error::{Result, TfsRustError};
use tracing::info;

/// One `<house>` element from `{map}-houses.xml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseXmlEntry {
    pub id: u32,
    pub name: String,
    pub entry: Position,
    pub rent: u32,
    pub town_id: u32,
    pub size: u32,
}

/// Parse TFS `Houses::loadHousesXML` (`house.cpp` ~586–628).
pub fn load_houses_xml(path: &Path) -> Result<Vec<HouseXmlEntry>> {
    info!("Loading house XML from {:?}", path);
    let xml = std::fs::read_to_string(path).map_err(|e| TfsRustError::Content {
        file: path.to_string_lossy().into_owned(),
        message: e.to_string(),
    })?;
    load_houses_xml_str(&xml, path)
}

/// Parse house XML from an in-memory string (`path` is used only in errors).
pub fn load_houses_xml_str(xml: &str, path: &Path) -> Result<Vec<HouseXmlEntry>> {
    let doc = Document::parse(xml).map_err(|e| TfsRustError::Content {
        file: path.to_string_lossy().into_owned(),
        message: e.to_string(),
    })?;

    let root = doc.root_element();
    let houses_el = if root.has_tag_name("houses") {
        root
    } else {
        doc.descendants()
            .find(|n| n.has_tag_name("houses"))
            .ok_or_else(|| TfsRustError::Content {
                file: path.to_string_lossy().into_owned(),
                message: "missing <houses> root".to_string(),
            })?
    };

    let mut out = Vec::new();
    for node in houses_el.children().filter(|n| n.has_tag_name("house")) {
        let Some(id) = node
            .attribute("houseid")
            .and_then(|s| s.parse::<u32>().ok())
        else {
            tracing::warn!("house XML entry missing houseid — skipped");
            continue;
        };
        let name = node.attribute("name").unwrap_or("").to_string();
        let entryx = node
            .attribute("entryx")
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(0);
        let entryy = node
            .attribute("entryy")
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(0);
        let entryz = node
            .attribute("entryz")
            .and_then(|s| s.parse::<u8>().ok())
            .unwrap_or(0);
        if entryx == 0 && entryy == 0 && entryz == 0 {
            tracing::warn!(houseid = id, "house entry position is (0,0,0)");
        }
        let rent = node
            .attribute("rent")
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        let town_id = node
            .attribute("townid")
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        let size = node
            .attribute("size")
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        out.push(HouseXmlEntry {
            id,
            name,
            entry: Position::new(entryx, entryy, entryz),
            rent,
            town_id,
            size,
        });
    }

    info!(count = out.len(), file = %path.display(), "loaded house XML");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn parses_sample_houses() {
        let xml = r#"<?xml version="1.0"?>
<houses>
	<house name="Spiritkeep" houseid="1" entryx="32265" entryy="32316" entryz="7" rent="19210" townid="1" size="687" />
	<house name="Sunset Homes, Flat 01" houseid="6" entryx="32333" entryy="32232" entryz="7" rent="520" townid="1" size="23" />
</houses>
"#;
        let houses = load_houses_xml_str(xml, Path::new("sample-house.xml")).expect("parse");
        assert_eq!(houses.len(), 2);
        assert_eq!(houses[0].id, 1);
        assert_eq!(houses[0].name, "Spiritkeep");
        assert_eq!(houses[0].rent, 19210);
        assert_eq!(houses[0].town_id, 1);
        assert_eq!(houses[0].size, 687);
        assert_eq!(houses[0].entry, Position::new(32265, 32316, 7));
        assert_eq!(houses[1].id, 6);
        assert_eq!(houses[1].rent, 520);
    }

    #[test]
    fn parses_forgotten_houses_xml() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/world/forgotten-houses.xml");
        if !path.is_file() {
            return;
        }
        let houses = load_houses_xml(&path).expect("forgotten-houses.xml");
        assert!(houses.len() > 100, "expected a full house list, got {}", houses.len());
        let spirit = houses.iter().find(|h| h.id == 1).expect("house 1");
        assert_eq!(spirit.name, "Spiritkeep");
        assert_eq!(spirit.rent, 19210);
        assert_eq!(spirit.town_id, 1);
        assert_eq!(spirit.entry, Position::new(32265, 32316, 7));
    }
}
