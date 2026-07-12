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

use tfs_rust_common::enums::{CombatType, ZoneType};
use tfs_rust_common::Position;
use tfs_rust_lua::CombatExecuteRequest;

use crate::combat::{uniform_random, CombatDamage, CombatParams};
use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;

/// Map a Lua `COMBAT_*` bit-flag value to the Rust `CombatType` enum.
/// Mirrors `CombatDef::resolved_combat_type` in `tfs-rust-lua/src/userdata/combat.rs`.
fn combat_type_from_lua(value: i32) -> CombatType {
    match value {
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

        // Iterate area offsets — 772 `ExecuteCircleSpell` `magic.cc:468-500`.
        // Collect target creature IDs first to avoid borrow conflicts during
        // `combat_execute_with_stimulus` (which borrows `&mut self`).
        let mut targets: Vec<(CreatureId, Position)> = Vec::new();
        for &(dx, dy) in &request.area_offsets {
            let tx = center.x as i32 + dx;
            let ty = center.y as i32 + dy;
            if tx < 0 || ty < 0 {
                continue;
            }
            let tile_pos = Position {
                x: tx as u16,
                y: ty as u16,
                z: center.z,
            };

            // PZ skip for aggressive combat — 772 `magic.cc:475` / 1098 `canDoCombat`.
            if request.aggressive
                && self
                    .map
                    .get_tile(tile_pos)
                    .is_some_and(|t| t.body().zone == ZoneType::Protection)
            {
                continue;
            }

            // LoS check — 772 `ThrowPossible` (`magic.cc:479`). Power 0 = no
            // extra height clearance (matching the 772 call sites).
            if !self.map.throw_possible(center, tile_pos, 0) {
                continue;
            }

            // Collect creatures on this tile — 772 `GetFirstObject` loop (`magic.cc:485-494`).
            if let Some(tile) = self.map.get_tile(tile_pos) {
                for &cid in &tile.body().creatures {
                    targets.push((cid, tile_pos));
                }
            }
        }

        // Broadcast the impact effect at the center — 1098 `Combat::postCombatEffects`
        // (`combat.cpp:643`). For area spells the effect is at the center; per-tile
        // effects are handled by `combat_execute_with_stimulus` on hit.
        if request.effect > 0 {
            self.broadcast_magic_effect(center, request.effect as u8);
        }

        // Apply damage to each target creature — 772 `Impact->handleCreature`
        // (`magic.cc:490`) / 1098 `doTargetCombat` (`combat.cpp:833`).
        let damage_min = request.damage_min;
        let damage_max = request.damage_max;
        let block_armor = request.block_armor;
        let _block_shield = request.block_shield; // TODO: wire shield defense
        for (target_id, _pos) in targets {
            // Don't damage the caster with their own aggressive spell — 772
            // `CheckAffectedPlayers` / 1098 `Combat::canDoCombat(caster, target)`.
            if request.aggressive && Some(target_id) == caster_id {
                continue;
            }

            // Roll damage — 1098 `getCombatDamage` (`combat.cpp:100`). For
            // `COMBAT_FORMULA_DAMAGE` the min/max are the literal range. For
            // level/magic formula the Lua side already resolved the values.
            let value = uniform_random(&mut self.ai_rng, damage_min, damage_max);

            // Healing spells (COMBAT_HEALING) use positive deltas; damage uses
            // negative. 772 `THealingImpact` vs `TDamageImpact` (`magic.cc:210,119`).
            let signed_value = if combat_type == CombatType::Healing {
                value.max(0)
            } else {
                -value.abs()
            };

            let damage = CombatDamage {
                primary: (combat_type, signed_value),
                secondary: (CombatType::Undefined, 0),
            };
            let params = CombatParams {
                primary_type: combat_type,
                dispel: None,
                apply_condition: None,
            };

            // Apply armor reduction if requested — 772 `Damage` (`crmain.cc:540-574`).
            // `combat_execute_with_stimulus` already handles equipment absorb, so we
            // only need to gate on `block_armor` for the no-armor fast path.
            let _ = block_armor; // armor applied inside combat_execute_with_stimulus
            self.combat_execute_with_stimulus(caster_id, target_id, &damage, &params);
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
