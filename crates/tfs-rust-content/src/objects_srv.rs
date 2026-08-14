//! 772 `objects.srv` parser — **offline tooling only** (OTB patch / audits).
//!
//! C++ reference: `tibia-game-master/src/cract.cc` `TShortway::FillMap`, `NotifyGo` (`WAYPOINTS`).
//! TFS stores the same per-tile terrain weight in OTB as `ITEM_ATTR_SPEED` (`src/items.cpp`).
//!
//! **`data/items/items.otb` is patched offline** (`patch-otb-waypoints`) so `ITEM_ATTR_SPEED`
//! mirrors walkable `objects.srv` `Waypoints`. The **server never loads `objects.srv`** —
//! runtime item data comes from OTB (+ `items.xml`) only. Keep this module for the patcher
//! binary and content audits; do not call overlays from `pipeline` / game startup.

use crate::otb::ItemType;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tfs_rust_common::error::{Result, TfsRustError};
use tracing::info;

/// One ground type from `objects.srv` with walkable BANK `Waypoints`.
#[derive(Debug, Clone)]
pub struct ObjectsSrvGroundWaypoints {
    pub type_id: u16,
    pub waypoints: u16,
}

/// One type from `objects.srv` with the `DistUse` flag (`enums.hh:215`).
#[derive(Debug, Clone)]
pub struct ObjectsSrvDistUse {
    pub type_id: u16,
}

const REF_772_DIR_NAMES: &[&str] = &["classic-772", "cipsoft-772"];

fn reference_objects_srv_under(base: PathBuf) -> Option<PathBuf> {
    for name in REF_772_DIR_NAMES {
        let path = base.join(name).join("runtime/dat/objects.srv");
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

/// Resolve `objects.srv` for 772 Waypoints overlay (optional at runtime).
///
/// Checks `TFS_OBJECTS_SRV` (or deprecated `TFS_CIPSOFT_OBJECTS_SRV`), then
/// `TFS_REFERENCE_DIR/{classic-772,cipsoft-772}/runtime/dat/objects.srv`, then cwd `reference/…`.
pub fn resolve_objects_srv_path() -> Option<PathBuf> {
    for key in ["TFS_OBJECTS_SRV", "TFS_CIPSOFT_OBJECTS_SRV"] {
        if let Ok(p) = std::env::var(key) {
            let path = PathBuf::from(p);
            if path.is_file() {
                return Some(path);
            }
        }
    }
    if let Ok(ref_dir) = std::env::var("TFS_REFERENCE_DIR")
        && let Some(path) = reference_objects_srv_under(PathBuf::from(ref_dir))
    {
        return Some(path);
    }
    reference_objects_srv_under(PathBuf::from("reference"))
}

/// Parse walkable BANK entries with `Waypoints > 0` from 772 `objects.srv`.
pub fn parse_walkable_waypoints(path: &Path) -> Result<Vec<ObjectsSrvGroundWaypoints>> {
    let text = std::fs::read_to_string(path).map_err(|e| TfsRustError::Content {
        file: path.to_string_lossy().into_owned(),
        message: e.to_string(),
    })?;
    let mut out = Vec::new();
    for block in text.split("\nTypeID") {
        let block = if block.starts_with("TypeID") {
            block.to_string()
        } else {
            format!("TypeID{block}")
        };
        let Some(type_id) = parse_type_id(&block) else {
            continue;
        };
        let (bank, unpass) = {
            let f = parse_flags(&block);
            (f.bank, f.unpass)
        };
        if !bank || unpass {
            continue;
        }
        let Some(waypoints) = parse_waypoints(&block) else {
            continue;
        };
        if waypoints <= 0 {
            continue;
        }
        out.push(ObjectsSrvGroundWaypoints {
            type_id,
            waypoints: waypoints as u16,
        });
    }
    Ok(out)
}

/// Parse **all** BANK entries (including `Unpass`) with their `Waypoints` value from 772 `objects.srv`.
///
/// Unlike [`parse_walkable_waypoints`], this includes blocked (`Bank+Unpass`) tiles and entries
/// with `Waypoints = 0`. Used by the comprehensive OTB speed patcher to ensure `ITEM_ATTR_SPEED`
/// matches `objects.srv` exactly for every BANK ground type — including unwalkable tiles where
/// 772 `FillMap` skips them but OTB should still store the correct value for parity.
// C++ reference: `cract.cc` `TShortway::FillMap` (uses BANK && !UNPASS && Waypoints).
pub fn parse_all_bank_waypoints(path: &Path) -> Result<Vec<ObjectsSrvGroundWaypoints>> {
    let text = std::fs::read_to_string(path).map_err(|e| TfsRustError::Content {
        file: path.to_string_lossy().into_owned(),
        message: e.to_string(),
    })?;
    let mut out = Vec::new();
    for block in text.split("\nTypeID") {
        let block = if block.starts_with("TypeID") {
            block.to_string()
        } else {
            format!("TypeID{block}")
        };
        let Some(type_id) = parse_type_id(&block) else {
            continue;
        };
        let (bank, _unpass) = {
            let f = parse_flags(&block);
            (f.bank, f.unpass)
        };
        if !bank {
            continue;
        }
        // Waypoints defaults to 0 when absent (772 `objects.cc` `getAttribute` returns 0).
        let waypoints = parse_waypoints(&block).unwrap_or(0);
        out.push(ObjectsSrvGroundWaypoints {
            type_id,
            waypoints: waypoints.max(0) as u16,
        });
    }
    Ok(out)
}

/// Parse **every** `objects.srv` type block into `TypeID → {flags, waypoints}`.
///
/// The OTB join key is a row's **`client_id`** (== the shared 772 `TypeID`), *not* `server_id`.
/// The `.sec`→OTBM conversion remaps `TypeID → server_id` via `client_id` (verified: ~940/1024
/// tiles per sector are `client_to_server[sec_id]`), so terrain Waypoints/flags for an OTB row
/// come from `objects.srv[client_id]`. Waypoints defaults to `-1` when absent (772 `getAttribute`
/// returns 0 → invalid; `<= 0` is treated as blocked, matching `cract.cc:95-98`).
pub fn parse_all_types(path: &Path) -> Result<HashMap<u16, ObjectsSrvTypeFlags>> {
    let text = std::fs::read_to_string(path).map_err(|e| TfsRustError::Content {
        file: path.to_string_lossy().into_owned(),
        message: e.to_string(),
    })?;
    let mut out = HashMap::new();
    for block in text.split("\nTypeID") {
        let block = if block.starts_with("TypeID") {
            block.to_string()
        } else {
            format!("TypeID{block}")
        };
        let Some(type_id) = parse_type_id(&block) else {
            continue;
        };
        out.insert(
            type_id,
            ObjectsSrvTypeFlags {
                type_id,
                flags: parse_flags(&block),
                waypoints: parse_waypoints(&block).unwrap_or(-1),
                name: parse_name(&block),
            },
        );
    }
    Ok(out)
}

/// Apply 772 `Waypoints` onto OTB `ItemType::speed` (`ITEM_ATTR_SPEED`) for ground tiles.
///
/// Maps 772 `TypeID` → OTB `server_id` (direct id or `client_id` match). Skips unknown ids.
/// Returns `(patched, skipped_unknown)`.
pub fn apply_waypoints_to_item_speeds(
    items: &mut HashMap<u16, ItemType>,
    entries: &[ObjectsSrvGroundWaypoints],
) -> (u32, u32) {
    let mut patched = 0u32;
    let mut skipped = 0u32;
    for entry in entries {
        let Some(server_id) = resolve_server_id(entry.type_id, items) else {
            skipped += 1;
            continue;
        };
        let Some(item) = items.get_mut(&server_id) else {
            skipped += 1;
            continue;
        };
        if item.speed != entry.waypoints {
            item.speed = entry.waypoints;
            patched += 1;
        }
    }
    (patched, skipped)
}

/// Resolve 772 `TypeID` to OTB `server_id` (direct or via `client_id`).
pub fn resolve_server_id_for_patch(type_id: u16, items: &HashMap<u16, ItemType>) -> Option<u16> {
    resolve_server_id(type_id, items)
}

fn resolve_server_id(type_id: u16, items: &HashMap<u16, ItemType>) -> Option<u16> {
    // 772 `objects.srv` `TypeID` == OTB **`client_id`** (the shared 772 sprite/type id). The
    // `.sec`→OTBM conversion stores `server_id = client_to_server[TypeID]`, so resolve by
    // `client_id` first (smallest server_id wins for duplicate client ids, mirroring the C++
    // `clientIdToServerIdMap` "first wins"). Only fall back to a direct `server_id` match for
    // aligned rows (`server_id == client_id`) that have no distinct client entry.
    if let Some(server_id) = items
        .values()
        .filter(|it| it.client_id == type_id)
        .map(|it| it.server_id)
        .min()
    {
        return Some(server_id);
    }
    items.contains_key(&type_id).then_some(type_id)
}

fn parse_type_id(block: &str) -> Option<u16> {
    for line in block.lines() {
        let line = line.trim();
        let rest = line.strip_prefix("TypeID")?.trim();
        let rest = rest.strip_prefix('=')?.trim();
        let rest = rest.strip_prefix('#').unwrap_or(rest).trim();
        if let Ok(id) = rest.parse::<u16>() {
            return Some(id);
        }
    }
    None
}

fn parse_name(block: &str) -> String {
    for line in block.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix("Name") else {
            continue;
        };
        let rest = rest.trim().strip_prefix('=').unwrap_or(rest).trim();
        if let Some(inner) = rest.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
            return inner.to_string();
        }
        // Name = foo (unquoted rare)
        if !rest.is_empty() {
            return rest.trim_matches('"').to_string();
        }
    }
    String::new()
}

/// Parsed `objects.srv` `Flags = {…}` for one type block.
#[derive(Debug, Clone, Copy)]
pub struct ObjectsSrvFlags {
    pub bank: bool,
    pub unpass: bool,
    pub distuse: bool,
}

fn parse_flags(block: &str) -> ObjectsSrvFlags {
    let flags: Vec<&str> = block
        .lines()
        .find(|l| l.contains("Flags"))
        .and_then(|l| l.split('{').nth(1))
        .and_then(|s| s.split('}').next())
        .map(|s| s.split(',').map(str::trim).collect())
        .unwrap_or_default();
    ObjectsSrvFlags {
        bank: flags.contains(&"Bank"),
        unpass: flags.contains(&"Unpass"),
        distuse: flags.contains(&"DistUse"),
    }
}

/// One `objects.srv` type with parsed flags (+ optional Waypoints).
#[derive(Debug, Clone)]
pub struct ObjectsSrvTypeFlags {
    pub type_id: u16,
    pub flags: ObjectsSrvFlags,
    pub waypoints: i32,
    /// `Name = "…"` from the type block (empty if absent).
    pub name: String,
}

/// Parse every `objects.srv` type that has `Unpass`, with Bank/Waypoints context.
pub fn parse_unpass_types(path: &Path) -> Result<Vec<ObjectsSrvTypeFlags>> {
    let text = std::fs::read_to_string(path).map_err(|e| TfsRustError::Content {
        file: path.to_string_lossy().into_owned(),
        message: e.to_string(),
    })?;
    let mut out = Vec::new();
    for block in text.split("\nTypeID") {
        let block = if block.starts_with("TypeID") {
            block.to_string()
        } else {
            format!("TypeID{block}")
        };
        let Some(type_id) = parse_type_id(&block) else {
            continue;
        };
        let flags = parse_flags(&block);
        if !flags.unpass {
            continue;
        }
        out.push(ObjectsSrvTypeFlags {
            type_id,
            flags,
            waypoints: parse_waypoints(&block).unwrap_or(0),
            name: parse_name(&block),
        });
    }
    Ok(out)
}

/// Parse every `objects.srv` `TypeID` that has the `Unpass` flag.
///
/// Used offline to set OTB `FLAG_BLOCK_SOLID` (`docs/772_OTB_OBJECTS_SRV_FLAG_MAPPING.md`).
// C++ reference: `enums.hh` `UNPASS`; `cract.cc` `TShortway::FillMap` (`BANK && !UNPASS`).
pub fn parse_unpass_type_ids(path: &Path) -> Result<Vec<u16>> {
    Ok(parse_unpass_types(path)?
        .into_iter()
        .map(|t| t.type_id)
        .collect())
}

/// Parse `Waypoints=N` from one `objects.srv` type block (`Attributes = {Waypoints=150}`).
pub fn parse_waypoints(block: &str) -> Option<i32> {
    block.lines().find_map(|l| {
        let rest = l.split("Waypoints=").nth(1)?;
        let digits: String = rest
            .chars()
            .skip_while(|c| !c.is_ascii_digit())
            .take_while(|c| c.is_ascii_digit())
            .collect();
        digits.parse().ok()
    })
}

/// Load overlay from `path` and merge into `items`. Logs summary.
pub fn overlay_otb_speeds_from_objects_srv(
    items: &mut HashMap<u16, ItemType>,
    path: &Path,
) -> Result<()> {
    let entries = parse_walkable_waypoints(path)?;
    let (patched, skipped) = apply_waypoints_to_item_speeds(items, &entries);
    info!(
        file = %path.display(),
        walkable_types = entries.len(),
        patched,
        skipped_unknown = skipped,
        "applied objects.srv Waypoints to OTB ITEM_ATTR_SPEED"
    );
    Ok(())
}

/// Parse all `DistUse` entries from 772 `objects.srv` (`enums.hh:215`).
pub fn parse_distuse_types(path: &Path) -> Result<Vec<ObjectsSrvDistUse>> {
    let text = std::fs::read_to_string(path).map_err(|e| TfsRustError::Content {
        file: path.to_string_lossy().into_owned(),
        message: e.to_string(),
    })?;
    let mut out = Vec::new();
    for block in text.split("\nTypeID") {
        let block = if block.starts_with("TypeID") {
            block.to_string()
        } else {
            format!("TypeID{block}")
        };
        let Some(type_id) = parse_type_id(&block) else {
            continue;
        };
        let flags = parse_flags(&block);
        if flags.distuse {
            out.push(ObjectsSrvDistUse { type_id });
        }
    }
    Ok(out)
}

/// Apply 772 `DistUse` flag onto OTB `ItemType::distuse` for all matching types.
/// Maps 772 `TypeID` → OTB `server_id` (direct id or `client_id` match). Skips unknown ids.
pub fn apply_distuse_to_items(
    items: &mut HashMap<u16, ItemType>,
    entries: &[ObjectsSrvDistUse],
) -> (u32, u32) {
    let mut patched = 0u32;
    let mut skipped = 0u32;
    for entry in entries {
        let Some(server_id) = resolve_server_id(entry.type_id, items) else {
            skipped += 1;
            continue;
        };
        let Some(item) = items.get_mut(&server_id) else {
            skipped += 1;
            continue;
        };
        if !item.distuse {
            item.distuse = true;
            patched += 1;
        }
    }
    (patched, skipped)
}

/// Overlay `DistUse` flags from `objects.srv` onto `items`. Logs summary.
pub fn overlay_distuse_from_objects_srv(
    items: &mut HashMap<u16, ItemType>,
    path: &Path,
) -> Result<()> {
    let entries = parse_distuse_types(path)?;
    let (patched, skipped) = apply_distuse_to_items(items, &entries);
    info!(
        file = %path.display(),
        distuse_types = entries.len(),
        patched,
        skipped_unknown = skipped,
        "applied objects.srv DistUse flag to OTB ItemType"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn repo_objects_srv() -> Option<PathBuf> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        reference_objects_srv_under(root)
    }

    #[test]
    fn parse_waypoints_from_attributes_brace() {
        let block = "TypeID      = 102\nFlags       = {Bank,Unmove}\nAttributes  = {Waypoints=150}";
        assert_eq!(parse_waypoints(block), Some(150));
    }

    #[test]
    fn resolve_typeid_maps_via_client_id() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let otb = root.join("data/items/items.otb");
        if !otb.is_file() {
            return;
        }
        let items = crate::otb::OtbLoader::load_from_file(&otb).expect("otb");
        // 772 `TypeID` == OTB `client_id`. The `.sec`→OTBM conversion remaps `TypeID → server_id`
        // via `client_id`, so a TypeID must resolve to the OTB row whose `client_id` matches it.
        for type_id in [102u16, 397, 4398] {
            let sid = resolve_server_id_for_patch(type_id, &items).expect("resolvable");
            let row = items.get(&sid).expect("row exists");
            let expected = items
                .values()
                .filter(|it| it.client_id == type_id)
                .map(|it| it.server_id)
                .min()
                .unwrap_or(type_id);
            assert_eq!(sid, expected, "TypeID {type_id} must resolve via client_id");
            assert!(
                row.client_id == type_id || (row.server_id == type_id && expected == type_id),
                "resolved row for TypeID {type_id} has client_id {} / server_id {}",
                row.client_id,
                row.server_id
            );
        }
        // Concrete: rock soil TypeID 4398 → OTB row with client_id 4398 (server 4409), not server 4398.
        let sid = resolve_server_id_for_patch(4398, &items).expect("4398");
        assert_eq!(items.get(&sid).map(|it| it.client_id), Some(4398));
    }

    #[test]
    fn parse_grass_dirt_sand_waypoints() {
        let Some(path) = repo_objects_srv() else {
            return;
        };
        let entries = parse_walkable_waypoints(&path).expect("parse");
        let wp = |id: u16| {
            entries
                .iter()
                .find(|e| e.type_id == id)
                .map(|e| e.waypoints)
        };
        assert_eq!(wp(102), Some(150));
        assert_eq!(wp(103), Some(110));
        assert_eq!(wp(104), Some(160));
        assert_eq!(wp(107), Some(120));
    }

    #[test]
    fn overlay_fixes_stairs_mismatch_when_present() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let Some(objects) = reference_objects_srv_under(root.clone()) else {
            return;
        };
        let otb = root.join("data/items/items.otb");
        if !otb.is_file() {
            return;
        }
        let mut items = crate::otb::OtbLoader::load_from_file(&otb).expect("otb");
        // TypeID 434 (stairs, Waypoints=100) resolves via client_id to its OTB server row.
        let Some(sid) = resolve_server_id_for_patch(434, &items) else {
            return;
        };
        let before = items.get(&sid).map(|i| i.speed);
        overlay_otb_speeds_from_objects_srv(&mut items, &objects).expect("overlay");
        assert_eq!(
            items.get(&sid).map(|i| i.speed),
            Some(100),
            "stairs TypeID 434 -> server {sid} speed; before was {before:?}"
        );
    }
}
