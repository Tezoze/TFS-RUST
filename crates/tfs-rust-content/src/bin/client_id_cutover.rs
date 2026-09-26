//! Apply the client-id cutover to the datapack, OTBM, and identity OTB.
//!
//! Does not edit Rust. Puzzle server id 4384 is left for the script patch.

use std::fs;
use std::path::{Path, PathBuf};

use tfs_rust_content::client_id_census::LOCKED_SURVIVORS;
use tfs_rust_content::client_id_rewrite::{
    rewrite_item_literals, rewrite_otbm_bytes, rewrite_sql_itemtype_selects, ClientIdMap,
};
use tfs_rust_content::otb::OtbLoader;
use tfs_rust_content::otb_identity::dedup_identity_otb;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let live_path = root.join("data/items/items.otb");
    let live = OtbLoader::load_from_file(&live_path).expect("live otb");
    let map = ClientIdMap::from_items(&live);
    for (server, label) in [
        (2148u16, "gold"),
        (2152, "platinum"),
        (2160, "crystal"),
        (1987, "bag"),
        (1487, "fire pvp"),
        (1492, "fire persistent"),
    ] {
        println!(
            "{label} {server} -> {:?}",
            map.client_of(server)
        );
    }

    let identity_path = root.join("data/items/clientid_output/items.otb");
    let identity = fs::read(&identity_path).expect("identity otb");
    let deduped = dedup_identity_otb(&identity, &live, LOCKED_SURVIVORS).expect("dedup");
    fs::write(&identity_path, &deduped).expect("write otb");
    println!("identity otb {} -> {} bytes", identity.len(), deduped.len());

    let mut files = 0usize;
    let mut replacements = 0usize;
    let mut puzzle = 0usize;
    walk_lua(&root.join("data"), &map, &mut files, &mut replacements, &mut puzzle);
    let seed = root.join("docker/seed_dev_account.sql");
    if seed.exists() {
        rewrite_sql_file(&seed, &map, &mut files, &mut replacements);
    }
    println!("lua/sql files {files} replacements {replacements} puzzle-4384 left {puzzle}");

    let otbm_path = root.join("data/world/forgotten.otbm");
    let otbm = fs::read(&otbm_path).expect("otbm");
    let rewritten = rewrite_otbm_bytes(&otbm, &map).expect("rewrite otbm");
    fs::write(&otbm_path, &rewritten).expect("write otbm");
    println!("otbm {} -> {} bytes", otbm.len(), rewritten.len());
}

fn walk_lua(
    dir: &Path,
    map: &ClientIdMap,
    files: &mut usize,
    replacements: &mut usize,
    puzzle: &mut usize,
) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.file_name().and_then(|s| s.to_str()) == Some("clientid_output") {
            continue;
        }
        if path.is_dir() {
            walk_lua(&path, map, files, replacements, puzzle);
            continue;
        }
        let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
            continue;
        };
        if ext != "lua" {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let (out, stats) = rewrite_item_literals(&text, map);
        if out != text {
            fs::write(&path, out).expect("write lua");
            *files += 1;
            *replacements += stats.replacements;
            *puzzle += stats.puzzle_lever_left;
        }
    }
}

fn rewrite_sql_file(path: &Path, map: &ClientIdMap, files: &mut usize, replacements: &mut usize) {
    let text = fs::read_to_string(path).expect("sql");
    let (out, stats) = rewrite_sql_itemtype_selects(&text, map);
    if out != text {
        fs::write(path, out).expect("write sql");
        *files += 1;
        *replacements += stats.replacements;
    }
}
