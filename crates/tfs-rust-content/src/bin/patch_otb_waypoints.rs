//! One-shot: patch `items.otb` `ITEM_ATTR_SPEED` from 772 `objects.srv` `Waypoints`.
//!
//! Patches **all** BANK ground types (including `Unpass` blocked tiles and `Waypoints=0`)
//! so `ITEM_ATTR_SPEED` matches `objects.srv` exactly. Also zeroes `speed` on non-BANK
//! items that have a stale `ITEM_ATTR_SPEED`.
//!
//! Usage (repo root):
//!   cargo run -p tfs-rust-content --bin patch-otb-waypoints
//!   cargo run -p tfs-rust-content --bin patch-otb-waypoints -- --dry-run
//!   cargo run -p tfs-rust-content --bin patch-otb-waypoints -- --walkable-only

use std::path::PathBuf;
use tfs_rust_content::objects_srv::resolve_objects_srv_path;
use tfs_rust_content::otb_patch::{build_all_speed_patches, build_speed_patches, patch_file};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dry_run = std::env::args().any(|a| a == "--dry-run");
    let walkable_only = std::env::args().any(|a| a == "--walkable-only");
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let otb = repo_root.join("data/items/items.otb");
    let objects = resolve_objects_srv_path()
        .or_else(|| {
            let p = repo_root.join("reference/cipsoft-772/runtime/dat/objects.srv");
            p.is_file().then_some(p)
        })
        .ok_or("objects.srv not found (set TFS_OBJECTS_SRV or TFS_REFERENCE_DIR)")?;

    let patches = if walkable_only {
        build_speed_patches(&objects, &otb)?
    } else {
        build_all_speed_patches(&objects, &otb)?
    };
    println!(
        "objects.srv: {}  |  OTB: {}  |  patch entries: {}{}",
        objects.display(),
        otb.display(),
        patches.len(),
        if walkable_only { " (walkable only)" } else { " (all BANK + non-BANK cleanup)" }
    );
    if dry_run {
        println!("dry-run: would patch {} server ids", patches.len());
        return Ok(());
    }

    let backup = otb.with_extension("otb.bak");
    if !backup.is_file() {
        std::fs::copy(&otb, &backup)?;
        println!("backup: {}", backup.display());
    } else {
        println!("backup exists (unchanged): {}", backup.display());
    }

    let patched_nodes = patch_file(&otb, &patches)?;
    println!("patched {patched_nodes} OTB item nodes — reload server to pick up changes");
    Ok(())
}
