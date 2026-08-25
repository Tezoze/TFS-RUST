use crate::groups::GroupDatabase;
use crate::houses_xml::{HouseXmlEntry, load_houses_xml};
use crate::items::ItemDatabase;
use crate::monsters::MonsterDatabase;
use crate::mounts::MountDatabase;
use crate::otbm::{MapData, OtbmLoader};
use crate::outfits::OutfitDatabase;
use crate::spawns::load_spawn_xml;
use crate::vocations::VocationRegistry;
use std::path::{Path, PathBuf};
use tfs_rust_common::error::Result;
use tracing::info;

pub struct Content {
    pub items: ItemDatabase,
    pub monsters: MonsterDatabase,
    pub vocations: VocationRegistry,
    pub outfits: OutfitDatabase,
    pub mounts: MountDatabase,
    pub groups: GroupDatabase,
    pub map: MapData,
    /// `{map}-houses.xml` entries (`Houses::loadHousesXML`). Empty when the file is missing.
    pub houses: Vec<HouseXmlEntry>,
}

/// Load server content. `map_otbm_relative` is under `data_dir` (e.g. `world/world.otbm`);
/// default for this repo’s data pack: `world/forgotten.otbm`.
///
pub async fn load_all(data_dir: &Path, map_otbm_relative: Option<&str>) -> Result<Content> {
    info!("Starting concurrent content pipeline...");

    let otb_path = data_dir.join("items/items.otb");
    let xml_path = data_dir.join("items/items.xml");
    let monsters_dir = data_dir.join("monster");
    let voc_path = data_dir.join("defs/vocations.lua");
    let groups_path = data_dir.join("defs/groups.lua");
    let out_path = data_dir.join("XML/outfits.xml");
    let mounts_path = data_dir.join("XML/mounts.xml");
    let map_rel = map_otbm_relative.unwrap_or("world/forgotten.otbm");
    let map_path = data_dir.join(map_rel);
    let map_path_for_task = map_path.clone();

    let items_future =
        tokio::task::spawn_blocking(move || ItemDatabase::load(&otb_path, &xml_path));

    let vocs_future = tokio::task::spawn_blocking(move || VocationRegistry::load(&voc_path));

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
    // Waypoints / DistUse live in patched `items.otb` only — never load `objects.srv` at runtime.
    // Offline: `cargo run -p tfs-rust-content --bin patch-otb-waypoints`
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
    // C++ `IOMap::loadSpawns` — OTBM `OTBM_ATTR_EXT_SPAWN_FILE`, else `{map}-spawn.xml`
    // (`iomap.h`). TVP OTBMs often name `spawns.xml`; this repo also ships `{stem}-spawn.xml`.
    let stem_spawn = format!("{stem}-spawn.xml");
    let otbm_spawn = map.spawn_file.clone();
    let primary_rel = otbm_spawn.clone().unwrap_or_else(|| stem_spawn.clone());
    let primary_path = base.join(&primary_rel);

    let primary_exists = primary_path.is_file();
    let fallback_path = base.join(&stem_spawn);
    let fallback_exists = fallback_path.is_file();
    let use_fallback =
        !primary_exists && otbm_spawn.is_some() && primary_rel != stem_spawn && fallback_exists;

    let spawn_path = if primary_exists {
        Some(primary_path)
    } else if use_fallback {
        tracing::warn!(
            otbm_spawn = %primary_rel,
            fallback = %stem_spawn,
            "OTBM spawn file missing; using map stem fallback"
        );
        Some(fallback_path)
    } else {
        tracing::warn!(
            primary = %primary_path.display(),
            fallback = %base.join(&stem_spawn).display(),
            "no spawn XML found for map"
        );
        None
    };

    if let Some(spawn_path) = spawn_path {
        map.spawn_zones = load_spawn_xml(&spawn_path)?;
        let entry_count: usize = map.spawn_zones.iter().map(|z| z.entries.len()).sum();
        info!(
            spawn_file = %spawn_path.display(),
            zones = map.spawn_zones.len(),
            entries = entry_count,
            used_fallback = use_fallback,
            "loaded spawn XML"
        );
    }

    // Associated house XML is `{mapName}-houses.xml` next to the OTBM
    // (`forgotten` → `forgotten-houses.xml`). TFS `iomap.h` uses `{map}-house.xml`;
    // OTBM `OTBM_ATTR_EXT_HOUSE_FILE` is a last-resort path.
    let house_path = resolve_house_xml_path(base, stem, map.house_file.as_deref());

    let houses = if let Some(house_path) = house_path {
        info!(house_file = %house_path.display(), "loading house XML");
        load_houses_xml(&house_path)?
    } else {
        Vec::new()
    };

    info!("Content pipeline loaded successfully.");

    Ok(Content {
        items,
        monsters,
        vocations: vocs_res.unwrap()?,
        outfits: out_res.unwrap()?,
        mounts: mounts_res.unwrap()?,
        groups: groups_res.unwrap()?,
        map,
        houses,
    })
}

/// House XML next to the OTBM: `{stem}-houses.xml`, then TFS `{stem}-house.xml`, then OTBM attr.
pub(crate) fn house_xml_candidate_names(stem: &str, otbm_house: Option<&str>) -> Vec<String> {
    let mut names = vec![format!("{stem}-houses.xml"), format!("{stem}-house.xml")];
    if let Some(rel) = otbm_house.map(str::trim).filter(|s| !s.is_empty()) {
        let leaf = Path::new(rel)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(rel);
        if !names.iter().any(|n| n == leaf || n == rel) {
            names.push(rel.to_string());
        }
    }
    names
}

fn resolve_house_xml_path(base: &Path, stem: &str, otbm_house: Option<&str>) -> Option<PathBuf> {
    for name in house_xml_candidate_names(stem, otbm_house) {
        let path = base.join(&name);
        if path.is_file() {
            return Some(path);
        }
    }
    tracing::warn!(
        dir = %base.display(),
        stem,
        "no house XML found for map"
    );
    None
}

#[cfg(test)]
mod tests {
    use super::house_xml_candidate_names;

    #[test]
    fn house_xml_prefers_plural_houses_file() {
        let names = house_xml_candidate_names("forgotten", None);
        assert_eq!(names[0], "forgotten-houses.xml");
        assert_eq!(names[1], "forgotten-house.xml");
        let names = house_xml_candidate_names("map", Some("map-houses.xml"));
        assert_eq!(names[0], "map-houses.xml");
        assert!(!names.iter().skip(1).any(|n| n == "map-houses.xml"));
    }
}

