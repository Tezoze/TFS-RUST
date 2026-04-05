use crate::groups::GroupDatabase;
use crate::items::ItemDatabase;
use crate::monsters::MonsterDatabase;
use crate::mounts::MountDatabase;
use crate::otbm::{MapData, OtbmLoader};
use crate::outfits::OutfitDatabase;
use crate::spawns::load_spawn_xml;
use crate::vocations::VocationDatabase;
use std::path::Path;
use tfs_rust_common::error::Result;
use tracing::info;

pub struct Content {
    pub items: ItemDatabase,
    pub monsters: MonsterDatabase,
    pub vocations: VocationDatabase,
    pub outfits: OutfitDatabase,
    pub mounts: MountDatabase,
    pub groups: GroupDatabase,
    pub map: MapData,
}

/// Load server content. `map_otbm_relative` is under `data_dir` (e.g. `world/world.otbm`);
/// default for this repo’s data pack: `world/forgotten.otbm`.
pub async fn load_all(data_dir: &Path, map_otbm_relative: Option<&str>) -> Result<Content> {
    info!("Starting concurrent content pipeline...");

    let otb_path = data_dir.join("items/items.otb");
    let xml_path = data_dir.join("items/items.xml");
    let monsters_dir = data_dir.join("monster");
    let voc_path = data_dir.join("XML/vocations.xml");
    let out_path = data_dir.join("XML/outfits.xml");
    let mounts_path = data_dir.join("XML/mounts.xml");
    let groups_path = data_dir.join("XML/groups.xml");
    let map_rel = map_otbm_relative.unwrap_or("world/forgotten.otbm");
    let map_path = data_dir.join(map_rel);
    let map_path_for_task = map_path.clone();

    let items_future =
        tokio::task::spawn_blocking(move || ItemDatabase::load(&otb_path, &xml_path));

    let vocs_future = tokio::task::spawn_blocking(move || VocationDatabase::load(&voc_path));

    let out_future = tokio::task::spawn_blocking(move || OutfitDatabase::load(&out_path));

    let mounts_future = tokio::task::spawn_blocking(move || MountDatabase::load(&mounts_path));

    let groups_future = tokio::task::spawn_blocking(move || GroupDatabase::load(&groups_path));

    let map_future =
        tokio::task::spawn_blocking(move || OtbmLoader::load_from_file(&map_path_for_task));

    let (items_res, vocs_res, out_res, mounts_res, groups_res, map_res) = tokio::join!(
        items_future,
        vocs_future,
        out_future,
        mounts_future,
        groups_future,
        map_future
    );

    let items = items_res.unwrap()?;
    let items_for_monsters = items.clone();
    let monsters_future = tokio::task::spawn_blocking(move || {
        MonsterDatabase::load_dir(&monsters_dir, &items_for_monsters)
    });
    let monsters = monsters_future.await.unwrap()?;

    let mut map = map_res.unwrap()?;
    let base = map_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = map_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("world");
    let spawn_rel = map
        .spawn_file
        .clone()
        .unwrap_or_else(|| format!("{stem}-spawn.xml"));
    let spawn_path = base.join(&spawn_rel);
    if spawn_path.exists() {
        map.spawn_zones = load_spawn_xml(&spawn_path)?;
    }

    info!("Content pipeline loaded successfully.");

    Ok(Content {
        items,
        monsters,
        vocations: vocs_res.unwrap()?,
        outfits: out_res.unwrap()?,
        mounts: mounts_res.unwrap()?,
        groups: groups_res.unwrap()?,
        map,
    })
}
