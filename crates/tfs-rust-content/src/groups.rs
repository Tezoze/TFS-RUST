//! Player groups — Lua-as-data (`data/defs/groups.lua`).
//!
//! Replaces the outgoing `quick-xml` `GroupDatabase::load` per the
//! `docs/DATA_FORMAT_MIGRATION.md` Phase 1 pattern established by `vocations.rs`.
//! The full `<group>` block (id, name, access, maxdepotitems, maxvipentries, and
//! the `<flags>` map) now lives in `data/defs/groups.lua` and deserializes into
//! [`Group`] via `mlua`'s `serde` feature. [`GroupDatabase`] indexes them by id
//! for game-thread lookups; consumers (`flags_for_group`, `player_is_access_player`,
//! depot limits) keep working unchanged because the public `Group` shape is
//! preserved.
//!
//! C++ reference: `src/groups.cpp` `Groups::load`, `src/const.h` `PlayerFlags`.

use std::collections::HashMap;
use std::path::Path;

use mlua::LuaSerdeExt;
use serde::Deserialize;
use tfs_rust_common::error::{Result, TfsRustError};
use tracing::info;

use crate::data_lua::{load_data_table, require_schema, sandboxed_data_lua};

/// Expected `schema` version for `data/defs/groups.lua`.
pub const GROUPS_SCHEMA: u32 = 1;

/// Player group — mirrors `data/defs/groups.lua` 1:1.
///
/// Fields are `snake_case` to match the Lua keys directly (no `serde(rename)`
/// noise). `flags` is a `HashMap<String, bool>` keyed by the lowercased C++
/// `<flag name="..."/>` attribute name so `flag_name_to_bit` in
/// `tfs-rust-core::player::flags` keeps working unchanged.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Group {
    pub id: u16,
    pub name: String,
    pub access: bool,
    pub max_depot_items: u32,
    pub max_vip_entries: u32,
    #[serde(default)]
    pub flags: HashMap<String, bool>,
}

/// Indexed group registry — materialized once at startup, immutable on the
/// game thread.
#[derive(Debug, Clone, Default)]
pub struct GroupDatabase {
    pub groups: HashMap<u16, Group>,
}

impl GroupDatabase {
    /// Load `data/defs/groups.lua` via the sandboxed data-Lua loader, deserialize
    /// into `Vec<Group>`, validate, and index by id.
    pub fn load(path: &Path) -> Result<Self> {
        info!("Loading groups from {:?}", path);
        let lua = sandboxed_data_lua()?;
        let root = load_data_table(&lua, path)?;
        require_schema(&root, GROUPS_SCHEMA)?;

        let groups_value = root
            .get("groups")
            .map_err(|e| TfsRustError::Content {
                file: path.to_string_lossy().into_owned(),
                message: format!("missing 'groups' array: {e}"),
            })?;
        let defs: Vec<Group> = lua
            .from_value(groups_value)
            .map_err(|e| TfsRustError::Content {
                file: path.to_string_lossy().into_owned(),
                message: format!("deserialize groups failed: {e}"),
            })?;

        validate_groups(&defs, path)?;

        let groups = defs
            .into_iter()
            .map(|d| (d.id, d))
            .collect::<HashMap<u16, Group>>();
        Ok(Self { groups })
    }
}

/// Semantic validation — unique ids, required name non-empty. Fails fast at
/// startup per the migration doc guardrails.
fn validate_groups(defs: &[Group], path: &Path) -> Result<()> {
    let mut seen = HashMap::new();
    for d in defs {
        if let Some(prev) = seen.insert(d.id, d.name.as_str()) {
            return Err(TfsRustError::Content {
                file: path.to_string_lossy().into_owned(),
                message: format!("duplicate group id {}: '{}' vs '{}'", d.id, prev, d.name),
            });
        }
        if d.name.is_empty() {
            return Err(TfsRustError::Content {
                file: path.to_string_lossy().into_owned(),
                message: format!("group {} has empty name", d.id),
            });
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Temporary XML loader — kept only for the dual-load golden equivalence test.
// Deleted once `data/XML/groups.xml` is retired.
// ---------------------------------------------------------------------------

/// Temporary: load the outgoing `data/XML/groups.xml` into the same `Group`
/// shape so the golden test can assert the Lua file carries the same data.
/// Not used in production — `GroupDatabase::load` is the real path.
#[cfg(test)]
fn load_xml_for_golden_test(path: &Path) -> Result<Vec<Group>> {
    use quick_xml::events::Event;
    use quick_xml::Reader;

    let xml = std::fs::read_to_string(path).map_err(|e| TfsRustError::Content {
        file: path.to_string_lossy().into_owned(),
        message: e.to_string(),
    })?;

    let mut reader = Reader::from_str(&xml);
    reader.trim_text(true);
    let mut buf = Vec::new();
    let mut groups = Vec::new();
    let mut current_group_id: Option<u16> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            // `<group ...>` (Start) — has `<flags>` children; set
            // `current_group_id` so the `Empty` `<flag/>` handler can attribute
            // them. The matching `</group>` End event resets it.
            Ok(Event::Start(e)) if e.name().as_ref() == b"group" => {
                let (id, group) = parse_group_attrs(&e, path)?;
                groups.push(group);
                current_group_id = Some(id);
            }
            // Self-closing `<group ... />` (Empty) — no `<flags>` child. Group 1
            // in the shipped XML uses this form. No `current_group_id` to set
            // (the `End` event never fires for self-closing tags).
            Ok(Event::Empty(e)) if e.name().as_ref() == b"group" => {
                let (_id, group) = parse_group_attrs(&e, path)?;
                groups.push(group);
            }
            Ok(Event::Empty(e)) if e.name().as_ref() == b"flag" => {
                let Some(group_id) = current_group_id else {
                    buf.clear();
                    continue;
                };
                let mut key_name = None;
                let mut enabled = false;
                for attr in e.attributes() {
                    let attr = attr.map_err(|err| TfsRustError::Content {
                        file: path.to_string_lossy().into_owned(),
                        message: err.to_string(),
                    })?;
                    let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                    let value = String::from_utf8_lossy(attr.value.as_ref()).to_string();
                    if key != "key" {
                        key_name = Some(key);
                        enabled = matches!(value.as_str(), "1" | "yes" | "true");
                    }
                }
                if let Some(key_name) = key_name {
                    if let Some(group) = groups.iter_mut().find(|g| g.id == group_id) {
                        group.flags.insert(key_name, enabled);
                    }
                }
            }
            Ok(Event::End(e)) if e.name().as_ref() == b"group" => {
                current_group_id = None;
            }
            Ok(Event::Eof) => break,
            Err(err) => {
                return Err(TfsRustError::Content {
                    file: path.to_string_lossy().into_owned(),
                    message: err.to_string(),
                });
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(groups)
}

/// Parse the `<group ...>` attributes (shared by `Start` and `Empty` events).
/// Returns `(id, Group)` so the caller can decide whether to set
/// `current_group_id` (Start has `<flags>` children; Empty does not).
#[cfg(test)]
fn parse_group_attrs(
    e: &quick_xml::events::BytesStart,
    path: &Path,
) -> Result<(u16, Group)> {
    let mut id = None;
    let mut name = String::new();
    let mut access = false;
    let mut max_depot_items = 0u32;
    let mut max_vip_entries = 0u32;

    for attr in e.attributes() {
        let attr = attr.map_err(|err| TfsRustError::Content {
            file: path.to_string_lossy().into_owned(),
            message: err.to_string(),
        })?;
        let key = attr.key.as_ref();
        let value = String::from_utf8_lossy(attr.value.as_ref()).into_owned();
        match key {
            b"id" => {
                id = Some(value.parse::<u16>().map_err(|err| TfsRustError::Content {
                    file: path.to_string_lossy().into_owned(),
                    message: format!("invalid group id '{value}': {err}"),
                })?)
            }
            b"name" => name = value,
            b"access" => access = matches!(value.as_str(), "1" | "yes" | "true"),
            b"maxdepotitems" => max_depot_items = value.parse::<u32>().unwrap_or(0),
            b"maxvipentries" => max_vip_entries = value.parse::<u32>().unwrap_or(0),
            _ => {}
        }
    }

    let id = id.ok_or_else(|| TfsRustError::Content {
        file: path.to_string_lossy().into_owned(),
        message: "group entry missing required 'id'".to_string(),
    })?;
    Ok((
        id,
        Group {
            id,
            name,
            access,
            max_depot_items,
            max_vip_entries,
            flags: HashMap::new(),
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Golden parse: `data/defs/groups.lua` carries the 6 shipped groups with
    /// known access tiers and flag bits (god=access, gamemaster=access,
    /// player/tutor/senior tutor=not access).
    #[test]
    fn golden_parse_groups_lua() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("data")
            .join("defs")
            .join("groups.lua");
        if !path.is_file() {
            eprintln!("skipping golden_parse_groups_lua — {} not found", path.display());
            return;
        }
        let reg = GroupDatabase::load(&path).expect("load groups.lua");

        // 6 groups shipped.
        assert_eq!(reg.groups.len(), 6, "expected 6 groups, got {}", reg.groups.len());

        let player = reg.groups.get(&1).expect("player group");
        assert_eq!(player.name, "player");
        assert!(!player.access);
        assert!(player.flags.is_empty());

        let god = reg.groups.get(&6).expect("god group");
        assert_eq!(god.name, "god");
        assert!(god.access);
        assert_eq!(god.max_vip_entries, 200);
        // Spot-check flag bits the engine reads (player/flags.rs).
        assert_eq!(god.flags.get("cannotpickupitem"), Some(&false));
        assert_eq!(god.flags.get("hasinfinitecapacity"), Some(&true));
        assert_eq!(god.flags.get("canbroadcast"), Some(&true));
        assert_eq!(god.flags.get("cannotbemuted"), Some(&true));
        // god can edit houses; gamemaster cannot.
        assert_eq!(god.flags.get("canedithouses"), Some(&true));

        let gm = reg.groups.get(&4).expect("gamemaster group");
        assert_eq!(gm.name, "gamemaster");
        assert!(gm.access);
        assert_eq!(gm.flags.get("canedithouses"), Some(&false));
        assert_eq!(gm.flags.get("hasinfinitemana"), Some(&false));

        let senior_tutor = reg.groups.get(&3).expect("senior tutor group");
        assert_eq!(senior_tutor.name, "senior tutor");
        assert!(!senior_tutor.access);
        assert_eq!(senior_tutor.flags.get("cantalkredchannel"), Some(&true));
    }

    /// Dual-load golden equivalence: the new `data/defs/groups.lua` loader
    /// produces the same `Group`s as the outgoing `groups.xml` parser
    /// (`DATA_FORMAT_MIGRATION.md` "golden equivalence").
    #[test]
    fn dual_load_xml_lua_equivalence() {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..");
        let lua_path = manifest.join("data").join("defs").join("groups.lua");
        let xml_path = manifest.join("data").join("XML").join("groups.xml");
        if !lua_path.is_file() || !xml_path.is_file() {
            eprintln!(
                "skipping dual_load_xml_lua_equivalence — missing data files (lua={} xml={})",
                lua_path.is_file(),
                xml_path.is_file()
            );
            return;
        }

        let reg = GroupDatabase::load(&lua_path).expect("load groups.lua");
        let xml_defs = load_xml_for_golden_test(&xml_path).expect("load groups.xml");

        // Every XML group must be present in the Lua registry with matching
        // fields. Compare id/name/access/limits/flags exactly.
        for xml_def in &xml_defs {
            let lua_def = reg
                .groups
                .get(&xml_def.id)
                .unwrap_or_else(|| panic!("group {} missing from lua", xml_def.id));
            assert_eq!(lua_def.id, xml_def.id, "id mismatch");
            assert_eq!(lua_def.name, xml_def.name, "name mismatch for id {}", xml_def.id);
            assert_eq!(lua_def.access, xml_def.access, "access mismatch for id {}", xml_def.id);
            assert_eq!(
                lua_def.max_depot_items, xml_def.max_depot_items,
                "max_depot_items mismatch for id {}", xml_def.id
            );
            assert_eq!(
                lua_def.max_vip_entries, xml_def.max_vip_entries,
                "max_vip_entries mismatch for id {}", xml_def.id
            );
            assert_eq!(
                lua_def.flags, xml_def.flags,
                "flags mismatch for id {} (lua={:?} xml={:?})",
                xml_def.id, lua_def.flags, xml_def.flags
            );
        }
        assert_eq!(
            reg.groups.len(),
            xml_defs.len(),
            "group count mismatch (lua={} xml={})",
            reg.groups.len(),
            xml_defs.len()
        );
    }
}
