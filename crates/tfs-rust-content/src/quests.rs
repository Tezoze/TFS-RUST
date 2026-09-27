//! Quest storage index for the 7.9+ "questlog has been updated" line.
//!
//! C++ reference: `800src/quests.cpp` `Quests::loadFromXml`, `Quests::isQuestStorage`.
//! The quest-log window opcodes are later than 8.0 and stay unsupported.

use std::path::Path;

use roxmltree::Document;
use tfs_rust_common::error::{Result, TfsRustError};

#[derive(Debug, Clone, Default)]
pub struct QuestCatalog {
    starts: Vec<(u32, i32)>,
    missions: Vec<MissionSpan>,
}

#[derive(Debug, Clone)]
struct MissionSpan {
    storage_id: u32,
    start: i32,
    end: i32,
    /// `mission` `description` attribute was empty, so any in-range write qualifies.
    main_description_empty: bool,
}

impl QuestCatalog {
    pub fn load(path: &Path) -> Result<Self> {
        let xml = std::fs::read_to_string(path).map_err(|e| TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: e.to_string(),
        })?;
        Self::parse(&xml).map_err(|message| TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message,
        })
    }

    pub fn parse(xml: &str) -> std::result::Result<Self, String> {
        let doc = Document::parse(xml).map_err(|e| e.to_string())?;
        let mut catalog = Self::default();
        let Some(root) = doc.descendants().find(|n| n.has_tag_name("quests")) else {
            return Ok(catalog);
        };
        for quest in root.children().filter(|n| n.has_tag_name("quest")) {
            if let (Some(id), Some(value)) = (
                attr_u32(quest, "startstorageid"),
                attr_i32(quest, "startstoragevalue"),
            ) {
                catalog.starts.push((id, value));
            }
            for mission in quest.children().filter(|n| n.has_tag_name("mission")) {
                let Some(storage_id) = attr_u32(mission, "storageid") else {
                    continue;
                };
                let Some(start) = attr_i32(mission, "startvalue") else {
                    continue;
                };
                let Some(end) = attr_i32(mission, "endvalue") else {
                    continue;
                };
                let description = mission.attribute("description").unwrap_or("");
                catalog.missions.push(MissionSpan {
                    storage_id,
                    start,
                    end,
                    main_description_empty: description.is_empty(),
                });
            }
        }
        Ok(catalog)
    }

    /// `Quests::isQuestStorage` (`800src/quests.cpp`).
    pub fn is_quest_storage(&self, key: u32, value: i32, old_value: i32) -> bool {
        if self
            .starts
            .iter()
            .any(|(id, start)| *id == key && *start == value)
        {
            return true;
        }
        for mission in &self.missions {
            if mission.storage_id != key || value < mission.start || value > mission.end {
                continue;
            }
            if mission.main_description_empty
                || old_value < mission.start
                || old_value > mission.end
            {
                return true;
            }
        }
        false
    }
}

fn attr_u32(node: roxmltree::Node, name: &str) -> Option<u32> {
    node.attribute(name)?.parse().ok()
}

fn attr_i32(node: roxmltree::Node, name: &str) -> Option<i32> {
    node.attribute(name)?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_quest_storage_matches_cpp() {
        let xml = r#"
            <quests>
                <quest name="Example" startstorageid="1001" startstoragevalue="1">
                    <mission name="M" storageid="1001" startvalue="1" endvalue="3">
                        <missionstate id="1" description="a" />
                    </mission>
                    <mission name="Described" storageid="9" startvalue="4" endvalue="5" description="main" />
                </quest>
            </quests>
        "#;
        let catalog = QuestCatalog::parse(xml).expect("parse");
        assert!(catalog.is_quest_storage(1001, 1, 0));
        assert!(catalog.is_quest_storage(1001, 2, 2));
        assert!(!catalog.is_quest_storage(1001, 9, 0));
        assert!(catalog.is_quest_storage(9, 4, 0));
        assert!(!catalog.is_quest_storage(9, 4, 4));
    }
}
