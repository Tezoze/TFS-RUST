//! Correlate 772 `objects.srv` flags with OTB `ItemType` fields.
//!
//! Run: `cargo test -p tfs-rust-content audit_objects_srv_flag_correlation -- --nocapture`
//!
//! See `docs/772_OTB_OBJECTS_SRV_FLAG_MAPPING.md`.

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

#[derive(Debug)]
struct SrvType {
    type_id: u16,
    flags: Vec<String>,
}

fn parse_objects_srv(path: &Path) -> Vec<SrvType> {
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
        let flags: Vec<String> = block
            .lines()
            .find(|l| l.contains("Flags"))
            .and_then(|l| l.split('{').nth(1))
            .and_then(|s| s.split('}').next())
            .map(|s| {
                s.split(',')
                    .map(str::trim)
                    .filter(|f| !f.is_empty())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();
        out.push(SrvType { type_id, flags });
    }
    out
}

fn lookup_otb<'a>(
    type_id: u16,
    by_server: &'a HashMap<u16, tfs_rust_content::otb::ItemType>,
) -> Option<&'a tfs_rust_content::otb::ItemType> {
    let server_id = tfs_rust_content::objects_srv::resolve_server_id_for_patch(type_id, by_server)?;
    by_server.get(&server_id)
}

fn has_flag(flags: &[String], name: &str) -> bool {
    flags.iter().any(|f| f == name)
}

#[test]
fn audit_objects_srv_flag_correlation() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let objects = objects_srv_path(&root);
    let otb_path = root.join("data/items/items.otb");
    let items_xml = root.join("data/items/items.xml");
    assert!(objects.is_file(), "missing {}", objects.display());
    assert!(otb_path.is_file(), "missing items.otb");

    let srv_types = parse_objects_srv(&objects);
    let by_server = tfs_rust_content::otb::OtbLoader::load_from_file(&otb_path).expect("otb");
    let db = tfs_rust_content::items::ItemDatabase::load(&otb_path, &items_xml).expect("items");

    let mut bank_ground_match = 0u32;
    let mut bank_ground_mismatch = 0u32;
    let mut unpass_solid_match = 0u32;
    let mut unpass_solid_mismatch = 0u32;
    let mut unmove_nomove_match = 0u32;
    let mut unmove_nomove_mismatch = 0u32;
    let mut missing_otb = 0u32;

    for entry in &srv_types {
        let Some(otb) = lookup_otb(entry.type_id, &by_server) else {
            missing_otb += 1;
            continue;
        };
        let db_type = db.items.get(&otb.server_id);

        if has_flag(&entry.flags, "Bank") {
            if otb.is_ground_tile() {
                bank_ground_match += 1;
            } else {
                bank_ground_mismatch += 1;
            }
        }

        if has_flag(&entry.flags, "Unpass") {
            let solid = db_type
                .map(|t| t.block_solid())
                .unwrap_or(otb.block_solid());
            if solid {
                unpass_solid_match += 1;
            } else {
                unpass_solid_mismatch += 1;
            }
        }

        if has_flag(&entry.flags, "Unmove") {
            let movable = db_type.map(|t| t.moveable()).unwrap_or(otb.moveable());
            if !movable {
                unmove_nomove_match += 1;
            } else {
                unmove_nomove_mismatch += 1;
            }
        }
    }

    println!("=== objects.srv flag ↔ OTB correlation ===\n");
    println!("objects.srv types: {}", srv_types.len());
    println!("OTB items loaded: {}", by_server.len());
    println!("TypeIDs missing OTB row: {missing_otb}\n");

    println!("Bank → isGroundTile():");
    println!("  match: {bank_ground_match}");
    println!("  mismatch (Bank but not ground group): {bank_ground_mismatch}\n");

    println!("Unpass → blockSolid (after items.xml):");
    println!("  match: {unpass_solid_match}");
    println!("  mismatch (Unpass but not blockSolid): {unpass_solid_mismatch}\n");

    println!("Unmove → !moveable (after items.xml):");
    println!("  match: {unmove_nomove_match}");
    println!("  mismatch (Unmove but moveable): {unmove_nomove_mismatch}");
}
