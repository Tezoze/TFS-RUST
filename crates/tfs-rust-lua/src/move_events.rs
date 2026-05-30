//! MoveEvent equip/deequip registry and XML loader.
//!
//! C++ reference: `src/movement.cpp` `MoveEvents`, `MoveEvent::fireEquip`.

use std::collections::HashMap;
use std::path::Path;

use crate::runtime::{CallbackRef, LuaError, LuaRuntime};

/// Equip vs deequip — `MoveEvent_t` subset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MoveEventKind {
    Equip,
    DeEquip,
}

#[derive(Debug)]
pub struct MoveEventEntry {
    pub kind: MoveEventKind,
    pub item_id: u16,
    pub slot_mask: u32,
    pub req_level: u32,
    pub callback: CallbackRef,
}

/// Registry of equip/deequip callbacks keyed by `(kind, item_id)`.
#[derive(Debug, Default)]
pub struct MoveEventsRegistry {
    by_item: HashMap<(MoveEventKind, u16), MoveEventEntry>,
}

impl MoveEventsRegistry {
    pub fn get(&self, kind: MoveEventKind, item_id: u16) -> Option<&MoveEventEntry> {
        self.by_item.get(&(kind, item_id))
    }

    pub fn len(&self) -> usize {
        self.by_item.len()
    }

    pub fn register(&mut self, entry: MoveEventEntry) {
        self.by_item.insert((entry.kind, entry.item_id), entry);
    }

    /// Load `movements/lib/movements.lua` and ensure default equip globals exist.
    ///
    /// C++ ref: `MoveEvents::load` loads lib before XML callback registration.
    fn ensure_movement_globals(
        runtime: &mut LuaRuntime,
        data_dir: &Path,
    ) -> Result<(), LuaError> {
        let lib_path = data_dir.join("movements/lib/movements.lua");
        if lib_path.exists() {
            let path_string = lib_path.display().to_string();
            if let Err(e) = runtime.load_script(&path_string) {
                tracing::warn!("Failed to load movements lib {}: {e}", lib_path.display());
            }
        }

        const DEFAULT_EQUIP_GLOBALS: &str = r#"
if onEquipItem == nil then
    function onEquipItem(player, item, slot, isCheck)
        return true
    end
end
if onDeEquipItem == nil then
    function onDeEquipItem(player, item, slot, isCheck)
        return true
    end
end
"#;
        runtime.exec_chunk("movements_defaults", DEFAULT_EQUIP_GLOBALS)?;
        Ok(())
    }

    /// Parse `data/movements/movements.xml` equip/deequip entries with `function="..."`.
    pub fn load_from_xml(
        &mut self,
        runtime: &mut LuaRuntime,
        data_dir: &Path,
    ) -> Result<(), LuaError> {
        Self::ensure_movement_globals(runtime, data_dir)?;

        let xml_path = data_dir.join("movements/movements.xml");
        if !xml_path.exists() {
            tracing::warn!("movements.xml not found: {}", xml_path.display());
            return Ok(());
        }
        let xml = std::fs::read_to_string(&xml_path)
            .map_err(|e| LuaError::ScriptIo(xml_path.display().to_string(), e.to_string()))?;

        #[derive(serde::Deserialize)]
        struct MovementsXml {
            #[serde(rename = "movevent", default)]
            movevents: Vec<MoveventXml>,
        }
        #[derive(serde::Deserialize)]
        struct MoveventXml {
            #[serde(rename = "@event")]
            event: String,
            #[serde(rename = "@itemid")]
            itemid: Option<u16>,
            #[serde(rename = "@fromid")]
            fromid: Option<u16>,
            #[serde(rename = "@toid")]
            toid: Option<u16>,
            #[serde(rename = "@slot")]
            slot: Option<String>,
            #[serde(rename = "@level")]
            level: Option<u32>,
            #[serde(rename = "@function")]
            function: Option<String>,
        }

        let parsed: MovementsXml = quick_xml::de::from_str(&xml)
            .map_err(|e| LuaError::SyntaxError(e.to_string()))?;

        for mv in parsed.movevents {
            let Some(function) = mv.function else {
                continue;
            };
            let event_lower = mv.event.to_ascii_lowercase();
            let kind = match event_lower.as_str() {
                "equip" => MoveEventKind::Equip,
                "deequip" => MoveEventKind::DeEquip,
                _ => continue,
            };

            let ids: Vec<u16> = if let Some(id) = mv.itemid {
                vec![id]
            } else if let (Some(from), Some(to)) = (mv.fromid, mv.toid) {
                (from..=to).collect()
            } else {
                continue;
            };

            let slot_mask = parse_slot_mask(mv.slot.as_deref().unwrap_or(""));
            let req_level = mv.level.unwrap_or(0);

            for item_id in ids {
                let callback = runtime.register_callback(
                    format!("move:{function}:{event_lower}:{item_id}"),
                    &function,
                )?;
                self.register(MoveEventEntry {
                    kind,
                    item_id,
                    slot_mask,
                    req_level,
                    callback,
                });
            }
        }

        Ok(())
    }
}

fn parse_slot_mask(slot: &str) -> u32 {
    match slot.to_ascii_lowercase().as_str() {
        "head" => 1 << 0,
        "necklace" => 1 << 1,
        "backpack" => 1 << 2,
        "armor" => 1 << 3,
        "right-hand" | "right" => 1 << 4,
        "left-hand" | "left" | "hand" | "shield" => (1 << 4) | (1 << 5),
        "legs" => 1 << 6,
        "feet" => 1 << 7,
        "ring" => 1 << 8,
        "ammo" => 1 << 9,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::parse_slot_mask;

    #[test]
    fn slot_mask_feet_and_hand() {
        assert_eq!(parse_slot_mask("feet"), 1 << 7);
        assert_eq!(parse_slot_mask("hand"), (1 << 4) | (1 << 5));
    }
}
