//! Second literal pass. Does not touch `id =`, corpses, or the OTBM.

use std::fs;
use std::path::{Path, PathBuf};

use tfs_rust_content::client_id_rewrite::{
    rewrite_mapped_ints_skip_lines, rewrite_named_ids, rewrite_shop_type_values, ClientIdMap,
};
use tfs_rust_content::otb::OtbLoader;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let live = OtbLoader::load_from_file(&root.join("data/items/items.otb")).expect("live otb");
    let map = ClientIdMap::from_items(&live);
    for id in [
        1487u16, 1488, 1489, 1490, 1491, 1492, 1493, 1494, 1495, 1496, 1497, 1498, 1499, 1500,
        1501, 1502, 1503, 1504, 2721, 2148, 2152, 2160, 1987,
    ] {
        println!("{id} -> {:?}", map.client_of(id));
    }

    let mut files = 0usize;
    let mut replacements = 0usize;
    let items_lua = root.join("data/items/#items.lua");
    apply_named(
        &items_lua,
        &map,
        &[b"fromId", b"toId", b"decayTo"],
        &mut files,
        &mut replacements,
    );
    walk_npc(&root.join("data/npc"), &map, &mut files, &mut replacements);
    for rel in [
        "data/defs/doors.lua",
        "data/defs/tiles.lua",
        "data/scripts/actions/other/transforms.lua",
    ] {
        apply_ints(&root.join(rel), &map, |_| false, &mut files, &mut replacements);
    }
    apply_ints(
        &root.join("data/defs/tools.lua"),
        &map,
        |line| {
            line.contains("schema")
                || line.contains("puzzleSwitch")
                || line.contains("sandstoneWall")
                || line.contains("sandHole =")
                || line.contains("pickHole =")
                || line.contains("destroyableStone")
                || line.contains("blockingTile")
                || line.contains("timerSecs")
                || line.contains("spawnChance")
                || line.contains("questObjectAid")
                || line.contains("postMoveEffectId")
        },
        &mut files,
        &mut replacements,
    );
    println!("pass2 files {files} replacements {replacements}");
    write_sql_migration(&root, &map);
}

fn walk_npc(dir: &Path, map: &ClientIdMap, files: &mut usize, replacements: &mut usize) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_npc(&path, map, files, replacements);
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("lua") {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let (mid, a) = rewrite_shop_type_values(&text, map);
        let (out, b) = rewrite_named_ids(&mid, map, &[b"item", b"count"]);
        let n = a.replacements + b.replacements;
        if out != text {
            fs::write(&path, out).expect("npc");
            *files += 1;
            *replacements += n;
        }
    }
}

fn apply_named(
    path: &Path,
    map: &ClientIdMap,
    markers: &[&[u8]],
    files: &mut usize,
    replacements: &mut usize,
) {
    let text = fs::read_to_string(path).expect("read");
    let (out, stats) = rewrite_named_ids(&text, map, markers);
    if out != text {
        fs::write(path, out).expect("write");
        *files += 1;
        *replacements += stats.replacements;
    }
}

fn apply_ints(
    path: &Path,
    map: &ClientIdMap,
    skip: impl Fn(&str) -> bool,
    files: &mut usize,
    replacements: &mut usize,
) {
    let text = fs::read_to_string(path).expect("read");
    let (out, stats) = rewrite_mapped_ints_skip_lines(&text, map, skip);
    if out != text {
        fs::write(path, out).expect("write");
        *files += 1;
        *replacements += stats.replacements;
    }
}

fn write_sql_migration(root: &Path, map: &ClientIdMap) {
    let mut pairs: Vec<(u16, u16)> = Vec::new();
    // ClientIdMap does not expose the table. Rebuild from the same OTB the caller loaded
    // by walking printed ids is not enough — read the file again.
    let live = OtbLoader::load_from_file(&root.join("data/items/items.otb")).expect("otb");
    for (sid, it) in &live {
        let cid = if it.client_id == 0 { *sid } else { it.client_id };
        if cid != *sid {
            pairs.push((*sid, cid));
        }
    }
    pairs.sort_unstable();
    let path = root.join("crates/tfs-rust-db/migrations/20260926000000_client_item_ids.sql");
    let mut sql = String::from(
        "-- Remap stored itemtype columns onto the client id. Do not apply by hand from the cutover tool.\n",
    );
    let tables = [
        "player_items",
        "player_depotitems",
        "player_inboxitems",
        "player_storeinboxitems",
        "market_offers",
        "market_history",
    ];
    for table in tables {
        sql.push_str(&format!("UPDATE `{table}` SET `itemtype` = CASE `itemtype`\n"));
        for (old, new) in &pairs {
            sql.push_str(&format!("  WHEN {old} THEN {new}\n"));
        }
        sql.push_str("  ELSE `itemtype` END\n");
        sql.push_str("WHERE `itemtype` IN (");
        for (i, (old, _)) in pairs.iter().enumerate() {
            if i > 0 {
                sql.push(',');
            }
            sql.push_str(&old.to_string());
        }
        sql.push_str(");\n\n");
    }
    fs::write(&path, sql).expect("migration");
    println!("migration {} pairs {}", path.display(), pairs.len());
    let _ = map;
}
