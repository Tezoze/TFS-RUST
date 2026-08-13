//! Script loaders for `data/scripts/weapons/*.lua` and `data/scripts/spells/**/*.lua`.
//!
//! PC-2b: drains `PendingWeapon` / `PendingSpell` entries from the Lua runtime's
//! `_pending_weapons` / `_pending_spells` global tables into `WeaponRegistry` /
//! `SpellRegistry` on `tfs-rust-content`.
//!
//! C++ reference: `weapons.cpp` `Weapons::load` (scans `data/weapons/scripts/`),
//! `spells.cpp` `Spells::load` (scans `data/spells/scripts/`).

use std::path::{Path, PathBuf};

use tfs_rust_common::enums::CombatType;
use tfs_rust_content::spells::{InstantSpellDef, SpellRegistry};
use tfs_rust_content::weapons::{DistanceWeaponDef, WandDef, WeaponRegistry};

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
        self.lua
            .globals()
            .set(
                "_pending_weapon_callbacks",
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
        let callbacks_table = self
            .lua
            .globals()
            .get::<mlua::Table>("_pending_weapon_callbacks")
            .map_err(|e| e.to_string())?;
        let mut registry = WeaponRegistry::default();
        for pair in pending.pairs::<i64, mlua::AnyUserData>() {
            let (idx, ud) = pair.map_err(|e| e.to_string())?;
            let pw = ud.borrow::<PendingWeapon>().map_err(|e| e.to_string())?;
            let callback_fn: Option<mlua::Function> =
                callbacks_table.get::<mlua::Function>(idx).ok();
            let has_callback = callback_fn.is_some();
            if let Some(func) = callback_fn {
                let reg_key = self
                    .lua
                    .create_registry_value(func)
                    .map_err(|e| e.to_string())?;
                self.register_weapon_callback(pw.item_id, reg_key);
            }
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
                    // WEAPON_DISTANCE / WEAPON_AMMO — PC-3a breakChance/action + onUseWeapon.
                    registry.distance.insert(
                        pw.item_id,
                        DistanceWeaponDef {
                            item_id: pw.item_id,
                            level: pw.level,
                            magic_level: pw.magic_level,
                            mana_cost: pw.mana_cost,
                            vocations: pw.vocations.clone(),
                            hit_chance: 0,
                            shoot_range: 0,
                            element: pw.element,
                            extra_element: CombatType::Physical,
                            break_chance: pw.break_chance,
                            consume_action: pw.consume_action,
                            has_on_use: pw.has_on_use || has_callback,
                        },
                    );
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

        // Clear the pending buffers.
        let _ = self
            .lua
            .globals()
            .set("_pending_weapons", self.lua.create_table().unwrap());
        let _ = self.lua.globals().set(
            "_pending_weapon_callbacks",
            self.lua.create_table().unwrap(),
        );

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

        // PC-3a: Initialize the parallel callback buffer for `__newindex`-captured
        // `onCastSpell` functions.
        self.lua
            .globals()
            .set(
                "_pending_spell_callbacks",
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

        // PC-3a Phase 1: Load `data/lib/core/*.lua` + `data/scripts/functions.lua`
        // before spell scripts. `functions.lua` defines `Player:conjureItem`,
        // `Player:computeDamage` / `computeHealing` / `computeSkillDamage` as
        // `function Player:method` table fields. The `CreatureRef` `__index`
        // fallback bridges these onto userdata so value-callback spell bodies can
        // call them. Shared helper also used by `run_server.rs` before actions.
        // Gap 5a: lib-stage failures propagate (fatal); per-spell loads below
        // stay warn-and-continue.
        crate::actions::load_data_lib(self, data_dir).map_err(|e| e.to_string())?;

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
                eprintln!("DBG Failed: {} — {}", path_str, e);
                tracing::warn!("Failed to load spell script {}: {}", path_str, e);
            }
        }

        // Drain pending spells into the registry.
        let pending = self
            .lua
            .globals()
            .get::<mlua::Table>("_pending_spells")
            .map_err(|e| e.to_string())?;
        // PC-3a: Drain the parallel callback table (functions captured via `__newindex`).
        let callbacks_table = self
            .lua
            .globals()
            .get::<mlua::Table>("_pending_spell_callbacks")
            .map_err(|e| e.to_string())?;
        let mut registry = SpellRegistry::default();
        for pair in pending.pairs::<i64, mlua::AnyUserData>() {
            let (idx, ud) = pair.map_err(|e| e.to_string())?;
            let ps = ud.borrow::<PendingSpell>().map_err(|e| e.to_string())?;

            // PC-3a: If a callback function was captured via `__newindex`,
            // store its `RegistryKey` on the `LuaRuntime` keyed by spell words.
            let callback_fn: Option<mlua::Function> =
                callbacks_table.get::<mlua::Function>(idx).ok();
            if let Some(func) = callback_fn {
                let reg_key = self
                    .lua
                    .create_registry_value(func)
                    .map_err(|e| e.to_string())?;
                if ps.is_rune() && ps.rune_id != 0 {
                    // PC-3a Gap 6: rune callbacks keyed by item id — words are empty.
                    self.register_spell_callback(&format!("rune:{}", ps.rune_id), reg_key);
                } else {
                    self.register_spell_callback(&ps.words, reg_key);
                }
            }
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
                    has_param: ps.has_param,
                    has_player_name_param: ps.has_player_name_param,
                    need_direction: ps.need_direction,
                    range: ps.range,
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
                    rune_magic_level: ps.rune_magic_level,
                    allow_far_use: ps.allow_far_use,
                    block_walls: ps.block_walls,
                    check_floor: ps.check_floor,
                    block_solid: ps.block_solid,
                    block_creature: ps.block_creature,
                    is_pz_lock: ps.is_pz_lock,
                    cooldown_spell_time: ps.cooldown_spell_time,
                    range: ps.range,
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

        // Clear the pending buffers.
        let _ = self
            .lua
            .globals()
            .set("_pending_spells", self.lua.create_table().unwrap());
        let _ = self
            .lua
            .globals()
            .set("_pending_spell_callbacks", self.lua.create_table().unwrap());

        Ok(registry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn workspace_data_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
    }

    /// Integration test: all spell scripts in `data/scripts/spells/` load
    /// without Lua errors. This catches missing constants, missing userdata
    /// methods, and broken `Condition`/`Combat`/`Spell` API surface.
    #[test]
    fn spell_scripts_load_without_errors() {
        let data_root = workspace_data_root();
        let spells_dir = data_root.join("scripts/spells");
        if !spells_dir.exists() {
            return;
        }

        let mut runtime = LuaRuntime::new().expect("runtime");
        let registry = runtime
            .load_spell_scripts(&data_root)
            .expect("spell scripts should load");

        // Should have loaded both instant and rune spells.
        assert!(
            registry.instant_by_words.len() > 0,
            "should have instant spells"
        );
        assert!(
            registry.runes_by_id.len() > 0,
            "should have rune spells — check for load errors above"
        );
    }
}

/// Recursively collect all `.lua` files in a directory.
pub(crate) fn collect_lua_files(dir: &Path, out: &mut Vec<PathBuf>) {
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
