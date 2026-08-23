//! One-shot XML → Lua-as-data converter for `data/monster/`.
//!
//! Usage (repo root):
//!   cargo run -p tfs-rust-content --bin export-monsters-lua
//!   cargo run -p tfs-rust-content --bin export-monsters-lua -- --data data --out data/monster
//!
//! Reads `monsters.xml` + `monsters/*.xml`. Overwrites existing `*.lua` (including
//! `dragon.lua`). Does not delete XML. Does not switch `MonsterDatabase::load_dir`.

use std::path::PathBuf;
use tfs_rust_content::items::ItemDatabase;
use tfs_rust_content::monster_lua::export_monsters_lua;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut data = PathBuf::from("data");
    let mut out: Option<PathBuf> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--data" => {
                data = PathBuf::from(args.next().ok_or("--data requires a directory")?);
            }
            "--out" => {
                out = Some(PathBuf::from(
                    args.next().ok_or("--out requires a directory")?,
                ));
            }
            other => {
                return Err(format!("unknown argument: {other}").into());
            }
        }
    }
    let out = out.unwrap_or_else(|| data.join("monster"));
    let monster_dir = data.join("monster");
    let otb = data.join("items/items.otb");
    let xml = data.join("items/items.xml");
    let items = ItemDatabase::load(&otb, &xml)?;
    let n = export_monsters_lua(&monster_dir, &out, &items)?;
    println!("wrote {n} monster lua files to {}", out.display());
    Ok(())
}
