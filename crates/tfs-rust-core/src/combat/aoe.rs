//! AoE combat execution from Lua `Combat:execute()` — PC-3a.
//!
//! C++ reference:
//! - 772 `ExecuteCircleSpell` — `tibia-game-master/src/magic.cc:459` iterates
//!   rings `0..=R`, checks `ThrowPossible` + `IsProtectionZone` per tile, then
//!   calls `Impact->handleField` + `Impact->handleCreature` per creature on the tile.
//! - 1098 `Combat::doCombat(caster, position)` — `src/combat.cpp:737` resolves
//!   the area tile list, checks `canDoCombat` per tile, and applies damage to
//!   every creature on each tile via `doAreaCombat` (`combat.cpp:929`).
//! - 1098 `luaCombatExecute` — `src/luascript.cpp:13198` dispatches on variant
//!   type (NUMBER → target, POSITION/TARGETPOSITION → area).
//! - Tile item create — `Combat::combatTileEffects` — `combat.cpp:557`.
//! - Distance FX — `Combat::postCombatEffects` — `combat.cpp:643`.

use tfs_rust_common::enums::{CombatType, ConditionType, WorldType, ZoneType};
use tfs_rust_common::Position;
use tfs_rust_lua::CombatExecuteRequest;

use crate::combat::{apply_condition, CombatDamage, CombatParams};
use crate::creature::{roll_target_defense, CreatureKind};
use crate::cylinder::CylinderFlags;
use crate::game_world::GameWorld;
use crate::game_world_chat::{active_condition_from_apply_spec, condition_type_from_lua};
use crate::ids::CreatureId;
use crate::item::Item;
use crate::item_attributes::ItemAttributes;
use crate::login_out::creature_wire_id;

/// Tile iteration result from 772 `ExecuteCircleSpell` filtering (`magic.cc:468-500`).
///
/// `tiles` is every position that passed PZ + LoS checks; `targets` is every
/// `(creature, tile)` pair from those tiles (after caster/NPC filtering).
/// Callers broadcast effects, create fields, and apply impacts from these lists.
pub(crate) struct AreaIteration {
    /// All tile positions that passed PZ + LoS filtering.
    pub tiles: Vec<Position>,
    /// Creatures on passing tiles: `(creature_id, tile_position)`.
    pub targets: Vec<(CreatureId, Position)>,
}

impl GameWorld {
    /// Physical spell mitigation — 772 `TDamageImpact` AllowDefense + `Damage()` armor.
    ///
    /// C++: `magic.cc:147-151` (`GetDefendDamage`); `crmain.cc:624-629` (`GetArmorStrength`).
    /// TFS domain: `COMBAT_PARAM_BLOCKSHIELD` / `BLOCKARMOR`. Shared by Lua AoE and
    /// monster CASTING (`AllowDefense=true` always for physical impacts).
    ///
    /// H1 — Armor is no longer subtracted here; it is returned as `Some(armor)` for the
    /// caller to pass via `CombatParams::armor` so `combat_execute_with_stimulus` applies
    /// it after PvP-half/absorb (`crmain.cc:624-630`). Defense is still rolled here (C++
    /// `GetDefendDamage` runs before `Damage()`).
    ///
    /// Returns `(post-defense magnitude, optional armor value)`. Broadcasts poff (3)
    /// when defense fully absorbs.
    pub(crate) fn mitigate_physical_spell_damage(
        &mut self,
        target_id: CreatureId,
        target_pos: Position,
        raw_magnitude: i32,
        block_shield: bool,
        block_armor: bool,
    ) -> (i32, Option<i32>) {
        let mut abs_dmg = raw_magnitude.abs();
        if abs_dmg <= 0 {
            return (0, None);
        }
        let profile = self.mechanics.profile;
        let server_ms = self.server_ms;
        let mut defense_snap = self.melee_defense_snapshot_for(target_id);
        if block_shield {
            let defense_roll;
            let defense_gate_passed = self
                .creatures
                .get(target_id)
                .is_some_and(|k| server_ms >= k.base().earliest_defend_ms);
            if defense_gate_passed && defense_snap.has_shield {
                self.player_shield_skill_learning(target_id, true);
                defense_snap = self.melee_defense_snapshot_for(target_id);
            }
            defense_roll = match self.creatures.get_mut(target_id) {
                Some(kind) => roll_target_defense(
                    kind.base_mut(),
                    server_ms,
                    &profile,
                    &self.mechanics.hooks,
                    defense_snap,
                    &self.parity_rng,
                ),
                None => 0,
            };
            if defense_gate_passed {
                self.player_shield_wearout(target_id);
            }
            abs_dmg = (abs_dmg - defense_roll).max(0);
        }
        // H1 — Return armor value for the shared path instead of subtracting here.
        let armor = if block_armor { Some(defense_snap.armor) } else { None };
        if abs_dmg <= 0 {
            // Defense >= attack → poff (3).
            self.broadcast_magic_effect(target_pos, 3u8);
        }
        (abs_dmg, armor)
    }

    /// 772 `ExecuteCircleSpell` tile iteration from offsets — `magic.cc:468-500`.
    ///
    /// Converts `offsets` relative to `center` into absolute positions (skipping
    /// negative coordinates), then delegates to [`filter_area_positions`] for the
    /// shared per-tile filtering. Used by the Lua `Combat:execute` path.
    pub(crate) fn collect_area_targets(
        &self,
        center: Position,
        los_origin: Position,
        offsets: &[(i32, i32)],
        aggressive: bool,
        skip_caster: Option<CreatureId>,
        skip_npcs: bool,
    ) -> AreaIteration {
        let positions: Vec<Position> = offsets
            .iter()
            .filter_map(|&(dx, dy)| {
                let tx = center.x as i32 + dx;
                let ty = center.y as i32 + dy;
                if tx < 0 || ty < 0 {
                    return None;
                }
                Some(Position {
                    x: tx as u16,
                    y: ty as u16,
                    z: center.z,
                })
            })
            .collect();
        self.filter_area_positions(&positions, los_origin, aggressive, skip_caster, skip_npcs)
    }

    /// Per-tile filtering shared by Lua and monster paths — 772 `ExecuteCircleSpell`
    /// `magic.cc:468-500`: PZ skip, `ThrowPossible` LoS, creature collection.
    ///
    /// Takes absolute tile positions (from `monster_idle_spell_tiles` or converted
    /// from offsets by `collect_area_targets`). Returns passing tiles + filtered
    /// `(creature, tile)` pairs. Callers handle effect broadcast, field creation,
    /// and impact application.
    pub(crate) fn filter_area_positions(
        &self,
        tiles: &[Position],
        los_origin: Position,
        aggressive: bool,
        skip_caster: Option<CreatureId>,
        skip_npcs: bool,
    ) -> AreaIteration {
        let mut result_tiles: Vec<Position> = Vec::new();
        let mut targets: Vec<(CreatureId, Position)> = Vec::new();

        for &tile_pos in tiles {
            // PZ skip — 772 `magic.cc:475`.
            if aggressive && self.tile_in_protection_zone(tile_pos) {
                continue;
            }

            // LoS — 772 `ThrowPossible` (`magic.cc:479/589`), power 0.
            if !self.map.throw_possible(los_origin, tile_pos, 0) {
                continue;
            }

            result_tiles.push(tile_pos);

            // Collect creatures — 772 `GetFirstObject` loop (`magic.cc:485-494`).
            if let Some(tile) = self.map.get_tile(tile_pos) {
                for &cid in &tile.body().creatures {
                    if skip_caster == Some(cid) {
                        continue;
                    }
                    if skip_npcs
                        && aggressive
                        && matches!(self.creatures.get(cid), Some(CreatureKind::Npc(_)))
                    {
                        continue;
                    }
                    targets.push((cid, tile_pos));
                }
            }
        }

        AreaIteration {
            tiles: result_tiles,
            targets,
        }
    }
}

/// Map a Lua `COMBAT_*` bit-flag value to the Rust `CombatType` enum.
/// Mirrors `CombatDef::resolved_combat_type` in `tfs-rust-lua/src/userdata/combat.rs`.
fn combat_type_from_lua(value: i32) -> CombatType {
    match value {
        0 => CombatType::Undefined, // COMBAT_NONE — condition/FX-only (utevo lux, haste, …)
        1 => CombatType::Physical,
        2 => CombatType::Energy,
        4 => CombatType::Earth,
        8 => CombatType::Fire,
        16 => CombatType::Undefined,
        32 => CombatType::LifeDrain,
        64 => CombatType::ManaDrain,
        128 => CombatType::Healing,
        _ => CombatType::Physical,
    }
}

/// TFS `const.h` / `combatTileEffects` field & wall item ids.
const ITEM_FIREFIELD_PVP_FULL: u16 = 1487;
const ITEM_FIREFIELD_PVP_MEDIUM: u16 = 1488;
const ITEM_FIREFIELD_PVP_SMALL: u16 = 1489;
const ITEM_FIREFIELD_PERSISTENT_FULL: u16 = 1492;
const ITEM_FIREFIELD_PERSISTENT_MEDIUM: u16 = 1493;
const ITEM_FIREFIELD_PERSISTENT_SMALL: u16 = 1494;
const ITEM_FIREFIELD_NOPVP: u16 = 1500;
const ITEM_POISONFIELD_PVP: u16 = 1490;
const ITEM_POISONFIELD_PERSISTENT: u16 = 1496;
const ITEM_POISONFIELD_NOPVP: u16 = 1503;
const ITEM_ENERGYFIELD_PVP: u16 = 1491;
const ITEM_ENERGYFIELD_PERSISTENT: u16 = 1495;
const ITEM_ENERGYFIELD_NOPVP: u16 = 1504;
const ITEM_MAGICWALL: u16 = 1497;
const ITEM_MAGICWALL_PERSISTENT: u16 = 1498;
const ITEM_MAGICWALL_NOPVP: u16 = 20669;
const ITEM_WILDGROWTH: u16 = 1499;
const ITEM_WILDGROWTH_PERSISTENT: u16 = 2721;
const ITEM_WILDGROWTH_NOPVP: u16 = 20670;

/// Remap CREATEITEM id — TFS `Combat::combatTileEffects` (`combat.cpp:560-614`).
fn remap_create_item_id(
    raw: u16,
    world_type: WorldType,
    tile_zone: ZoneType,
    caster_is_player_or_summon: bool,
) -> u16 {
    let mut item_id = match raw {
        ITEM_FIREFIELD_PERSISTENT_FULL => ITEM_FIREFIELD_PVP_FULL,
        ITEM_FIREFIELD_PERSISTENT_MEDIUM => ITEM_FIREFIELD_PVP_MEDIUM,
        ITEM_FIREFIELD_PERSISTENT_SMALL => ITEM_FIREFIELD_PVP_SMALL,
        ITEM_ENERGYFIELD_PERSISTENT => ITEM_ENERGYFIELD_PVP,
        ITEM_POISONFIELD_PERSISTENT => ITEM_POISONFIELD_PVP,
        ITEM_MAGICWALL_PERSISTENT => ITEM_MAGICWALL,
        ITEM_WILDGROWTH_PERSISTENT => ITEM_WILDGROWTH,
        other => other,
    };

    if caster_is_player_or_summon
        && (world_type == WorldType::NoPvp || tile_zone == ZoneType::NoPvp)
    {
        item_id = match item_id {
            ITEM_FIREFIELD_PVP_FULL => ITEM_FIREFIELD_NOPVP,
            ITEM_POISONFIELD_PVP => ITEM_POISONFIELD_NOPVP,
            ITEM_ENERGYFIELD_PVP => ITEM_ENERGYFIELD_NOPVP,
            ITEM_MAGICWALL => ITEM_MAGICWALL_NOPVP,
            ITEM_WILDGROWTH => ITEM_WILDGROWTH_NOPVP,
            other => other,
        };
    }

    item_id
}

fn create_item_applies_infight(item_id: u16) -> bool {
    matches!(
        item_id,
        ITEM_FIREFIELD_PVP_FULL | ITEM_POISONFIELD_PVP | ITEM_ENERGYFIELD_PVP
    )
}

impl GameWorld {
    /// Execute a Lua-originated combat request — PC-3a.
    ///
    /// C++ reference: `Combat::doCombat(caster, position)` — `combat.cpp:737`.
    /// The Lua side resolved area offsets + formula min/max; this method iterates
    /// tiles, checks `throw_possible` + PZ per tile (matching 772
    /// `ExecuteCircleSpell` `magic.cc:475-481`), and applies damage to every
    /// creature on each affected tile via `combat_execute_with_stimulus`.
    pub fn combat_execute_from_lua(&mut self, request: &CombatExecuteRequest) -> Result<(), String> {
        let caster_id = self.resolve_creature_u64(request.caster_id);
        let center = Position {
            x: request.center_x,
            y: request.center_y,
            z: request.center_z,
        };
        let combat_type = combat_type_from_lua(request.combat_type);

        // 772 `CastSpell` PZ gate — `magic.cc:3403-3407`: aggressive spells cast
        // from a PZ tile are rejected (unless the caster has ATTACK_EVERYWHERE).
        // We skip the right check here (GM flags wired separately); the tile-level
        // PZ skip below still applies per-tile for aggressive combat.
        if request.aggressive {
            if let Some(caster) = caster_id {
                if let Some(cpos) = self.creatures.get(caster).map(|k| k.position()) {
                    if self
                        .map
                        .get_tile(cpos)
                        .is_some_and(|t| t.body().zone == ZoneType::Protection)
                    {
                        // C++ throws PROTECTIONZONE — we silently skip (the Lua
                        // spell script handles the cancel message).
                        return Ok(());
                    }
                }
            }
        }

        // Shared tile iteration — 772 `ExecuteCircleSpell` `magic.cc:468-500`.
        // `collect_area_targets` handles PZ skip, LoS (`ThrowPossible`), and creature
        // collection (with NPC skip for aggressive). Effect broadcast and create-item
        // collection are deferred to after `CheckAffectedPlayers` (which may abort).
        //
        // LoS origin: 772 `AngleShapeSpell` checks `ThrowPossible` from the
        // **caster** position (`magic.cc:589`), while `ExecuteCircleSpell`
        // checks from the **center** (`magic.cc:479`). When center == caster
        // (non-directional spells), these are identical. For directional spells
        // (beams/waves), the center is 1 tile in front of the caster, so LoS
        // must be from the caster to correctly block tiles behind walls.
        let caster_pos = Position {
            x: request.caster_x,
            y: request.caster_y,
            z: request.caster_z,
        };
        let los_origin = if caster_pos == center {
            center
        } else {
            caster_pos
        };

        let skip_caster = if request.aggressive { caster_id } else { None };
        let iter = self.collect_area_targets(
            center,
            los_origin,
            &request.area_offsets,
            request.aggressive,
            skip_caster,
            true, // skip NPCs for aggressive — TFS `Combat::canDoCombat` (`combat.cpp:243-248`)
        );
        let targets = iter.targets;

        // Collect create-item tiles from passing positions — Lua-specific
        // (`Combat::combatTileEffects` — `combat.cpp:557`).
        let create_tiles: Vec<(Position, ZoneType)> = if request.create_item > 0 {
            iter.tiles
                .iter()
                .map(|&pos| {
                    let zone = self
                        .map
                        .get_tile(pos)
                        .map(|t| t.body().zone)
                        .unwrap_or(ZoneType::Normal);
                    (pos, zone)
                })
                .collect()
        } else {
            Vec::new()
        };

        // 772 `CheckAffectedPlayers` — deny whole cast under secure + unmarked player on any tile.
        if request.aggressive {
            if let Some(caster) = caster_id {
                let mut checked = std::collections::HashSet::new();
                for (_, tile_pos) in &targets {
                    if !checked.insert(*tile_pos) {
                        continue;
                    }
                    if !self.check_affected_players(caster, *tile_pos) {
                        return Err(
                            "Turn secure mode off if you really want to attack unmarked players."
                                .into(),
                        );
                    }
                }
                for (tile_pos, _) in &create_tiles {
                    if !checked.insert(*tile_pos) {
                        continue;
                    }
                    if !self.check_affected_players(caster, *tile_pos) {
                        return Err(
                            "Turn secure mode off if you really want to attack unmarked players."
                                .into(),
                        );
                    }
                }
            }
        }

        // Distance shoot — once from caster → center (`postCombatEffects`).
        if request.distance_effect > 0 {
            if let Some(cid) = caster_id {
                let from = self
                    .creatures
                    .get(cid)
                    .map(|k| k.position())
                    .unwrap_or(caster_pos);
                self.broadcast_distance_shoot(from, center, request.distance_effect as u8);
            }
        }

        // Broadcast magic effects at all passing tiles — deferred until after
        // `CheckAffectedPlayers` (which may abort the cast). 772 `ExecuteCircleSpell`
        // applies `GraphicalEffect` per-tile (`magic.cc:496-498`); TFS
        // `Combat::postCombatEffects` (`combat.cpp:643`) also broadcasts per-tile.
        if request.effect > 0 {
            for &pos in &iter.tiles {
                self.broadcast_magic_effect(pos, request.effect as u8);
            }
        }

        // CREATEITEM — `combatTileEffects` (`combat.cpp:557-631`).
        if request.create_item > 0 {
            let world_type = self.pvp_config.world_type;
            let (caster_is_player_or_summon, owner_wire, infight_player) =
                match caster_id.and_then(|cid| self.creatures.get(cid).map(|k| (cid, k))) {
                    Some((cid, kind)) => {
                        let owner = creature_wire_id(cid, kind);
                        let is_player = matches!(kind, CreatureKind::Player(_));
                        let is_summon = kind.base().is_summon();
                        let infight = if is_player {
                            Some(cid)
                        } else if is_summon {
                            kind.base().master
                        } else {
                            None
                        };
                        (is_player || is_summon, Some(owner), infight)
                    }
                    None => (false, None, None),
                };

            let mut applied_infight = false;
            for (tile_pos, zone) in create_tiles {
                let item_type = remap_create_item_id(
                    request.create_item as u16,
                    world_type,
                    zone,
                    caster_is_player_or_summon,
                );
                let mut item = Item::new_single(item_type);
                if let Some(owner) = owner_wire {
                    item.attributes
                        .get_or_insert_with(|| Box::new(ItemAttributes::new()))
                        .set_owner(owner);
                }
                let iid = self.items.insert(item);
                match self.internal_add_item_to_tile(tile_pos, iid, CylinderFlags::NONE) {
                    Ok(_) => {
                        if !applied_infight
                            && create_item_applies_infight(item_type)
                            && infight_player.is_some()
                        {
                            // 772 `CreateField` fire/poison/energy — `BlockLogout(60, true)`
                            // (`magic.cc:1091-1095`). TFS `addInFightTicks()` alone does not
                            // PZ-lock; field placement is the explicit PZ-lock case.
                            if let Some(pid) = infight_player {
                                self.player_block_logout_infight(pid, true);
                                applied_infight = true;
                            }
                        }
                    }
                    Err(_) => {
                        // Quiet skip — C++ deletes the item on failed add.
                        self.items.remove(iid);
                    }
                }
            }
        }

        // Apply damage / heal / conditions / dispel per target.
        // C++ `Combat::doTargetCombat` + `postCombatEffects` (`combat.cpp:643`):
        // damage first, then conditionList, then dispelType. Heal+DISPEL both run
        // (antidote is damage=0 + dispel only).
        // `COMBAT_PARAM_NODAMAGE` skips the damage arm only (soulfire FX path).
        let damage_min = request.damage_min;
        let damage_max = request.damage_max;
        let block_armor = request.block_armor;
        let block_shield = request.block_shield;
        let condition_specs = &request.conditions;
        let dispel_flag = request.dispel_type;
        let no_damage = request.no_damage
            || combat_type == CombatType::Undefined
            || (damage_min == 0 && damage_max == 0);

        for (target_id, target_pos) in targets {
            // Don't damage the caster with their own aggressive spell — 772
            // `CheckAffectedPlayers` / 1098 `Combat::canDoCombat(caster, target)`.
            // Non-aggressive buffs (light/haste) still apply to the caster.
            if request.aggressive && Some(target_id) == caster_id {
                continue;
            }

            // Capture the notify snapshot BEFORE `combat_execute_with_stimulus` —
            // that path may kill the target (`apply_creature_death`), making
            // `self.creatures.get` return `None`. Without this, the killing-blow
            // damage text + health bar are never sent. Mirrors `strike.rs:145`.
            let notify_snap = self.combat_notify_snapshot(target_id);
            let hp_before = self
                .creatures
                .get(target_id)
                .map(|k| k.base().health)
                .unwrap_or(0);

            if !no_damage {
                // Roll damage — 1098 `getCombatDamage` (`combat.cpp:100`). For
                // `COMBAT_FORMULA_DAMAGE` the min/max are the literal range. For
                // level/magic formula the Lua side already resolved the values.
                let value = crate::combat::uniform_random_glibc(
                    &self.parity_rng,
                    damage_min,
                    damage_max,
                );

                // Healing spells (COMBAT_HEALING) use positive deltas; damage uses
                // negative. 772 `THealingImpact` vs `TDamageImpact` (`magic.cc:210,119`).
                let mut physical_armor: Option<i32> = None;
                let signed_value = if combat_type == CombatType::Healing {
                    value.max(0)
                } else if combat_type == CombatType::Physical {
                    // Shared with monster CASTING — `mitigate_physical_spell_damage`.
                    let (abs_dmg, armor) = self.mitigate_physical_spell_damage(
                        target_id,
                        target_pos,
                        value,
                        block_shield,
                        block_armor,
                    );
                    physical_armor = armor;
                    -abs_dmg
                } else {
                    -value.abs()
                };

                let damage = CombatDamage {
                    primary: (combat_type, signed_value),
                    secondary: (CombatType::Undefined, 0),
                };
                // Dispel is applied after damage so heal+paralyze-clear both work.
                let params = CombatParams {
                    primary_type: combat_type,
                    dispel: None,
                    apply_condition: None,
                    armor: physical_armor,
                };

                self.combat_execute_with_stimulus(caster_id, target_id, &damage, &params);
            }

            // Apply `combat:addCondition` list — C++ `conditionList` post-effects.
            for spec in condition_specs {
                let cond = active_condition_from_apply_spec(spec);
                let ctype = cond.ctype;
                apply_condition(&mut self.creatures, target_id, cond);
                self.on_condition_started(target_id, ctype);
            }

            // `COMBAT_PARAM_DISPEL` — C++ `dispelType` in `postCombatEffects`.
            if let Some(flag) = dispel_flag {
                let dtype = condition_type_from_lua(flag);
                if dtype != ConditionType::None {
                    let removed = if let Some(kind) = self.creatures.get_mut(target_id) {
                        let before = kind.base().active_conditions.len();
                        kind.base_mut()
                            .active_conditions
                            .retain(|c| c.ctype != dtype);
                        before != kind.base().active_conditions.len()
                    } else {
                        false
                    };
                    if removed {
                        self.on_condition_ended(target_id, dtype);
                    }
                }
            }

            // Client HP/mana bar fan-out after combat apply.
            // Damage: `notify_player_combat_damage` (animated text + stats).
            // Healing: `notify_creature_healed` — 772 `THealingImpact` has no
            // TextualEffect but still `SendPlayerData` + health announce
            // (`TSkillHitpoints::Set` `crskill.cc:682-683`). Using damage_done
            // alone would skip heals (`hp_after > hp_before` → delta 0).
            if !no_damage {
                if let Some(snap) = notify_snap {
                    if combat_type == CombatType::Healing {
                        self.notify_creature_healed(target_id, snap);
                    } else {
                        let hp_after = self
                            .creatures
                            .get(target_id)
                            .map(|k| k.base().health)
                            .unwrap_or(0);
                        let damage_done = (hp_before - hp_after).max(0);
                        self.notify_player_combat_damage(
                            caster_id,
                            target_id,
                            damage_done,
                            combat_type,
                            snap,
                        );
                    }
                }
            }
        }

        Ok(())
    }
}

/// Helper: extract a creature's position from the SlotMap.
#[allow(dead_code)]
fn creature_position(world: &GameWorld, cid: CreatureId) -> Option<Position> {
    world.creatures.get(cid).map(|k| k.position())
}

/// Helper: check if a creature is a player (for PVP secure-mode gating).
#[allow(dead_code)]
fn is_player(world: &GameWorld, cid: CreatureId) -> bool {
    matches!(world.creatures.get(cid), Some(CreatureKind::Player(_)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remap_persistent_fire_to_pvp() {
        assert_eq!(
            remap_create_item_id(
                ITEM_FIREFIELD_PERSISTENT_FULL,
                WorldType::Pvp,
                ZoneType::Normal,
                true
            ),
            ITEM_FIREFIELD_PVP_FULL
        );
    }

    #[test]
    fn remap_pvp_fire_to_nopvp_in_nopvp_world() {
        assert_eq!(
            remap_create_item_id(
                ITEM_FIREFIELD_PVP_FULL,
                WorldType::NoPvp,
                ZoneType::Normal,
                true
            ),
            ITEM_FIREFIELD_NOPVP
        );
    }

    #[test]
    fn remap_skips_nopvp_for_non_player_casters() {
        assert_eq!(
            remap_create_item_id(
                ITEM_FIREFIELD_PVP_FULL,
                WorldType::NoPvp,
                ZoneType::Normal,
                false
            ),
            ITEM_FIREFIELD_PVP_FULL
        );
    }

    /// Physical Combat:execute with BLOCKARMOR reduces damage by armor roll.
    #[test]
    fn physical_block_armor_reduces_spell_damage() {
        use crate::creature::MonsterAiConfig;
        use crate::sim_harness::{insert_monster_with_config, minimal_world, sim_hero_player};
        use slotmap::Key;
        use tfs_rust_lua::CombatExecuteRequest;

        let mut world = minimal_world();
        let pos = Position::new(100, 100, 7);
        let caster = world
            .creatures
            .insert(CreatureKind::Player(sim_hero_player("Mage", pos)));
        let target_pos = Position::new(101, 100, 7);
        let mut cfg = MonsterAiConfig::default();
        cfg.armor = 50;
        cfg.defense = 0;
        let target = insert_monster_with_config(&mut world, "Armored", target_pos, 100, cfg);

        let req = CombatExecuteRequest {
            caster_id: caster.data().as_ffi(),
            center_x: target_pos.x,
            center_y: target_pos.y,
            center_z: target_pos.z,
            caster_x: pos.x,
            caster_y: pos.y,
            caster_z: pos.z,
            combat_type: 1, // PHYSICAL
            effect: 0,
            aggressive: true,
            block_armor: true,
            block_shield: false,
            area_offsets: vec![(0, 0)],
            damage_min: 40,
            damage_max: 40,
            conditions: vec![],
            dispel_type: None,
            create_item: 0,
            no_damage: false,
            distance_effect: 0,
        };
        world.combat_execute_from_lua(&req).expect("execute");
        let hp = world.creatures.get(target).unwrap().base().health;
        // Armor 50 vs fixed 40 → often fully absorbed; without armor HP would be 60.
        assert!(
            hp > 60,
            "BLOCKARMOR must mitigate physical spell damage (hp={hp}, expected >60)"
        );
    }
}
