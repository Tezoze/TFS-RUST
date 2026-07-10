//! Script loaders for `data/scripts/weapons/*.lua` and `data/scripts/spells/**/*.lua`.
//!
//! PC-2b: drains `PendingWeapon` / `PendingSpell` entries from the Lua runtime's
//! `_pending_weapons` / `_pending_spells` global tables into `WeaponRegistry` /
//! `SpellRegistry` on `tfs-rust-content`.
//!
//! C++ reference: `weapons.cpp` `Weapons::load` (scans `data/weapons/scripts/`),
//! `spells.cpp` `Spells::load` (scans `data/spells/scripts/`).

use std::path::{Path, PathBuf};

use tfs_rust_content::spells::{InstantSpellDef, SpellRegistry};
use tfs_rust_content::weapons::{WandDef, WeaponRegistry};

use crate::LuaRuntime;
use crate::userdata::{PendingSpell, PendingWeapon};

impl LuaRuntime {
    /// Load all weapon scripts from `data/scripts/weapons/*.lua`.
    /// Drains `_pending_weapons` into a `WeaponRegistry`.
    pub fn load_weapon_scripts(&mut self, data_dir: &Path) -> Result<WeaponRegistry, String> {
        let weapons_dir = data_dir.join("scripts/weapons");
        if !weapons_dir.exists() {
            tracing::warn!("Weapons scripts dir not found: {}", weapons_dir.display());
            return Ok(WeaponRegistry::default());
        }

        // Initialize the pending buffer before loading scripts.
        self.lua
            .globals()
            .set(
                "_pending_weapons",
                self.lua.create_table().map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;

        let mut lua_files: Vec<PathBuf> = std::fs::read_dir(&weapons_dir)
            .map_err(|e| format!("read weapons dir: {e}"))?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|ext| ext == "lua"))
            .filter(|p| {
                // Skip files starting with `#` (TFS convention for example/disabled scripts).
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_none_or(|n| !n.starts_with('#'))
            })
            .collect();
        lua_files.sort();

        for path in &lua_files {
            let path_str = path.display().to_string();
            if let Err(e) = self
                .lua
                .load(&std::fs::read_to_string(path).map_err(|e| e.to_string())?)
                .set_name(&path_str)
                .exec()
            {
                tracing::warn!("Failed to load weapon script {}: {}", path_str, e);
            }
        }

        // Drain pending weapons into the registry.
        let pending = self
            .lua
            .globals()
            .get::<mlua::Table>("_pending_weapons")
            .map_err(|e| e.to_string())?;
        let mut registry = WeaponRegistry::default();
        for pair in pending.pairs::<i64, mlua::AnyUserData>() {
            let (_, ud) = pair.map_err(|e| e.to_string())?;
            let pw = ud.borrow::<PendingWeapon>().map_err(|e| e.to_string())?;
            match pw.weapon_type {
                6 => {
                    // WEAPON_WAND
                    registry.wands.insert(
                        pw.item_id,
                        WandDef {
                            item_id: pw.item_id,
                            level: pw.level,
                            mana_cost: pw.mana_cost,
                            element: pw.element,
                            damage_min: pw.damage_min,
                            damage_max: pw.damage_max,
                            vocations: pw.vocations.clone(),
                        },
                    );
                }
                5 | 7 => {
                    // WEAPON_DISTANCE / WEAPON_AMMO — PC-3 scope; store minimally.
                    tracing::debug!("Distance/ammo weapon {} loaded (PC-3 scope)", pw.item_id);
                }
                1..=3 => {
                    // WEAPON_SWORD / WEAPON_CLUB / WEAPON_AXE — melee; store minimally.
                    tracing::debug!("Melee weapon {} loaded (PC-2b struct only)", pw.item_id);
                }
                _ => {
                    tracing::warn!(
                        "Unknown weapon type {} for item {}",
                        pw.weapon_type,
                        pw.item_id
                    );
                }
            }
        }

        tracing::info!(
            "Loaded {} weapon scripts: {} wands, {} distance, {} melee",
            lua_files.len(),
            registry.wands.len(),
            registry.distance.len(),
            registry.melee.len()
        );

        // Clear the pending buffer.
        let _ = self
            .lua
            .globals()
            .set("_pending_weapons", self.lua.create_table().unwrap());

        Ok(registry)
    }

    /// Load all spell scripts from `data/scripts/spells/**/*.lua` (recursive).
    /// Also loads `data/scripts/spells/areas.lua` first (plain Lua table definitions).
    /// Drains `_pending_spells` into a `SpellRegistry`.
    pub fn load_spell_scripts(&mut self, data_dir: &Path) -> Result<SpellRegistry, String> {
        let spells_dir = data_dir.join("scripts/spells");
        if !spells_dir.exists() {
            tracing::warn!("Spells scripts dir not found: {}", spells_dir.display());
            return Ok(SpellRegistry::default());
        }

        // Initialize the pending buffer.
        self.lua
            .globals()
            .set(
                "_pending_spells",
                self.lua.create_table().map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;

        // Load areas.lua first — it defines AREA_* tables referenced by spell scripts.
        let areas_path = spells_dir.join("areas.lua");
        if areas_path.exists() {
            let path_str = areas_path.display().to_string();
            if let Err(e) = self
                .lua
                .load(&std::fs::read_to_string(&areas_path).map_err(|e| e.to_string())?)
                .set_name(&path_str)
                .exec()
            {
                tracing::warn!("Failed to load areas.lua: {}", e);
            }
        }

        // Recursively collect all .lua files (excluding areas.lua and #example.lua).
        let mut lua_files: Vec<PathBuf> = Vec::new();
        collect_lua_files(&spells_dir, &mut lua_files);
        lua_files.retain(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n != "areas.lua" && !n.starts_with('#'))
        });
        lua_files.sort();

        for path in &lua_files {
            let path_str = path.display().to_string();
            if let Err(e) = self
                .lua
                .load(&std::fs::read_to_string(path).map_err(|e| e.to_string())?)
                .set_name(&path_str)
                .exec()
            {
                tracing::warn!("Failed to load spell script {}: {}", path_str, e);
            }
        }

        // Drain pending spells into the registry.
        let pending = self
            .lua
            .globals()
            .get::<mlua::Table>("_pending_spells")
            .map_err(|e| e.to_string())?;
        let mut registry = SpellRegistry::default();
        for pair in pending.pairs::<i64, mlua::AnyUserData>() {
            let (_, ud) = pair.map_err(|e| e.to_string())?;
            let ps = ud.borrow::<PendingSpell>().map_err(|e| e.to_string())?;
            if ps.is_instant() {
                let def = InstantSpellDef {
                    name: ps.name.clone(),
                    words: ps.words.clone(),
                    level: ps.level,
                    magic_level: ps.magic_level,
                    mana: ps.mana,
                    mana_percent: ps.mana_percent,
                    soul: ps.soul,
                    group: ps.group,
                    cooldown: ps.cooldown,
                    group_cooldown: ps.group_cooldown,
                    is_premium: ps.is_premium,
                    is_aggressive: ps.is_aggressive,
                    need_target: ps.need_target,
                    need_weapon: ps.need_weapon,
                    need_learn: ps.need_learn,
                    is_self_target: ps.is_self_target,
                    vocations: ps.vocations.clone(),
                    on_cast_callback: ps.on_cast_callback.clone(),
                };
                registry
                    .instant_by_words
                    .insert(ps.words.to_ascii_lowercase(), def.clone());
                registry.instant_by_name.insert(ps.name.clone(), def);
            } else if ps.is_rune() {
                let def = tfs_rust_content::spells::RuneSpellDef {
                    name: ps.name.clone(),
                    rune_id: ps.rune_id,
                    charges: ps.charges,
                    level: ps.level,
                    magic_level: ps.magic_level,
                    mana: ps.mana,
                    group: ps.group,
                    cooldown: ps.cooldown,
                    group_cooldown: ps.group_cooldown,
                    is_aggressive: ps.is_aggressive,
                    need_target: ps.need_target,
                    vocations: ps.vocations.clone(),
                    on_cast_callback: ps.on_cast_callback.clone(),
                };
                registry.runes_by_id.insert(ps.rune_id, def.clone());
                registry.runes_by_name.insert(ps.name.clone(), def);
            }
        }

        tracing::info!(
            "Loaded {} spell scripts: {} instant, {} runes",
            lua_files.len(),
            registry.instant_by_words.len(),
            registry.runes_by_id.len()
        );

        // Clear the pending buffer.
        let _ = self
            .lua
            .globals()
            .set("_pending_spells", self.lua.create_table().unwrap());

        Ok(registry)
    }
}

/// Recursively collect all `.lua` files in a directory.
fn collect_lua_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_lua_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "lua") {
            out.push(path);
        }
    }
}
