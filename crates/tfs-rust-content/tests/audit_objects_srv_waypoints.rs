//! Audit `objects.srv` BANK `Waypoints` vs `items.otb` `ITEM_ATTR_SPEED`.
//!
//! Run: `cargo test -p tfs-rust-content --test audit_objects_srv_waypoints -- --nocapture`
//!
//! Terminology:
//! - **Waypoints** — BANK attribute; drives `TShortway` and `NotifyGo` (`cract.cc`).
//! - **Creature GetSpeed()** — separate movement stat (not audited here).
//! - TFS **ITEM_ATTR_SPEED** — OTB field Rust reads via `ground_speed_for_item` for 772 terrain cost.
//! - **items.xml `speed`** — equipment bonus only.
//! - SEC/OTBM — ground item id per tile only; no per-tile Waypoints byte.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn objects_srv_path(root: &Path) -> PathBuf {
    for name in ["classic-772", "cipsoft-772"] {
        let path = root.join("reference").join(name).join("runtime/dat/objects.srv");
        if path.is_file() {
            return path;
        }
    }
    root.join("reference/classic-772/runtime/dat/objects.srv")
}
const ITEMS_OTB: &str = "data/items/items.otb";
const RUST_DEFAULT_WP: u32 = 150;

#[derive(Debug, Clone)]
struct ObjectsSrvGround {
    type_id: u16,
    name: String,
    waypoints: i32,
    bank: bool,
    unpass: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Status {
    Match,
    Mismatch,
    MissingOtb,
    MissingOtbSpeed,
    Blocked,
    NoWaypoints,
}

fn repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn parse_objects_srv(path: &Path) -> Vec<ObjectsSrvGround> {
    let text = std::fs::read_to_string(path).expect("objects.srv");
    let mut out = Vec::new();
    for block in text.split("\nTypeID") {
        let block = if block.starts_with("TypeID") {
            block.to_string()
        } else {
            format!("TypeID{block}")
        };
        let type_id = block
            .lines()
            .find_map(|l| l.strip_prefix("TypeID").map(str::trim))
            .and_then(|s| s.strip_prefix('='))
            .map(str::trim)
            .and_then(|s| s.strip_prefix('#').map(str::trim).or(Some(s)))
            .and_then(|s| s.parse::<u16>().ok());
        let Some(type_id) = type_id else {
            continue;
        };
        let name = block
            .lines()
            .find_map(|l| l.strip_prefix("Name").map(str::trim))
            .and_then(|s| s.strip_prefix('='))
            .map(str::trim)
            .and_then(|s| s.trim_matches('"').parse::<String>().ok())
            .unwrap_or_default();
        let flags_line = block.lines().find(|l| l.contains("Flags"));
        let flags: Vec<&str> = flags_line
            .and_then(|l| l.split('{').nth(1))
            .and_then(|s| s.split('}').next())
            .map(|s| s.split(',').map(str::trim).collect())
            .unwrap_or_default();
        let bank = flags.iter().any(|f| *f == "Bank");
        let unpass = flags.iter().any(|f| *f == "Unpass");
        let waypoints = block
            .lines()
            .find_map(|l| l.split("Waypoints=").nth(1))
            .and_then(|s| s.split(',').next())
            .and_then(|s| s.trim().parse::<i32>().ok())
            .unwrap_or(-1);
        out.push(ObjectsSrvGround {
            type_id,
            name,
            waypoints,
            bank,
            unpass,
        });
    }
    out
}

fn lookup_otb<'a>(
    type_id: u16,
    by_server: &'a HashMap<u16, tfs_rust_content::otb::ItemType>,
    by_client: &HashMap<u16, u16>,
) -> Option<&'a tfs_rust_content::otb::ItemType> {
    by_client
        .get(&type_id)
        .and_then(|sid| by_server.get(sid))
}

fn rust_effective_wp(otb_speed: u16) -> u32 {
    if otb_speed == 0 {
        RUST_DEFAULT_WP
    } else {
        u32::from(otb_speed)
    }
}

#[test]
fn audit_objects_srv_waypoints_vs_otb() {
    let root = repo_root();
    let objects = objects_srv_path(&root);
    let otb_path = root.join(ITEMS_OTB);
    let items_xml = root.join("data/items/items.xml");
    assert!(objects.is_file(), "missing {}", objects.display());
    assert!(otb_path.is_file(), "missing {ITEMS_OTB}");

    let srv_grounds = parse_objects_srv(&objects);
    let by_server = tfs_rust_content::otb::OtbLoader::load_from_file(&otb_path).expect("otb");
    let by_client: HashMap<u16, u16> = by_server
        .values()
        .map(|it| (it.client_id, it.server_id))
        .collect();

    let mut status_counts: HashMap<Status, u32> = HashMap::new();
    let mut problems: Vec<String> = Vec::new();
    let mut matches = 0u32;

    for g in &srv_grounds {
        if !g.bank {
            continue;
        }
        if g.unpass || g.waypoints <= 0 {
            *status_counts.entry(Status::Blocked).or_default() += 1;
            continue;
        }
        let otb = lookup_otb(g.type_id, &by_server, &by_client);
        let status = match otb {
            None => Status::MissingOtb,
            Some(it) if rust_effective_wp(it.speed) == g.waypoints as u32 => Status::Match,
            Some(it) if it.speed == 0 => Status::MissingOtbSpeed,
            Some(_) => Status::Mismatch,
        };
        if status == Status::Match {
            matches += 1;
        } else if problems.len() < 30 {
            let (srv, spd, eff) = otb.map(|it| {
                (
                    it.server_id,
                    it.speed,
                    rust_effective_wp(it.speed),
                )
            }).unwrap_or((0, 0, RUST_DEFAULT_WP));
            problems.push(format!(
                "  TypeID {:5} {:20} cip={:3} otb_srv={srv:5} otb_spd={spd:3} rust={eff:3} [{status:?}]",
                g.type_id, format!("{:?}", g.name), g.waypoints
            ));
        }
        *status_counts.entry(status).or_default() += 1;
    }

    let ground_with_speed = by_server
        .values()
        .filter(|it| it.group == 1 && it.speed > 0)
        .count();

    println!("=== objects.srv Waypoints vs items.otb ITEM_ATTR_SPEED ===\n");
    println!("OTB items loaded: {}", by_server.len());
    println!("OTB ground types with speed>0: {ground_with_speed}\n");
    println!("Walkable BANK types audited: {}", status_counts.values().sum::<u32>());
    for (st, cnt) in [
        (Status::Match, "match"),
        (Status::Mismatch, "mismatch"),
        (Status::MissingOtb, "missing_otb"),
        (Status::MissingOtbSpeed, "missing_otb_speed"),
        (Status::Blocked, "blocked/unpass/wp0"),
        (Status::NoWaypoints, "no_waypoints"),
    ] {
        if let Some(n) = status_counts.get(&st) {
            println!("  {cnt}: {n}");
        }
    }
    println!("\nExact matches: {matches}");
    println!("Rust default when OTB speed missing: {RUST_DEFAULT_WP}");
    println!("\nSample problems:");
    for line in &problems {
        println!("{line}");
    }

    // Canonical types from items.rs test
    let db = tfs_rust_content::items::ItemDatabase::load(&otb_path, &items_xml)
    .expect("items load");
    for (id, label) in [(102u16, "grass"), (103, "dirt"), (104, "sand")] {
        let wp = db.ground_speed_for_item(id);
        println!("  canonical {label} server {id} -> rust effective wp {wp}");
    }
}
