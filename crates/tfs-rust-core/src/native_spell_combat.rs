//! Native spell/rune cast for pure `combat:execute` scripts — skip `onCastSpell` Lua.
//!
//! Pack: TFS `InstantSpell::castSpell` → `Combat::doCombat` — `spells.cpp` / `combat.cpp`.
//! Boot specs from [`tfs_rust_lua::compile_native_spell_combats`].

use rustc_hash::FxHashMap;
use slotmap::Key;
use tfs_rust_common::{Position, ScriptContext};
use tfs_rust_lua::{
    oriented_area_offsets, CombatExecuteRequest, CompiledNativeSpellCombat, CompiledSpellDamage,
};

use crate::game_world::GameWorld;
use crate::ids::CreatureId;

/// Boot-built registry of native combat casts keyed by spell words or `rune:{id}`.
#[derive(Debug, Default, Clone)]
pub struct NativeSpellCombatRegistry {
    entries: FxHashMap<String, CompiledNativeSpellCombat>,
}

impl NativeSpellCombatRegistry {
    pub fn from_compiled(compiled: Vec<CompiledNativeSpellCombat>) -> Self {
        let mut entries = FxHashMap::default();
        for entry in compiled {
            entries.insert(entry.key.clone(), entry);
        }
        Self { entries }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_native(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    fn get(&self, key: &str) -> Option<&CompiledNativeSpellCombat> {
        self.entries.get(key)
    }
}

/// `Some(ok)` when a native handler ran; `None` → fall back to Lua `onCastSpell`.
pub fn try_native_instant_cast(
    world: &mut GameWorld,
    words: &str,
    cid: CreatureId,
    need_direction: bool,
    has_param: bool,
    param: &str,
) -> Option<bool> {
    let key = words.to_ascii_lowercase();
    let compiled = world.native_spell_combats.get(&key)?.clone();
    let center = resolve_instant_center(world, cid, need_direction || compiled.need_direction, has_param, param)?;
    Some(execute_compiled(world, cid, &compiled, center))
}

/// Native rune `onCastSpell` — `rune:{item_id}` key.
pub fn try_native_rune_cast(
    world: &mut GameWorld,
    rune_id: u16,
    cid: CreatureId,
    target_creature: Option<CreatureId>,
    target_pos: Option<(u16, u16, u8)>,
) -> Option<bool> {
    let key = format!("rune:{rune_id}");
    let compiled = world.native_spell_combats.get(&key)?.clone();
    let center = resolve_rune_center(world, cid, target_creature, target_pos)?;
    Some(execute_compiled(world, cid, &compiled, center))
}

fn resolve_instant_center(
    world: &GameWorld,
    cid: CreatureId,
    need_direction: bool,
    has_param: bool,
    param: &str,
) -> Option<Position> {
    let cid_u64 = cid.data().as_ffi();
    if has_param && !param.is_empty() {
        let target = world.get_player_by_name(param.trim())?;
        return world.get_player_position(target);
    }
    let caster_pos = world.get_player_position(cid_u64)?;
    if !need_direction {
        return Some(caster_pos);
    }
    let dir = world.get_player_direction(cid_u64)?;
    Some(offset_by_direction(caster_pos, dir))
}

fn resolve_rune_center(
    world: &GameWorld,
    cid: CreatureId,
    target_creature: Option<CreatureId>,
    target_pos: Option<(u16, u16, u8)>,
) -> Option<Position> {
    if let Some(tid) = target_creature {
        return world.creatures.get(tid).map(|k| k.position());
    }
    if let Some((x, y, z)) = target_pos {
        return Some(Position::new(x, y, z));
    }
    world.get_player_position(cid.data().as_ffi())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tfs_rust_lua::compile_native_spell_combats;
    use std::path::PathBuf;

    #[test]
    fn registry_finds_energy_strike() {
        let data = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        if !data.join("scripts/spells/attack/energy_strike.lua").exists() {
            return;
        }
        let reg = NativeSpellCombatRegistry::from_compiled(compile_native_spell_combats(&data));
        assert!(reg.is_native("ex,ori, vis"));
    }
}

fn offset_by_direction(pos: Position, dir: u8) -> Position {
    let (dx, dy) = match dir {
        0 => (0i16, -1),
        1 => (1, 0),
        2 => (0, 1),
        3 => (-1, 0),
        4 => (-1, 1),
        5 => (1, 1),
        6 => (-1, -1),
        7 => (1, -1),
        _ => (0, 0),
    };
    Position::new(
        (pos.x as i16 + dx).max(0) as u16,
        (pos.y as i16 + dy).max(0) as u16,
        pos.z,
    )
}

fn execute_compiled(
    world: &mut GameWorld,
    cid: CreatureId,
    compiled: &CompiledNativeSpellCombat,
    center: Position,
) -> bool {
    let caster_pos = world
        .creatures
        .get(cid)
        .map(|k| k.position())
        .unwrap_or(center);
    let dx = i32::from(center.x) - i32::from(caster_pos.x);
    let dy = i32::from(center.y) - i32::from(caster_pos.y);
    let area_offsets = match &compiled.area {
        Some(area) => oriented_area_offsets(area, dx, dy),
        None => vec![(0, 0)],
    };
    let (damage_min, damage_max) = resolve_damage(world, cid, compiled);
    let request = CombatExecuteRequest {
        caster_id: cid.data().as_ffi(),
        center_x: center.x,
        center_y: center.y,
        center_z: center.z,
        caster_x: caster_pos.x,
        caster_y: caster_pos.y,
        caster_z: caster_pos.z,
        combat_type: compiled.combat_type,
        effect: compiled.effect,
        aggressive: compiled.aggressive,
        block_armor: compiled.block_armor,
        block_shield: compiled.block_shield,
        area_offsets,
        damage_min,
        damage_max,
        conditions: compiled.conditions.clone(),
        dispel_type: if compiled.dispel_type != 0 {
            Some(compiled.dispel_type)
        } else {
            None
        },
        create_item: compiled.create_item,
        no_damage: compiled.no_damage,
        distance_effect: compiled.distance_effect,
    };
    world.combat_execute_from_lua(&request).is_ok()
}

fn resolve_damage(world: &GameWorld, cid: CreatureId, compiled: &CompiledNativeSpellCombat) -> (i32, i32) {
    let cid_u64 = cid.data().as_ffi();
    match &compiled.damage {
        CompiledSpellDamage::None => (0, 0),
        CompiledSpellDamage::LevelMagic {
            base,
            variation,
            pvp_half,
            healing,
        } => {
            let (lo, hi) = world.compute_magic_damage_range(
                cid_u64,
                *base,
                *variation,
                false,
                *pvp_half,
            );
            if *healing {
                (lo, hi)
            } else {
                (-lo, -hi)
            }
        }
        CompiledSpellDamage::Skill {
            base,
            variation,
            limit_min,
            limit_max,
        } => {
            let level = world.get_player_level(cid_u64).unwrap_or(0);
            let (lo, hi) = world.compute_magic_damage_range(
                cid_u64,
                *base,
                *variation,
                *limit_min,
                *limit_max,
            );
            let lo = (lo * level) / 25;
            let hi = (hi * level) / 25;
            (-lo, -hi)
        }
    }
}
