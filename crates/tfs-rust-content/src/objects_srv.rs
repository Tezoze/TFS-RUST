//! 772 `objects.srv` parser — BANK `Waypoints` for OTB `ITEM_ATTR_SPEED` parity.
//!
//! C++ reference: `tibia-game-master/src/cract.cc` `TShortway::FillMap`, `NotifyGo` (`WAYPOINTS`).
//! TFS stores the same per-tile terrain weight in OTB as `ITEM_ATTR_SPEED` (`src/items.cpp`).

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

const REF_772_DIR_NAMES: &[&str] = &["classic-772", "cipsoft-772"];

fn reference_772_objects_srv_under(base: PathBuf) -> Option<PathBuf> {
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
    if let Ok(ref_dir) = std::env::var("TFS_REFERENCE_DIR") {
        if let Some(path) = reference_772_objects_srv_under(PathBuf::from(ref_dir)) {
            return Some(path);
        }
    }
    reference_772_objects_srv_under(PathBuf::from("reference"))
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
        let (bank, unpass) = parse_flags(&block);
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
    items
        .values()
        .find(|it| it.client_id == type_id)
        .map(|it| it.server_id)
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

fn parse_flags(block: &str) -> (bool, bool) {
    let flags: Vec<&str> = block
        .lines()
        .find(|l| l.contains("Flags"))
        .and_then(|l| l.split('{').nth(1))
        .and_then(|s| s.split('}').next())
        .map(|s| s.split(',').map(str::trim).collect())
        .unwrap_or_default();
    let bank = flags.iter().any(|f| *f == "Bank");
    let unpass = flags.iter().any(|f| *f == "Unpass");
    (bank, unpass)
}

fn parse_waypoints(block: &str) -> Option<i32> {
    block
        .lines()
        .find_map(|l| l.split("Waypoints=").nth(1))
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse().ok())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn repo_objects_srv() -> Option<PathBuf> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        reference_772_objects_srv_under(root)
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
        let Some(objects) = reference_772_objects_srv_under(root.clone()) else {
            return;
        };
        let otb = root.join("data/items/items.otb");
        if !otb.is_file() {
            return;
        }
        let mut items = crate::otb::OtbLoader::load_from_file(&otb).expect("otb");
        let before = items.get(&434).map(|i| i.speed);
        overlay_otb_speeds_from_objects_srv(&mut items, &objects).expect("overlay");
        // TypeID 434 stairs — Waypoints=100 (`objects.srv`).
        assert_eq!(items.get(&434).map(|i| i.speed), Some(100), "before was {before:?}");
    }
}
