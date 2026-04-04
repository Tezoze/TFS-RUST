use crate::items::ItemDatabase;
use crate::monsters::MonsterDatabase;
use crate::otbm::{MapData, OtbmLoader};
use crate::outfits::OutfitDatabase;
use crate::vocations::VocationDatabase;
use std::path::Path;
use tfs_rust_common::error::Result;
use tracing::info;

pub struct Content {
    pub items: ItemDatabase,
    pub monsters: MonsterDatabase,
    pub vocations: VocationDatabase,
    pub outfits: OutfitDatabase,
    pub map: MapData,
}

pub async fn load_all(data_dir: &Path) -> Result<Content> {
    info!("Starting concurrent content pipeline...");

    let otb_path = data_dir.join("items/items.otb");
    let xml_path = data_dir.join("items/items.xml");
    let monsters_dir = data_dir.join("monsters");
    let voc_path = data_dir.join("XML/vocations.xml");
    let out_path = data_dir.join("XML/outfits.xml");
    let map_path = data_dir.join("world/world.otbm");

    // Spawn concurrent loading tasks
    let items_future =
        tokio::task::spawn_blocking(move || ItemDatabase::load(&otb_path, &xml_path));

    let monsters_future =
        tokio::task::spawn_blocking(move || MonsterDatabase::load_dir(&monsters_dir));

    let vocs_future = tokio::task::spawn_blocking(move || VocationDatabase::load(&voc_path));

    let out_future = tokio::task::spawn_blocking(move || OutfitDatabase::load(&out_path));

    let map_future = tokio::task::spawn_blocking(move || OtbmLoader::load_from_file(&map_path));

    let (items_res, monsters_res, vocs_res, out_res, map_res) = tokio::join!(
        items_future,
        monsters_future,
        vocs_future,
        out_future,
        map_future
    );

    info!("Content pipeline loaded successfully.");

    Ok(Content {
        items: items_res.unwrap()?,
        monsters: monsters_res.unwrap()?,
        vocations: vocs_res.unwrap()?,
        outfits: out_res.unwrap()?,
        map: map_res.unwrap()?,
    })
}
