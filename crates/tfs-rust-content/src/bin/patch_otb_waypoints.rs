//! One-shot: patch `items.otb` `ITEM_ATTR_SPEED` from CipSoft `objects.srv` `Waypoints`.
//!
//! Usage (repo root):
//!   cargo run -p tfs-rust-content --bin patch-otb-waypoints
//!   cargo run -p tfs-rust-content --bin patch-otb-waypoints -- --dry-run

use std::path::PathBuf;
use tfs_rust_content::objects_srv::resolve_objects_srv_path;
use tfs_rust_content::otb_patch::{build_speed_patches, patch_file};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dry_run = std::env::args().any(|a| a == "--dry-run");
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let otb = repo_root.join("data/items/items.otb");
    let objects = resolve_objects_srv_path()
        .or_else(|| {
            let p = repo_root.join("reference/cipsoft-772/runtime/dat/objects.srv");
            p.is_file().then_some(p)
        })
        .ok_or("objects.srv not found (set TFS_CIPSOFT_OBJECTS_SRV or TFS_REFERENCE_DIR)")?;

    let patches = build_speed_patches(&objects, &otb)?;
    println!(
        "objects.srv: {}  |  OTB: {}  |  patch entries: {}",
        objects.display(),
        otb.display(),
        patches.len()
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
