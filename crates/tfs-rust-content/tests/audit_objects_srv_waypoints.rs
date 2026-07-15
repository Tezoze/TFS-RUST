//! Audit `items.otb` `ITEM_ATTR_SPEED` vs `objects.srv` BANK `Waypoints`, keyed by **`client_id`**.
//!
//! Run: `cargo test -p tfs-rust-content --test audit_objects_srv_waypoints -- --nocapture`
//!
//! Terminology:
//! - **Waypoints** — BANK attribute; drives `TShortway` and `NotifyGo` (`cract.cc`).
//! - **Creature GetSpeed()** — separate movement stat (not audited here).
//! - TFS **ITEM_ATTR_SPEED** — OTB field Rust reads via `ground_speed_for_item` for 772 terrain cost.
//! - **items.xml `speed`** — equipment bonus only.
//! - SEC/OTBM — ground item id per tile only; no per-tile Waypoints byte.
//!
//! Join model (verified against `.sec` vs OTBM, ~940/1024 tiles/sector): the `.sec`→OTBM
//! conversion remaps 772 `TypeID → server_id` via `client_id`, so an OTB row's true 772 identity
//! is `objects.srv[client_id]`. This audit iterates **OTB rows** and validates each row's `speed`
//! against `objects.srv[client_id]` — the direction the runtime actually uses. Auditing by
//! `server_id` (the old bug) validated the wrong rows against themselves.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn objects_srv_path(root: &Path) -> PathBuf {
    for name in ["classic-772", "cipsoft-772"] {
        let path = root
            .join("reference")
            .join(name)
            .join("runtime/dat/objects.srv");
        if path.is_file() {
            return path;
        }
    }
    root.join("reference/classic-772/runtime/dat/objects.srv")
}
const ITEMS_OTB: &str = "data/items/items.otb";

#[derive(Debug, Clone, Copy)]
struct SrvType {
    waypoints: i32,
    bank: bool,
    unpass: bool,
}

fn repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Parse `objects.srv` into `TypeID -> {waypoints, bank, unpass}`.
fn parse_objects_srv(path: &Path) -> HashMap<u16, SrvType> {
    let text = std::fs::read_to_string(path).expect("objects.srv");
    let mut out = HashMap::new();
    for block in text.split("\nTypeID") {
        let block = if block.starts_with("TypeID") {
            block.to_string()
        } else {
            format!("TypeID{block}")
        };
        let type_id = block
            .lines()
            .find_map(|l| l.trim().strip_prefix("TypeID").map(str::trim))
            .and_then(|s| s.strip_prefix('='))
            .map(str::trim)
            .and_then(|s| s.strip_prefix('#').map(str::trim).or(Some(s)))
            .and_then(|s| s.parse::<u16>().ok());
        let Some(type_id) = type_id else {
            continue;
        };
        let flags_line = block.lines().find(|l| l.contains("Flags"));
        let flags: Vec<&str> = flags_line
            .and_then(|l| l.split('{').nth(1))
            .and_then(|s| s.split('}').next())
            .map(|s| s.split(',').map(str::trim).collect())
            .unwrap_or_default();
        out.insert(
            type_id,
            SrvType {
                waypoints: tfs_rust_content::objects_srv::parse_waypoints(&block).unwrap_or(-1),
                bank: flags.contains(&"Bank"),
                unpass: flags.contains(&"Unpass"),
            },
        );
    }
    out
}

/// 772 identity of an OTB row = `objects.srv[client_id]` (fallback `server_id` when client is 0).
fn srv_identity(
    it: &tfs_rust_content::otb::ItemType,
    srv: &HashMap<u16, SrvType>,
) -> Option<SrvType> {
    if it.client_id != 0 {
        if let Some(t) = srv.get(&it.client_id) {
            return Some(*t);
        }
    }
    srv.get(&it.server_id).copied()
}

#[test]
fn audit_objects_srv_waypoints_vs_otb() {
    let root = repo_root();
    let objects = objects_srv_path(&root);
    let otb_path = root.join(ITEMS_OTB);
    let items_xml = root.join("data/items/items.xml");
    assert!(objects.is_file(), "missing {}", objects.display());
    assert!(otb_path.is_file(), "missing {ITEMS_OTB}");

    let srv = parse_objects_srv(&objects);
    let by_server = tfs_rust_content::otb::OtbLoader::load_from_file(&otb_path).expect("otb");

    let mut walkable_match = 0u32;
    let mut walkable_mismatch = 0u32;
    let mut blocked_match = 0u32; // srv blocked bank -> OTB speed must be 0
    let mut blocked_mismatch = 0u32;
    let mut ground_no_srv_identity = 0u32;
    let mut problems: Vec<String> = Vec::new();

    for it in by_server.values() {
        if it.group != tfs_rust_content::otb::ItemType::GROUP_GROUND {
            continue; // only ground rows carry terrain Waypoints
        }
        let Some(t) = srv_identity(it, &srv) else {
            ground_no_srv_identity += 1;
            continue;
        };
        if !t.bank {
            continue;
        }
        let walkable = !t.unpass && t.waypoints > 0;
        if walkable {
            if it.speed as i32 == t.waypoints {
                walkable_match += 1;
            } else {
                walkable_mismatch += 1;
                if problems.len() < 30 {
                    problems.push(format!(
                        "  WALKABLE server={:5} client={:5} srv_wp={:3} otb_speed={:3} [MISMATCH]",
                        it.server_id, it.client_id, t.waypoints, it.speed
                    ));
                }
            }
        } else {
            // Blocked bank (mountain/water): OTB speed must be 0 so FillMap treats it as -1.
            if it.speed == 0 {
                blocked_match += 1;
            } else {
                blocked_mismatch += 1;
                if problems.len() < 30 {
                    problems.push(format!(
                        "  BLOCKED  server={:5} client={:5} srv_wp={:3} otb_speed={:3} [SHOULD BE 0]",
                        it.server_id, it.client_id, t.waypoints, it.speed
                    ));
                }
            }
        }
    }

    println!("=== items.otb ITEM_ATTR_SPEED vs objects.srv[client_id] Waypoints ===\n");
    println!("OTB items loaded: {}", by_server.len());
    println!("Walkable BANK ground rows:  match={walkable_match}  mismatch={walkable_mismatch}");
    println!("Blocked  BANK ground rows:  match={blocked_match}  mismatch={blocked_mismatch}");
    println!("Ground rows with no objects.srv identity (TVP-only): {ground_no_srv_identity}");
    println!("Re-patch if needed: cargo run -p tfs-rust-content --bin patch-otb-waypoints");
    if !problems.is_empty() {
        println!("\nSample problems:");
        for line in &problems {
            println!("{line}");
        }
    }

    // Canonical sanity via the runtime accessor.
    let db =
        tfs_rust_content::items::ItemDatabase::load(&otb_path, &items_xml).expect("items load");
    for (id, label) in [(102u16, "grass"), (103, "dirt"), (104, "sand")] {
        println!(
            "  canonical {label} server {id} -> rust effective wp {}",
            db.ground_speed_for_item(id)
        );
    }

    // The bug this audit exists to catch: walkable rock soil stored as wp0 (monster-impassable).
    assert_eq!(
        walkable_mismatch, 0,
        "walkable BANK ground rows must have OTB speed == objects.srv[client_id] Waypoints"
    );
    assert_eq!(
        blocked_mismatch, 0,
        "blocked BANK ground rows (Unpass / wp<=0) must have OTB speed 0"
    );
}
