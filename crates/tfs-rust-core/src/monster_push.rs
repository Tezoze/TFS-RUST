//! Monster push-before-step — the 772 `TMonster::MovePossible` kick gate (both eras).
//!
//! The execute side-effects of `TMonster::MovePossible`
//! (`crnonpl.cc:2141–2291`) — `TMonster::KickCreature` (`crnonpl.cc:3036`) and
//! `TMonster::KickBoxes` (`crnonpl.cc:2994`). An ATTACKING/PANIC monster that has a target and
//! the `KickCreatures` race flag shoves a blocking **pushable monster** aside in fixed
//! **N, S, W, E** order, killing it when no adjacent tile is free. A monster with
//! `CanKickBoxes()` (race flag, or inherited from a monster master) shoves a blocking movable
//! **box/field** aside (same N,S,W,E order, `BANK && !UNPASS` destination), deleting it on
//! failure. Stepping onto a **player** tile, or a `KickCreature` that has to kill, is the C++
//! `EXHAUSTED` case → [`MonsterKickOutcome::Exhausted`] (kick-kill: target preserved, C++
//! `Execute` catch `cract.cc:870-877`) or [`MonsterKickOutcome::ExhaustedDropTarget`]
//! (player-tile: target cleared, C++ `crnonpl.cc:2236-2238`). The caller waits 1000 ms in both
//! cases.
//!
//! Called from [`crate::walk::GameWorld::on_walk`] before the mover steps. A successful kick
//! relocates via `::Move` parity: `0x6D` broadcast + `NotifyTurn`/`NotifyGo` (dest floor speed).

use std::time::Instant;

use tfs_rust_common::enums::{CombatType, Direction};
use tfs_rust_common::Position;

use crate::creature::{CreatureKind, MonsterState};
use crate::cylinder::{Cylinder, CylinderFlags};
use crate::game_world::GameWorld;
use crate::ids::{CreatureId, ItemId};
use crate::player_flags::{flags_for_group, has_player_flag, PLAYER_FLAG_IGNORED_BY_MONSTERS};
use crate::tile::flags as tilestate;

/// 772 `TMonster::KickCreature` / `KickBoxes` shove order — `crnonpl.cc:3057-3058`, `:3014-3015`
/// (`OffsetX={0,0,-1,1}, OffsetY={-1,1,0,0}` = North, South, West, East). Deterministic, no RNG.
const KICK_DIRS: [Direction; 4] = [
    Direction::North,
    Direction::South,
    Direction::West,
    Direction::East,
];

/// CipSoft `EFFECT_BLOCK_HIT` — graphical effect on a kick-kill / box delete
/// (`crnonpl.cc:3071`, `:3025`). This is the raw client wire byte: `encode_magic_effect` writes
/// `effect_id` verbatim on both eras (matching tvp-772 `ProtocolGame::sendMagicEffect`), so this
/// must be the on-wire value, **not** the 0-indexed `MagicEffect` enum.
///
/// Wire id `4` = `CONST_ME_BLOCKHIT` (the gray "spark"): CipSoft `EFFECT_BLOCK_HIT = 4`
/// (`tibia-game-master/src/enums.hh:180`) and tvp-772 `CONST_ME_BLOCKHIT = 4`
/// (`gameserver/src/const.h:14`). Was `3`, which is `CONST_ME_POFF` — the client rendered a poff
/// instead of the block-hit spark on kick-kills / box deletes.
const EFFECT_BLOCK_HIT: u8 = 4;

/// F2: cycle guard for recursive chain-push. C++ `KickCreature` has no explicit depth guard —
/// it relies on the fixed N,S,W,E offset order + `skip kicker's tile` check (`crnonpl.cc:3062-3064`)
/// to terminate. Rust is explicit: 8 levels is deeper than any realistic chain-push (a 5-monster
/// convoy only needs depth 4) while bounding pathological cycles (A→B→C→A).
const MAX_KICK_DEPTH: u8 = 8;

/// Outcome of the 772 pre-step kick gate — mirrors the `Execute=true` side of
/// `TMonster::MovePossible` (`crnonpl.cc:2225-2244`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MonsterKickOutcome {
    /// Not a 772 kick situation, or the destination was cleared — proceed with the normal step.
    Proceed,
    /// 772 `EXHAUSTED` — a `KickCreature` kill (blocker boxed in). The mover must **not** step
    /// this beat; the caller runs `ToDoClear; Wait(1000); ToDoStart` (**Target preserved** —
    /// C++ `Execute` catch `cract.cc:870-877`; throw site `crnonpl.cc:2241-2242`).
    Exhausted,
    /// 772 `EXHAUSTED` — a player blocker on the destination tile. The mover must **not** step
    /// this beat; the caller runs `Target=0; ToDoClear; Wait(1000); ToDoStart` (**Target
    /// cleared** — C++ `crnonpl.cc:2236-2238` clears `Target` before `throw EXHAUSTED`).
    ExhaustedDropTarget,
}

impl GameWorld {
    /// Push blocking creatures/boxes off the destination tile before the mover steps.
    pub(crate) fn monster_push_before_step(
        &mut self,
        mover: CreatureId,
        dest: Position,
        now: Instant,
    ) -> MonsterKickOutcome {
        self.monster_kick_before_step(mover, dest, now)
    }

    // ───────────────────────── 772 (`MovePossible` / `KickCreature`) ─────────────────────────

    /// 772 `CanKickBoxes()` — race `KickBoxes` flag, or inherited from a monster master
    /// (`crnonpl.cc:2984-2992`).
    fn monster_can_kick_boxes(&self, mover: CreatureId) -> bool {
        let mut current = mover;
        // Bounded walk up the master chain to avoid cycles (summon-of-summon is shallow in 772).
        for _ in 0..8 {
            match self.creatures.get(current) {
                Some(CreatureKind::Monster(m)) => {
                    if m.can_push_items {
                        return true;
                    }
                    match m.base.master {
                        Some(master) => current = master,
                        None => return false,
                    }
                }
                _ => return false,
            }
        }
        false
    }

    /// 772 pre-step kick side-effects — `TMonster::MovePossible(Execute=true)` (`crnonpl.cc:2225-2272`).
    ///
    /// Includes the C++ kick-and-retry loop (`crnonpl.cc:2185` `for Attempt 0..100`): after each
    /// successful kick, the destination tile is re-checked. If still blocked by another creature,
    /// the loop kicks again. This lets a monster step through a multi-deep creature wall on the
    /// same beat. After 100 attempts or a non-recoverable block (player, IGNORED, invisible,
    /// forced kill), returns `Exhausted`.
    fn monster_kick_before_step(
        &mut self,
        mover: CreatureId,
        dest: Position,
        now: Instant,
    ) -> MonsterKickOutcome {
        let Some((
            mover_pos,
            master,
            target_attack,
            target_follow,
            state,
            can_push_creatures,
            see_invisible,
        )) = ({
            match self.creatures.get(mover) {
                Some(CreatureKind::Monster(m)) => Some((
                    m.base.position,
                    m.base.master,
                    m.base.attack_target,
                    m.base.follow_target,
                    m.state,
                    m.can_push_creatures,
                    m.see_invisible,
                )),
                _ => return MonsterKickOutcome::Proceed,
            }
        })
        else {
            return MonsterKickOutcome::Proceed;
        };

        // C++ creature-block gate: only an ATTACKING/PANIC monster with a target and the
        // `KickCreatures` race flag ever kicks a blocking creature. P1-A1: no `!is_summon` gate —
        // C++ `MovePossible` (`crnonpl.cc:2202`) has no summon check; a summon with KickCreatures
        // can kick blocking monsters.
        let has_target = target_attack.is_some() || target_follow.is_some();
        let posture = matches!(state, MonsterState::Attacking | MonsterState::Panic);
        let creature_kicker = can_push_creatures && posture && has_target;

        // C++ box-block gate (`CanKickBoxes`) is independent of attack posture.
        let can_kick_boxes = self.monster_can_kick_boxes(mover);

        // C++ kick-and-retry loop (`crnonpl.cc:2185` `for Attempt 0..100`): after each kick,
        // re-check the destination. If still blocked, kick again. Up to 100 attempts.
        if creature_kicker {
            for _attempt in 0..100 {
                let blockers: Vec<CreatureId> = self
                    .map
                    .get_tile(dest)
                    .map(|t| {
                        t.body()
                            .creatures
                            .iter()
                            .copied()
                            .filter(|&c| c != mover)
                            .collect()
                    })
                    .unwrap_or_default();
                if blockers.is_empty() {
                    break; // destination clear — proceed with the step
                }
                // C++ `MovePossible` processes the first creature on the tile; if it's a hard
                // block or a kick-kill, throw EXHAUSTED. Otherwise kick it and loop.
                let blocker = blockers[0];
                // C++ `MovePossible` creature gate (`crnonpl.cc:2207-2210`): never kick the
                // mover's own target or master — these are hard blocks, not `EXHAUSTED`.
                if Some(blocker) == target_attack
                    || Some(blocker) == target_follow
                    || Some(blocker) == master
                {
                    break; // hard block — stop kicking, but still proceed (step will fail at tile_query)
                }
                match self.creatures.get(blocker) {
                    // P1-B3: C++ `crnonpl.cc:2221-2223`: invisible blocker (when mover lacks
                    // SeeInvisible) is a hard block — not kicked, not EXHAUSTED.
                    Some(k) if !see_invisible && k.base().is_invisible() => break,
                    // P1-B2: C++ `crnonpl.cc:2230`: a summon (Master != 0) treats a player tile
                    // as a hard block. A player with `IGNORED_BY_MONSTERS` is also a hard block.
                    // Otherwise (`crnonpl.cc:2236-2238`): a player blocker clears `Target` and
                    // throws `EXHAUSTED`.
                    Some(CreatureKind::Player(p)) if master.is_some() => break,
                    Some(CreatureKind::Player(p))
                        if has_player_flag(
                            flags_for_group(&self.groups, p.group_id),
                            PLAYER_FLAG_IGNORED_BY_MONSTERS,
                        ) =>
                    {
                        break
                    }
                    // C++ `crnonpl.cc:2236-2238`: player-tile `EXHAUSTED` — `Target = 0` before
                    // `throw EXHAUSTED`. The `Execute` catch (`cract.cc:870-877`) does NOT clear
                    // `Target` itself; the throw site does. F3: split from kick-kill `Exhausted`.
                    Some(CreatureKind::Player(_)) => {
                        return MonsterKickOutcome::ExhaustedDropTarget
                    }
                    // NPC / unpushable monster → hard block (`crnonpl.cc:2216,2228`), not kicked.
                    Some(CreatureKind::Npc(_)) => break,
                    Some(CreatureKind::Monster(m)) if !m.is_pushable() => break,
                    Some(CreatureKind::Monster(_)) => {
                        // C++ `crnonpl.cc:2240-2242`: kick the blocker; a forced kill (no free
                        // adjacent tile) still throws `EXHAUSTED`. F3: `Exhausted` (not
                        // `ExhaustedDropTarget`) — the kick-kill throw site does NOT clear
                        // `Target` (`crnonpl.cc:2241-2242`); the `Execute` catch preserves it
                        // (`cract.cc:870-877`).
                        if !self.monster_kick_creature(mover, blocker, mover_pos, now) {
                            return MonsterKickOutcome::Exhausted;
                        }
                        // Kick succeeded — loop re-checks the destination tile for more blockers.
                    }
                    None => break,
                }
            }
        }

        // Boxes / hazard fields — `MovePossible` `UNPASS`/`AVOID` branches (`crnonpl.cc:2249-2287`).
        if can_kick_boxes {
            self.monster_kick_boxes(mover, dest, state);
        }

        MonsterKickOutcome::Proceed
    }

    /// 772 `TMonster::KickBoxes` — `crnonpl.cc:2994-3033`.
    ///
    /// Shoves every blocking movable `UNPASS` / non-ignored `AVOID` item off `dest` to the first
    /// adjacent `BANK && !UNPASS` tile in fixed N,S,W,E order (skipping the mover's own tile),
    /// deleting it (with `EFFECT_BLOCK_HIT`) when no destination is free. Immovable (`UNMOVE`)
    /// items are hard blocks and left in place (handled by the `MovePossible` planning gate).
    fn monster_kick_boxes(&mut self, mover: CreatureId, dest: Position, state: MonsterState) {
        let mover_pos = match self.creatures.get(mover) {
            Some(k) => k.position(),
            None => return,
        };
        // P1-B1: C++ `MovePossible` AVOID branch (`crnonpl.cc:2264-2267`) — per-damage-type
        // immunity. PANIC ignores all hazards; NoPoison/NoBurning/NoEnergy ignore matching
        // fields only. Was poison-only, now per-type.
        let (immunity_poison, immunity_fire, immunity_energy) = match self.creatures.get(mover) {
            Some(CreatureKind::Monster(m)) => {
                (m.immunity_poison, m.immunity_fire, m.immunity_energy)
            }
            _ => (false, false, false),
        };

        // Snapshot the blocking items first — kicking mutates the tile chain.
        let to_kick: Vec<ItemId> = self
            .map
            .get_tile(dest)
            .map(|t| {
                t.body()
                    .down_items
                    .iter()
                    .chain(t.body().top_items.iter())
                    .copied()
                    .filter(|&iid| {
                        self.item_is_kickable_box(
                            iid,
                            state,
                            immunity_poison,
                            immunity_fire,
                            immunity_energy,
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();

        for item_id in to_kick {
            self.monster_kick_single_box(mover, item_id, dest, mover_pos);
        }
    }

    /// True when an item on a destination tile is a movable blocker the mover must shove
    /// (`MovePossible` `UNPASS`/`AVOID` branches, `crnonpl.cc:2250-2284`). Hazard `AVOID` fields
    /// are ignored while `PANIC` or when the mover is immune to the field's damage type
    /// (`crnonpl.cc:2264-2267`).
    fn item_is_kickable_box(
        &self,
        item_id: ItemId,
        state: MonsterState,
        immunity_poison: bool,
        immunity_fire: bool,
        immunity_energy: bool,
    ) -> bool {
        let Some(item) = self.items.get(item_id) else {
            return false;
        };
        let server_id = item.item_type;
        if self.items_db.is_unpassable(server_id) {
            return !self.items_db.is_immovable(server_id);
        }
        if self.items_db.is_avoid_hazard(server_id) {
            // P1-B1: per-damage-type immunity — PANIC ignores all; type-specific immunity
            // ignores matching fields only.
            let ignore_hazard = state == MonsterState::Panic
                || match self.items_db.avoid_damage_type(server_id) {
                    Some(tfs_rust_content::items::FieldDamageType::Poison) => immunity_poison,
                    Some(tfs_rust_content::items::FieldDamageType::Fire) => immunity_fire,
                    Some(tfs_rust_content::items::FieldDamageType::Energy) => immunity_energy,
                    None => false,
                };
            return !ignore_hazard && !self.items_db.is_immovable(server_id);
        }
        false
    }

    /// Move one box to an adjacent `BANK && !UNPASS` tile, or delete it (`crnonpl.cc:3001-3027`).
    fn monster_kick_single_box(
        &mut self,
        mover: CreatureId,
        item_id: ItemId,
        item_pos: Position,
        mover_pos: Position,
    ) {
        let count = self.items.get(item_id).map(|i| i.count).unwrap_or(1);
        for dir in KICK_DIRS {
            let target = item_pos.offset(dir);
            // C++: skip the tile the mover itself stands on.
            if target.x == mover_pos.x && target.y == mover_pos.y && target.z == mover_pos.z {
                continue;
            }
            if !self.tile_is_bank_and_passable(target) {
                continue;
            }
            if self
                .internal_move_item(
                    Some(mover),
                    Cylinder::Tile { pos: item_pos },
                    Cylinder::Tile { pos: target },
                    item_id,
                    count,
                    CylinderFlags::default(),
                    None,
                )
                .is_ok()
            {
                return;
            }
        }

        // C++ KickBoxes: "Kein Platz zum Verschieben => löschen." Delete with the block-hit effect.
        self.broadcast_magic_effect(item_pos, EFFECT_BLOCK_HIT);
        let _ = self.internal_remove_item_from_tile(item_pos, item_id, count);
    }

    /// 772 `CoordinateFlag(Dest, BANK) && !CoordinateFlag(Dest, UNPASS)` — a walkable terrain tile
    /// with no solid blocker (`crnonpl.cc:3018-3019`).
    fn tile_is_bank_and_passable(&self, pos: Position) -> bool {
        let Some(tile) = self.map.get_tile(pos) else {
            return false;
        };
        let body = tile.body();
        let Some(ground) = body.ground else {
            return false;
        };
        if !self.items_db.is_terrain_bank(ground) {
            return false;
        }
        // Any UNPASS item (or a solid-blocking tile flag) makes the tile non-passable.
        if (body.flags & tilestate::BLOCKSOLID) != 0 {
            return false;
        }
        !body
            .down_items
            .iter()
            .chain(body.top_items.iter())
            .any(|&iid| {
                self.items
                    .get(iid)
                    .is_some_and(|it| self.items_db.is_unpassable(it.item_type))
            })
    }

    /// 772 `TMonster::KickCreature` — `crnonpl.cc:3036`.
    ///
    /// Tries the fixed N,S,W,E offsets (skipping the kicker's own tile and `AVOID`/magic-field
    /// tiles); relocates the blocker to the first valid one. If none work, the blocker is **killed**
    /// with full parity to C++ (`crnonpl.cc:3076-3080`): full-HP physical damage attributed to the
    /// kicker (so kill credit / loot / experience go to it) + the block-hit effect, then the death
    /// pipeline (corpse + loot drop + exp distribution). Returns `true` if moved, `false` if killed.
    ///
    /// F2: destination validation uses the execute-mode `MovePossible` gate
    /// ([`Self::monster_move_possible_execute_for_kick`]) — the blocker's own
    /// `MovePossible(Execute=true)` (`crnonpl.cc:3066`) — which recursively kicks pushable
    /// creatures on the escape tile (chain-push). Was: planning gate (`Execute=false`) which
    /// skipped the recursive kick and caused stacking + spurious kills in dense convoys.
    fn monster_kick_creature(
        &mut self,
        kicker: CreatureId,
        blocker: CreatureId,
        mover_pos: Position,
        now: Instant,
    ) -> bool {
        self.monster_kick_creature_inner(kicker, blocker, mover_pos, now, 0)
    }

    /// F2: depth-threaded inner variant of [`Self::monster_kick_creature`].
    ///
    /// `depth` bounds the recursive chain-push (A→B→C→…). C++ has no explicit guard — it relies
    /// on the fixed N,S,W,E offset order + `skip kicker's tile` check (`crnonpl.cc:3062-3064`).
    /// Rust is explicit via [`MAX_KICK_DEPTH`].
    fn monster_kick_creature_inner(
        &mut self,
        kicker: CreatureId,
        blocker: CreatureId,
        mover_pos: Position,
        now: Instant,
        depth: u8,
    ) -> bool {
        // F2 cycle guard.
        if depth >= MAX_KICK_DEPTH {
            return false;
        }
        // C++ `KickCreature` only kicks monsters (`crnonpl.cc:3042-3045`). The top-level caller
        // only invokes this for monster blockers, but the recursive chain-push may encounter
        // non-monsters on escape tiles — guard prevents accidentally killing players.
        if !self
            .creatures
            .get(blocker)
            .is_some_and(|k| matches!(k, CreatureKind::Monster(_)))
        {
            return false;
        }
        let blocker_pos = match self.creatures.get(blocker) {
            Some(k) => k.position(),
            None => return false,
        };

        for dir in KICK_DIRS {
            let try_pos = blocker_pos.offset(dir);
            // C++: skip the tile the kicker itself stands on.
            if try_pos.x == mover_pos.x && try_pos.y == mover_pos.y && try_pos.z == mover_pos.z {
                continue;
            }
            // C++ `!CoordinateFlag(Dest, AVOID)` — don't shove a creature onto a hazard field.
            if let Some(tile) = self.map.get_tile(try_pos) {
                if (tile.body().flags & tilestate::MAGICFIELD) != 0 {
                    continue;
                }
            }
            // F2: C++ `Creature->MovePossible(Dest, Execute=true)` (`crnonpl.cc:3066`) — the
            // blocker's own `MovePossible` in execute mode, which recursively kicks pushable
            // creatures on the escape tile (chain-push). Was: planning gate (`Execute=false`)
            // which treated pushable monsters as plannable-through and skipped the recursive
            // kick, causing stacking + spurious kills in dense convoys (audit F2).
            let can_occupy = self.monster_move_possible_execute_for_kick(
                blocker,
                try_pos,
                blocker_pos,
                now,
                depth,
            );
            if !can_occupy {
                continue;
            }
            // Forced relocate — bypasses `tile_query_add_creature` (no walk-timer gate, no 1098
            // creature-block model). C++ `KickCreature` calls `Creature::Move` directly after
            // `MovePossible` passes.
            //
            // C++ `Creature::Move` updates spectators (`sendCreatureMove`). We must do the same:
            // capture the blocker's old tile creature list *before* the relocate, then broadcast
            // a `0x6D` move packet. Without this the client keeps the creature on its old tile;
            // a later removal packet (computed at the NEW tile) is then discarded by the client,
            // leaving the creature visibly alive after it has died server-side (the reported
            // cyclops-walks-over-wolf desync). The creature list is needed for per-viewer stack
            // position computation (`Tile::getClientIndexOfCreature`, `tile.cpp:1207-1214`).
            let old_creatures = self
                .map
                .get_tile(blocker_pos)
                .map(|t| t.body().creatures.clone())
                .unwrap_or_default();
            // C++ `::Move` order (`operate.cc:1403–1434`): NotifyTurn → AnnounceMoving →
            // MoveObject → NotifyGo. Broadcast *before* relocate so stackpos matches the
            // still-occupied tile (C++ `GetObjectRNum` while CrObject is on the old field).
            let kick_dir = dir;
            if let Some(k) = self.creatures.get_mut(blocker) {
                // NotifyTurn — facing only (`cract.cc:1566–1581`); no `0x6B`.
                crate::walk::set_direction_from_step_for_kick(blocker_pos, try_pos, k);
            }
            self.broadcast_spectator_move(blocker, blocker_pos, try_pos, &old_creatures);
            self.move_creature_on_map(blocker, blocker_pos, try_pos);
            self.flush_pending_creature_step_events();
            // C++ `KickCreature` → `::Move` relocates the creature but does NOT clear its
            // ToDoList (`operate.cc:1403-1446`). The displacement is detected on the next
            // `Execute` when `Go(oldDestX, oldDestY, oldDestZ)` checks `Distance > 1`
            // (`cract.cc:386-389`) → `throw NOTACCESSIBLE` → `ToDoClear + ToDoYield`
            // (`cract.cc:870-877`). The Rust `walk_destinations` overlay (now populated for
            // monsters too) stores the absolute destination of each queued step, so `on_walk`
            // detects the displacement via the same adjacency check as players.
            // NotifyGo already applied facing — pass `apply_notify_turn=false`.
            self.apply_notify_go_after_relocate(blocker, blocker_pos, try_pos, kick_dir, false);
            // Pending wakeup may predate the kick; push it out to EarliestWalkTime so a
            // premature `Go` cannot race the client walk animation (OTC dash/skip).
            self.reschedule_wakeup_for_earliest_walk(blocker);
            return true;
        }

        // C++ KickCreature: "Kein Platz zum Verschieben => Töten." (`crnonpl.cc:3074-3080`).
        self.monster_kick_creature_kill(kicker, blocker, blocker_pos);
        false
    }

    /// KickCreature kill arm — `crnonpl.cc:3074-3080`:
    /// `GraphicalEffect(EFFECT_BLOCK_HIT); Combat.AddDamageToCombatList(kicker, fullHP); Kill()`.
    /// Used when no N/S/W/E escape tile is free.
    fn monster_kick_creature_kill(
        &mut self,
        kicker: CreatureId,
        blocker: CreatureId,
        blocker_pos: Position,
    ) {
        self.broadcast_magic_effect(blocker_pos, EFFECT_BLOCK_HIT);
        let victim_hp = self
            .creatures
            .get(blocker)
            .map(|k| k.base().health)
            .unwrap_or(0);
        if victim_hp > 0 {
            crate::combat::execute(
                &mut self.creatures,
                Some(kicker),
                blocker,
                &crate::combat::CombatDamage {
                    primary: (CombatType::Physical, -victim_hp),
                    secondary: (CombatType::Physical, 0),
                },
                &crate::combat::CombatParams::default(),
            );
        }
        // C++ `Kill()` — death xp/events/corpse + remove (mirrors `combat_execute_with_stimulus`'s
        // post-apply death branch without re-running `DamageStimulus`, which `Kill()` skips).
        self.apply_creature_death(blocker);
    }

    /// F2: 772 `TMonster::MovePossible(Execute=true)` for `KickCreature` dest validation
    /// (`crnonpl.cc:3066`). Unlike [`Self::monster_move_possible_planning`] (Execute=false),
    /// this recursively kicks pushable creatures on the escape tile (chain-push) before declaring
    /// it passable. Returns `true` if the blocker can occupy `try_pos` (after any chain-kick
    /// side-effects), `false` on a hard block or a failed chain-kick.
    ///
    /// `kicker_pos` is the blocker's current position (the blocker is the "kicker" in the
    /// recursive call) — used to skip the blocker's own tile in the recursive kick.
    fn monster_move_possible_execute_for_kick(
        &mut self,
        blocker: CreatureId,
        try_pos: Position,
        kicker_pos: Position,
        now: Instant,
        depth: u8,
    ) -> bool {
        // Reuse the planning gate for non-creature blocks (leash, PZ, house, items, terrain).
        // Hard blocks (unpushable, target, master, invisible, NPC, summon-player, IGNORED) return
        // false here. Pushable monsters and players are plannable-through (planning `continue`s).
        if !self.monster_move_possible_planning(blocker, try_pos) {
            return false;
        }
        // Planning passed — but if a pushable creature is on `try_pos`, planning treated it as
        // plannable-through. Execute-mode must KICK it (chain-push) before declaring the tile
        // occupiable. Re-check the tile each iteration (creatures move during chain-kicks).
        loop {
            let other = self
                .map
                .get_tile(try_pos)
                .and_then(|t| t.body().creatures.iter().copied().find(|&c| c != blocker));
            let Some(other) = other else {
                break; // tile clear → passable
            };
            // Only pushable monsters reach here (hard blocks returned false via planning;
            // non-monsters cause `monster_kick_creature_inner` to return false). Recursively
            // kick — C++ `Creature->MovePossible(Execute=true)` (`crnonpl.cc:3066`).
            if !self.monster_kick_creature_inner(blocker, other, kicker_pos, now, depth + 1) {
                return false; // kick failed (kill or no escape) → tile not passable
            }
        }
        true
    }
}

#[cfg(test)]
#[path = "monster_push_tests.rs"]
mod tests;
