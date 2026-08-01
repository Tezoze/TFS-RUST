//! 772 drain-triggered idle AI — `IdleStimulus` on ToDo queue drain.
//!
//! - `TCreature::IdleStimulus` — virtual dispatch after `Execute` drains the action list.
//! - `TMonster::IdleStimulus` — `crnonpl.cc:2386`.
//!
//! Phase 3: monsters run on this engine for **both** eras (1098 monster AI deleted).
//! Phase 6: the `beat_driven_loop` flag is collapsed — both eras run on this engine.

use std::time::Instant;

use slotmap::Key;
use tfs_rust_common::enums::{
    CombatType, ConditionType, Direction, SpeakType, WorldType, ZoneType,
};
use tfs_rust_common::game_packet::ThrowPayload;
use tfs_rust_common::Position;

use crate::chase_debug;
use crate::combat::math::spell_damage;
use crate::combat::{disc_offsets, CombatDamage, CombatParams};
use crate::condition::{ActiveCondition, ConditionData};
use crate::creature::{
    drunk_power_from_xml, duration_ms_to_rounds, monster_weapon_attack_distance, speed_mdact,
    ChaseMode, CreatureBase, CreatureKind, MonsterFieldType, MonsterSpell, MonsterState,
    SpellImpact, SpellShape,
};
use crate::cylinder::CylinderFlags;
use crate::item::Item;
use crate::item_attributes::ItemAttributes;
use crate::login_out::creature_wire_id;
use crate::tile::MapStackEntry;
use crate::creature_think::EVENT_CREATURE_THINK_INTERVAL_MS;
use crate::creature_todo::{
    trace_creature_todo, ActionObjectRef, CreatureAction, MONSTER_IDLE_WAIT_MS,
};
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::monster_ai::{
    chebyshev, compute_look_toward_target, manhattan, monster_idle_chase_step_budget,
    monster_master_follow_wait_before_go, monster_master_follow_wait_only_band,
    MonsterCombatCloseChaseEnqueue, MonsterEnqueueAttackResult,
    MonsterIdleChaseRepathOutcome,
};
use crate::player_flags::{flags_for_group, has_player_flag, PLAYER_FLAG_IGNORED_BY_MONSTERS};
use crate::return_value::ReturnValue;
use crate::walk::creature_turn_with_broadcast;

/// C++ `TMonster::IdleStimulus` walking arms — `crnonpl.cc:2676`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MonsterIdleWalkBranch {
    /// `crnonpl.cc:2678` — `IsFleeing` + `SearchFlightField`.
    Flee,
    /// `crnonpl.cc:2686` — summon following master.
    MasterFollow,
    /// `crnonpl.cc:2732` — melee `ToDoGo` toward target.
    MeleeChase,
    /// `crnonpl.cc:2751` — adjacent cardinal sidestep.
    MeleeDance,
    /// `crnonpl.cc:2762` — too close for keep-distance band.
    DistFlee,
    /// `crnonpl.cc:2769` — approach keep-distance band.
    DistChase,
    /// `crnonpl.cc:2787` — lateral at keep-distance.
    DistDance,
    /// `crnonpl.cc:2850` — random roam when no target.
    Roam,
    /// At band / no movement this idle tick.
    Hold,
}

/// Result of executing one idle walk arm.
enum MonsterIdleWalkOutcome {
    QueuedGo { via: &'static str, wait_after: bool },
    /// `ToDoWait` then `ToDoGo` — master follow Manhattan 3 (`crnonpl.cc:2769-2773`).
    QueuedWaitThenGo { via: &'static str },
    QueuedWait,
    /// No walk arm matched — fall through to roam tail (`crnonpl.cc:2902`).
    FallthroughRoam,
    Noway,
    Hold,
}

/// Which todo action ran — drives post-execute chaining.
#[derive(Debug)]
pub(crate) enum TodoExecuteKind {
    Go,
    Wait,
    Attack,
    DistanceAttack,
    AttackDeferred,
    /// F8 S3 — generic `CalculateDelay` gate deferral (e.g. two-object `Use` waiting on
    /// `EarliestMultiuseTime`). The wakeup was already armed by `todo_start_from_action`
    /// in the gate check, so the post-execute handler is a no-op — mirrors C++ `Execute`'s
    /// "Delay > 0 → schedule + break" (`cract.cc:795-801`).
    Deferred,
}

/// Driver for C++ `TCreature::Execute`'s `while (true)` loop (`cract.cc:783-898`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TodoExecuteLoopControl {
    /// Zero-delay queue tail — continue same wakeup.
    Continue,
    /// Delay armed, idle ran, stop requested, or queue empty — exit loop.
    Break,
}

impl GameWorld {
    /// 772 `TCreature::IdleStimulus` — dispatch on creature kind.
    ///
    /// Phase 1 walk-engine unification: widened to dispatch **players** to
    /// [`player_idle_stimulus`] (`crplayer.cc:388-405`) in addition to monsters
    /// (`crnonpl.cc:2386`). NPC-6: NPCs dispatch to [`GameWorld::npc_idle_stimulus`]
    /// (`crnonpl.cc:1718`).
    /// Phase 3: monsters run on the ToDo/IdleStimulus engine for **both** eras.
    /// Phase 4: players also run on the ToDo/IdleStimulus engine for both eras (1098
    /// player logic deleted). Phase 6 collapsed `beat_driven_loop` — both eras
    /// unconditionally use the beat-driven ToDo engine.
    pub(crate) fn idle_stimulus(&mut self, cid: CreatureId) {
        if !self.creatures.contains_key(cid) {
            return;
        }
        if self
            .creatures
            .get(cid)
            .is_some_and(|k| k.base().todo.locked)
        {
            return;
        }
        match self.creatures.get(cid) {
            Some(CreatureKind::Monster(_)) => {
                trace_creature_todo(self, cid, "idle_stimulus_enter");
                self.monster_idle_stimulus(cid);
                trace_creature_todo(self, cid, "idle_stimulus_exit");
            }
            Some(CreatureKind::Player(_)) => {
                trace_creature_todo(self, cid, "idle_stimulus_enter");
                self.player_idle_stimulus(cid);
                trace_creature_todo(self, cid, "idle_stimulus_exit");
            }
            Some(CreatureKind::Npc(_)) => {
                trace_creature_todo(self, cid, "idle_stimulus_enter");
                let mut sink = crate::npc::DialogueTrace::default();
                self.npc_idle_stimulus(cid, &mut sink);
                trace_creature_todo(self, cid, "idle_stimulus_exit");
            }
            _ => {}
        }
    }

    /// 772 `TPlayer::IdleStimulus` — `crplayer.cc:388-405`.
    ///
    /// Player idle handles **only** `Combat.AttackDest`: `ToDoAttack` → `ToDoStart`. The thrown
    /// `RESULT` → `ToDoClear` + `SendResult` (unless `NOERROR`/`NOWAY`) + `ToDoWait(1000)` +
    /// `ToDoStart` path is handled in [`Self::player_execute_attack`] (the `TDAttack` execute
    /// arm + `CanToDoAttack` chase, Phase 1.4). There is **no** separate follow re-path — follow
    /// lives in Combat (`CanToDoAttack`).
    ///
    /// With no attack target the player simply goes idle — the ToDo queue is empty and no
    /// wakeup is armed. This is the key fix for audit P2/P6: players no longer re-arm via the
    /// old `walk_queue` + `add_event_walk` path.
    fn player_idle_stimulus(&mut self, cid: CreatureId) {
        let has_attack_target = self
            .creatures
            .get(cid)
            .is_some_and(|k| k.base().attack_target.is_some());
        if !has_attack_target {
            // No attack target → idle drain complete, no re-arm (`crplayer.cc:388-405`).
            return;
        }
        // `Combat.AttackDest` is set → `ToDoAttack(); ToDoStart();` (`crplayer.cc:392-395`).
        // The thrown `RESULT` catch lives in `player_execute_attack` (the `TDAttack` execute arm).
        let _ = self.enqueue_creature_attack(cid);
        let attack_delay = self.todo_attack_delay_ms(cid);
        let delay = if attack_delay > 0 { attack_delay } else { 1 };
        self.todo_start_from_action(cid, delay);
    }

    /// Request idle when the action queue is drained — sync or deferred to next wakeup.
    ///
    /// Phase 1: widened from monster-only to all creatures on the unified ToDo path
    /// (players + monsters) via [`creature_uses_todo_execute`](Self::creature_uses_todo_execute).
    /// Phase 3: monsters use the ToDo path for both eras; `creature_uses_todo_execute`
    /// returns true for monsters unconditionally.
    pub(crate) fn request_idle_stimulus(&mut self, cid: CreatureId) {
        if !self.creature_uses_todo_execute(cid) {
            return;
        }
        if !self
            .creatures
            .get(cid)
            .is_some_and(|k| k.base().walk_timer_idle())
        {
            return;
        }
        if !self.creature_todo_queue_empty(cid) {
            return;
        }
        // Monster-only dedupe: one `IdleStimulus` pass per beat (`crnonpl.cc:2345`).
        // Players have no equivalent throttle — `IdleStimulus` is only called on queue drain.
        if self.creatures.get(cid).is_some_and(|k| {
            matches!(
                k,
                CreatureKind::Monster(m) if m.idle_stimulus_last_ms == Some(self.server_ms)
            )
        }) {
            return;
        }
        if self
            .creatures
            .get(cid)
            .is_some_and(|k| k.base().todo.has_wait())
        {
            return;
        }
        // C++ `ToDoYield` — schedule `ToDoWait(0)` + `ToDoStart`; `IdleStimulus` runs when
        // the todo list drains on wakeup, not inline from appear/move stimuli (`cract.cc:1001`).
        trace_creature_todo(self, cid, "request_idle_stimulus");
        self.creature_todo_yield(cid);
    }

    /// 772 `Execute` catch `EXHAUSTED` recovery (`cract.cc:870-877`):
    /// `ToDoClear() + ToDoWait(1000) + ToDoStart()`. The `Execute` catch does NOT clear `Target`
    /// itself — it relies on the throw site:
    /// - player-tile (`crnonpl.cc:2236-2238`): `Target = 0` before `throw EXHAUSTED`
    /// - kick-kill (`crnonpl.cc:2241-2242`): `KickCreature` returned false → `throw EXHAUSTED`
    ///   (Target NOT cleared)
    ///
    /// `clear_target` mirrors this distinction. The previous implementation unconditionally
    /// cleared the target, citing the `IdleStimulus` catch (`crnonpl.cc:2890-2898`) — wrong catch
    /// block (audit F3).
    pub(crate) fn monster_exhausted_wait(&mut self, cid: CreatureId, clear_target: bool) {
        if let Some(k) = self.creatures.get_mut(cid) {
            let base = k.base_mut();
            if clear_target {
                base.clear_targets(); // player-tile only (`crnonpl.cc:2237`)
            }
            base.walk_queue.clear();
            base.walk_destinations.clear();
            base.has_follow_path = false;
            base.force_update_follow_path = true;
            base.todo.queue.clear();
            base.todo.locked = false;
        }
        trace_creature_todo(self, cid, "monster_exhausted_wait");
        self.idle_enqueue_wait_and_start(cid, MONSTER_IDLE_WAIT_MS);
    }

    /// Apply combat damage and fire 772 `DamageStimulus` when a monster loses HP.
    ///
    /// C++ reference: `Game::combatChangeHealth` → `TMonster::DamageStimulus` — `crnonpl.cc:2278`,
    /// `TCreature::Damage` — `crmain.cc:486-760`.
    ///
    /// PC-2a M2/M3/M4: physical immunity, equipment damage reduction, and invisibility removal
    /// are handled here (the shared `Damage` path) before the HP delta is applied.
    /// PC-3 M3′/M5: non-physical immunities (`NoPoison`/`NoBurning`/`NoEnergy`/`NoLifeDrain`) and
    /// mana shield (`SKILL_MANASHIELD`) are handled here as well.
    pub(crate) fn combat_execute_with_stimulus(
        &mut self,
        attacker: Option<CreatureId>,
        target: CreatureId,
        damage: &CombatDamage,
        params: &CombatParams,
    ) -> bool {
        // NPCs are not attackable by default — TFS `Npc::isAttackable` / `isImmune`
        // (`npc.h:302-310`, `Npc::reset` sets `attackable = false`).
        // `Game::combatChangeHealth` poffs and returns without HP change (`game.cpp:4176-4180`).
        // Covers melee, ranged, spells, AoE, and field DoT that share this path.
        let target_is_npc = matches!(self.creatures.get(target), Some(CreatureKind::Npc(_)));
        if target_is_npc {
            let damaging = damage.primary.1 < 0 || damage.secondary.1 < 0;
            if damaging || params.apply_condition.is_some() {
                if let Some(pos) = self.creatures.get(target).map(|k| k.position()) {
                    // C++ `CONST_ME_POFF` / `EFFECT_POFF` (wire byte 3).
                    self.broadcast_magic_effect(pos, 3u8);
                }
                return false;
            }
        }

        // M1 — INVULNERABLE right check: C++ `Damage` checks `CheckRight(target, INVULNERABLE)`
        // and zeroes incoming damage for GMs with the invulnerability right (`crmain.cc:536-538`).
        // Maps to the TFS `PlayerFlag_CannotBeAttacked` group flag. Player targets only; monsters
        // use race-data immunities (handled below). Skipped for healing/positive deltas and
        // condition applications (C++ checks `Damage > 0` after the invulnerability gate, but the
        // gate itself runs unconditionally — a heal through `Damage(UNDEFINED)` would also be
        // zeroed. In practice `Damage` is only called with negative values for damage, so we gate
        // on `primary.1 < 0 || secondary.1 < 0` to avoid blocking heals/condition applies).
        if (damage.primary.1 < 0 || damage.secondary.1 < 0) && self.player_is_invulnerable(target) {
            if let Some(pos) = self.creatures.get(target).map(|k| k.position()) {
                // C++ `EFFECT_POFF` (wire byte 3) — `crmain.cc:578` (Damage <= 0 path).
                self.broadcast_magic_effect(pos, 3u8);
            }
            return false;
        }

        // M3 / M3′ — Typed immunities: C++ `Damage` checks `RaceData[Race].NoHit` (physical),
        // `NoPoison`, `NoBurning`, `NoEnergy`, `NoLifeDrain` and emits `EFFECT_BLOCK_HIT` (4) +
        // returns 0 for the matching `DamageType` (`crmain.cc:615-622`). Monster-only; players
        // don't have race immunities. Skipped when a condition is being applied (DoT paths handle
        // their own immunity check via `creature_immune_poison`).
        if params.apply_condition.is_none() {
            let immune = self.creatures.get(target).is_some_and(|k| match k {
                CreatureKind::Monster(m) => match damage.primary.0 {
                    CombatType::Physical => m.immunity_physical,
                    CombatType::Earth => m.immunity_poison,
                    CombatType::Fire => m.immunity_fire,
                    CombatType::Energy => m.immunity_energy,
                    CombatType::LifeDrain => m.immunity_life_drain,
                    _ => false,
                },
                _ => false,
            });
            if immune {
                if let Some(pos) = self.creatures.get(target).map(|k| k.position()) {
                    // C++ `EFFECT_BLOCK_HIT` (wire byte 4) — `crmain.cc:620`.
                    self.broadcast_magic_effect(pos, 4u8);
                }
                return false;
            }
        }

        // M2 — Equipment damage reduction: C++ `Damage` iterates equipped `PROTECTION`+`CLOTHES`
        // items and reduces incoming damage by `DAMAGEREDUCTION%` per item (`crmain.cc:540-574`).
        // The TFS 1.4.2 equivalent is `absorb_percent[combat_type]` on `ItemAbilities`, summed
        // across all equipped items. Player targets only (monsters/NPCs have no inventory).
        // Applied before the poff check, matching C++ order.
        let mut reduced_damage = *damage;
        if reduced_damage.primary.1 < 0 || reduced_damage.secondary.1 < 0 {
            let absorb_pct = self.player_absorb_percent(target, reduced_damage.primary.0);
            if absorb_pct > 0 {
                let factor = 100 - absorb_pct;
                reduced_damage.primary.1 = (reduced_damage.primary.1 * factor) / 100;
                reduced_damage.secondary.1 = (reduced_damage.secondary.1 * factor) / 100;
            }
        }

        // M5 — Mana shield: C++ `Damage` checks `SKILL_MANASHIELD` timer/value and absorbs
        // damage to mana first, only spilling the remainder into HP (`crmain.cc:662-688`).
        // TFS uses `CONDITION_MANASHIELD` (set by the "magic shield" spell / item ability). When
        // the target has the condition, subtract incoming damage from mana; if mana covers it
        // fully, emit the mana-hit effect + "You lose X mana" text and return without touching
        // HP. Otherwise drain mana to 0 and let the remainder flow to HP. Skipped for
        // `UNDEFINED` damage (C++ excludes it) and for healing/positive deltas.
        if reduced_damage.primary.1 < 0 {
            let absorbed = self.apply_mana_shield(target, -reduced_damage.primary.1);
            if absorbed > 0 {
                reduced_damage.primary.1 += absorbed;
                // Clamp secondary as well? C++ only shields the primary `Damage` scalar, so we
                // leave `secondary` untouched (it is 0 for wand/distance strikes and most spells).
            }
        }

        let stimulus_damage = (-(reduced_damage.primary.1 + reduced_damage.secondary.1)).max(0);
        if let Some(attacker_id) = attacker {
            if stimulus_damage > 0 {
                // C++ `DamageStimulus` runs before HP apply — `crmain.cc:631`, `694`.
                self.monster_damage_stimulus(target, attacker_id, stimulus_damage);
            }
            // Logout / swords icon — 772 `Attack` + `Damage` refresh (`crcombat.cc:601-602`,
            // `crmain.cc:525`, `TPlayer::DamageStimulus`). Initial lock on target *acquire* is
            // `SetAttackDest` → `AttackStimulus` (`combat_on_attack_dest_changed`).
            // - Attacker: `BlockLogout(60, Target->Type == PLAYER)` (no-op for monsters).
            // - Target: `BlockLogout(60, false)` — refresh even on 0-damage hits.
            // Skip pure heals (`primary/secondary > 0`). Covers melee, ranged, spells, AoE.
            let healing = reduced_damage.primary.1 > 0 || reduced_damage.secondary.1 > 0;
            if !healing {
                let target_is_player =
                    matches!(self.creatures.get(target), Some(CreatureKind::Player(_)));
                self.player_block_logout_infight(attacker_id, target_is_player);
                if target_is_player {
                    self.player_block_logout_infight(target, false);
                }
            }
        }
        let applied = crate::combat::execute(
            &mut self.creatures,
            attacker,
            target,
            &reduced_damage,
            params,
        );
        if applied {
            // C++ `magic.cc:1512` `CREATURE_SPEED_CHANGED` — announce when a speed-altering
            // condition (haste/paralyze) is applied via spell cast.
            if let Some(ref cond) = params.apply_condition {
                if matches!(cond.ctype, ConditionType::Haste | ConditionType::Paralyze) {
                    self.announce_creature_speed(target);
                }
            }
            let hp_after = self
                .creatures
                .get(target)
                .map(|k| k.base().health)
                .unwrap_or(0);

            // Hit graphical effect: physical uses blood family, typed damage uses the combat-type
            // effect (`TCreature::Damage`, `crmain.cc:706-765`; TFS `Game::combatGetTypeInfo`,
            // `game.cpp:3999`). Emitted for any damage that landed, including the killing blow
            // (C++ emits the effect before `Kill()`); the full-blood pool is added afterwards by
            // the death path.
            if stimulus_damage > 0 {
                if let Some(pos) = self.creatures.get(target).map(|k| k.position()) {
                    if reduced_damage.primary.0 == CombatType::Physical {
                        self.apply_physical_hit_blood(target, pos);

                        // M4 — Invisibility removal on hit: C++ `Damage` clears non-player
                        // invisibility (`SKILL_ILLUSION` timer → restore original outfit +
                        // announce) when damage lands (`crmain.cc:636-641`). Players keep
                        // invisibility through damage (C++ gates on `this->Type != PLAYER`).
                        if !matches!(self.creatures.get(target), Some(CreatureKind::Player(_))) {
                            self.clear_nonplayer_invisibility(target);
                        }
                    } else if let Some(effect) =
                        crate::combat::combat_type_hit_effect(reduced_damage.primary.0)
                    {
                        self.broadcast_magic_effect(pos, effect);
                    }

                    if reduced_damage.secondary.1 < 0
                        && reduced_damage.secondary.0 != CombatType::Physical
                    {
                        if let Some(effect) =
                            crate::combat::combat_type_hit_effect(reduced_damage.secondary.0)
                        {
                            self.broadcast_magic_effect(pos, effect);
                        }
                    }
                }
            }

            if hp_after <= 0 && self.creatures.contains_key(target) {
                self.apply_creature_death(target);
            }
        }
        applied
    }

    /// M5 — Mana shield absorb. C++ `Damage` checks `SKILL_MANASHIELD` timer/value
    /// (`crmain.cc:662-688`); TFS uses `CONDITION_MANASHIELD`. Returns the amount of `incoming`
    /// damage absorbed from mana (caller adds it back to the HP delta so the remainder flows to
    /// HP). Emits the mana-hit graphical effect + "You lose X mana" status message when any
    /// damage is absorbed. No-op for non-players or players without the condition.
    pub(crate) fn apply_mana_shield(&mut self, target: CreatureId, incoming: i32) -> i32 {
        if incoming <= 0 {
            return 0;
        }
        let has_shield = self.creatures.get(target).is_some_and(|k| match k {
            CreatureKind::Player(p) => p
                .base
                .active_conditions
                .iter()
                .any(|c| c.ctype == ConditionType::ManaShield),
            _ => false,
        });
        if !has_shield {
            return 0;
        }
        // Borrow the player mutably to drain mana.
        let Some(CreatureKind::Player(p)) = self.creatures.get_mut(target) else {
            return 0;
        };
        let mana_points = p.mana.max(0);
        let absorbed = incoming.min(mana_points);
        p.mana -= absorbed;
        let pos = p.base.position;
        let mana_after = p.mana;
        let _ = p;

        if absorbed > 0 {
            // C++ `EFFECT_MANA_HIT` — `crmain.cc:670`. The 772 client effect id for the blue
            // mana-hit spark. TFS uses `CONST_ME_MAGIC_BLUE` (wire byte 13).
            self.broadcast_magic_effect(pos, 13u8);
            // Animated "X" damage text in blue (C++ `TextualEffect COLOR_BLUE` — `crmain.cc:671`).
            use tfs_rust_net::codec::wire::AnimatedTextWire;
            let animated = self.codec.encode_animated_text(&AnimatedTextWire {
                pos,
                color: 5, // COLOR_BLUE — `crmain.cc:671`.
                text: absorbed.to_string(),
            });
            if !animated.as_bytes().is_empty() {
                self.broadcast_to_spectators(pos, animated.into_bytes());
            }
            // Private "You lose X mana" status message + stats update (player-only by construction).
            self.send_player_stats(target);
            self.send_player_status_message(target, &format!("You lose {absorbed} mana."));
            // If fully absorbed, the caller's `reduced_damage.primary.1` becomes 0 and HP is
            // untouched. If partially absorbed, the remainder flows to HP below.
            let _ = mana_after;
        }
        absorbed
    }

    /// M2 — Sum `absorb_percent[combat_type]` across all equipped items for a player target.
    /// C++ `Damage` iterates equipped `PROTECTION`+`CLOTHES` items (`crmain.cc:540-574`);
    /// the TFS 1.4.2 equivalent is `ItemAbilities.absorb_percent` keyed by `CombatType`.
    /// Returns 0 for non-players or when no items have absorb for the given type.
    fn player_absorb_percent(&self, cid: CreatureId, combat_type: CombatType) -> i32 {
        let slots = match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.equipment_slots,
            _ => return 0,
        };
        let absorb_idx = tfs_rust_content::item_abilities::combat_absorb_index(combat_type);
        let mut total: i32 = 0;
        for slot_iid in slots.iter().flatten().copied() {
            let Some(item) = self.items.get(slot_iid) else {
                continue;
            };
            let Some(it) = self.items_db.items.get(&item.item_type) else {
                continue;
            };
            total += it.abilities.absorb_percent[absorb_idx] as i32;
        }
        total
    }

    /// M4 — Clear `ConditionType::Invisible` from a non-player creature and announce the outfit
    /// change. C++ `Damage` sets `SKILL_ILLUSION` timer to 0 (clearing invisibility), restores
    /// `OrgOutfit`, and calls `AnnounceChangedCreature(CREATURE_OUTFIT_CHANGED)` +
    /// `NotifyAllCreatures(OBJECT_CHANGED)` (`crmain.cc:636-641`). No-op if the creature has no
    /// invisible condition. Player targets are excluded by the caller (C++ gates on
    /// `this->Type != PLAYER`).
    fn clear_nonplayer_invisibility(&mut self, cid: CreatureId) {
        let had_invisible = self.creatures.get(cid).is_some_and(|k| {
            k.base()
                .active_conditions
                .iter()
                .any(|c| c.ctype == ConditionType::Invisible)
        });
        if !had_invisible {
            return;
        }
        // Snapshot the outfit + position + wire id before mutation (avoid holding borrows).
        let (pos, wire_id, outfit_wire) = match self.creatures.get(cid) {
            Some(kind) => {
                let pos = kind.position();
                let wire_id = crate::login_out::creature_wire_id(cid, kind);
                let outfit = kind.base().outfit.clone();
                let wire = tfs_rust_net::creature_encode::OutfitWire {
                    look_type: outfit.look_type.max(0) as u16,
                    look_head: outfit.look_head.clamp(0, 255) as u8,
                    look_body: outfit.look_body.clamp(0, 255) as u8,
                    look_legs: outfit.look_legs.clamp(0, 255) as u8,
                    look_feet: outfit.look_feet.clamp(0, 255) as u8,
                    look_addons: outfit.look_addons.clamp(0, 255) as u8,
                    look_mount: 0,
                    look_type_ex: 0,
                };
                (pos, wire_id, wire)
            }
            None => return,
        };
        if let Some(kind) = self.creatures.get_mut(cid) {
            kind.base_mut()
                .active_conditions
                .retain(|c| c.ctype != ConditionType::Invisible);
        }
        // Announce the outfit change so spectators see the creature reappear.
        // Mirrors C++ `AnnounceChangedCreature(CREATURE_OUTFIT_CHANGED)`.
        let msg = self.codec.encode_creature_outfit(wire_id, &outfit_wire);
        self.broadcast_to_spectators(pos, msg.into_bytes());
    }

    /// C++ `TMonster::DamageStimulus` — `crnonpl.cc:2278`.
    pub(crate) fn monster_damage_stimulus(
        &mut self,
        victim_id: CreatureId,
        attacker_id: CreatureId,
        damage: i32,
    ) {
        if damage <= 0 || attacker_id == victim_id {
            return;
        }
        let snapshot = {
            let Some(CreatureKind::Monster(m)) = self.creatures.get(victim_id) else {
                return;
            };
            let has_target = m.base.attack_target.is_some() || m.base.follow_target.is_some();
            let old_state = m.state;
            let was_sleeping = old_state == MonsterState::Sleeping;
            let new_state = if was_sleeping {
                if has_target {
                    MonsterState::UnderAttack
                } else {
                    MonsterState::Panic
                }
            } else if !has_target {
                MonsterState::Panic
            } else if old_state == MonsterState::Idle {
                MonsterState::UnderAttack
            } else {
                old_state
            };
            (
                old_state,
                new_state,
                has_target,
                was_sleeping,
                m.base.name.clone(),
            )
        };
        let (old_state, new_state, has_target, was_sleeping, name) = snapshot;
        let state_changed = new_state != old_state;

        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(victim_id) {
            m.state = new_state;
            if new_state == MonsterState::Panic || new_state == MonsterState::UnderAttack {
                m.is_idle = false;
            }
            if !has_target {
                m.opponent_ids.retain(|&id| id != attacker_id);
                if !m.opponent_ids.contains(&attacker_id) {
                    m.opponent_ids.push(attacker_id);
                }
            }
        }

        if chase_debug::chase_path_debug_enabled() {
            chase_debug::log_damage_stimulus(
                self.chase_trace_tick(),
                victim_id,
                name.as_str(),
                Self::monster_state_trace_str(old_state),
                Self::monster_state_trace_str(new_state),
                attacker_id.data().as_ffi(),
                damage,
                has_target,
            );
        }

        if state_changed || was_sleeping {
            if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(victim_id) {
                // First melee after damage lands on the second post-damage idle (`tick=4000` in panic sim).
                m.base.delay_attack_ms(self.server_ms, 4000);
            }
            self.creature_todo_yield(victim_id);
        }
        // C++ `TMonster::DamageStimulus` — state + `ToDoYield` only (`crnonpl.cc:2304`);
        // target pick is idle `Strategy[]`, not synchronous `searchTarget`.
        // Phase 3: 1098 synchronous `searchTarget` deleted — both eras use idle `Strategy[]`.
    }

    fn monster_state_trace_str(state: MonsterState) -> &'static str {
        match state {
            MonsterState::Sleeping => "sleeping",
            MonsterState::Idle => "idle",
            MonsterState::UnderAttack => "under_attack",
            MonsterState::Attacking => "attacking",
            MonsterState::Panic => "panic",
        }
    }

    /// C++ `TMonster::CreatureMoveStimulus` sleep wake — `crnonpl.cc:2943-2982`.
    /// Phase 3: runs for both eras (772 monster AI is the single system).
    pub(crate) fn monster_sleep_wake_on_creature_move(
        &mut self,
        monster_id: CreatureId,
        moved_id: CreatureId,
    ) {
        let sleeping = self.creatures.get(monster_id).is_some_and(
            |k| matches!(k, CreatureKind::Monster(m) if m.state == MonsterState::Sleeping),
        );
        if !sleeping {
            return;
        }
        if moved_id == monster_id {
            if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(monster_id) {
                m.state = MonsterState::Idle;
                m.is_idle = false;
            }
            self.creature_todo_yield(monster_id);
            return;
        }
        // C++ wake gate (`crnonpl.cc:2969-2975`): NPC → no; Monster → only if
        // `IsPlayerControlled()` (master is a PLAYER, `crnonpl.cc:3139-3146`);
        // Player → yes. Wild monsters and NPC-owned summons do NOT wake sleepers.
        // `is_summon()` alone is too broad (any master) — require a player master.
        let should_wake = match self.creatures.get(moved_id) {
            None => false,
            Some(CreatureKind::Npc(_)) => false,
            Some(CreatureKind::Player(_)) => true,
            Some(CreatureKind::Monster(m)) => m.base.master.is_some_and(|mid| {
                matches!(self.creatures.get(mid), Some(CreatureKind::Player(_)))
            }),
        };
        if !should_wake {
            return;
        }
        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(monster_id) {
            m.state = MonsterState::Idle;
            m.is_idle = false;
        }
        self.creature_todo_yield(monster_id);
    }

    /// Roll 772 `Strategy[]` bucket — `crnonpl.cc:2424` (last bucket is random).
    fn monster_idle_roll_strategy_from_roll(
        nearest: u8,
        health: u8,
        damage: u8,
        mut roll: i32,
    ) -> u8 {
        let thresholds = [nearest, health, damage];
        for (idx, &threshold) in thresholds.iter().enumerate() {
            if roll < i32::from(threshold) {
                return idx as u8;
            }
            roll -= i32::from(threshold);
        }
        3
    }

    /// C++ target validity + `LoseTarget` — `crnonpl.cc:2368-2384`.
    /// 772 summon despawn / re-bind block — `crnonpl.cc:2359–2405`.
    ///
    /// Runs at the top of `IdleStimulus` for summons (`Master != 0`). Despawns when the master is
    /// gone, on a different floor (non-player master), or beyond 30 tiles / `|Δz| > 1`. Otherwise
    /// re-binds the summon's target: `Master->Combat.Following ? Target=0 : Target=Master->AttackDest`,
    /// falling back to `Target=Master` when that clears or points at self.
    ///
    /// Returns `true` when the summon was despawned (caller must early-return — the creature is gone).
    fn monster_idle_summon_lifecycle(&mut self, cid: CreatureId) -> bool {
        let (master_id, master_is_player, summon_pos) = match self.creatures.get(cid) {
            Some(k) => match k.base().master {
                Some(m) => (
                    m,
                    matches!(self.creatures.get(m), Some(CreatureKind::Player(_))),
                    k.position(),
                ),
                None => return false, // Not a summon — skip the block.
            },
            None => return false,
        };

        let master_present = self.creatures.contains_key(master_id);
        let should_despawn = if !master_present {
            // C++ `Master == NULL` → despawn (`crnonpl.cc:2363`).
            tracing::debug!(
                ?cid,
                ?master_id,
                master_is_player,
                "summon despawn: master gone"
            );
            true
        } else {
            let master_pos = self
                .creatures
                .get(master_id)
                .map(|k| k.position())
                .unwrap_or(summon_pos);
            // C++ non-player master on a different floor → despawn (`crnonpl.cc:2373`).
            if !master_is_player && master_pos.z != summon_pos.z {
                tracing::debug!(
                    ?cid,
                    ?master_id,
                    "summon despawn: monster master on different floor"
                );
                true
            } else {
                // C++ `|Δz| > 1 || |Δx| > 30 || |Δy| > 30` → despawn (`crnonpl.cc:2376`).
                let dz = (master_pos.z as i32 - summon_pos.z as i32).unsigned_abs();
                let dx = (master_pos.x as i32 - summon_pos.x as i32).unsigned_abs();
                let dy = (master_pos.y as i32 - summon_pos.y as i32).unsigned_abs();
                if dz > 1 || dx > 30 || dy > 30 {
                    tracing::debug!(
                        ?cid,
                        ?master_id,
                        dz,
                        dx,
                        dy,
                        "summon despawn: too far from master"
                    );
                    true
                } else {
                    false
                }
            }
        };

        if should_despawn {
            // C++ player master → `StartLogout(true, true)`; monster master → `Kill()` (`crnonpl.cc:2388`).
            // Both paths set `State = SLEEPING` and return. Rust `remove_creature` covers the
            // disappear broadcast + summon-chain cleanup; `apply_creature_death` is reserved for
            // combat kills (loot/XP), not lifecycle despawns.
            self.remove_creature(cid);
            return true;
        }

        // Re-bind — `crnonpl.cc:2397–2405`.
        //
        // `Combat.Following` is the player follow-mode flag (`SetAttackDest(…, Follow=true)`),
        // NOT "has a chase `follow_target`". Monsters always set `follow_target` while
        // attacking — treating that as Following made summons Target=0 → fall back to Master
        // (giant spider) instead of AttackDest (the player).
        //
        // Player mapping: follow packet sets `follow_target`; attack packet clears it
        // (`player_set_attack_dest`). Monster masters: Following is always false.
        let (master_following, master_attack_dest) = match self.creatures.get(master_id) {
            Some(CreatureKind::Player(p)) => (
                p.base.follow_target.is_some(),
                p.base.attack_target,
            ),
            Some(k) => (false, k.base().attack_target),
            None => (false, None),
        };

        let new_target = if master_following {
            None // `Target = 0`
        } else {
            master_attack_dest // `Target = Master->Combat.AttackDest`
        };
        // C++ `if (Target == 0 || Target == self) Target = Master` (`crnonpl.cc:2403`).
        let new_target = match new_target {
            Some(t) if t != cid => Some(t),
            _ => Some(master_id),
        };

        // Apply via the existing target helpers so follow/attack stay aligned. The later
        // `monster_think_summon_stub` pass refines the follow target; this block sets the
        // authoritative attack-dest per C++.
        if let Some(target_id) = new_target {
            // 772 assigns `Target` only here (`crnonpl.cc:2397-2404`). `Combat.AttackDest` is
            // set in the walk prelude via `SetAttackDest` (`:2784`) — do not write
            // `attack_target` or fire AttackStimulus here.
            let _ = self.monster_set_follow_creature(cid, Some(target_id));
        } else {
            let _ = self.monster_set_follow_creature(cid, None);
            if let Some(k) = self.creatures.get_mut(cid) {
                k.base_mut().attack_target = None;
            }
        }

        false
    }

    fn monster_idle_lose_existing_target(&mut self, cid: CreatureId) {
        let target_id = self.creatures.get(cid).and_then(|k| k.base().follow_target);
        let Some(target_id) = target_id else {
            return;
        };
        if self.monster_idle_should_lose_target(cid, target_id) {
            // 772 only clears `Target` (`crnonpl.cc:2433`) — not `Combat.AttackDest`.
            if let Some(k) = self.creatures.get_mut(cid) {
                k.base_mut().follow_target = None;
            }
        }
    }

    fn monster_idle_should_lose_target(&self, cid: CreatureId, target_id: CreatureId) -> bool {
        let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) else {
            return true;
        };
        if m.base.master.is_some() {
            return false;
        }
        let Some(target) = self.creatures.get(target_id) else {
            return true;
        };
        let pos = m.base.position;
        let tp = target.position();
        if tp.z != pos.z {
            return true;
        }
        if (pos.x as i32 - tp.x as i32).unsigned_abs() > 10
            || (pos.y as i32 - tp.y as i32).unsigned_abs() > 10
        {
            return true;
        }
        if let Some(tile) = self.map.get_tile(tp) {
            // C++ `crnonpl.cc:2426` `IsProtectionZone(Target->posx, …)`.
            if tile.body().zone == ZoneType::Protection {
                return true;
            }
            // C++ `crnonpl.cc:2427` `IsHouse(Target->posx, Target->posy, Target->posz)`
            // (AI#25). House tiles are a hard lose-target — monsters don't chase into houses.
            if matches!(tile, crate::tile::Tile::House(_)) {
                return true;
            }
        }
        // C++ `crnonpl.cc:2429` `(Target->IsInvisible() && !RaceData[this->Race].SeeInvisible)`
        // (AI#25). Monsters without `SeeInvisible` lose invisible targets.
        if target.base().is_invisible() && !m.see_invisible {
            return true;
        }
        // C++ `|| (Master==0 && random(0,99) < LoseTarget)` — draw always when no master
        // (`crnonpl.cc:2381`), even at LoseTarget=0.
        if m.base.master.is_none() {
            let _trace = crate::sim_glibc_rand::sim_rng_trace_site("idle_lose_target");
            let roll = self.parity_random(0, 99);
            if roll < i32::from(m.lose_target_percent) {
                return true;
            }
        }
        false
    }

    /// C++ `TFindCreatures` + `Strategy[]` target pick — `crnonpl.cc:2420-2516`.
    ///
    /// C++ `TCreature::CanSeeFloor` — `cr.hh:576-582`.
    ///
    /// Used by idle acquire's `ShouldSleep` gate (`crnonpl.cc:2504`): a player/summon that can
    /// see the monster's floor keeps the monster awake even when it cannot be targeted
    /// (different Z / outside the 10-tile box).
    #[inline]
    fn creature_can_see_floor(viewer_z: u8, floor_z: u8) -> bool {
        if viewer_z <= 7 {
            floor_z <= 7
        } else {
            (viewer_z as i32 - floor_z as i32).abs() <= 2
        }
    }

    /// Z floors `TFindCreatures` can hit for idle acquire — XY search is 12×12; CipSoft's
    /// creature chain is XY-only so other floors in that box appear. Mirror that with a
    /// compact per-era Z span instead of scanning the whole map.
    fn idle_acquire_search_z_range(monster_z: u8) -> std::ops::RangeInclusive<u8> {
        if monster_z <= 7 {
            0..=7
        } else {
            let min_z = monster_z.saturating_sub(2);
            let max_z = (monster_z + 2).min(15);
            min_z..=max_z
        }
    }

    /// Returns `true` when idle should stop (monster entered sleep).
    ///
    /// C++ `IdleStimulus` targeting block — `crnonpl.cc:2470-2557`.
    fn monster_idle_acquire_target(&mut self, cid: CreatureId) -> bool {
        let snapshot = self.creatures.get(cid).and_then(|k| {
            let CreatureKind::Monster(m) = k else {
                return None;
            };
            if m.base.is_summon() || m.base.master.is_some() {
                return None;
            }
            Some((
                m.base.position,
                m.base.follow_target,
                m.state,
                m.strategy_nearest,
                m.strategy_health,
                m.strategy_damage,
                m.see_invisible,
            ))
        });
        let Some((pos, existing_follow, _state, strat_near, strat_hp, strat_dmg, see_invisible)) =
            snapshot
        else {
            return false;
        };

        let has_target = existing_follow.is_some();
        if has_target {
            return false;
        }

        let strategy =
            Self::monster_idle_roll_strategy_from_roll(strat_near, strat_hp, strat_dmg, {
                let _trace = crate::sim_glibc_rand::sim_rng_trace_site("idle_strategy");
                self.parity_random(0, 99)
            });
        let mut should_sleep = true;
        let mut best_param = i32::MIN;
        let mut best_id = None;
        let mut best_tie = 0i32;

        // C++ `TFindCreatures Search(12, 12, …, FIND_PLAYERS | FIND_MONSTERS)` — XY box only;
        // chain membership spans floors, so scan the CanSeeFloor-relevant Z set.
        // IDLE-3: 16×16 sector order + generation-marked dedup (no SlotMap-key sort).
        self.scratch_spectators.clear();
        let gen = self.bump_spectator_gen();
        let mut sector_buf = std::mem::take(&mut self.scratch_sector_buf);
        sector_buf.clear();
        for z in Self::idle_acquire_search_z_range(pos.z) {
            self.map.grid.collect_spectators_sector_order(
                pos.x,
                pos.y,
                z,
                12,
                12,
                &mut sector_buf,
            );
            for target_id in sector_buf.drain(..) {
                if self.spectator_mark_new(target_id, gen) {
                    self.scratch_spectators.push(target_id);
                }
            }
        }
        self.scratch_sector_buf = sector_buf;
        self.obs
            .record_idle_candidates(self.scratch_spectators.len());

        for target_id in self.scratch_spectators.iter().copied() {
            if target_id == cid {
                continue;
            }
            let Some(target) = self.creatures.get(target_id) else {
                continue;
            };
            // C++ `crnonpl.cc:2500-2502`: wild (non-player-controlled) monsters are skipped
            // **before** `CanSeeFloor` / targeting. Using them for `ShouldSleep` kept entire
            // spawn packs awake forever (each rat prevented every other rat from sleeping).
            if matches!(target, CreatureKind::Monster(m) if !m.base.is_summon()) {
                continue;
            }
            // FIND_PLAYERS | FIND_MONSTERS — NPCs are not in the search mask.
            if matches!(target, CreatureKind::Npc(_)) {
                continue;
            }

            let tp = target.position();
            // C++ `Target->CanSeeFloor(this->posz)` — `crnonpl.cc:2504`.
            if Self::creature_can_see_floor(tp.z, pos.z) {
                should_sleep = false;
            }

            // Targeting requires same Z + ≤10 axis box (`crnonpl.cc:2512-2518`).
            if tp.z != pos.z {
                continue;
            }
            let dx = (tp.x as i32 - pos.x as i32).abs();
            let dy = (tp.y as i32 - pos.y as i32).abs();
            if dx > 10 || dy > 10 {
                continue;
            }
            if let Some(tile) = self.map.get_tile(tp) {
                if tile.body().zone == ZoneType::Protection {
                    continue;
                }
                // C++ `IsHouse` — `crnonpl.cc:2516`.
                if matches!(tile, crate::tile::Tile::House(_)) {
                    continue;
                }
            }
            if matches!(target, CreatureKind::Player(p) if {
                let flags = flags_for_group(&self.groups, p.group_id);
                has_player_flag(flags, PLAYER_FLAG_IGNORED_BY_MONSTERS)
            }) {
                continue;
            }
            // C++ `crnonpl.cc:2514` `(Target->IsInvisible() && !RaceData[Race].SeeInvisible)`.
            if target.base().is_invisible() && !see_invisible {
                continue;
            }
            let param = match strategy {
                0 => -(dx + dy),
                1 => -target.base().health,
                2 => self
                    .creatures
                    .get(cid)
                    .map(|k| k.base().damage_map.get(&target_id).copied().unwrap_or(0) as i32)
                    .unwrap_or(0),
                _ => 0,
            };
            let tie = self.parity_random(0, 99);
            if param > best_param || (param == best_param && tie > best_tie) {
                best_param = param;
                best_tie = tie;
                best_id = Some(target_id);
            }
        }

        if let Some(target_id) = best_id {
            self.monster_add_opponent(cid, target_id, true);
            let _ = self.monster_select_target(cid, target_id);
        }

        let state = self.creatures.get(cid).and_then(|k| match k {
            CreatureKind::Monster(m) => Some(m.state),
            _ => None,
        });
        let still_no_target = self
            .creatures
            .get(cid)
            .is_none_or(|k| k.base().follow_target.is_none());

        if should_sleep
            && still_no_target
            && !matches!(state, Some(MonsterState::UnderAttack | MonsterState::Panic))
        {
            // C++ wild: `State = SLEEPING; return;` — no `ToDoStart` (`crnonpl.cc:2550-2556`).
            if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
                m.state = MonsterState::Sleeping;
                m.is_idle = true;
                m.base.clear_targets();
                m.base.todo.queue.clear();
                m.base.todo.locked = false;
                m.base.walk_queue.clear();
                m.base.walk_destinations.clear();
                m.base.next_wakeup = None;
            }
            return true;
        }

        if state == Some(MonsterState::Panic) && still_no_target {
            if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
                m.state = MonsterState::Idle;
            }
        }
        if state == Some(MonsterState::UnderAttack) {
            if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
                m.state = MonsterState::Idle;
            }
        }
        false
    }

    /// Resolve cast target — 772 `this->Target` (`crnonpl.cc:2572`).
    /// Rust: `follow_target` = C++ Target; `attack_target` = Combat.AttackDest (melee only).
    fn monster_cast_target_id(base: &CreatureBase) -> Option<CreatureId> {
        base.follow_target
    }

    /// Tile set for a spell shape — `crnonpl.cc:2627`; shape spell bodies in `magic.cc`.
    ///
    /// - `Angle`: 772 `AngleShapeSpell` (`magic.cc:550`) — forward cone by **caster direction**
    ///   (`Forward=1..=length`, `Across = ±Forward*spread/90`), NOT a target-vector line.
    /// - `Destination`: 772 `DestinationShapeSpell` → `CircleShapeSpell` at victim tile
    ///   (`magic.cc:537`) — circle of `radius` around the target.
    /// - `Origin`: 772 `OriginShapeSpell` (`magic.cc:503`) — circle of `radius` around caster.
    fn monster_idle_spell_tiles(
        spell: &MonsterSpell,
        caster_pos: Position,
        caster_dir: Direction,
        target_pos: Position,
    ) -> Vec<Position> {
        let clamp = |x: i32, y: i32| {
            Position::new(
                x.clamp(0, u16::MAX as i32) as u16,
                y.clamp(0, u16::MAX as i32) as u16,
                caster_pos.z,
            )
        };
        match spell.shape {
            SpellShape::Actor => vec![caster_pos],
            SpellShape::Victim => vec![target_pos],
            SpellShape::Destination | SpellShape::Origin => {
                let center = match spell.shape {
                    SpellShape::Destination => target_pos,
                    _ => caster_pos,
                };
                // 772 `ExecuteCircleSpell` (`magic.cc:459`) iterates `Circle[0..=R]` from
                // `circles.dat` — a proper disc, NOT a Chebyshev square. Reuse the baked
                // `disc_offsets` (verified vs `circles.dat` / 1098 `setupArea`).
                disc_offsets(spell.radius.max(0) as usize)
                    .into_iter()
                    .map(|(dx, dy)| clamp(center.x as i32 + dx, center.y as i32 + dy))
                    .collect()
            }
            SpellShape::Angle => {
                // 772 `AngleShapeSpell` — `magic.cc:550-588`. Walks forward `length` steps in
                // the caster's facing direction, spreading `±Forward*Angle/90` across.
                // TFS data-pack `spread` maps to 772 `Angle` as `spread * 10`
                // (dragon.xml `spread=3` ↔ dragon.mon `Angle(30,8,7)`; demon `spread=0` ↔ `Angle(0,8,…)`).
                let mut tiles = Vec::new();
                let range = spell.length.max(0);
                let angle = spell.spread.max(0) * 10;
                let (fx, fy) = match caster_dir {
                    Direction::North => (0, -1),
                    Direction::East => (1, 0),
                    Direction::South => (0, 1),
                    Direction::West => (-1, 0),
                    // Non-cardinal facing: fall back to longest-axis step (rare for monsters).
                    Direction::NorthEast => (1, -1),
                    Direction::SouthEast => (1, 1),
                    Direction::SouthWest => (-1, 1),
                    Direction::NorthWest => (-1, -1),
                };
                let (ax, ay) = (-fy, fx); // across-axis (perpendicular to forward)
                for forward in 1..=range {
                    let half = (forward * angle) / 90;
                    for across in -half..=half {
                        let x = caster_pos.x as i32 + fx * forward + ax * across;
                        let y = caster_pos.y as i32 + fy * forward + ay * across;
                        tiles.push(clamp(x, y));
                    }
                }
                tiles
            }
        }
    }

    /// C++ CASTING block — `crnonpl.cc:2521-2667`.
    fn monster_idle_try_casting(&mut self, cid: CreatureId) {
        let (spell_len, cast_target, pos) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) if !m.spells.is_empty() => (
                m.spells.len(),
                Self::monster_cast_target_id(&m.base),
                m.base.position,
            ),
            _ => return,
        };
        // Attack + defense spells are merged at spawn (`combat_from_monster_type`, audit IDLE-1).
        // 772 CASTING (`crnonpl.cc:2521-2667`): Target may be 0 — the loop still runs,
        // consuming delay + flee rolls for every spell. Non-aggressive spells (Healing)
        // pass the `!isAggressive()` gate and cast on self; aggressive spells skip
        // (`crnonpl.cc:2682`: `!isAggressive() || (Target != 0 && Target != Master)`).
        let target_id = cast_target;
        let target_pos = target_id.and_then(|tid| self.creatures.get(tid).map(|k| k.position()));
        let fleeing = self
            .creatures
            .get(cid)
            .is_some_and(|k| matches!(k, CreatureKind::Monster(m) if m.is_fleeing()));

        for spell_idx in 0..spell_len {
            let Some(spell) = self.creatures.get(cid).and_then(|k| match k {
                CreatureKind::Monster(m) => m.spells.get(spell_idx).cloned(),
                _ => None,
            }) else {
                break;
            };
            if spell.delay <= 0 || self.parity_rand_mod(spell.delay as u32) != 0 {
                continue;
            }
            if fleeing && self.parity_random(1, 3) != 1 {
                continue;
            }

            // 772 `isAggressive` gate (`crnonpl.cc:2682`):
            // `if(!Impact->isAggressive() || (this->Target != 0 && this->Target != this->Master))`
            // Non-aggressive spells (Healing) always cast; aggressive spells need a valid
            // target that isn't the master.
            let is_aggressive = spell.impact.is_aggressive();
            if is_aggressive {
                let Some(tid) = target_id else { continue };
                if self.creatures.get(cid).is_some_and(|k| {
                    matches!(k, CreatureKind::Monster(m) if m.base.master == Some(tid))
                }) {
                    continue;
                }
            }
            let Some(target_pos) = target_pos else {
                // No target — only non-aggressive spells with self-centered shapes can fire.
                // 772 shape dispatch: Actor/Origin/Angle don't need Target; Victim/Destination
                // check `if(Target != NULL)`.
                if !is_aggressive {
                    match spell.shape {
                        SpellShape::Actor => {
                            if let Some(effect) = spell.area_effect {
                                self.broadcast_magic_effect(pos, effect);
                            }
                            self.monster_idle_apply_spell_impact(cid, cid, &spell);
                        }
                        SpellShape::Origin | SpellShape::Angle => {
                            let caster_dir = self
                                .creatures
                                .get(cid)
                                .map(|k| k.base().direction)
                                .unwrap_or(Direction::North);
                            let tiles = Self::monster_idle_spell_tiles(
                                &spell, pos, caster_dir, pos,
                            );
                            for tile in tiles {
                                if !self.monster_sight_clear(pos, tile) {
                                    continue;
                                }
                                if let Some(effect) = spell.area_effect {
                                    self.broadcast_magic_effect(tile, effect);
                                }
                                let victims: Vec<CreatureId> = self
                                    .map
                                    .get_tile(tile)
                                    .map(|t| t.body().creatures.clone())
                                    .unwrap_or_default();
                                for victim_id in victims {
                                    if victim_id == cid {
                                        continue;
                                    }
                                    self.monster_idle_apply_spell_impact(cid, victim_id, &spell);
                                }
                            }
                        }
                        // Victim/Destination require a target — skip.
                        _ => {}
                    }
                }
                continue;
            };

            let dist = chebyshev(pos, target_pos);
            if spell.range > 0 && dist > spell.range {
                continue;
            }
            // C++ `VictimShapeSpell` (`magic.cc:423`) and `CircleShapeSpell` (used by
            // `DestinationShapeSpell`, `magic.cc:522`) both check `Actor->posz != DestZ`
            // → return. `OriginShapeSpell`/`AngleShapeSpell` use `Actor->posz` for all
            // tiles (implicitly same-Z). `Actor` is self-cast. So only `Victim` and
            // `Destination` need the gate — `Origin`/`Angle` tiles already use `caster_pos.z`
            // (`monster_idle_spell_tiles` lines 684, 701).
            if matches!(spell.shape, SpellShape::Victim | SpellShape::Destination)
                && pos.z != target_pos.z
            {
                continue;
            }
            // 772 CASTING (`crnonpl.cc:2521-2667`) has no adjacent-melee spell suppression —
            // melee and spells run on independent cooldowns. The dragon casts fire wave/fireball
            // even when adjacent to the target. Range checks are inside the shape spell
            // functions (`VictimShapeSpell` `magic.cc:423`, `CircleShapeSpell` `magic.cc:520`),
            // NOT in the CASTING block. `AngleShapeSpell` has no range check at all.

            // 772 `SHAPE_ANGLE` calls `this->Rotate(Target)` before building the beam
            // (`crnonpl.cc:2725`), so the cone follows the freshly-faced direction.
            // At this point `target_id` is guaranteed `Some` — aggressive spells without
            // a target `continue`d above, and non-aggressive without target took the
            // self-cast `continue` path.
            let tid = target_id.unwrap();
            let caster_dir = if matches!(spell.shape, SpellShape::Angle) {
                self.monster_face_toward(cid, tid, false);
                self.creatures
                    .get(cid)
                    .map(|k| k.base().direction)
                    .unwrap_or(Direction::North)
            } else {
                Direction::North
            };
            let tiles = Self::monster_idle_spell_tiles(&spell, pos, caster_dir, target_pos);

            match spell.shape {
                SpellShape::Victim => {
                    if !self.monster_sight_clear(pos, target_pos) {
                        continue;
                    }
                    // Face target for the cast. Wire 0x6B is deferred to the idle combat
                    // rotate tail (or suppressed when a Go is pending) so Victim spells do
                    // not spam stand-still turns between chase batches.
                    self.monster_face_toward(cid, tid, false);
                    if let Some(shoot) = spell.shoot_effect {
                        self.broadcast_distance_shoot(pos, target_pos, shoot);
                    }
                    if let Some(effect) = spell.area_effect {
                        self.broadcast_magic_effect(target_pos, effect);
                    }
                    self.monster_idle_apply_spell_impact(cid, tid, &spell);
                }
                SpellShape::Destination => {
                    // 772 `DestinationShapeSpell` → `CircleShapeSpell` (`magic.cc:537,522`):
                    // gate actor→dest center + `Missile` to dest, then `ExecuteCircleSpell`
                    // over `radius` applying impact + `GraphicalEffect` per tile.
                    if !self.monster_sight_clear(pos, target_pos) {
                        continue;
                    }
                    self.monster_face_toward(cid, tid, false);
                    if let Some(shoot) = spell.shoot_effect {
                        self.broadcast_distance_shoot(pos, target_pos, shoot);
                    }
                    for tile in tiles {
                        // `ExecuteCircleSpell` PZ skip — `magic.cc:475–477`.
                        if spell.impact.is_aggressive()
                            && self.tile_in_protection_zone(tile)
                        {
                            continue;
                        }
                        if !self.monster_sight_clear(pos, tile) {
                            continue;
                        }
                        if let Some(effect) = spell.area_effect {
                            self.broadcast_magic_effect(tile, effect);
                        }
                        self.monster_idle_apply_spell_field(cid, tile, &spell);
                        let victims: Vec<CreatureId> = self
                            .map
                            .get_tile(tile)
                            .map(|t| t.body().creatures.clone())
                            .unwrap_or_default();
                        for victim_id in victims {
                            if victim_id == cid {
                                continue;
                            }
                            self.monster_idle_apply_spell_impact(cid, victim_id, &spell);
                        }
                    }
                }
                SpellShape::Actor => {
                    // 772 `ActorShapeSpell` — aggressive + PZ → return (`magic.cc:406–408`).
                    if spell.impact.is_aggressive() && self.tile_in_protection_zone(pos) {
                        continue;
                    }
                    // 772 `ActorShapeSpell` — `GraphicalEffect` on actor tile (`magic.cc:400`).
                    if let Some(effect) = spell.area_effect {
                        self.broadcast_magic_effect(pos, effect);
                    }
                    self.monster_idle_apply_spell_impact(cid, cid, &spell);
                }
                SpellShape::Origin | SpellShape::Angle => {
                    // 772 `OriginShapeSpell`/`AngleShapeSpell` — `ExecuteCircleSpell`/beam:
                    // `handleField` then `handleCreature` per victim (`magic.cc:483–494`).
                    for tile in tiles {
                        // `ExecuteCircleSpell` PZ skip — `magic.cc:475–477`.
                        if spell.impact.is_aggressive()
                            && self.tile_in_protection_zone(tile)
                        {
                            continue;
                        }
                        if !self.monster_sight_clear(pos, tile) {
                            continue;
                        }
                        if let Some(effect) = spell.area_effect {
                            self.broadcast_magic_effect(tile, effect);
                        }
                        // `TSummonImpact` / `TFieldImpact` override `handleField` only.
                        self.monster_idle_apply_spell_field(cid, tile, &spell);
                        let victims: Vec<CreatureId> = self
                            .map
                            .get_tile(tile)
                            .map(|t| t.body().creatures.clone())
                            .unwrap_or_default();
                        for victim_id in victims {
                            if victim_id == cid {
                                continue;
                            }
                            if let Some(shoot) = spell.shoot_effect {
                                self.broadcast_distance_shoot(pos, tile, shoot);
                            }
                            self.monster_idle_apply_spell_impact(cid, victim_id, &spell);
                        }
                    }
                }
            }

            // C++ CASTING (`crnonpl.cc:2521-2667`) has **no** `break` — every spell whose delay/flee
            // gates pass is evaluated and cast in the same idle, and each spell's delay roll is drawn
            // regardless (audit Finding 2). Stopping after the first cast desyncs the glibc stream.
        }
    }

    pub(crate) fn monster_idle_apply_spell_impact(
        &mut self,
        caster_id: CreatureId,
        target_id: CreatureId,
        spell: &MonsterSpell,
    ) {
        if chase_debug::chase_path_debug_enabled() {
            if let Some(CreatureKind::Monster(m)) = self.creatures.get(caster_id) {
                let spell_label = match &spell.impact {
                    SpellImpact::Damage { .. } => "damage".into(),
                    SpellImpact::Condition { condition, .. } => format!("condition:{condition:?}"),
                    SpellImpact::Healing { .. } => "healing".into(),
                    SpellImpact::Speed { .. } => "speed".into(),
                    SpellImpact::Field { .. } => "field".into(),
                    SpellImpact::Summon { race, .. } => format!("summon:{race}"),
                    SpellImpact::Drunk { .. } => "drunk".into(),
                    SpellImpact::Outfit { .. } => "outfit".into(),
                    SpellImpact::Invisible { .. } => "invisible".into(),
                };
                let shape = match spell.shape {
                    SpellShape::Victim => "victim",
                    SpellShape::Actor => "actor",
                    SpellShape::Origin => "origin",
                    SpellShape::Destination => "destination",
                    SpellShape::Angle => "angle",
                };
                chase_debug::log_spell_cast(
                    self.chase_trace_tick(),
                    caster_id,
                    m.base.name.as_str(),
                    &spell_label,
                    target_id.data().as_ffi(),
                    shape,
                    spell.range,
                );
            }
        }
        let profile = self.mechanics.profile;
        let hooks = &self.mechanics.hooks;

        match &spell.impact {
            SpellImpact::Condition {
                condition,
                cycle,
                min_cycle,
            } => {
                let min_c = (*min_cycle).max(1);
                let max_c = (*cycle).max(min_c);
                let strength = self.parity_random(min_c, max_c);
                let cond = ActiveCondition {
                    id: 0,
                    sub_id: 0,
                    ctype: *condition,
                    data: ConditionData::Damage {
                        total_rank: strength,
                    },
                    timer_rounds_left: None,
                };
                let params = CombatParams {
                    primary_type: CombatType::Physical,
                    dispel: None,
                    apply_condition: Some(cond),
                };
                let _ = self.combat_execute_with_stimulus(
                    Some(caster_id),
                    target_id,
                    &CombatDamage {
                        primary: (CombatType::Physical, 0),
                        secondary: (CombatType::Physical, 0),
                    },
                    &params,
                );
            }
            SpellImpact::Damage {
                element,
                base,
                variation,
            } => {
                let min_dmg = (*base).saturating_sub(*variation);
                let max_dmg = (*base).saturating_add(*variation);
                let scaled = spell_damage(&profile, hooks, 0, 0, max_dmg, false, false);
                let mut dmg = if scaled > 0 {
                    scaled
                } else {
                    // C++ `ComputeDamage` monster path: `Damage + random(-Var, Var)` (`magic.cc:776`)
                    // — glibc parity stream, not `ai_rng` (Finding 14).
                    self.parity_random(min_dmg, max_dmg).max(0)
                };
                // 772 CASTING `TDamageImpact(..., AllowDefense=true)` (`crnonpl.cc:2592`) —
                // physical always subtracts GetDefendDamage then armor (`magic.cc:147-151`).
                if *element == CombatType::Physical && dmg > 0 {
                    if let Some(target_pos) =
                        self.creatures.get(target_id).map(|k| k.position())
                    {
                        dmg = self.mitigate_physical_spell_damage(
                            target_id, target_pos, dmg, true, true,
                        );
                    }
                }
                let params = CombatParams {
                    primary_type: *element,
                    ..CombatParams::default()
                };
                // Capture snapshot + HP before — same pattern as monster melee/ranged
                // (`monster_ai.rs:444,592`). `combat_execute_with_stimulus` broadcasts the
                // magic hit effect but NOT the animated damage text / health bar / status
                // message — `notify_player_combat_damage` owns those (`game_world_spectators.rs:481`).
                let notify_snap = self.combat_notify_snapshot(target_id);
                let hp_before = self
                    .creatures
                    .get(target_id)
                    .map(|k| k.base().health)
                    .unwrap_or(0);
                let _ = self.combat_execute_with_stimulus(
                    Some(caster_id),
                    target_id,
                    &CombatDamage {
                        primary: (*element, -dmg),
                        secondary: (CombatType::Physical, 0),
                    },
                    &params,
                );
                let hp_after = self
                    .creatures
                    .get(target_id)
                    .map(|k| k.base().health)
                    .unwrap_or(hp_before);
                if let Some(snap) = notify_snap {
                    let damage_done = (hp_before - hp_after).max(0);
                    if damage_done > 0 {
                        self.notify_player_combat_damage(
                            Some(caster_id),
                            target_id,
                            damage_done,
                            *element,
                            snap,
                        );
                    }
                }
            }
            SpellImpact::Healing { base, variation } => {
                let min_heal = (*base).saturating_sub(*variation);
                let max_heal = (*base).saturating_add(*variation);
                let heal = self.parity_random(min_heal, max_heal).max(0);
                // 772 `THealingImpact::handleCreature` (`magic.cc:191`) changes HP directly — no
                // `TextualEffect` (animated text), but the health bar must still update. We capture
                // the snapshot for pos/wire_id, then broadcast health + stats only (no damage text).
                let notify_snap = self.combat_notify_snapshot(target_id);
                let hp_before = self
                    .creatures
                    .get(target_id)
                    .map(|k| k.base().health)
                    .unwrap_or(0);
                let _ = self.combat_execute_with_stimulus(
                    Some(caster_id),
                    target_id,
                    &CombatDamage {
                        primary: (CombatType::Healing, heal),
                        secondary: (CombatType::Physical, 0),
                    },
                    &CombatParams::default(),
                );
                let hp_after = self
                    .creatures
                    .get(target_id)
                    .map(|k| k.base().health)
                    .unwrap_or(hp_before);
                if let Some(snap) = notify_snap {
                    let heal_done = hp_after.saturating_sub(hp_before);
                    if heal_done > 0 {
                        self.notify_creature_healed(target_id, snap);
                    }
                }
            }
            SpellImpact::Speed {
                percent,
                variation,
                duration,
            } => {
                // 772 `TSpeedImpact` (`magic.cc:226-250`): roll percent, MDAct from Go Act.
                let min_pct = (*percent).saturating_sub(*variation);
                let max_pct = (*percent).saturating_add(*variation);
                let rolled_pct = self.parity_random(min_pct, max_pct);
                let go_act = self
                    .creatures
                    .get(target_id)
                    .map(|k| k.base().base_speed)
                    .unwrap_or(0);
                let flat_delta = speed_mdact(go_act, rolled_pct);
                let ctype = if flat_delta >= 0 {
                    ConditionType::Haste
                } else {
                    ConditionType::Paralyze
                };
                let cond = ActiveCondition {
                    id: 0,
                    sub_id: 0,
                    ctype,
                    data: ConditionData::Speed { flat_delta },
                    timer_rounds_left: duration_ms_to_rounds(*duration),
                };
                let params = CombatParams {
                    primary_type: CombatType::Physical,
                    dispel: None,
                    apply_condition: Some(cond),
                };
                let _ = self.combat_execute_with_stimulus(
                    Some(caster_id),
                    target_id,
                    &CombatDamage {
                        primary: (CombatType::Physical, 0),
                        secondary: (CombatType::Physical, 0),
                    },
                    &params,
                );
                self.on_condition_started(target_id, ctype);
            }
            SpellImpact::Drunk {
                drunkness,
                duration,
            } => {
                // 772 `TDrunkenImpact` (`magic.cc:255-285`): Power = drunkness/20 ≤ 6.
                let power = drunk_power_from_xml(*drunkness);
                let suppressed = match self.creatures.get(target_id) {
                    Some(CreatureKind::Player(p)) => {
                        p.condition_suppressions
                            & tfs_rust_content::item_abilities::CONDITION_DRUNK
                            != 0
                    }
                    _ => false,
                };
                if !suppressed {
                    let current = self
                        .creatures
                        .get(target_id)
                        .map(|k| k.base().drunkenness)
                        .unwrap_or(0);
                    // Refresh only when new Power ≥ current TimerValue.
                    if power >= current {
                        if let Some(kind) = self.creatures.get_mut(target_id) {
                            kind.base_mut().drunkenness = power;
                        }
                        let cond = ActiveCondition {
                            id: 0,
                            sub_id: 0,
                            ctype: ConditionType::Drunk,
                            data: ConditionData::Generic {
                                ticks: *duration,
                            },
                            timer_rounds_left: duration_ms_to_rounds(*duration),
                        };
                        let params = CombatParams {
                            primary_type: CombatType::Physical,
                            dispel: None,
                            apply_condition: Some(cond),
                        };
                        let _ = self.combat_execute_with_stimulus(
                            Some(caster_id),
                            target_id,
                            &CombatDamage {
                                primary: (CombatType::Physical, 0),
                                secondary: (CombatType::Physical, 0),
                            },
                            &params,
                        );
                        self.on_condition_started(target_id, ConditionType::Drunk);
                    }
                }
            }
            SpellImpact::Outfit {
                monster,
                item,
                duration,
            } => {
                let look_type = monster
                    .as_ref()
                    .and_then(|name| {
                        self.monsters_db
                            .get_by_name(name)
                            .map(|t| t.outfit.look_type)
                    })
                    .unwrap_or(0);
                let look_type_ex = item.unwrap_or(0);
                if look_type != 0 || look_type_ex != 0 {
                    let cond = ActiveCondition {
                        id: 0,
                        sub_id: 0,
                        ctype: ConditionType::Outfit,
                        data: ConditionData::Outfit {
                            look_type,
                            look_type_ex,
                        },
                        timer_rounds_left: duration_ms_to_rounds(*duration),
                    };
                    let params = CombatParams {
                        primary_type: CombatType::Physical,
                        dispel: None,
                        apply_condition: Some(cond),
                    };
                    let _ = self.combat_execute_with_stimulus(
                        Some(caster_id),
                        target_id,
                        &CombatDamage {
                            primary: (CombatType::Physical, 0),
                            secondary: (CombatType::Physical, 0),
                        },
                        &params,
                    );
                    self.on_condition_started(target_id, ConditionType::Outfit);
                }
            }
            SpellImpact::Invisible { duration } => {
                let cond = ActiveCondition {
                    id: 0,
                    sub_id: 0,
                    ctype: ConditionType::Invisible,
                    data: ConditionData::Generic {
                        ticks: *duration,
                    },
                    timer_rounds_left: duration_ms_to_rounds(*duration),
                };
                let params = CombatParams {
                    primary_type: CombatType::Physical,
                    dispel: None,
                    apply_condition: Some(cond),
                };
                let _ = self.combat_execute_with_stimulus(
                    Some(caster_id),
                    target_id,
                    &CombatDamage {
                        primary: (CombatType::Physical, 0),
                        secondary: (CombatType::Physical, 0),
                    },
                    &params,
                );
                self.on_condition_started(target_id, ConditionType::Invisible);
            }
            SpellImpact::Field { .. } => {
                // Field is `handleField`-only (`magic.cc:167`); creature hits are no-ops.
            }
            SpellImpact::Summon { .. } => {
                // Summon is `handleField`-only (`magic.cc:385`); creature hits are no-ops.
            }
        }
    }

    /// 772 `FieldPossible` — `info.cc:728` (fire/poison/energy; no MAGICWALL/WILDGROWTH arm).
    fn monster_field_possible(&self, pos: Position) -> bool {
        let Some(tile) = self.map.get_tile(pos) else {
            return false;
        };
        let body = tile.body();
        let chain = body.map_object_chain();
        let Some(MapStackEntry::Ground(server_id)) = chain.first() else {
            return false;
        };
        if !self.items_db.is_terrain_bank(*server_id) {
            return false;
        }
        for entry in &chain {
            match entry {
                MapStackEntry::Creature(_) => {}
                MapStackEntry::Ground(sid) => {
                    if self.items_db.is_unpassable(*sid) {
                        return false;
                    }
                }
                MapStackEntry::Item(item_id) => {
                    let Some(item) = self.items.get(*item_id) else {
                        return false;
                    };
                    if self.items_db.is_unpassable(item.item_type) {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// 772 `CreateField` — `magic.cc:984` (PvP/dangerous ids; NoPvP remap for peaceful casters).
    pub(crate) fn monster_create_field(
        &mut self,
        caster_id: CreatureId,
        field_pos: Position,
        field_type: MonsterFieldType,
    ) {
        if !self.monster_field_possible(field_pos) {
            return;
        }

        // TFS field item ids — same as `aoe.rs` CREATEITEM path (`combatTileEffects`).
        const ITEM_FIREFIELD_PVP: u16 = 1487;
        const ITEM_POISONFIELD_PVP: u16 = 1490;
        const ITEM_ENERGYFIELD_PVP: u16 = 1491;
        const ITEM_FIREFIELD_NOPVP: u16 = 1500;
        const ITEM_POISONFIELD_NOPVP: u16 = 1503;
        const ITEM_ENERGYFIELD_NOPVP: u16 = 1504;

        let (caster_is_player, summon_master) = match self.creatures.get(caster_id) {
            Some(CreatureKind::Player(_)) => (true, None),
            Some(CreatureKind::Monster(m)) => (false, m.base.master),
            _ => (false, None),
        };
        let peaceful = self.pvp_config.world_type == WorldType::NoPvp
            && (caster_is_player
                || summon_master.is_some_and(|mid| {
                    matches!(self.creatures.get(mid), Some(CreatureKind::Player(_)))
                }));

        let item_type = match (field_type, peaceful) {
            (MonsterFieldType::Fire, false) => ITEM_FIREFIELD_PVP,
            (MonsterFieldType::Poison, false) => ITEM_POISONFIELD_PVP,
            (MonsterFieldType::Energy, false) => ITEM_ENERGYFIELD_PVP,
            (MonsterFieldType::Fire, true) => ITEM_FIREFIELD_NOPVP,
            (MonsterFieldType::Poison, true) => ITEM_POISONFIELD_NOPVP,
            (MonsterFieldType::Energy, true) => ITEM_ENERGYFIELD_NOPVP,
        };

        // Delete existing MAGICFIELD items — `CreateField` (`magic.cc:1034–1041`).
        let existing: Vec<_> = self
            .map
            .get_tile(field_pos)
            .map(|t| {
                t.body()
                    .down_items
                    .iter()
                    .chain(t.body().top_items.iter())
                    .copied()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for iid in existing {
            if self
                .items
                .get(iid)
                .and_then(|item| self.items_db.items.get(&item.item_type))
                .is_some_and(|t| t.is_magic_field())
            {
                let _ = self.internal_remove_item_from_tile(field_pos, iid, u16::MAX);
            }
        }

        let owner_wire = self
            .creatures
            .get(caster_id)
            .map(|kind| creature_wire_id(caster_id, kind));
        let mut item = Item::new_single(item_type);
        if let Some(owner) = owner_wire {
            item.attributes
                .get_or_insert_with(|| Box::new(ItemAttributes::new()))
                .set_owner(owner);
        }
        let iid = self.items.insert(item);
        if self
            .internal_add_item_to_tile(field_pos, iid, CylinderFlags::NONE)
            .is_err()
        {
            self.items.remove(iid);
        }
    }

    /// 772 `TImpact::handleField` path — `ExecuteCircleSpell` (`magic.cc:483`).
    ///
    /// `TSummonImpact` / `TFieldImpact` override this; damage/heal leave it as a no-op.
    fn monster_idle_apply_spell_field(
        &mut self,
        caster_id: CreatureId,
        field_pos: Position,
        spell: &MonsterSpell,
    ) {
        match &spell.impact {
            SpellImpact::Field { field_type } => {
                // `TFieldImpact::handleField` → `CreateField` (`magic.cc:167–172`).
                self.monster_create_field(caster_id, field_pos, *field_type);
            }
            SpellImpact::Summon { race, max, force } => {
                // `crnonpl.cc:2647` — only wild masters build `TSummonImpact`.
                if self
                    .creatures
                    .get(caster_id)
                    .is_some_and(|k| k.base().is_summon())
                {
                    return;
                }
                // `magic.cc:391` — `Actor->SummonedCreatures < Maximum`.
                let summoned = self
                    .creatures
                    .iter()
                    .filter(|(_, k)| k.base().master == Some(caster_id))
                    .count() as i32;
                if summoned >= *max {
                    return;
                }
                let _ = self.monster_create_summon(caster_id, race, *force, field_pos);
            }
            _ => {}
        }
    }

    /// 772 `TMonster::IdleStimulus` — chase/repath/roam decisions (772 only).
    pub(crate) fn monster_idle_stimulus(&mut self, cid: CreatureId) {
        self.monster_idle_stimulus_inner(cid, false);
    }

    /// Close-chase restep after clearing a stale Go — walk/attack arms only.
    ///
    /// C++ `CreatureMoveStimulus` (`crmain.cc:920`) re-arms Wait+Attack; it does **not** re-run
    /// the CASTING block. Passing `skip_casting=true` avoids Destination/Victim `Rotate` mid-chase
    /// (run → face → run), which looked like flee turn-dancing on casters (e.g. giant spider).
    pub(crate) fn monster_idle_stimulus_after_creature_move(&mut self, cid: CreatureId) {
        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
            m.idle_stimulus_last_ms = None;
        }
        self.monster_idle_stimulus_inner(cid, true);
    }

    /// C++ `TMonster::IdleStimulus` — `crnonpl.cc:2345`.
    ///
    /// When `skip_casting` is true the CASTING block was already executed on this
    /// todo drain pass (`TDAttack` distance tail — `cract.cc:764-767`).
    fn monster_idle_stimulus_inner(&mut self, cid: CreatureId, skip_casting: bool) {
        if !self.creatures.contains_key(cid) {
            return;
        }
        if self.creatures.get(cid).is_some_and(|k| {
            matches!(
                k,
                CreatureKind::Monster(m) if m.idle_stimulus_last_ms == Some(self.server_ms)
            )
        }) {
            return;
        }
        self.obs.record_idle_pass();
        if chase_debug::chase_path_debug_enabled() {
            if let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) {
                chase_debug::log_idle_stimulus(self.chase_trace_tick(), cid, &m.base.name);
            }
        }
        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
            m.idle_stimulus_last_ms = Some(self.server_ms);
            // C++ logs `combat_state` each idle pass; harness compare is per-tick bucketed.
            m.last_combat_trace = None;
        }
        if self
            .creatures
            .get(cid)
            .is_some_and(|k| matches!(k, CreatureKind::Monster(m) if m.wants_lua_think()))
        {
            return;
        }

        let (is_idle, is_summon, has_opponents, follow, fleeing, pos, sleeping) = {
            let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) else {
                return;
            };
            (
                m.is_idle,
                m.base.is_summon(),
                !m.opponent_ids.is_empty(),
                m.base.follow_target,
                m.is_fleeing(),
                m.base.position,
                m.state == MonsterState::Sleeping,
            )
        };

        // C++ summon despawn / re-bind block — runs at the very top of `IdleStimulus`
        // (`crnonpl.cc:2359–2405`), BEFORE the sleeping/idle checks. A sleeping summon still
        // gets despawned if its master is gone / too far / on a different floor.
        if is_summon && self.monster_idle_summon_lifecycle(cid) {
            return;
        }

        if sleeping {
            if is_idle {
                return;
            }
            // Bridge: legacy/test paths may clear `is_idle` before promoting `state` off Sleeping.
            if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
                m.state = MonsterState::Idle;
            }
        }

        // Phase 3: 772 idle path runs unconditionally for both eras.
        self.monster_idle_lose_existing_target(cid);
        self.monster_idle_reset_combat_state(cid);
        self.monster_idle_try_talk(cid);
        if self.monster_idle_acquire_target(cid) {
            return;
        }
        if !skip_casting {
            self.monster_idle_try_casting(cid);
        }

        // Phase 3: 1098 synchronous target search / on_think_target / look update deleted —
        // both eras use the 772 idle `Strategy[]` acquisition above.
        let _ = (has_opponents, follow, fleeing, pos);

        if !self.creatures.get(cid).is_some_and(|k| {
            k.base().health > 0 && (k.base().walk_timer_idle() || k.base().force_update_follow_path)
        }) {
            return;
        }

        self.monster_idle_prepare_and_enqueue_go(cid);

        // C++ order is Rotate then ToDoAttack (`crnonpl.cc:2871-2877`). Attack's
        // `CanToDoAttack` often prepends `ToDoGo`. CipSoft Execute runs that Go in the same
        // stimulus window so the turn is masked by the move packet. Our P4-3 deferral arms
        // Go for a later wakeup — broadcasting 0x6B *before* Attack would face-turn on the
        // spot every idle. Enqueue Attack first, then Rotate: suppress the wire turn when a
        // Go is pending (direction still updates for combat/LOS).
        let attack_enqueued = self.monster_idle_maybe_enqueue_attack(cid);
        self.monster_idle_rotate_toward_attack_target(cid);
        if self.creature_todo_queue_empty(cid) {
            self.monster_idle_maybe_enqueue_at_goal_wait(cid, attack_enqueued);
        }

        self.monster_idle_reschedule_target_bound_if_parked(cid);

        // RC2: C++ `IdleStimulus` idle-wandering catch-all — `crnonpl.cc:2920–2939`.
        // Every path through the C++ function either returns early with `ToDoStart()` (combat
        // branches) or falls through to the idle-wandering tail which always ends with
        // `ToDoWait(1000) + ToDoStart()`. The decomposed Rust helpers above cover the combat
        // branches and the chase-target parked case, but when no target exists and roam fails
        // (Hold outcome), no wakeup was scheduled — causing ~750 ms stalls. This tail mirrors
        // the C++ catch-all: if nothing above armed a wakeup, enqueue a 1000 ms re-think.
        let already_armed = self.creatures.get(cid).is_some_and(|k| {
            let base = k.base();
            !base.todo.is_empty() || base.next_wakeup.is_some()
        });
        if !already_armed {
            self.idle_enqueue_wait_and_start(cid, MONSTER_IDLE_WAIT_MS);
        }
    }

    /// C++ trailing `ToDoStart()` — never leave a live target without a heap wakeup (`crnonpl.cc:2809`).
    fn monster_idle_reschedule_target_bound_if_parked(&mut self, cid: CreatureId) {
        let parked = self.creatures.get(cid).and_then(|k| {
            let CreatureKind::Monster(m) = k else {
                return None;
            };
            let base = k.base();
            let chase_target = base.follow_target.or(base.attack_target)?;
            if !base.todo.is_empty() || base.next_wakeup.is_some() {
                return None;
            }
            if !self.creatures.contains_key(chase_target) {
                return None;
            }
            Some((
                base.name.clone(),
                base.position,
                base.follow_target,
                base.attack_target,
                m.state,
                m.base.chase_mode,
                chase_target,
            ))
        });
        let Some((name, pos, follow_target, attack_target, state, chase_mode, chase_id)) = parked
        else {
            return;
        };
        if chase_debug::chase_path_debug_enabled() {
            let target_pos = self
                .creatures
                .get(chase_id)
                .map(|k| k.position())
                .unwrap_or(pos);
            let cheb = chebyshev(pos, target_pos);
            let los_clear = self.monster_sight_clear(pos, target_pos);
            let state_str = format!("{state:?}");
            let chase_mode_str = format!("{chase_mode:?}");
            chase_debug::log_parked(
                self.chase_trace_tick(),
                cid,
                name.as_str(),
                pos,
                &state_str,
                follow_target.map(|id| id.data().as_ffi()),
                attack_target.map(|id| id.data().as_ffi()),
                &chase_mode_str,
                cheb,
                los_clear,
            );
        }
        // `ToDoWait(1000)+ToDoStart` fallback when idle arms produced nothing (`crnonpl.cc:2861`).
        self.idle_enqueue_wait_and_start(cid, MONSTER_IDLE_WAIT_MS);
    }

    /// C++ `TMonster::IdleStimulus` — `crnonpl.cc:2387` (reset unless PANIC/UNDERATTACK).
    fn monster_idle_reset_combat_state(&mut self, cid: CreatureId) {
        let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) else {
            return;
        };
        if !matches!(m.state, MonsterState::Panic | MonsterState::UnderAttack) {
            m.state = MonsterState::Idle;
        }
    }

    /// C++ talk gate + broadcast — `crnonpl.cc:2442–2458`:
    /// `if (Talks > 0 && (rand() % 50) == 0)` → `TalkNr = random(1, Talks)` → fetch text →
    /// `Talk(this->ID, Mode, NULL, Text, false)`. `Mode = TALK_ANIMAL_LOW`; a `#y `/`#Y ` prefix
    /// (decompile) or `<voice yell="1">` (TVP) switches to `TALK_ANIMAL_LOUD` and strips the prefix.
    ///
    /// Wire speak types (TVP `gameserver/src/const.h`): `TALKTYPE_MONSTER_YELL = 0x10`,
    /// `TALKTYPE_MONSTER_SAY = 0x11`. The RNG draw order (gate then pick) is preserved exactly so
    /// the glibc parity stream stays aligned with the sim harness.
    fn monster_idle_try_talk(&mut self, cid: CreatureId) {
        let talks = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => m.talks,
            _ => return,
        };
        if talks == 0 {
            return;
        }
        let _trace_gate = crate::sim_glibc_rand::sim_rng_trace_site("idle_talk_gate");
        if self.parity_rand_mod(50) != 0 {
            return;
        }
        let _trace_pick = crate::sim_glibc_rand::sim_rng_trace_site("idle_talk_pick");
        // C++ `TalkNr = random(1, Talks)` — 1-indexed; Rust `talk_texts` is 0-indexed.
        let talk_nr = self.parity_random(1, i32::from(talks));
        let idx = (talk_nr.max(1) as usize).saturating_sub(1);
        let Some(raw) = self.creatures.get(cid).and_then(|k| match k {
            CreatureKind::Monster(m) => m.talk_texts.get(idx).cloned(),
            _ => None,
        }) else {
            return;
        };

        // C++ `if (Text[0] == '#' && Text[1] != 0 && Text[2] == ' ')` yell marker (`crnonpl.cc:2450`).
        // TVP equivalent: `<voice yell="1">` sets `voiceBlock.yellText` (`monster.cpp:851`).
        // We support the decompile `#y `/`#Y ` prefix in the sentence text.
        const TALKTYPE_MONSTER_SAY: u8 = 0x11;
        const TALKTYPE_MONSTER_YELL: u8 = 0x10;
        let (speak_type, text) = if raw.len() >= 3
            && raw.as_bytes()[0] == b'#'
            && (raw.as_bytes()[1] == b'y' || raw.as_bytes()[1] == b'Y')
            && raw.as_bytes()[2] == b' '
        {
            (TALKTYPE_MONSTER_YELL, raw[3..].to_string())
        } else {
            (TALKTYPE_MONSTER_SAY, raw)
        };

        // C++ `if (Text != 0 && Text[0] != 0)` — skip empty text after prefix strip.
        if text.is_empty() {
            return;
        }
        self.broadcast_creature_say_viewport(cid, speak_type, &text);
    }

    /// 772 walking prelude — `crnonpl.cc:2778-2786`.
    ///
    /// `SKILL_FIST > 0 && State != PANIC` → `State = ATTACKING`. Then if
    /// `ATTACKING || PANIC` → `Combat.SetAttackDest(Target, false)` + `SetChaseMode(NONE)`.
    /// No melee-range / ranged-spell gate — that is only for later walk/attack arms.
    pub(crate) fn monster_idle_maybe_enter_attacking(&mut self, cid: CreatureId) {
        let snapshot = {
            let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) else {
                return;
            };
            if m.is_fleeing() {
                return;
            }
            // 772 `this->Target` — Rust chase target (`follow_target`).
            let Some(target_id) = m.base.follow_target else {
                return;
            };
            if m.base.master == Some(target_id) {
                return;
            }
            (target_id, m.melee_skill, m.state, m.base.attack_target)
        };

        let (target_id, melee_skill, state, prev_attack) = snapshot;

        // `if (Skills[SKILL_FIST]->Get() > 0 && State != PANIC) State = ATTACKING`
        if melee_skill > 0 && state != MonsterState::Panic {
            if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
                m.state = MonsterState::Attacking;
            }
        }

        let combat_state = self.creatures.get(cid).and_then(|k| match k {
            CreatureKind::Monster(m) => Some(m.state),
            _ => None,
        });
        if !matches!(
            combat_state,
            Some(MonsterState::Attacking | MonsterState::Panic)
        ) {
            return;
        }

        // `Combat.SetAttackDest(this->Target, false)` — early-out when AttackDest unchanged
        // (`crcombat.cc:358-360`). Side effects only on change (`:432-437`).
        if prev_attack == Some(target_id) {
            // Still reset chase mode like C++ `SetChaseMode(CHASE_MODE_NONE)` before the
            // distance arm re-selects CLOSE/NONE (`monster_idle_set_combat_chase_mode`).
            if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
                m.base.chase_mode = ChaseMode::None;
            }
            return;
        }

        // `ObjectDistance > 8` → `StopAttack` (`crcombat.cc:424-427`).
        let too_far = match (
            self.creatures.get(cid).map(|k| k.position()),
            self.creatures.get(target_id).map(|k| k.position()),
        ) {
            (Some(from), Some(to)) => chebyshev(from, to) > 8,
            _ => true,
        };
        if too_far {
            if let Some(k) = self.creatures.get_mut(cid) {
                k.base_mut().attack_target = None;
            }
            if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
                m.base.chase_mode = ChaseMode::None;
            }
            return;
        }

        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().attack_target = Some(target_id);
        }
        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
            m.base.chase_mode = ChaseMode::None;
        }
        self.combat_on_attack_dest_changed(cid, target_id);
    }

    /// C++ ATTACKING walk prelude — `crnonpl.cc:2709-2726` (`SetChaseMode` reset then CLOSE for melee).
    pub(crate) fn monster_idle_prepare_combat_chase(&mut self, cid: CreatureId) {
        self.monster_idle_set_combat_chase_mode(cid);
        self.monster_idle_emit_combat_state(cid);
    }

    /// Set `chase_mode` from posture/target band — no JSONL side effect.
    fn monster_idle_set_combat_chase_mode(&mut self, cid: CreatureId) {
        // 772 `SetAttackDest(this->Target)` copies Target → AttackDest (`crnonpl.cc:2784`).
        // Do not overwrite `follow_target` from `attack_target` (that inverted the decompile).
        let snapshot = self.creatures.get(cid).and_then(|k| {
            let CreatureKind::Monster(m) = k else {
                return None;
            };
            Some((
                m.state,
                m.base.follow_target,
                m.base.position,
                m.target_distance,
            ))
        });
        let Some((state, follow_id, pos, raw_target_distance)) = snapshot else {
            return;
        };
        let new_mode = if !matches!(state, MonsterState::Attacking | MonsterState::Panic) {
            ChaseMode::None
        } else if let Some(follow_id) = follow_id {
            let target_distance = self.monster_effective_target_distance(raw_target_distance);
            let uses_dist_branch =
                self.monster_idle_uses_dist_branch(cid, pos, follow_id, target_distance);
            if uses_dist_branch {
                ChaseMode::None
            } else {
                ChaseMode::Close
            }
        } else {
            ChaseMode::None
        };
        if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
            m.base.chase_mode = new_mode;
        }
    }

    /// Emit `combat_state` JSONL when posture/chase_mode changed this idle pass.
    fn monster_idle_emit_combat_state(&mut self, cid: CreatureId) {
        let combat_log = self.creatures.get_mut(cid).and_then(|k| {
            let CreatureKind::Monster(m) = k else {
                return None;
            };
            if !matches!(m.state, MonsterState::Attacking | MonsterState::Panic) {
                m.last_combat_trace = None;
                return None;
            }
            let trace_key = (m.state, m.base.chase_mode);
            if m.last_combat_trace == Some(trace_key) {
                return None;
            }
            m.last_combat_trace = Some(trace_key);
            let mode = match m.base.chase_mode {
                ChaseMode::Close => "close",
                ChaseMode::Range => "range",
                ChaseMode::None => "none",
            };
            let state = match m.state {
                MonsterState::Attacking => "attacking",
                MonsterState::Panic => "panic",
                MonsterState::UnderAttack => "under_attack",
                MonsterState::Idle => "idle",
                MonsterState::Sleeping => "sleeping",
            };
            Some((
                m.base.name.clone(),
                state,
                mode,
                m.base.attack_target.map(|id| id.data().as_ffi()),
            ))
        });
        if chase_debug::chase_path_debug_enabled() {
            if let Some((name, state, mode, attack_target)) = combat_log {
                chase_debug::log_combat_state(
                    self.chase_trace_tick(),
                    cid,
                    name.as_str(),
                    state,
                    mode,
                    attack_target,
                );
            }
        }
    }

    /// C++ `TCreature::ToDoAttack` action list — `cract.cc:1325-1334`.
    pub(crate) fn monster_enqueue_todo_attack_actions(
        &mut self,
        cid: CreatureId,
    ) -> MonsterEnqueueAttackResult {
        let (weapon_distance, needs_close_step) = self
            .creatures
            .get(cid)
            .map(|k| match k {
                CreatureKind::Monster(m) => {
                    let weapon_distance = monster_weapon_attack_distance(
                        m.melee_skill,
                        m.spells.iter().any(|s| s.range > 1),
                    );
                    let needs_close_step = m.base.attack_target.is_some_and(|aid| {
                        self.creatures.get(aid).is_some_and(|t| {
                            weapon_distance == 1 && chebyshev(m.base.position, t.position()) > 1
                        })
                    });
                    (weapon_distance, needs_close_step)
                }
                _ => (1, false),
            })
            .unwrap_or((1, false));
        let skip_idle_melee_chase = self.monster_idle_skip_idle_melee_chase(cid);
        let already_has_close_go = self.monster_close_chase_go_already_armed(cid);
        let close_chase = if already_has_close_go {
            MonsterCombatCloseChaseEnqueue::Skipped
        } else {
            self.monster_combat_enqueue_close_chase_go(cid)
        };
        if close_chase == MonsterCombatCloseChaseEnqueue::Retry {
            return MonsterEnqueueAttackResult::Retry;
        }
        if close_chase == MonsterCombatCloseChaseEnqueue::Noway {
            return MonsterEnqueueAttackResult::Noway;
        }
        if needs_close_step
            && close_chase != MonsterCombatCloseChaseEnqueue::Queued
            && !already_has_close_go
            && !skip_idle_melee_chase
        {
            return MonsterEnqueueAttackResult::Failed;
        }
        if weapon_distance != 1 {
            self.enqueue_creature_wait(cid, 100);
        }
        let close_label = if already_has_close_go || skip_idle_melee_chase {
            "idle_tail"
        } else {
            match close_chase {
                MonsterCombatCloseChaseEnqueue::Queued => "queued",
                MonsterCombatCloseChaseEnqueue::Skipped => "skipped",
                MonsterCombatCloseChaseEnqueue::Retry => "retry",
                MonsterCombatCloseChaseEnqueue::Noway => "noway",
            }
        };
        if self.enqueue_creature_attack(cid) {
            if chase_debug::chase_path_debug_enabled() {
                if let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) {
                    let wait_ms = if weapon_distance != 1 { 100 } else { 0 };
                    chase_debug::log_attack_enqueue(
                        self.chase_trace_tick(),
                        cid,
                        m.base.name.as_str(),
                        wait_ms,
                        needs_close_step && !already_has_close_go && !skip_idle_melee_chase,
                        close_label,
                    );
                }
            }
            // C++ `ToDoStart` always arms `NextWakeup` for the head todo entry when the list is
            // non-empty (`cract.cc:1010-1023`). The walk branch normally schedules the Go via
            // `idle_enqueue_paced_go`, but when `skip_idle_melee_chase` is true (ATTACKING/PANIC
            // at dist>1) the walk branch is `Hold` and the Go is enqueued here by
            // `monster_combat_enqueue_close_chase_go` — without a wakeup. That left the monster
            // parked with `[Go, Attack]` and no heap entry until the ~1 Hz think tick rescued it
            // via `monster_combat_reschedule_if_stalled`, causing a visible ~1 s stall after every
            // chase-batch drain while the target was kiting (audit: close-chase-wakeup-gap).
            let needs_wakeup = self
                .creatures
                .get(cid)
                .is_some_and(|k| k.base().next_wakeup.is_none());
            if needs_wakeup {
                let has_go = self
                    .creatures
                    .get(cid)
                    .is_some_and(|k| k.base().todo.has_go());
                if has_go {
                    // Go was enqueued by close-chase — schedule its walk delay (head entry).
                    let _ = self.todo_start_go_delay(cid, true);
                } else {
                    let delay_ms = self.todo_attack_delay_ms(cid);
                    self.todo_start_from_action(cid, delay_ms);
                }
            }
            MonsterEnqueueAttackResult::Enqueued
        } else {
            MonsterEnqueueAttackResult::Failed
        }
    }

    /// 772 melee tail uses cheb band for strike; enqueue uses `ToDoAttack` walk path (`cract.cc:1325`).
    fn monster_idle_can_enqueue_attack(
        &self,
        cid: CreatureId,
        pos: Position,
        attack_id: CreatureId,
        target_pos: Position,
    ) -> bool {
        let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) else {
            return false;
        };
        let target_distance = self.monster_effective_target_distance(m.target_distance);
        let _dist = chebyshev(pos, target_pos);
        // `CanToDoAttack` close walk at cheb>1 — no strike-range cap (`crcombat.cc:496`).
        if m.melee_skill > 0 && m.base.chase_mode == ChaseMode::Close {
            return true;
        }
        if target_distance <= 1 {
            return true;
        }
        self.monster_can_use_attack(cid, pos, attack_id)
    }

    /// C++ `Rotate(Target)` at idle combat tail — `crnonpl.cc:2871`.
    ///
    /// Direction always updates. The `0x6B` broadcast is skipped when a `Go` is already
    /// queued (`todo.has_go` or non-empty `walk_queue`): the move packet carries facing, and
    /// broadcasting here (with deferred Execute) caused visible stand-still turn spam.
    pub(crate) fn monster_idle_rotate_toward_attack_target(&mut self, cid: CreatureId) {
        let target_id = self.creatures.get(cid).and_then(|k| {
            let CreatureKind::Monster(m) = k else {
                return None;
            };
            if !matches!(m.state, MonsterState::Attacking | MonsterState::Panic) {
                return None;
            }
            m.base.attack_target
        });
        let Some(target_id) = target_id else {
            return;
        };
        let go_pending = self.creatures.get(cid).is_some_and(|k| {
            let b = k.base();
            b.todo.has_go() || !b.walk_queue.is_empty()
        });
        self.monster_face_toward(cid, target_id, !go_pending);
    }

    /// Face `target_id` — optional spectator `0x6B`. C++ `Rotate(TCreature*)` — `cract.cc:452-473`.
    pub(crate) fn monster_face_toward(
        &mut self,
        cid: CreatureId,
        target_id: CreatureId,
        broadcast: bool,
    ) {
        let (pos, current) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => (m.base.position, m.base.direction),
            _ => return,
        };
        let target_pos = match self.creatures.get(target_id) {
            Some(k) => k.position(),
            None => return,
        };
        let new_dir = compute_look_toward_target(pos, target_pos, current);
        if new_dir == current {
            return;
        }
        if broadcast {
            creature_turn_with_broadcast(self, cid, new_dir);
        } else if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().direction = new_dir;
        }
        if chase_debug::chase_path_debug_enabled() {
            if let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) {
                chase_debug::log_rotate(
                    self.chase_trace_tick(),
                    cid,
                    m.base.name.as_str(),
                    new_dir as u8,
                    Some(target_id.data().as_ffi()),
                );
            }
        }
    }

    /// C++ `Rotate(TCreature *Target)` — `cract.cc:452-473`. Broadcasts `0x6B` when facing changes.
    pub(crate) fn monster_execute_rotate_toward(&mut self, cid: CreatureId, target_id: CreatureId) {
        self.monster_face_toward(cid, target_id, true);
    }

    /// 772 NOWAY fall-through — clear chase target and roam (`crnonpl.cc:2890-2898` + `:2900-2939`).
    ///
    /// Mirrors the C++ `catch(RESULT r)` block in `TMonster::IdleStimulus`: when the close-chase
    /// `ToDoGo` (via `CanToDoAttack`) throws NOWAY because `TShortway::Calculate` found no path,
    /// C++ clears `Target`, `ToDoClear()`, and — for NOWAY (non-EXHAUSTED) — falls through to the
    /// idle-wandering roam tail (`crnonpl.cc:2900-2939`). EXHAUSTED (kick-kill / player-tile) is
    /// handled separately by the walk executor (`monster_exhausted_wait`).
    ///
    /// Used by the attack-tail NOWAY arm ([`Self::monster_idle_maybe_enqueue_attack`]) so an
    /// ATTACKING melee monster with no path to the target clears its target and roams instead of
    /// parking indefinitely. The walk-branch NOWAY handler
    /// ([`Self::monster_idle_prepare_and_enqueue_go`]) inlines the same clear+roam via its own
    /// `match outcome` block.
    pub(crate) fn monster_idle_noway_clear_and_roam(&mut self, cid: CreatureId) {
        self.monster_on_chase_noway(cid);
        let outcome = self.monster_idle_execute_walk_branch(cid, MonsterIdleWalkBranch::Roam);
        match outcome {
            MonsterIdleWalkOutcome::QueuedGo { via, wait_after } => {
                self.idle_enqueue_paced_go(
                    cid,
                    true,
                    Some(via),
                    wait_after.then_some(MONSTER_IDLE_WAIT_MS),
                );
            }
            MonsterIdleWalkOutcome::QueuedWait => {
                self.idle_enqueue_wait_and_start(cid, MONSTER_IDLE_WAIT_MS);
            }
            MonsterIdleWalkOutcome::QueuedWaitThenGo { via } => {
                self.idle_enqueue_wait_then_paced_go(cid, MONSTER_IDLE_WAIT_MS, Some(via));
            }
            // Roam found no walkable tile — C++ `ToDoWait(1000) + ToDoStart()`
            // (`crnonpl.cc:2937-2939`). The idle catch-all (`crnonpl.cc:2920-2939` tail) also
            // covers this, but arming the wait here keeps the contract explicit.
            MonsterIdleWalkOutcome::Hold
            | MonsterIdleWalkOutcome::Noway
            | MonsterIdleWalkOutcome::FallthroughRoam => {
                self.idle_enqueue_wait_and_start(cid, MONSTER_IDLE_WAIT_MS);
            }
        }
    }

    /// C++ idle combat tail — `Rotate` + `ToDoAttack` (`crnonpl.cc:2795`).
    fn monster_idle_maybe_enqueue_attack(&mut self, cid: CreatureId) -> bool {
        let (attack_id, pos) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => {
                if m.is_fleeing() {
                    return false;
                }
                let Some(attack_id) = m.base.attack_target else {
                    return false;
                };
                (attack_id, m.base.position)
            }
            _ => return false,
        };
        if !self.creatures.contains_key(attack_id) {
            return false;
        }
        let target_pos = match self.creatures.get(attack_id) {
            Some(k) => k.position(),
            None => return false,
        };
        if !self.monster_idle_can_enqueue_attack(cid, pos, attack_id, target_pos) {
            return false;
        }
        // C++ `CanToDoAttack` close walk does not require LOS — only strike does (`crcombat.cc:496`).
        // C++ always appends `ToDoAttack` at the idle tail (`crnonpl.cc:2795`); cadence is enforced
        // by `TDAttack` on execute (`cract.cc:909`), not by skipping enqueue here.
        match self.monster_enqueue_todo_attack_actions(cid) {
            MonsterEnqueueAttackResult::Enqueued => {
                trace_creature_todo(self, cid, "idle_enqueue_attack");
                true
            }
            MonsterEnqueueAttackResult::Retry => {
                self.idle_enqueue_wait_and_start(cid, MONSTER_IDLE_WAIT_MS);
                false
            }
            MonsterEnqueueAttackResult::Noway => {
                // C++ `catch(RESULT r)` for NOWAY: clear `Target` + fall through to roam
                // (`crnonpl.cc:2890-2898` + `:2900-2939`). Was: recurse into
                // `monster_idle_prepare_and_enqueue_go` which, with the target still set and
                // state ATTACKING, re-entered the same Hold→close-chase→Noway path — an
                // infinite recursion / parking loop.
                self.monster_idle_noway_clear_and_roam(cid);
                false
            }
            MonsterEnqueueAttackResult::Failed => {
                self.monster_combat_handle_close_chase_blocked(cid);
                false
            }
        }
    }

    /// Yield and retry close-chase when still off-band; short wait at strike range (`cract.cc:845-852`).
    pub(crate) fn monster_combat_handle_close_chase_blocked(&mut self, cid: CreatureId) {
        let still_off_band = self.creatures.get(cid).and_then(|k| {
            let CreatureKind::Monster(m) = k else {
                return None;
            };
            let attack_id = m.base.attack_target?;
            let target_pos = self.creatures.get(attack_id)?.position();
            Some(chebyshev(m.base.position, target_pos) > 1)
        });
        if still_off_band == Some(true) {
            self.idle_enqueue_wait_and_start(cid, MONSTER_IDLE_WAIT_MS);
        } else {
            self.idle_enqueue_wait_and_start(cid, 200);
        }
    }

    /// `ToDoWait(1000)` when at-goal dance could not arm (`crnonpl.cc:2791` dist band).
    /// Melee `ATTACKING` tail gets `ToDoAttack` only — no trailing wait (`crnonpl.cc:2795–2807`).
    fn monster_idle_maybe_enqueue_at_goal_wait(&mut self, cid: CreatureId, attack_enqueued: bool) {
        let branch = self.monster_idle_classify_walk_branch(cid);
        match branch {
            MonsterIdleWalkBranch::DistDance => {}
            MonsterIdleWalkBranch::MeleeDance => {
                if attack_enqueued {
                    return;
                }
                if self
                    .creatures
                    .get(cid)
                    .is_some_and(|k| k.base().todo.has_attack())
                {
                    return;
                }
                let hostile_melee_at_band = match self.creatures.get(cid) {
                    Some(CreatureKind::Monster(m)) => match m.base.follow_target {
                        None => false,
                        Some(follow_id) => {
                            let target_distance =
                                self.monster_effective_target_distance(m.target_distance);
                            if target_distance > 1 {
                                false
                            } else if let Some(t) = self.creatures.get(follow_id) {
                                chebyshev(m.base.position, t.position()) == 1
                                    && self.monster_idle_is_attacking_posture(cid, target_distance)
                            } else {
                                false
                            }
                        }
                    },
                    _ => false,
                };
                if hostile_melee_at_band {
                    return;
                }
            }
            _ => return,
        }
        self.idle_enqueue_wait_and_start(cid, MONSTER_IDLE_WAIT_MS);
    }

    /// When idle should run [`GameWorld::monster_idle_chase_repath`] for an active chase (772 only).
    pub(crate) fn monster_idle_chase_needs_repath(
        &mut self,
        cid: CreatureId,
    ) -> (bool, Option<&'static str>) {
        let Some(k) = self.creatures.get(cid) else {
            return (false, None);
        };
        let base = k.base();
        if base.force_update_follow_path {
            if let Some(follow_id) = base.follow_target {
                let pos = k.position();
                if let Some(target_pos) = self.creatures.get(follow_id).map(|t| t.position()) {
                    let (fleeing, target_distance) = match self.creatures.get(cid) {
                        Some(CreatureKind::Monster(m)) => (
                            m.is_fleeing(),
                            self.monster_effective_target_distance(m.target_distance),
                        ),
                        _ => return (true, Some("force_update")),
                    };
                    if self.monster_at_follow_goal(
                        cid,
                        follow_id,
                        pos,
                        target_pos,
                        fleeing,
                        target_distance,
                    ) {
                        if let Some(k) = self.creatures.get_mut(cid) {
                            k.base_mut().force_update_follow_path = false;
                        }
                        return (false, None);
                    }
                }
            }
            return (true, Some("force_update"));
        }
        if !base.walk_queue.is_empty() {
            return (false, None);
        }
        if !base.has_follow_path {
            return (true, Some("idle_drain"));
        }
        let Some(follow_id) = base.follow_target else {
            return (false, None);
        };
        let pos = k.position();
        let Some(target_pos) = self.creatures.get(follow_id).map(|t| t.position()) else {
            return (false, None);
        };
        let (fleeing, target_distance) = match self.creatures.get(cid) {
            Some(CreatureKind::Monster(m)) => (
                m.is_fleeing(),
                self.monster_effective_target_distance(m.target_distance),
            ),
            _ => return (false, None),
        };
        if self.monster_at_follow_goal(cid, follow_id, pos, target_pos, fleeing, target_distance) {
            return (false, None);
        }
        (true, Some("off_band"))
    }

    /// Classify the idle walk arm — `crnonpl.cc:2676` priority order.
    ///
    /// Melee vs ranged split mirrors `!DistanceFighting || !ThrowPossible` (`crnonpl.cc:2795-2797`)
    /// via [`GameWorld::monster_idle_uses_dist_branch`].
    pub(crate) fn monster_idle_classify_walk_branch(
        &self,
        cid: CreatureId,
    ) -> MonsterIdleWalkBranch {
        let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) else {
            return MonsterIdleWalkBranch::Hold;
        };

        let follow_id = match m.base.follow_target {
            Some(id) => id,
            None => return MonsterIdleWalkBranch::Roam,
        };

        let pos = m.base.position;
        let target_pos = match self.creatures.get(follow_id) {
            Some(k) => k.position(),
            None => return MonsterIdleWalkBranch::Roam,
        };
        let target_distance = self.monster_effective_target_distance(m.target_distance);
        let dist = chebyshev(pos, target_pos);

        if m.is_fleeing() {
            return MonsterIdleWalkBranch::Flee;
        }

        if m.base.master == Some(follow_id) {
            return MonsterIdleWalkBranch::MasterFollow;
        }

        let uses_dist_branch =
            self.monster_idle_uses_dist_branch(cid, pos, follow_id, target_distance);

        if uses_dist_branch {
            if dist < target_distance {
                MonsterIdleWalkBranch::DistFlee
            } else if dist > target_distance {
                MonsterIdleWalkBranch::DistChase
            } else {
                MonsterIdleWalkBranch::DistDance
            }
        } else if dist > 1 {
            if self.monster_idle_skip_idle_melee_chase(cid) {
                MonsterIdleWalkBranch::Hold
            } else {
                MonsterIdleWalkBranch::MeleeChase
            }
        } else if dist == 1 {
            MonsterIdleWalkBranch::MeleeDance
        } else {
            MonsterIdleWalkBranch::Hold
        }
    }

    fn monster_idle_log_walk_branch(
        &self,
        cid: CreatureId,
        branch: &str,
        dest: Position,
        must: bool,
        max_steps: i32,
        reason: Option<&str>,
    ) {
        if !chase_debug::chase_path_debug_enabled() {
            return;
        }
        let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) else {
            return;
        };
        chase_debug::log_branch(
            self.chase_trace_tick(),
            cid,
            m.base.name.as_str(),
            branch,
            m.base.position,
            dest,
            must,
            max_steps,
            reason,
        );
    }

    /// Execute one classified walk arm — returns outcome without enqueuing `Go`.
    fn monster_idle_execute_walk_branch(
        &mut self,
        cid: CreatureId,
        branch: MonsterIdleWalkBranch,
    ) -> MonsterIdleWalkOutcome {
        match branch {
            MonsterIdleWalkBranch::Flee => {
                if self.monster_idle_flee_step(cid) {
                    MonsterIdleWalkOutcome::QueuedGo {
                        via: "idle_flee",
                        wait_after: false,
                    }
                } else {
                    MonsterIdleWalkOutcome::Hold
                }
            }
            MonsterIdleWalkBranch::DistFlee => {
                if self.monster_idle_flee_step(cid) {
                    MonsterIdleWalkOutcome::QueuedGo {
                        via: "idle_flee",
                        wait_after: false,
                    }
                } else {
                    MonsterIdleWalkOutcome::QueuedWait
                }
            }
            MonsterIdleWalkBranch::MasterFollow => {
                let (pos, follow_id) = match self.creatures.get(cid) {
                    Some(CreatureKind::Monster(m)) => {
                        let Some(follow_id) = m.base.follow_target else {
                            return MonsterIdleWalkOutcome::FallthroughRoam;
                        };
                        (m.base.position, follow_id)
                    }
                    _ => return MonsterIdleWalkOutcome::Hold,
                };
                let target_pos = match self.creatures.get(follow_id) {
                    Some(k) => k.position(),
                    None => return MonsterIdleWalkOutcome::FallthroughRoam,
                };
                let dist = manhattan(pos, target_pos);
                if dist <= 1 {
                    return MonsterIdleWalkOutcome::FallthroughRoam;
                }
                if !self
                    .creatures
                    .get(cid)
                    .is_some_and(|k| k.base().walk_queue.is_empty())
                {
                    return MonsterIdleWalkOutcome::Hold;
                }
                if monster_master_follow_wait_only_band(dist) {
                    if chase_debug::chase_path_debug_enabled() {
                        if let Some(CreatureKind::Monster(m)) = self.creatures.get(cid) {
                            chase_debug::log_branch(
                                self.chase_trace_tick(),
                                cid,
                                m.base.name.as_str(),
                                "master_follow_wait",
                                pos,
                                target_pos,
                                false,
                                0,
                                None,
                            );
                        }
                    }
                    return MonsterIdleWalkOutcome::QueuedWait;
                }
                let repath_reason = if self
                    .creatures
                    .get(cid)
                    .is_some_and(|k| k.base().force_update_follow_path)
                {
                    Some("force_update")
                } else if self
                    .creatures
                    .get(cid)
                    .is_some_and(|k| !k.base().has_follow_path)
                {
                    Some("idle_drain")
                } else {
                    Some("off_band")
                };
                match self.monster_idle_master_follow(cid, repath_reason) {
                    MonsterIdleChaseRepathOutcome::PathQueued => {
                        let via = repath_reason.unwrap_or("idle_drain");
                        if monster_master_follow_wait_before_go(dist) {
                            MonsterIdleWalkOutcome::QueuedWaitThenGo { via }
                        } else {
                            MonsterIdleWalkOutcome::QueuedGo {
                                via,
                                wait_after: false,
                            }
                        }
                    }
                    MonsterIdleChaseRepathOutcome::AtGoal => MonsterIdleWalkOutcome::Hold,
                    MonsterIdleChaseRepathOutcome::Noway => MonsterIdleWalkOutcome::Noway,
                }
            }
            MonsterIdleWalkBranch::MeleeChase | MonsterIdleWalkBranch::DistChase => {
                let (needs_repath, repath_reason) = self.monster_idle_chase_needs_repath(cid);
                if !needs_repath {
                    return MonsterIdleWalkOutcome::Hold;
                }
                let branch_name = if branch == MonsterIdleWalkBranch::MeleeChase {
                    "melee_chase"
                } else {
                    "dist_chase"
                };
                let cheb = self
                    .creatures
                    .get(cid)
                    .and_then(|k| {
                        let follow_id = k.base().follow_target?;
                        let target_pos = self.creatures.get(follow_id)?.position();
                        Some(chebyshev(k.position(), target_pos))
                    })
                    .unwrap_or(0);
                let is_melee_chase = branch == MonsterIdleWalkBranch::MeleeChase;
                let is_dist_chase = branch == MonsterIdleWalkBranch::DistChase;
                let target_distance = self
                    .creatures
                    .get(cid)
                    .map(|k| match k {
                        CreatureKind::Monster(m) => {
                            self.monster_effective_target_distance(m.target_distance)
                        }
                        _ => 1,
                    })
                    .unwrap_or(1);
                let (max_steps, must_reach) = monster_idle_chase_step_budget(
                    is_melee_chase,
                    is_dist_chase,
                    cheb,
                    target_distance,
                );
                if let Some(target_pos) = self
                    .creatures
                    .get(cid)
                    .and_then(|k| k.base().follow_target)
                    .and_then(|tid| self.creatures.get(tid).map(|t| t.position()))
                {
                    self.monster_idle_log_walk_branch(
                        cid,
                        branch_name,
                        target_pos,
                        must_reach,
                        max_steps as i32,
                        repath_reason,
                    );
                }
                match self.monster_idle_chase_repath(cid, repath_reason, max_steps, must_reach) {
                    MonsterIdleChaseRepathOutcome::PathQueued => MonsterIdleWalkOutcome::QueuedGo {
                        via: repath_reason.unwrap_or("idle_drain"),
                        wait_after: false,
                    },
                    MonsterIdleChaseRepathOutcome::AtGoal => MonsterIdleWalkOutcome::Hold,
                    MonsterIdleChaseRepathOutcome::Noway => MonsterIdleWalkOutcome::Noway,
                }
            }
            MonsterIdleWalkBranch::MeleeDance => {
                if self.monster_idle_dance_step(cid) {
                    let queued = self
                        .creatures
                        .get(cid)
                        .is_some_and(|k| !k.base().walk_queue.is_empty());
                    if queued {
                        MonsterIdleWalkOutcome::QueuedGo {
                            via: "idle_dance",
                            wait_after: false,
                        }
                    } else {
                        // C++ `rand()%5` hold — branch may log but no `ToDoGo` (`crnonpl.cc:2814`).
                        MonsterIdleWalkOutcome::Hold
                    }
                } else {
                    MonsterIdleWalkOutcome::Hold
                }
            }
            MonsterIdleWalkBranch::DistDance => {
                if self.monster_idle_dance_step(cid) {
                    MonsterIdleWalkOutcome::QueuedGo {
                        via: "idle_dance",
                        wait_after: true,
                    }
                } else {
                    MonsterIdleWalkOutcome::Hold
                }
            }
            MonsterIdleWalkBranch::Roam => {
                let pos = self
                    .creatures
                    .get(cid)
                    .map(|k| k.position())
                    .unwrap_or(Position::new(0, 0, 7));
                self.monster_idle_log_walk_branch(cid, "roam", pos, false, 1, None);
                if self.monster_idle_roam_step(cid) {
                    MonsterIdleWalkOutcome::QueuedGo {
                        via: "roam",
                        wait_after: true,
                    }
                } else {
                    MonsterIdleWalkOutcome::Hold
                }
            }
            MonsterIdleWalkBranch::Hold => MonsterIdleWalkOutcome::Hold,
        }
    }

    /// Fill walk queue from reference-ordered idle arms, then enqueue `Go` + heap arm.
    ///
    /// C++ walking section — `crnonpl.cc:2676`.
    fn monster_idle_prepare_and_enqueue_go(&mut self, cid: CreatureId) {
        self.monster_idle_maybe_enter_attacking(cid);
        self.monster_idle_set_combat_chase_mode(cid);
        let branch = self.monster_idle_classify_walk_branch(cid);
        let mut outcome = self.monster_idle_execute_walk_branch(cid, branch);

        if matches!(outcome, MonsterIdleWalkOutcome::Noway) {
            self.monster_on_chase_noway(cid);
            outcome = self.monster_idle_execute_walk_branch(cid, MonsterIdleWalkBranch::Roam);
        }

        if matches!(
            (branch, &outcome),
            (MonsterIdleWalkBranch::Flee, MonsterIdleWalkOutcome::Hold)
                | (_, MonsterIdleWalkOutcome::FallthroughRoam)
        ) {
            outcome = self.monster_idle_execute_walk_branch(cid, MonsterIdleWalkBranch::Roam);
        }

        // C++ logs `combat_state` after PANIC melee-dance promotion (`crnonpl.cc:2830`).
        self.monster_idle_emit_combat_state(cid);

        match outcome {
            MonsterIdleWalkOutcome::QueuedGo { via, wait_after } => {
                self.idle_enqueue_paced_go(
                    cid,
                    true,
                    Some(via),
                    wait_after.then_some(MONSTER_IDLE_WAIT_MS),
                );
            }
            MonsterIdleWalkOutcome::QueuedWaitThenGo { via } => {
                self.idle_enqueue_wait_then_paced_go(cid, MONSTER_IDLE_WAIT_MS, Some(via));
            }
            MonsterIdleWalkOutcome::QueuedWait => {
                self.idle_enqueue_wait_and_start(cid, MONSTER_IDLE_WAIT_MS);
            }
            MonsterIdleWalkOutcome::Hold => {
                // 772 idle drain owns dance pacing — no TFS `getNextStep` poll (X5).
                // Phase 3: 1098 `getNextStep` dance poll deleted — both eras use idle drain.
            }
            MonsterIdleWalkOutcome::FallthroughRoam | MonsterIdleWalkOutcome::Noway => {}
        }
    }

    /// Execute the front todo action for 772 monsters.
    pub(crate) fn execute_creature_todo_action(
        &mut self,
        cid: CreatureId,
    ) -> Option<TodoExecuteKind> {
        /// Post-unlock idle work — `idle_stimulus` must not run while `todo.locked`.
        enum CombatExecuteFollowUp {
            None,
            IdleStimulus,
            CloseChaseBlocked,
        }

        // Batch `LockToDo` stays true for the whole ToDo list (`cract.cc:1012`); do not
        // refuse Execute when locked — that gate is for IdleStimulus / ToDoYield only.
        let action = {
            let k = self.creatures.get_mut(cid)?;
            k.base_mut().todo.queue.pop_front()
        };
        let action = action?;

        // Ensure batch lock is held while draining (ToDoStart may have armed it already).
        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().todo.locked = true;
        }

        let mut follow_up = CombatExecuteFollowUp::None;
        let kind = match action {
            CreatureAction::Go => {
                trace_creature_todo(self, cid, "execute_go");
                let now = Instant::now();
                self.on_walk(cid, false, now, None);
                trace_creature_todo(self, cid, "execute_go_done");
                TodoExecuteKind::Go
            }
            CreatureAction::Wait { deadline_ms } => {
                trace_creature_todo(self, cid, "execute_wait");
                // C++ chase trace logs `ToDoWait` enqueue only — not execute drain.
                // C++ `CalculateDelay(TDWait)` — `cract.cc:905-915`:
                // `WaitTime = max(TD->Wait.Time, EarliestWalkTime)`;
                // `Delay = WaitTime - ServerMilliseconds` when still in the future.
                // `TD->Wait.Time` is absolute at enqueue (`cract.cc:1033`).
                let earliest = self
                    .creatures
                    .get(cid)
                    .map(|k| k.base().earliest_walk_server_ms)
                    .unwrap_or(0);
                let wait_time = deadline_ms.max(earliest);
                if wait_time > self.server_ms {
                    self.schedule_creature_wakeup(
                        cid,
                        wait_time.max(self.server_ms.saturating_add(1)),
                    );
                }
                trace_creature_todo(self, cid, "execute_wait_done");
                TodoExecuteKind::Wait
            }
            CreatureAction::Talk { text } => {
                // C++ `TDTalk` — `cract.cc:848-851`, `:1367-1390`: `this->Talk(Mode, NULL, Text, false)`.
                // Talk mode: `TALK_SAY` for players/NPCs, `TALK_ANIMAL_LOW` for monsters (`cract.cc:409`).
                trace_creature_todo(self, cid, "execute_talk");
                let is_monster = self
                    .creatures
                    .get(cid)
                    .is_some_and(|k| matches!(k, CreatureKind::Monster(_)));
                let speak_type = if is_monster {
                    SpeakType::MonsterSay as u8
                } else {
                    SpeakType::Say as u8
                };
                self.broadcast_creature_say_viewport(cid, speak_type, &text);
                trace_creature_todo(self, cid, "execute_talk_done");
                TodoExecuteKind::Wait
            }
            CreatureAction::ChangeNpcState { to_idle } => {
                // C++ `TDChangeState` → `ChangeNPCState(..., Stimulus=true)` (`cract.cc:859-861`).
                // While `LockToDo` is set, `ToDoYield` is a no-op (`cract.cc:1026-1031`); the
                // drain path calls `IdleStimulus` when the queue empties.
                trace_creature_todo(self, cid, "execute_change_npc_state");
                if to_idle {
                    if let Some(CreatureKind::Npc(npc)) = self.creatures.get_mut(cid) {
                        npc.runtime.activity = crate::creature::NpcActivity::Idle;
                        npc.runtime.focus = None;
                    }
                }
                trace_creature_todo(self, cid, "execute_change_npc_state_done");
                TodoExecuteKind::Wait
            }
            CreatureAction::Attack => {
                // Phase 1.4: player Attack execute routes through `CanToDoAttack` chase
                // (`crcombat.cc:442-511`). The melee **strike** is deferred until a player
                // weapon-combat system exists; the chase (`ToDoGo` toward target) and the
                // thrown-`RESULT` `ToDoClear` + `SendResult` + `ToDoWait(1000)` path land here
                // (`crplayer.cc:388-405`, `cract.cc:870-889`).
                let is_player = self
                    .creatures
                    .get(cid)
                    .is_some_and(|k| matches!(k, CreatureKind::Player(_)));
                if is_player {
                    self.player_execute_attack(cid)
                } else {
                    let delay = self.todo_attack_delay_ms(cid);
                    if delay > 0 {
                        if let Some(k) = self.creatures.get_mut(cid) {
                            k.base_mut().todo.queue.push_front(CreatureAction::Attack);
                        }
                        trace_creature_todo(self, cid, "execute_attack_deferred");
                        self.todo_start_from_action(cid, delay);
                        trace_creature_todo(self, cid, "execute_attack_deferred_done");
                        TodoExecuteKind::AttackDeferred
                    } else {
                        let needs_close_step = self
                            .creatures
                            .get(cid)
                            .and_then(|k| {
                                let CreatureKind::Monster(m) = k else {
                                    return None;
                                };
                                let aid = m.base.attack_target?;
                                let cheb =
                                    chebyshev(m.base.position, self.creatures.get(aid)?.position());
                                let weapon_dist = monster_weapon_attack_distance(
                                    m.melee_skill,
                                    m.spells.iter().any(|s| s.range > 1),
                                );
                                Some(weapon_dist == 1 && cheb > 1)
                            })
                            .unwrap_or(false);
                        if needs_close_step {
                            if let Some(k) = self.creatures.get_mut(cid) {
                                k.base_mut().todo.queue.push_front(CreatureAction::Attack);
                            }
                            if self
                                .creatures
                                .get(cid)
                                .is_some_and(|k| k.base().todo.has_go())
                            {
                                trace_creature_todo(self, cid, "execute_attack_wait_for_go");
                                TodoExecuteKind::AttackDeferred
                            } else {
                                match self.monster_combat_enqueue_close_chase_go(cid) {
                                    MonsterCombatCloseChaseEnqueue::Queued => {
                                        if self
                                            .creatures
                                            .get(cid)
                                            .is_some_and(|k| k.base().todo.has_go())
                                        {
                                            if self.todo_start_go_delay(cid, false) {
                                                self.schedule_immediate_todo_wakeup(cid);
                                            } else if self
                                                .creatures
                                                .get(cid)
                                                .is_some_and(|k| k.base().next_wakeup.is_none())
                                            {
                                                let _ = self.todo_start_go_delay(cid, false);
                                            }
                                        }
                                    }
                                    MonsterCombatCloseChaseEnqueue::Retry => {
                                        if let Some(k) = self.creatures.get_mut(cid) {
                                            k.base_mut().todo.queue.pop_front();
                                        }
                                        self.idle_enqueue_wait_and_start(cid, MONSTER_IDLE_WAIT_MS);
                                    }
                                    MonsterCombatCloseChaseEnqueue::Noway => {
                                        if let Some(k) = self.creatures.get_mut(cid) {
                                            k.base_mut().todo.queue.pop_front();
                                        }
                                        follow_up = CombatExecuteFollowUp::IdleStimulus;
                                    }
                                    MonsterCombatCloseChaseEnqueue::Skipped => {
                                        if let Some(k) = self.creatures.get_mut(cid) {
                                            k.base_mut().todo.queue.pop_front();
                                        }
                                        follow_up = CombatExecuteFollowUp::CloseChaseBlocked;
                                    }
                                }
                                trace_creature_todo(self, cid, "execute_attack_out_of_range");
                                TodoExecuteKind::AttackDeferred
                            }
                        } else {
                            let distance_fighter = self.creatures.get(cid).is_some_and(|k| {
                            matches!(
                                k,
                                CreatureKind::Monster(m)
                                    if self.monster_effective_target_distance(m.target_distance) > 1
                            )
                        });
                            trace_creature_todo(self, cid, "execute_attack");
                            self.monster_do_attacking(cid, EVENT_CREATURE_THINK_INTERVAL_MS);
                            if distance_fighter {
                                if let Some(wakeup) = self
                                    .creatures
                                    .get(cid)
                                    .map(|k| k.base().earliest_attack_ms)
                                    .filter(|&wakeup| wakeup > self.server_ms)
                                {
                                    self.schedule_creature_wakeup(cid, wakeup);
                                }
                            }
                            trace_creature_todo(self, cid, "execute_attack_done");
                            if distance_fighter {
                                TodoExecuteKind::DistanceAttack
                            } else {
                                TodoExecuteKind::Attack
                            }
                        }
                    }
                }
            }
            // F8 S4 — `Use`/`Move`/`Turn` execute arms. S3 multiuse gate still runs first
            // (two-object `Use` deferral); the executor dispatch + `RESULT` catch land here.
            // F8 S5 — not-adjacent Use/Move prepend `Go` + re-enqueue instead of the bespoke
            // `walk_action_due` path (C++ `Use` executor `cract.cc:600-760`).
            CreatureAction::Use {
                obj1,
                obj2,
                open_index,
            } => {
                // F8 S3 — C++ `CalculateDelay(TDUse)` gate (`cract.cc:925-932`): two-object
                // use defers when `EarliestMultiuseTime > ServerMilliseconds`; single-object
                // use is ungated (delay 0). The action was already popped at the top of
                // `execute_creature_todo_action`, so we pass `obj2.is_some()` directly to
                // `multiuse_gate_delay_ms`. On deferral we push it back to the front and
                // arm a wakeup at `earliest_multiuse_server_ms` — same pattern as the
                // `Attack` defer above (`cract.cc:870-889`, `:795-801`).
                let delay = self.multiuse_gate_delay_ms(cid, obj2.is_some());
                if delay > 0 {
                    if let Some(k) = self.creatures.get_mut(cid) {
                        k.base_mut().todo.queue.push_front(CreatureAction::Use {
                            obj1,
                            obj2,
                            open_index,
                        });
                    }
                    trace_creature_todo(self, cid, "execute_use_deferred");
                    self.todo_start_from_action(cid, delay);
                    trace_creature_todo(self, cid, "execute_use_deferred_done");
                    TodoExecuteKind::Deferred
                } else {
                    // F8 S5 — walk-to-reach via `Go`-prepend (C++ `Use` executor
                    // `cract.cc:600-760`): if obj1 is a map tile and the player isn't
                    // adjacent, prepend `Go` + re-enqueue `Use` + `ToDoStart`. This
                    // replaces the bespoke `walk_action_due` path for the ToDo flow.
                    //
                    // F8 D6 — reach predicate uses the same-z Chebyshev `dx>1 || dy>1`
                    // form (matches `ObjectInRange(1)` for same-z, `info.cc:233-257`),
                    // NOT `look_distance_tfs` (which adds +15 for Δz and would misroute
                    // a cross-floor source into a walk). Cross-floor sources are now
                    // rejected at enqueue by `validate_action_object_z_floor` (D2).
                    let needs_walk = obj1.pos.x != 0xFFFF
                        && self.creatures.get(cid).is_some_and(|k| {
                            let pp = k.position();
                            let dx = (pp.x as i32 - obj1.pos.x as i32).unsigned_abs();
                            let dy = (pp.y as i32 - obj1.pos.y as i32).unsigned_abs();
                            dx > 1 || dy > 1
                        });
                    if needs_walk {
                        let now = Instant::now();
                        match self.setup_player_walk_to_target(cid, obj1.pos, now) {
                            Ok(()) => {
                                // `push_front(Use)` then `push_front(Go)` → `[Go, Use, ...]`
                                // (C++ `ToDoGo(dest)` + re-enqueue `ToDoUse`, `cract.cc:600-760`).
                                if let Some(k) = self.creatures.get_mut(cid) {
                                    k.base_mut().todo.queue.push_front(CreatureAction::Use {
                                        obj1,
                                        obj2,
                                        open_index,
                                    });
                                    k.base_mut().todo.queue.push_front(CreatureAction::Go);
                                }
                                if self.todo_start_go_delay(cid, true) {
                                    self.schedule_immediate_todo_wakeup(cid);
                                }
                                trace_creature_todo(self, cid, "execute_use_walk_to_reach");
                                TodoExecuteKind::Deferred
                            }
                            Err(rv) => {
                                self.apply_todo_result_catch(cid, rv);
                                TodoExecuteKind::Wait
                            }
                        }
                    } else {
                        // F8 S4/D7 — `TDUse` execute (`cract.cc:833-836` + `Use` executor
                        // `cract.cc:727-768`). After Obj1 adjacency is confirmed, check Obj2
                        // range for two-object use — mirrors C++ `Use()` executor:
                        //   `DistUse = Obj1.getFlag(DISTUSE)` (`cract.cc:737`)
                        //   `!DistUse && !ObjectInRange(Obj2, 1)` → walk to Obj2 + re-enqueue
                        //   `DistUse && !ObjectInRange(Obj2, 7)` → throw OUTOFRANGE
                        // For non-DistUse with Obj1 on map + Obj2 far: C++ picks up Obj1
                        // (`ToDoMove`) then walks to Obj2. Pickup is not yet ported, so this
                        // returns `TooFarAway` for that edge case (both map tiles, far apart).
                        // Common cases (Obj1 in inventory: key/rope/fishing rod) work fully.
                        if let Some(o2) = obj2 {
                            let obj2_far = o2.pos.x != 0xFFFF
                                && self.creatures.get(cid).is_some_and(|k| {
                                    let pp = k.position();
                                    let dx = (pp.x as i32 - o2.pos.x as i32).unsigned_abs();
                                    let dy = (pp.y as i32 - o2.pos.y as i32).unsigned_abs();
                                    dx > 1 || dy > 1
                                });
                            if obj2_far {
                                // Resolve Obj1 item_id for DISTUSE / rune allowFarUse.
                                // Runes set `allowFarUse` via Spell Lua (`RuneSpell`), not only
                                // the OTB DistUse flag — without this, inventory SD/GFB walked
                                // to Obj2 (C++ `Actions::canExecuteAction` → `canUseFar`).
                                let item_id = self.resolve_use_object(
                                    cid,
                                    obj1.pos,
                                    obj1.stack_pos,
                                    obj1.sprite_id,
                                );
                                let item_type = item_id
                                    .and_then(|id| self.items.get(id).map(|i| i.item_type));
                                let distuse =
                                    item_type.is_some_and(|t| self.items_db.is_distuse(t));
                                let rune_far = item_type
                                    .and_then(|t| self.spells.runes_by_id.get(&t))
                                    .is_some_and(|r| r.allow_far_use);
                                if distuse || rune_far {
                                    // DistUse: Chebyshev ≤ 7 (`cract.cc:761`).
                                    // Rune allowFarUse: TFS `canUseFar` `areInRange<7,5>`
                                    // (`actions.cpp:255-274`) — fire from standing tile.
                                    let (too_far, floor_rv) =
                                        self.creatures.get(cid).map_or((true, None), |k| {
                                            let pp = k.position();
                                            if rune_far {
                                                let rune = item_type
                                                    .and_then(|t| self.spells.runes_by_id.get(&t));
                                                let check_floor = rune
                                                    .is_none_or(|r| r.check_floor);
                                                if check_floor && pp.z != o2.pos.z {
                                                    let rv = if pp.z > o2.pos.z {
                                                        ReturnValue::FirstGoUpStairs
                                                    } else {
                                                        ReturnValue::FirstGoDownStairs
                                                    };
                                                    return (false, Some(rv));
                                                }
                                                let dx = (pp.x as i32 - o2.pos.x as i32)
                                                    .unsigned_abs();
                                                let dy = (pp.y as i32 - o2.pos.y as i32)
                                                    .unsigned_abs();
                                                (dx > 7 || dy > 5, None)
                                            } else {
                                                let dx = (pp.x as i32 - o2.pos.x as i32)
                                                    .unsigned_abs();
                                                let dy = (pp.y as i32 - o2.pos.y as i32)
                                                    .unsigned_abs();
                                                (dx > 7 || dy > 7, None)
                                            }
                                        });
                                    if let Some(rv) = floor_rv {
                                        self.apply_todo_result_catch(cid, rv);
                                        trace_creature_todo(
                                            self,
                                            cid,
                                            "execute_use_far_use_floor",
                                        );
                                        return Some(TodoExecuteKind::Wait);
                                    }
                                    if too_far {
                                        self.apply_todo_result_catch(
                                            cid,
                                            ReturnValue::DestinationOutOfReach,
                                        );
                                        trace_creature_todo(
                                            self,
                                            cid,
                                            "execute_use_distuse_out_of_range",
                                        );
                                        return Some(TodoExecuteKind::Wait);
                                    }
                                    // Far-use within range → proceed (no walk to Obj2).
                                } else {
                                    // Non-DistUse + Obj2 > 1 tile → walk to Obj2 + re-enqueue
                                    // (C++ `cract.cc:738-758`). If Obj1 is in inventory, the
                                    // player carries it to Obj2. If Obj1 is on the map, C++
                                    // picks it up first (`ToDoMove`) — not yet ported, so fail
                                    // with `TooFarAway` for the both-map-tiles-far-apart case.
                                    if obj1.pos.x != 0xFFFF {
                                        self.apply_todo_result_catch(
                                            cid,
                                            ReturnValue::TooFarAway,
                                        );
                                        trace_creature_todo(self, cid, "execute_use_obj2_far_obj1_on_map");
                                        return Some(TodoExecuteKind::Wait);
                                    }
                                    // Obj1 in inventory — walk to Obj2 and re-enqueue.
                                    let now = Instant::now();
                                    match self.setup_player_walk_to_target(cid, o2.pos, now) {
                                        Ok(()) => {
                                            let has_steps = self.creatures.get(cid).is_some_and(
                                                |k| !k.base().walk_queue.is_empty(),
                                            );
                                            if has_steps {
                                                if let Some(k) = self.creatures.get_mut(cid) {
                                                    k.base_mut().todo.queue.push_front(
                                                        CreatureAction::Use {
                                                            obj1,
                                                            obj2,
                                                            open_index,
                                                        },
                                                    );
                                                    k.base_mut().todo.queue.push_front(CreatureAction::Go);
                                                }
                                                if self.todo_start_go_delay(cid, true) {
                                                    self.schedule_immediate_todo_wakeup(cid);
                                                }
                                                trace_creature_todo(self, cid, "execute_use_walk_to_obj2");
                                                return Some(TodoExecuteKind::Deferred);
                                            }
                                        }
                                        Err(rv) => {
                                            self.apply_todo_result_catch(cid, rv);
                                            return Some(TodoExecuteKind::Wait);
                                        }
                                    }
                                }
                            }
                        }
                        // F8 S4 — `TDUse` execute (`cract.cc:833-836`). Re-validate the
                        // object (mirrors C++ `Obj.exists()` in the executor), then dispatch
                        // to `player_use_item_core` / `player_use_item_ex_core` (S5: core
                        // helpers skip the ready check + walk-to-reach — the ToDo arm handles
                        // adjacency via `Go`-prepend and timing via `Wait{100}` +
                        // `CalculateDelay`). On `Err(rv)` apply the C++ `RESULT` catch
                        // (`cract.cc:870-889`). Multiuse exhaustion is set inside
                        // `player_use_item_ex_core` on two-object success (`cract.cc:765`).
                        trace_creature_todo(self, cid, "execute_use");
                        let result = self.execute_player_use(cid, obj1, obj2, open_index);
                        if let Err(rv) = result {
                            self.apply_todo_result_catch(cid, rv);
                        }
                        trace_creature_todo(self, cid, "execute_use_done");
                        TodoExecuteKind::Wait
                    }
                }
            }
            CreatureAction::Move { obj, dest, count } => {
                // F8 S5 — walk-to-reach via `Go`-prepend. For map-tile sources, if the
                // player isn't within 1 tile of the source, prepend `Go` + re-enqueue
                // `Move` + `ToDoStart`. C++ `Move` executor re-validates the object at
                // execute time (`Obj.exists()` → `NOTACCESSIBLE`); the throw-destination
                // range check runs inside `player_move_item` after the walk (matches C++
                // behavior — walk there, then fail if dest unreachable). The reactive
                // path's `throw_dest_reachable_after_walk_to_item` pre-check stays in
                // `player_move_item` for the reactive caller; on the ToDo path it's a
                // no-op (player is adjacent after the walk, so `dx <= 1 && dy <= 1`).
                let needs_walk = obj.pos.x != 0xFFFF
                    && self.creatures.get(cid).is_some_and(|k| {
                        let pp = k.position();
                        let dx = (pp.x as i32 - obj.pos.x as i32).unsigned_abs();
                        let dy = (pp.y as i32 - obj.pos.y as i32).unsigned_abs();
                        dx > 1 || dy > 1
                    });
                if needs_walk {
                    let now = Instant::now();
                    match self.setup_player_walk_to_target(cid, obj.pos, now) {
                        Ok(()) => {
                            if let Some(k) = self.creatures.get_mut(cid) {
                                k.base_mut().todo.queue.push_front(CreatureAction::Move {
                                    obj,
                                    dest,
                                    count,
                                });
                                k.base_mut().todo.queue.push_front(CreatureAction::Go);
                            }
                            if self.todo_start_go_delay(cid, true) {
                                self.schedule_immediate_todo_wakeup(cid);
                            }
                            trace_creature_todo(self, cid, "execute_move_walk_to_reach");
                            TodoExecuteKind::Deferred
                        }
                        Err(rv) => {
                            self.apply_todo_result_catch(cid, rv);
                            TodoExecuteKind::Wait
                        }
                    }
                } else {
                    // F8 S4 — `TDMove` execute (`cract.cc:823-826`). C++ `CalculateDelay`
                    // `default` case: delay 0 (`cract.cc:946-948`); no gate. Re-validate the
                    // object, then dispatch to `player_move_thing` (reroute only — already
                    // exists, F8 §0.1 F5). On `Err(rv)` apply the `RESULT` catch.
                    trace_creature_todo(self, cid, "execute_move");
                    let result = self.execute_player_move(cid, obj, dest, count);
                    if let Err(rv) = result {
                        self.apply_todo_result_catch(cid, rv);
                    }
                    trace_creature_todo(self, cid, "execute_move_done");
                    TodoExecuteKind::Wait
                }
            }
            CreatureAction::Turn { obj } => {
                // F8 S4 — `TDTurn` execute (`cract.cc:838-841`). C++ `CalculateDelay`
                // `default` case: delay 0 (`cract.cc:946-948`); no gate.
                //
                // F8 D3 — walk-to-reach via `Go`-prepend (C++ `ToDoTurn` `cract.cc:1340-1341`:
                // `if(!ObjectInRange(this->ID, Obj, 1)) this->ToDoGo(ObjX, ObjY, ObjZ, false,
                // INT_MAX)`). For map-tile sources, if the player isn't within 1 tile,
                // prepend `Go` + re-enqueue `Turn` + `ToDoStart` — same shape as the `Use`/
                // `Move` S5 arms. The reach predicate uses the same-z Chebyshev `dx>1 ||
                // dy>1` form as the `Move` arm (matches `ObjectInRange(1)` for same-z; the
                // Δz case is D2/D6, handled separately). On `Err(rv)` from
                // `setup_player_walk_to_target` apply the C++ `RESULT` catch
                // (`cract.cc:870-889`).
                let needs_walk = obj.pos.x != 0xFFFF
                    && self.creatures.get(cid).is_some_and(|k| {
                        let pp = k.position();
                        let dx = (pp.x as i32 - obj.pos.x as i32).unsigned_abs();
                        let dy = (pp.y as i32 - obj.pos.y as i32).unsigned_abs();
                        dx > 1 || dy > 1
                    });
                if needs_walk {
                    let now = Instant::now();
                    match self.setup_player_walk_to_target(cid, obj.pos, now) {
                        Ok(()) => {
                            // `push_front(Turn)` then `push_front(Go)` → `[Go, Turn, ...]`
                            // (C++ `ToDoGo(dest)` + re-enqueue `ToDoTurn`, `cract.cc:1341`).
                            if let Some(k) = self.creatures.get_mut(cid) {
                                k.base_mut()
                                    .todo
                                    .queue
                                    .push_front(CreatureAction::Turn { obj });
                                k.base_mut().todo.queue.push_front(CreatureAction::Go);
                            }
                            if self.todo_start_go_delay(cid, true) {
                                self.schedule_immediate_todo_wakeup(cid);
                            }
                            trace_creature_todo(self, cid, "execute_turn_walk_to_reach");
                            TodoExecuteKind::Deferred
                        }
                        Err(rv) => {
                            self.apply_todo_result_catch(cid, rv);
                            TodoExecuteKind::Wait
                        }
                    }
                } else {
                    // Adjacent (or inventory/container source) — dispatch to the
                    // `player_rotate_item` executor (F8 §0.1 F2 — nothing existed to
                    // reuse). On `Err(rv)` apply the `RESULT` catch.
                    trace_creature_todo(self, cid, "execute_turn");
                    let result = self.player_rotate_item(cid, obj);
                    if let Err(rv) = result {
                        self.apply_todo_result_catch(cid, rv);
                    }
                    trace_creature_todo(self, cid, "execute_turn_done");
                    TodoExecuteKind::Wait
                }
            }
        };

        // Batch LockToDo stays set while todos/walk_queue remain; release before idle follow-up
        // so IdleStimulus can run (`cract.cc` ToDoClear then IdleStimulus when list drained).
        self.creature_todo_release_lock_if_drained(cid);

        match follow_up {
            CombatExecuteFollowUp::IdleStimulus => self.monster_idle_stimulus(cid),
            CombatExecuteFollowUp::CloseChaseBlocked => {
                self.monster_combat_handle_close_chase_blocked(cid);
            }
            CombatExecuteFollowUp::None => {}
        }

        Some(kind)
    }

    /// F8 S4/S5 — `TDUse` execute dispatch. Re-validates the object(s) at execute time
    /// (mirrors C++ `Obj.exists()` in the `Use` executor, `cract.cc:727-760`), resolves
    /// the `ItemId` from the `ActionObjectRef`, then calls the core use helpers
    /// (`player_use_item_core` / `player_use_item_ex_core`) directly — **skipping** the
    /// reactive-path ready check + walk-to-reach (S5: the ToDo arm handles adjacency via
    /// `Go`-prepend and timing via `Wait{100}` + `CalculateDelay`). Returns `Err(rv)` on
    /// re-validation or executor failure so the caller can apply the `RESULT` catch.
    pub(crate) fn execute_player_use(
        &mut self,
        cid: CreatureId,
        obj1: ActionObjectRef,
        obj2: Option<ActionObjectRef>,
        open_index: u8,
    ) -> Result<(), ReturnValue> {
        // Re-validate obj1 (and obj2 if present) — `validate_action_object_ref` resolves
        // the item + checks the sprite, returning `Err(NotPossible)` on mismatch (C++
        // `NOTACCESSIBLE` → `NotPossible`, `walk/mod.rs:1506` convention).
        // obj2 may be a creature target (needTarget runes) — use `validate_use_ex_target_ref`.
        self.validate_action_object_ref(cid, obj1)?;
        if let Some(o2) = obj2 {
            self.validate_use_ex_target_ref(cid, o2)?;
        }

        let Some(conn_id) = self.conn_for_creature(cid) else {
            // Player disconnected — no conn to send results/open containers to.
            tracing::debug!(?cid, "execute_player_use: no conn — skipping");
            return Ok(());
        };

        // Resolve `ItemId` for obj1 — same resolution path as `validate_action_object_ref`.
        let item_id = self.resolve_use_object(cid, obj1.pos, obj1.stack_pos, obj1.sprite_id);
        let Some(item_id) = item_id else {
            return Err(ReturnValue::NotPossible);
        };

        if obj2.is_some() {
            // Two-object use — `CUseTwoObjects` (`receiving.cc:430`). Core helper sets
            // multiuse exhaustion on success (`cract.cc:765`).
            let o2 = obj2.expect("obj2 checked");
            self.player_use_item_ex_core(conn_id, cid, item_id, o2)
        } else {
            // Single-object use — `CUseObject` (`receiving.cc:384`).
            let preferred_cid =
                if matches!(
                    self.mechanics.profile.container_window_alloc,
                    crate::formulas::ContainerWindowAlloc::ClientChooses
                ) {
                    Some(open_index.min(crate::container::MAX_CONTAINER_WINDOWS - 1))
                } else {
                    (open_index < crate::container::MAX_CONTAINER_WINDOWS).then_some(open_index)
                };
            let is_map_tile = obj1.pos.x != 0xFFFF;
            self.player_use_item_core(conn_id, cid, item_id, is_map_tile, obj1.pos, preferred_cid)
        }
    }

    /// F8 S4 — `TDMove` execute dispatch. Re-validates the source object, reconstructs
    /// the `ThrowPayload` from the `ActionObjectRef` + destination, and calls the
    /// existing `player_move_thing` executor (reroute only — F8 §0.1 F5). Returns
    /// `Err(rv)` on re-validation or executor failure for the `RESULT` catch.
    pub(crate) fn execute_player_move(
        &mut self,
        cid: CreatureId,
        obj: ActionObjectRef,
        dest: Position,
        count: u8,
    ) -> Result<(), ReturnValue> {
        self.validate_move_object_ref(cid, obj)?;

        let Some(conn_id) = self.conn_for_creature(cid) else {
            tracing::debug!(?cid, "execute_player_move: no conn — skipping");
            return Ok(());
        };
        let now = Instant::now();
        let payload = ThrowPayload {
            from_pos: obj.pos,
            sprite_id: obj.sprite_id,
            from_stack_pos: obj.stack_pos,
            to_pos: dest,
            count,
        };
        self.player_move_thing(
            conn_id,
            cid,
            payload.from_pos,
            payload.sprite_id,
            payload.from_stack_pos,
            payload.to_pos,
            payload.count,
            now,
        )
    }

    /// Execute one `CreatureAction::Go` for 772 monsters — returns true if an action ran.
    #[cfg(test)]
    pub(crate) fn execute_creature_todo_go(&mut self, cid: CreatureId) -> bool {
        matches!(
            self.execute_creature_todo_action(cid),
            Some(TodoExecuteKind::Go)
        )
    }

    /// After Go/Attack execute: schedule next step or chain queued actions.
    pub(crate) fn finish_creature_todo_execute(&mut self, cid: CreatureId) {
        if self.finish_creature_todo_execute_step(cid) == TodoExecuteLoopControl::Continue {
            self.run_monster_todo_execute(cid);
        }
    }

    /// Post-Go/Attack tail — returns [`TodoExecuteLoopControl::Continue`] when the queue still
    /// has zero-delay work for the same wakeup (`cract.cc:783-898`).
    fn finish_creature_todo_execute_step(&mut self, cid: CreatureId) -> TodoExecuteLoopControl {
        if !self.creature_uses_todo_execute(cid) {
            return TodoExecuteLoopControl::Break;
        }

        // C++ `Execute` checks `Stop` after a successful action (`cract.cc:891-897`) and when the
        // next step's `Delay > 0` (`cract.cc:797-801`): `ToDoClear + SendSnapback` (player only).
        // `todo_stop` is set by `player_stop_auto_walk` (772 `ToDoStop` locked branch,
        // `cract.cc:1003-1004`). The in-flight step has just landed; now clear + snapback.
        let stop_requested = self
            .creatures
            .get(cid)
            .is_some_and(|k| matches!(k, CreatureKind::Player(_)) && k.base().todo.todo_stop);
        if stop_requested {
            if let Some(conn) = self.conn_for_creature(cid) {
                let dir_byte = self
                    .creatures
                    .get(cid)
                    .map(|k| k.base().direction as u8)
                    .unwrap_or(0);
                self.enqueue_encoded(conn, self.codec.encode_cancel_walk(dir_byte));
            }
            self.creature_todo_clear(cid);
            return TodoExecuteLoopControl::Break;
        }

        let walk_queue_has_more = self
            .creatures
            .get(cid)
            .is_some_and(|k| !k.base().walk_queue.is_empty());

        if walk_queue_has_more {
            // `force_update_follow_path` is a monster chase-repath flag — only
            // monsters have a follow target and `IdleStimulus` to clear it.
            // For players, it must never gate step chaining (C++ `Execute` catch
            // does not set any follow-path flag — `cract.cc:870-889`).
            let force_repath = self.creatures.get(cid).is_some_and(|k| {
                matches!(k, CreatureKind::Monster(_)) && k.base().force_update_follow_path
            });
            if force_repath {
                if let Some(k) = self.creatures.get_mut(cid) {
                    let base = k.base_mut();
                    base.walk_queue.clear();
                    base.walk_destinations.clear();
                    base.has_follow_path = false;
                }
                self.creature_todo_release_lock_if_drained(cid);
                self.request_idle_stimulus(cid);
                return TodoExecuteLoopControl::Break;
            }
            // Re-arm `Go` before pending `Attack` — one step per execute (`cract.cc:728`).
            let _ = self.enqueue_creature_go_at(cid, true);
            let immediate = self.todo_start_go_delay(cid, false);
            if immediate {
                self.schedule_immediate_todo_wakeup(cid);
            }
            return TodoExecuteLoopControl::Break;
        }

        if let Some(k) = self.creatures.get_mut(cid) {
            k.base_mut().has_follow_path = false;
        }

        if !self.creature_todo_queue_empty(cid) {
            // C++ `ToDoGo` completes before chained `TDAttack` when target kited away (`crmain.cc:950`).
            let defer_attack_after_go = self.creatures.get(cid).is_some_and(|k| {
                let CreatureKind::Monster(m) = k else {
                    return false;
                };
                if !m.base.todo.has_attack() || m.base.todo.has_go() {
                    return false;
                }
                if !self.monster_idle_skip_idle_melee_chase(cid) {
                    return false;
                }
                m.base.attack_target.is_some_and(|aid| {
                    self.creatures
                        .get(aid)
                        .is_some_and(|t| chebyshev(k.position(), t.position()) > 1)
                })
            });
            if defer_attack_after_go {
                if let Some(CreatureKind::Monster(m)) = self.creatures.get_mut(cid) {
                    m.base.next_wakeup = None;
                }
                let mut delay_ms = self.todo_attack_delay_ms(cid);
                if delay_ms == 0 {
                    delay_ms = 200;
                }
                self.todo_start_from_action(cid, delay_ms);
                return TodoExecuteLoopControl::Break;
            }
            return TodoExecuteLoopControl::Continue;
        }

        self.creature_todo_release_lock_if_drained(cid);
        self.maybe_idle_stimulus_after_go_complete(cid);
        TodoExecuteLoopControl::Break
    }

    /// Gate harness idle re-entry after todo drain — shared by [`finish_creature_todo_execute`]
    /// and [`GameWorld::process_creature_todo`].
    ///
    /// Dispatches to `idle_stimulus` (not `monster_idle_stimulus` directly) so players also get
    /// their `IdleStimulus` — `player_idle_stimulus` re-arms `ToDoAttack` when `attack_target`
    /// is set (`crplayer.cc:392-395`). Without this, a player who walks while attacking loses
    /// the attack re-arm: `player_move_request` clears the ToDo queue (removing the queued
    /// `Attack`), and after the `Go` completes the monster-only `monster_idle_stimulus` returns
    /// early for players, leaving the attack dead.
    pub(crate) fn maybe_idle_stimulus_after_go_complete(&mut self, cid: CreatureId) {
        self.idle_stimulus(cid);
    }

    /// Run one queued action (772 monsters).
    ///
    /// C++ `TCreature::Execute` — explicit `while (true)` loop (`cract.cc:783-898`), not recursion.
    pub(crate) fn run_monster_todo_execute(&mut self, cid: CreatureId) {
        const MAX_TODO_EXECUTE_ITERATIONS: u32 = 512;
        let mut iterations = 0u32;
        loop {
            iterations = iterations.saturating_add(1);
            if iterations > MAX_TODO_EXECUTE_ITERATIONS {
                let queue_len = self
                    .creatures
                    .get(cid)
                    .map(|k| k.base().todo.queue.len())
                    .unwrap_or(0);
                tracing::warn!(
                    creature = ?cid,
                    queue_len,
                    iterations,
                    "todo execute iteration guard tripped — breaking zero-delay chain"
                );
                // `process_creature_todo` already took `next_wakeup`. Leaving `todo.locked`
                // with a non-empty queue and no heap entry stalls the creature forever
                // (`idle_stimulus` no-ops while locked). Re-arm for the next beat.
                if !self.creature_todo_queue_empty(cid) {
                    self.todo_start_from_action(cid, 1);
                } else {
                    self.creature_todo_release_lock_if_drained(cid);
                }
                break;
            }

            let kind = match self.execute_creature_todo_action(cid) {
                Some(k) => k,
                None => break,
            };

            let control = match kind {
                TodoExecuteKind::Go | TodoExecuteKind::Attack => {
                    self.finish_creature_todo_execute_step(cid)
                }
                TodoExecuteKind::DistanceAttack => {
                    self.monster_idle_try_casting(cid);
                    if self.creature_todo_queue_empty(cid) {
                        // Future attack cadence lives in `earliest_attack_ms`; do not block the
                        // post-`TDAttack` idle walk arm (`cract.cc:764-767`, `crnonpl.cc:2741`).
                        if let Some(k) = self.creatures.get_mut(cid) {
                            let base = k.base_mut();
                            if base.next_wakeup.is_some_and(|w| w > self.server_ms) {
                                base.next_wakeup = None;
                            }
                        }
                        self.creature_todo_release_lock_if_drained(cid);
                        self.monster_idle_stimulus_inner(cid, true);
                        self.monster_idle_reschedule_target_bound_if_parked(cid);
                        TodoExecuteLoopControl::Break
                    } else {
                        self.finish_creature_todo_execute_step(cid)
                    }
                }
                TodoExecuteKind::Wait => {
                    // C++ `CalculateDelay(TDWait)` with `Delay > 0` breaks the Execute loop
                    // and leaves the entry pending (`cract.cc:795-801`, `:905-915`). Rust pops
                    // Wait up front; if a future wakeup was armed, do **not** IdleStimulus yet
                    // — otherwise Go+Wait(2000) roam immediately re-idles and loses the pause
                    // (NPC-6; also trips the zero-delay iteration guard).
                    let has_future_wakeup = self.creatures.get(cid).is_some_and(|k| {
                        k.base()
                            .next_wakeup
                            .is_some_and(|w| w > self.server_ms)
                    });
                    if has_future_wakeup {
                        self.monster_combat_reschedule_if_stalled(cid);
                        TodoExecuteLoopControl::Break
                    } else if self.creature_todo_queue_empty(cid) {
                        // Expired Wait(0) / past deadline — drained list runs IdleStimulus
                        // (`cract.cc:764-767`), including after `ToDoYield`'s `ToDoWait(0)`.
                        self.creature_todo_release_lock_if_drained(cid);
                        self.idle_stimulus(cid);
                        if self.creature_todo_queue_empty(cid) {
                            TodoExecuteLoopControl::Break
                        } else {
                            TodoExecuteLoopControl::Continue
                        }
                    } else {
                        // Consecutive zero-delay entries in the same wakeup (`cract.cc:784`).
                        let has_armed_wakeup = self
                            .creatures
                            .get(cid)
                            .is_some_and(|k| k.base().next_wakeup.is_some());
                        if has_armed_wakeup {
                            self.monster_combat_reschedule_if_stalled(cid);
                            TodoExecuteLoopControl::Break
                        } else {
                            TodoExecuteLoopControl::Continue
                        }
                    }
                }
                TodoExecuteKind::AttackDeferred => {
                    self.monster_combat_reschedule_if_stalled(cid);
                    TodoExecuteLoopControl::Break
                }
                // F8 S3 — gate-deferred action (two-object Use waiting on multiuse exhaustion).
                // The wakeup was already armed by `todo_start_from_action` in the gate check;
                // no reschedule needed (`cract.cc:795-801` "Delay > 0 → schedule + break").
                TodoExecuteKind::Deferred => TodoExecuteLoopControl::Break,
            };

            if control == TodoExecuteLoopControl::Break {
                break;
            }
        }
    }
}

#[cfg(test)]
#[path = "idle_stimulus_tests.rs"]
mod tests;
