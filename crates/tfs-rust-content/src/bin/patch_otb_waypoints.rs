//! One-shot: patch `items.otb` from 772 `objects.srv`.
//!
//! 1. `ITEM_ATTR_SPEED` ← BANK `Waypoints` (incl. Unpass / wp=0; zeroes stale non-BANK).
//! 2. Non-Bank `Unpass` → `FLAG_BLOCK_SOLID` (trees/walls for MovePossible).
//! 3. Bank+Unpass **`"a mountain"`** only → **clear** `FLAG_BLOCK_SOLID` (OTBM cliff rock-soil;
//!    dirt/earth/stone walls keep solid so players cannot pathfind through them).
//! 4. Passable ground speed 0 (Clip-as-ground) → Waypoints 150.
//!
//! Usage (repo root):
//!   cargo run -p tfs-rust-content --bin patch-otb-waypoints
//!   cargo run -p tfs-rust-content --bin patch-otb-waypoints -- --dry-run
//!   cargo run -p tfs-rust-content --bin patch-otb-waypoints -- --walkable-only
//!   cargo run -p tfs-rust-content --bin patch-otb-waypoints -- --flags-only

use std::collections::HashMap;
use std::path::PathBuf;
use tfs_rust_content::objects_srv::resolve_objects_srv_path;
use tfs_rust_content::otb_patch::{
    build_all_speed_patches, build_bank_unpass_clear_solid, build_passable_zero_speed_defaults,
    build_speed_patches, build_unpass_block_solid_ors, patch_file_speeds_and_flags,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dry_run = std::env::args().any(|a| a == "--dry-run");
    let walkable_only = std::env::args().any(|a| a == "--walkable-only");
    let flags_only = std::env::args().any(|a| a == "--flags-only");
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let otb = repo_root.join("data/items/items.otb");
    let objects = resolve_objects_srv_path()
        .or_else(|| {
            let p = repo_root.join("reference/cipsoft-772/runtime/dat/objects.srv");
            p.is_file().then_some(p)
        })
        .ok_or("objects.srv not found (set TFS_OBJECTS_SRV or TFS_REFERENCE_DIR)")?;

    let mut speeds = if flags_only {
        HashMap::new()
    } else if walkable_only {
        build_speed_patches(&objects, &otb)?
    } else {
        build_all_speed_patches(&objects, &otb)?
    };

    let (flag_ors, flag_clears) = if walkable_only && !flags_only {
        (HashMap::new(), HashMap::new())
    } else {
        let ors = build_unpass_block_solid_ors(&objects, &otb)?;
        let clears = build_bank_unpass_clear_solid(&objects, &otb)?;
        (ors, clears)
    };

    if !walkable_only || flags_only {
        // Clip-as-ground defaults (merge after BANK speeds so Unpass banks keep wp0).
        for (sid, wp) in build_passable_zero_speed_defaults(&objects, &otb)? {
            speeds.entry(sid).or_insert(wp);
        }
    }

    println!(
        "objects.srv: {}  |  OTB: {}\n  speed patches: {}\n  Unpass→blockSolid: {}\n  mountain bank clear solid: {}{}",
        objects.display(),
        otb.display(),
        speeds.len(),
        flag_ors.len(),
        flag_clears.len(),
        if flags_only {
            " (flags + clip defaults)"
        } else if walkable_only {
            " (walkable speeds only)"
        } else {
            ""
        }
    );
    if dry_run {
        println!(
            "dry-run: would patch {} speed ids, {} flag-or, {} flag-clear",
            speeds.len(),
            flag_ors.len(),
            flag_clears.len()
        );
        return Ok(());
    }

    let backup = otb.with_extension("otb.bak");
    if !backup.is_file() {
        std::fs::copy(&otb, &backup)?;
        println!("backup: {}", backup.display());
    } else {
        println!("backup exists (unchanged): {}", backup.display());
    }

    let (speed_n, flag_n) =
        patch_file_speeds_and_flags(&otb, &speeds, &flag_ors, &flag_clears)?;
    println!(
        "patched {speed_n} speed nodes, {flag_n} flag nodes — reload server to pick up changes"
    );
    Ok(())
}
