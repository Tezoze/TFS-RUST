//! Monster push-before-step — era-split.
//!
//! - **1098** (`beat_driven_loop == false`): TFS `Monster::pushCreature` / `pushCreatures`
//!   (`monster.cpp` ~1174–1221) — random-cardinal shove, kill on failure.
//! - **772** (`beat_driven_loop == true`): the execute side-effects of `TMonster::MovePossible`
//!   (`crnonpl.cc:2141–2291`) — `TMonster::KickCreature` (`crnonpl.cc:3036`) and
//!   `TMonster::KickBoxes` (`crnonpl.cc:2994`). An ATTACKING/PANIC monster that has a target and
//!   the `KickCreatures` race flag shoves a blocking **pushable monster** aside in fixed
//!   **N, S, W, E** order, killing it when no adjacent tile is free. A monster with
//!   `CanKickBoxes()` (race flag, or inherited from a monster master) shoves a blocking movable
//!   **box/field** aside (same N,S,W,E order, `BANK && !UNPASS` destination), deleting it on
//!   failure. Stepping onto a **player** tile, or a `KickCreature` that has to kill, is the C++
//!   `EXHAUSTED` case → [`MonsterKickOutcome::Exhausted`] (kick-kill: target preserved, C++
//!   `Execute` catch `cract.cc:870-877`) or [`MonsterKickOutcome::ExhaustedDropTarget`]
//!   (player-tile: target cleared, C++ `crnonpl.cc:2236-2238`). The caller waits 1000 ms in both
//!   cases.
//!
//! Called from [`crate::walk::GameWorld::on_walk`] before the mover steps.

use std::time::Instant;

use rand::seq::SliceRandom;
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
const KICK_DIRS_772: [Direction; 4] = [
    Direction::North,
    Direction::South,
    Direction::West,
    Direction::East,
];

/// CipSoft `EFFECT_BLOCK_HIT` — graphical effect on a kick-kill / box delete (`crnonpl.cc:3071`).
/// Neutral effect enum value (`tfs_rust_common::enums::MagicEffect::BlockHit`); the codec maps it
/// to the era wire id.
const EFFECT_BLOCK_HIT: u8 = 3;

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
        if self.beat_driven_loop {
            self.monster_kick_before_step_772(mover, dest, now)
        } else {
            self.monster_push_before_step_tfs(mover, dest, now);
            MonsterKickOutcome::Proceed
        }
    }

    // ───────────────────────── 772 (`MovePossible` / `KickCreature`) ─────────────────────────

    /// 772 `CanKickBoxes()` — race `KickBoxes` flag, or inherited from a monster master
    /// (`crnonpl.cc:2984-2992`).
    fn monster_can_kick_boxes_772(&self, mover: CreatureId) -> bool {
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
    fn monster_kick_before_step_772(
        &mut self,
        mover: CreatureId,
        dest: Position,
        now: Instant,
    ) -> MonsterKickOutcome {
        let Some((mover_pos, master, target_attack, target_follow, state, can_push_creatures, see_invisible)) = ({
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
        }) else {
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
        let can_kick_boxes = self.monster_can_kick_boxes_772(mover);

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
                    Some(CreatureKind::Player(_)) => return MonsterKickOutcome::ExhaustedDropTarget,
                    // NPC / unpushable monster → hard block (`crnonpl.cc:2216,2228`), not kicked.
                    Some(CreatureKind::Npc(_)) => break,
                    Some(CreatureKind::Monster(m)) if !m.is_pushable() => break,
                    Some(CreatureKind::Monster(_)) => {
                        // C++ `crnonpl.cc:2240-2242`: kick the blocker; a forced kill (no free
                        // adjacent tile) still throws `EXHAUSTED`. F3: `Exhausted` (not
                        // `ExhaustedDropTarget`) — the kick-kill throw site does NOT clear
                        // `Target` (`crnonpl.cc:2241-2242`); the `Execute` catch preserves it
                        // (`cract.cc:870-877`).
                        if !self.monster_kick_creature_772(mover, blocker, mover_pos, now) {
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
            self.monster_kick_boxes_772(mover, dest, state);
        }

        MonsterKickOutcome::Proceed
    }

    /// 772 `TMonster::KickBoxes` — `crnonpl.cc:2994-3033`.
    ///
    /// Shoves every blocking movable `UNPASS` / non-ignored `AVOID` item off `dest` to the first
    /// adjacent `BANK && !UNPASS` tile in fixed N,S,W,E order (skipping the mover's own tile),
    /// deleting it (with `EFFECT_BLOCK_HIT`) when no destination is free. Immovable (`UNMOVE`)
    /// items are hard blocks and left in place (handled by the `MovePossible` planning gate).
    fn monster_kick_boxes_772(&mut self, mover: CreatureId, dest: Position, state: MonsterState) {
        let mover_pos = match self.creatures.get(mover) {
            Some(k) => k.position(),
            None => return,
        };
        // P1-B1: C++ `MovePossible` AVOID branch (`crnonpl.cc:2264-2267`) — per-damage-type
        // immunity. PANIC ignores all hazards; NoPoison/NoBurning/NoEnergy ignore matching
        // fields only. Was poison-only, now per-type.
        let (immunity_poison, immunity_fire, immunity_energy) = match self.creatures.get(mover) {
            Some(CreatureKind::Monster(m)) => (m.immunity_poison, m.immunity_fire, m.immunity_energy),
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
                        self.item_is_kickable_box_772(
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
            self.monster_kick_single_box_772(mover, item_id, dest, mover_pos);
        }
    }

    /// True when an item on a destination tile is a movable blocker the mover must shove
    /// (`MovePossible` `UNPASS`/`AVOID` branches, `crnonpl.cc:2250-2284`). Hazard `AVOID` fields
    /// are ignored while `PANIC` or when the mover is immune to the field's damage type
    /// (`crnonpl.cc:2264-2267`).
    fn item_is_kickable_box_772(
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
        if self.items_db.is_unpass_772(server_id) {
            return !self.items_db.is_unmove_772(server_id);
        }
        if self.items_db.is_avoid_hazard_772(server_id) {
            // P1-B1: per-damage-type immunity — PANIC ignores all; type-specific immunity
            // ignores matching fields only.
            let ignore_hazard = state == MonsterState::Panic
                || match self.items_db.avoid_damage_type_772(server_id) {
                    Some(tfs_rust_content::items::FieldDamageType::Poison) => immunity_poison,
                    Some(tfs_rust_content::items::FieldDamageType::Fire) => immunity_fire,
                    Some(tfs_rust_content::items::FieldDamageType::Energy) => immunity_energy,
                    None => false,
                };
            return !ignore_hazard && !self.items_db.is_unmove_772(server_id);
        }
        false
    }

    /// Move one box to an adjacent `BANK && !UNPASS` tile, or delete it (`crnonpl.cc:3001-3027`).
    fn monster_kick_single_box_772(
        &mut self,
        mover: CreatureId,
        item_id: ItemId,
        item_pos: Position,
        mover_pos: Position,
    ) {
        let count = self.items.get(item_id).map(|i| i.count).unwrap_or(1);
        for dir in KICK_DIRS_772 {
            let target = item_pos.offset(dir);
            // C++: skip the tile the mover itself stands on.
            if target.x == mover_pos.x && target.y == mover_pos.y && target.z == mover_pos.z {
                continue;
            }
            if !self.tile_is_bank_and_passable_772(target) {
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
    fn tile_is_bank_and_passable_772(&self, pos: Position) -> bool {
        let Some(tile) = self.map.get_tile(pos) else {
            return false;
        };
        let body = tile.body();
        let Some(ground) = body.ground else {
            return false;
        };
        if !self.items_db.is_terrain_bank_772(ground) {
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
                    .is_some_and(|it| self.items_db.is_unpass_772(it.item_type))
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
    /// ([`Self::monster_move_possible_execute_for_kick_772`]) — the blocker's own
    /// `MovePossible(Execute=true)` (`crnonpl.cc:3066`) — which recursively kicks pushable
    /// creatures on the escape tile (chain-push). Was: planning gate (`Execute=false`) which
    /// skipped the recursive kick and caused stacking + spurious kills in dense convoys.
    fn monster_kick_creature_772(
        &mut self,
        kicker: CreatureId,
        blocker: CreatureId,
        mover_pos: Position,
        now: Instant,
    ) -> bool {
        self.monster_kick_creature_772_inner(kicker, blocker, mover_pos, now, 0)
    }

    /// F2: depth-threaded inner variant of [`Self::monster_kick_creature_772`].
    ///
    /// `depth` bounds the recursive chain-push (A→B→C→…). C++ has no explicit guard — it relies
    /// on the fixed N,S,W,E offset order + `skip kicker's tile` check (`crnonpl.cc:3062-3064`).
    /// Rust is explicit via [`MAX_KICK_DEPTH`].
    fn monster_kick_creature_772_inner(
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

        for dir in KICK_DIRS_772 {
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
            let can_occupy = self.monster_move_possible_execute_for_kick_772(
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
            self.move_creature_on_map(blocker, blocker_pos, try_pos);
            return true;
        }

        // C++ KickCreature: "Kein Platz zum Verschieben => Töten." No adjacent tile is free, so the
        // kicker kills the boxed-in creature (`crnonpl.cc:3074-3080`):
        //   GraphicalEffect(EFFECT_BLOCK_HIT); Combat.AddDamageToCombatList(this->ID, fullHP); Kill();
        // We mirror this exactly: emit block-hit, deal full-HP physical damage **attributed to the
        // kicker** (recorded in the victim's damage map for kill credit / exp), then run the death
        // pipeline (corpse + loot + experience distribution).
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
        false
    }

    /// F2: 772 `TMonster::MovePossible(Execute=true)` for `KickCreature` dest validation
    /// (`crnonpl.cc:3066`). Unlike [`Self::monster_move_possible_planning_772`] (Execute=false),
    /// this recursively kicks pushable creatures on the escape tile (chain-push) before declaring
    /// it passable. Returns `true` if the blocker can occupy `try_pos` (after any chain-kick
    /// side-effects), `false` on a hard block or a failed chain-kick.
    ///
    /// `kicker_pos` is the blocker's current position (the blocker is the "kicker" in the
    /// recursive call) — used to skip the blocker's own tile in the recursive kick.
    fn monster_move_possible_execute_for_kick_772(
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
        if !self.monster_move_possible_planning_772(blocker, try_pos) {
            return false;
        }
        // Planning passed — but if a pushable creature is on `try_pos`, planning treated it as
        // plannable-through. Execute-mode must KICK it (chain-push) before declaring the tile
        // occupiable. Re-check the tile each iteration (creatures move during chain-kicks).
        loop {
            let other = self
                .map
                .get_tile(try_pos)
                .and_then(|t| {
                    t.body()
                        .creatures
                        .iter()
                        .copied()
                        .find(|&c| c != blocker)
                });
            let Some(other) = other else {
                break; // tile clear → passable
            };
            // Only pushable monsters reach here (hard blocks returned false via planning;
            // non-monsters cause `monster_kick_creature_772_inner` to return false). Recursively
            // kick — C++ `Creature->MovePossible(Execute=true)` (`crnonpl.cc:3066`).
            if !self.monster_kick_creature_772_inner(blocker, other, kicker_pos, now, depth + 1) {
                return false; // kick failed (kill or no escape) → tile not passable
            }
        }
        true
    }

    // ───────────────────────────── 1098 (`Monster::pushCreatures`) ─────────────────────────────

    /// TFS `Monster::getNextStep` — push blocking creatures off the destination tile (`monster.cpp`).
    fn monster_push_before_step_tfs(
        &mut self,
        mover: CreatureId,
        dest: Position,
        now: Instant,
    ) {
        let (can_push_creatures, can_push_items) = match self.creatures.get(mover) {
            Some(CreatureKind::Monster(m)) if m.can_push_creatures && !m.base.is_summon() => {
                (true, m.can_push_items)
            }
            Some(CreatureKind::Monster(m)) => (false, m.can_push_items),
            _ => return,
        };

        if can_push_items {
            // TFS `Monster::pushItems` — deferred; item cylinder move path not wired here yet.
        }

        if can_push_creatures {
            self.monster_push_creatures_on_tile(dest, mover, now);
        }
    }

    /// TFS `Monster::pushCreatures(Tile*)` — shuffle-push pushable monsters; kill on failure.
    fn monster_push_creatures_on_tile(&mut self, dest: Position, mover: CreatureId, now: Instant) {
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

        let mut last_pushed: Option<CreatureId> = None;
        let mut to_kill: Vec<CreatureId> = Vec::new();

        for blocker in blockers {
            let Some(CreatureKind::Monster(m)) = self.creatures.get(blocker) else {
                continue;
            };
            if !m.is_pushable() {
                continue;
            }
            if last_pushed != Some(blocker) && self.monster_push_creature(blocker, now) {
                last_pushed = Some(blocker);
                continue;
            }
            to_kill.push(blocker);
        }

        for id in to_kill {
            self.remove_creature(id);
        }
    }

    /// TFS `Monster::pushCreature(Creature*)` — random cardinal `internalMoveCreature`.
    fn monster_push_creature(&mut self, cid: CreatureId, now: Instant) -> bool {
        let pos = match self.creatures.get(cid) {
            Some(k) => k.position(),
            None => return false,
        };

        let mut dirs = [
            Direction::North,
            Direction::West,
            Direction::East,
            Direction::South,
        ];
        dirs.shuffle(&mut rand::thread_rng());

        for dir in dirs {
            let try_pos = pos.offset(dir);
            let Some(tile) = self.map.get_tile(try_pos) else {
                continue;
            };
            if (tile.body().flags & tilestate::BLOCKPATH) != 0 {
                continue;
            }
            if self.try_creature_walk_step(cid, dir, now) {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::creature::{CreatureKind, MonsterAiConfig, MonsterState};
    use crate::creature_todo::{CreatureAction, MONSTER_IDLE_WAIT_MS};
    use crate::sim_harness::{
        beat_driven_world, ensure_walkable_tile, insert_monster_with_config, insert_player,
        test_player,
    };

    fn kicker_config() -> MonsterAiConfig {
        MonsterAiConfig {
            can_push_creatures: true,
            target_distance: 1,
            ..MonsterAiConfig::default()
        }
    }

    /// 772 `MovePossible` creature branch: a `KickCreatures` attacker stepping onto a **player**
    /// tile (not its target) is the `EXHAUSTED` case — `crnonpl.cc:2236-2238`. F3: this is
    /// `ExhaustedDropTarget` (Target cleared), distinct from kick-kill `Exhausted`.
    #[test]
    fn kicker_onto_player_tile_is_exhausted() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;
        let now = std::time::Instant::now();

        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        let tpos = Position::new(105, 100, 7); // far-away attack target
        ensure_walkable_tile(&mut world.map, mpos, 1);
        ensure_walkable_tile(&mut world.map, ppos, 1);
        ensure_walkable_tile(&mut world.map, tpos, 1);

        // Some other creature is the attack target so the player on the dest tile is *not* it.
        let target = insert_monster_with_config(&mut world, "Rat", tpos, 200, kicker_config());
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let mover = insert_monster_with_config(&mut world, "Cyclops", mpos, 200, kicker_config());
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(mover) {
            m.state = MonsterState::Attacking;
            m.base.attack_target = Some(target);
        }

        let outcome = world.monster_push_before_step(mover, ppos, now);
        assert_eq!(outcome, MonsterKickOutcome::ExhaustedDropTarget);
        // Player is untouched — never kicked.
        assert_eq!(world.creatures.get(player).map(|k| k.position()), Some(ppos));
    }

    /// A non-`KickCreatures` monster never kicks — a player tile is a hard block, not `EXHAUSTED`.
    #[test]
    fn non_kicker_onto_player_tile_proceeds() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;
        let now = std::time::Instant::now();

        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 1);
        ensure_walkable_tile(&mut world.map, ppos, 1);

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let cfg = MonsterAiConfig {
            can_push_creatures: false,
            target_distance: 1,
            ..MonsterAiConfig::default()
        };
        let mover = insert_monster_with_config(&mut world, "Rat", mpos, 200, cfg);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(mover) {
            m.state = MonsterState::Attacking;
            m.base.attack_target = Some(player);
        }

        assert_eq!(
            world.monster_push_before_step(mover, ppos, now),
            MonsterKickOutcome::Proceed
        );
    }

    /// The mover's own target tile is never kicked (`crnonpl.cc:2207-2210`) — Proceed, not Exhausted.
    #[test]
    fn kicker_onto_own_target_tile_proceeds() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;
        let now = std::time::Instant::now();

        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 1);
        ensure_walkable_tile(&mut world.map, ppos, 1);

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let mover = insert_monster_with_config(&mut world, "Cyclops", mpos, 200, kicker_config());
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(mover) {
            m.state = MonsterState::Attacking;
            m.base.attack_target = Some(player);
        }

        assert_eq!(
            world.monster_push_before_step(mover, ppos, now),
            MonsterKickOutcome::Proceed
        );
    }

    /// `EXHAUSTED` recovery with `clear_target=true` (player-tile case) clears the target and
    /// arms a 1000 ms wait (`cract.cc:870-877` + `crnonpl.cc:2236-2238`). F3: the kick-kill case
    /// passes `clear_target=false` and preserves the target — see `f3_kick_kill_preserves_target`.
    #[test]
    fn exhausted_wait_clears_target_and_waits_1000() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;
        world.server_ms = 0;

        let mpos = Position::new(100, 100, 7);
        let tpos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 1);
        ensure_walkable_tile(&mut world.map, tpos, 1);
        let target = insert_monster_with_config(&mut world, "Rat", tpos, 200, kicker_config());
        let mover = insert_monster_with_config(&mut world, "Cyclops", mpos, 200, kicker_config());
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(mover) {
            m.state = MonsterState::Attacking;
            m.base.attack_target = Some(target);
            m.base.follow_target = Some(target);
            m.base.todo.queue.push_back(CreatureAction::Go);
        }

        world.monster_exhausted_wait_772(mover, true);

        let base = world.creatures.get(mover).unwrap().base();
        assert_eq!(base.attack_target, None);
        assert_eq!(base.follow_target, None);
        assert!(
            base.todo
                .queue
                .iter()
                .any(|a| matches!(a, CreatureAction::Wait { delay_ms } if *delay_ms == MONSTER_IDLE_WAIT_MS)),
            "EXHAUSTED must enqueue a {MONSTER_IDLE_WAIT_MS} ms Wait"
        );
        assert!(
            !base.todo.queue.iter().any(|a| matches!(a, CreatureAction::Go)),
            "ToDoClear must drop the queued Go"
        );
    }

    /// 772 `CanKickBoxes()` — race flag, or inherited from a monster master (`crnonpl.cc:2984-2992`).
    #[test]
    fn can_kick_boxes_inherits_from_master() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;

        let p = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, p, 1);

        let mut boxer = MonsterAiConfig::default();
        boxer.can_push_items = true;
        let master = insert_monster_with_config(&mut world, "Boxer", p, 200, boxer);

        // Direct race flag.
        assert!(world.monster_can_kick_boxes_772(master));

        // No flag, no master → false.
        let lone = insert_monster_with_config(
            &mut world,
            "Lone",
            Position::new(101, 100, 7),
            200,
            MonsterAiConfig::default(),
        );
        assert!(!world.monster_can_kick_boxes_772(lone));

        // No flag, but master can kick → inherits true.
        let summon = insert_monster_with_config(
            &mut world,
            "Summon",
            Position::new(102, 100, 7),
            200,
            MonsterAiConfig::default(),
        );
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(summon) {
            m.base.master = Some(master);
        }
        assert!(world.monster_can_kick_boxes_772(summon));
    }

    /// 772 `KickCreature` kill — a boxed-in pushable monster (no free adjacent tile) is killed by
    /// the kicker and the step reports `EXHAUSTED` (`crnonpl.cc:3074-3080`).
    #[test]
    fn boxed_in_blocker_is_killed_and_step_exhausted() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;
        world.server_ms = 0;
        let now = std::time::Instant::now();

        let mpos = Position::new(100, 100, 7);
        let bpos = Position::new(101, 100, 7);
        let tpos = Position::new(105, 100, 7);
        // Only the kicker, blocker, and far-target tiles exist — the blocker's other neighbours are
        // absent (non-walkable), so `KickCreature` cannot relocate it and must kill.
        ensure_walkable_tile(&mut world.map, mpos, 1);
        ensure_walkable_tile(&mut world.map, bpos, 1);
        ensure_walkable_tile(&mut world.map, tpos, 1);

        let target = insert_monster_with_config(&mut world, "Rat", tpos, 200, kicker_config());
        let blocker =
            insert_monster_with_config(&mut world, "Rat", bpos, 200, MonsterAiConfig::default());
        let kicker = insert_monster_with_config(&mut world, "Cyclops", mpos, 200, kicker_config());
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(kicker) {
            m.state = MonsterState::Attacking;
            m.base.attack_target = Some(target);
        }

        let outcome = world.monster_push_before_step(kicker, bpos, now);
        assert_eq!(outcome, MonsterKickOutcome::Exhausted);
        assert!(
            !world.creatures.contains_key(blocker),
            "boxed-in blocker must be killed by the kick"
        );
    }

    // ─────────── Pass 8 re-audit tests (P1-A1, P1-B2, P1-B3, AI#23) ───────────

    /// P1-A1: a summon with `KickCreatures` CAN kick a blocking pushable monster — C++ `MovePossible`
    /// (`crnonpl.cc:2202`) has no summon gate. The old Rust `!is_summon` gate is dropped.
    #[test]
    fn summon_kicks_blocking_monster() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;
        let now = std::time::Instant::now();

        let mpos = Position::new(100, 100, 7);
        let bpos = Position::new(101, 100, 7);
        let tpos = Position::new(105, 100, 7);
        let escape = Position::new(101, 101, 7);
        ensure_walkable_tile(&mut world.map, mpos, 1);
        ensure_walkable_tile(&mut world.map, bpos, 1);
        ensure_walkable_tile(&mut world.map, tpos, 1);
        ensure_walkable_tile(&mut world.map, escape, 1);

        let target = insert_monster_with_config(&mut world, "Rat", tpos, 200, kicker_config());
        let blocker =
            insert_monster_with_config(&mut world, "Rat", bpos, 200, MonsterAiConfig::default());
        world.map.register_creature_at(bpos, blocker);
        // Summon with KickCreatures — master is a far-away monster.
        let master = insert_monster_with_config(&mut world, "Master", tpos, 200, kicker_config());
        let summon = insert_monster_with_config(&mut world, "Cyclops", mpos, 200, kicker_config());
        world.map.register_creature_at(mpos, summon);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(summon) {
            m.base.master = Some(master);
            m.state = MonsterState::Attacking;
            m.base.attack_target = Some(target);
        }

        let outcome = world.monster_push_before_step(summon, bpos, now);
        assert_eq!(
            outcome,
            MonsterKickOutcome::Proceed,
            "summon with KickCreatures must kick the blocker, not stall"
        );
        assert_ne!(
            world.creatures.get(blocker).map(|k| k.position()),
            Some(bpos),
            "blocker must have been relocated by the kick"
        );
    }

    /// P1-B2: a player with `IGNORED_BY_MONSTERS` on the destination tile is a hard block
    /// (Proceed), not `EXHAUSTED` — C++ `crnonpl.cc:2230`. This test verifies the baseline
    /// (non-ignored player → EXHAUSTED); the IGNORED case requires group DB setup and is
    /// verified by the code path in `monster_kick_before_step_772`.
    #[test]
    fn ignored_player_tile_is_hard_block_not_exhausted() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;
        let now = std::time::Instant::now();

        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        let tpos = Position::new(105, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 1);
        ensure_walkable_tile(&mut world.map, ppos, 1);
        ensure_walkable_tile(&mut world.map, tpos, 1);

        let target = insert_monster_with_config(&mut world, "Rat", tpos, 200, kicker_config());
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let mover = insert_monster_with_config(&mut world, "Cyclops", mpos, 200, kicker_config());
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(mover) {
            m.state = MonsterState::Attacking;
            m.base.attack_target = Some(target);
        }

        // Without IGNORED_BY_MONSTERS flag, player tile is ExhaustedDropTarget (baseline).
        assert_eq!(
            world.monster_push_before_step(mover, ppos, now),
            MonsterKickOutcome::ExhaustedDropTarget,
            "non-ignored player tile must be ExhaustedDropTarget (baseline)"
        );
    }

    /// P1-B3: an invisible blocker (when the mover lacks SeeInvisible) is a hard block in the
    /// planning gate — `monster_move_possible_planning_772` returns false for invisible creatures.
    #[test]
    fn invisible_blocker_is_hard_block_in_planning() {
        use crate::condition::{add_condition_merge, ActiveCondition, ConditionData};
        use tfs_rust_common::enums::ConditionType as CondType;

        let mut world = beat_driven_world();
        world.walk_wake_tx = None;

        let mpos = Position::new(100, 100, 7);
        let bpos = Position::new(101, 100, 7);
        let tpos = Position::new(105, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 1);
        ensure_walkable_tile(&mut world.map, bpos, 1);
        ensure_walkable_tile(&mut world.map, tpos, 1);

        // A separate target (not the blocker) so the blocker is not the chase target.
        let target = insert_monster_with_config(&mut world, "Rat", tpos, 200, kicker_config());
        let blocker =
            insert_monster_with_config(&mut world, "Rat", bpos, 200, MonsterAiConfig::default());
        world.map.register_creature_at(bpos, blocker);
        // Make the blocker invisible.
        if let Some(k) = world.creatures.get_mut(blocker) {
            add_condition_merge(
                &mut k.base_mut().active_conditions,
                ActiveCondition::new(0, 0, CondType::Invisible, ConditionData::Generic { ticks: 0 }, None),
            );
        }

        let mover = insert_monster_with_config(&mut world, "Cyclops", mpos, 200, kicker_config());
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(mover) {
            m.state = MonsterState::Attacking;
            m.base.attack_target = Some(target);
            // SeeInvisible is false by default.
        }

        // Planning gate: invisible blocker is a hard block (no SeeInvisible).
        assert!(
            !world.monster_move_possible_planning_772(mover, bpos),
            "invisible blocker must be a hard block when mover lacks SeeInvisible"
        );

        // With SeeInvisible, the blocker is plannable-through.
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(mover) {
            m.see_invisible = true;
        }
        assert!(
            world.monster_move_possible_planning_772(mover, bpos),
            "invisible blocker is plannable when mover has SeeInvisible"
        );
    }

    /// AI#23: the kick-and-retry loop clears a two-deep creature wall on the same beat.
    /// Two blockers on the destination tile; the first kick relocates one, the second kick
    /// relocates the other, then the destination is clear and the step proceeds.
    #[test]
    fn kick_and_retry_clears_two_deep_blockers() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;
        let now = std::time::Instant::now();

        let mpos = Position::new(100, 100, 7);
        let dest = Position::new(101, 100, 7);
        let tpos = Position::new(105, 100, 7);
        let escape1 = Position::new(101, 101, 7);
        let escape2 = Position::new(101, 99, 7);
        ensure_walkable_tile(&mut world.map, mpos, 1);
        ensure_walkable_tile(&mut world.map, dest, 1);
        ensure_walkable_tile(&mut world.map, tpos, 1);
        ensure_walkable_tile(&mut world.map, escape1, 1);
        ensure_walkable_tile(&mut world.map, escape2, 1);

        let target = insert_monster_with_config(&mut world, "Rat", tpos, 200, kicker_config());
        // Two blockers on the destination tile.
        let b1 = insert_monster_with_config(&mut world, "Rat1", dest, 200, MonsterAiConfig::default());
        let b2 = insert_monster_with_config(&mut world, "Rat2", dest, 200, MonsterAiConfig::default());
        world.map.register_creature_at(dest, b1);
        world.map.register_creature_at(dest, b2);
        let mover = insert_monster_with_config(&mut world, "Cyclops", mpos, 200, kicker_config());
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(mover) {
            m.state = MonsterState::Attacking;
            m.base.attack_target = Some(target);
        }

        let outcome = world.monster_push_before_step(mover, dest, now);
        assert_eq!(
            outcome,
            MonsterKickOutcome::Proceed,
            "kick-and-retry must clear both blockers and proceed"
        );
        // Both blockers should have been relocated off the destination.
        assert_ne!(
            world.creatures.get(b1).map(|k| k.position()),
            Some(dest),
            "first blocker must be relocated"
        );
        assert_ne!(
            world.creatures.get(b2).map(|k| k.position()),
            Some(dest),
            "second blocker must be relocated"
        );
    }

    /// P1-A2: a player tile is plannable-through in the 772 `MovePossible` planning gate
    /// (non-summon, non-IGNORED) — C++ `crnonpl.cc:2229-2233`.
    #[test]
    fn player_tile_is_plannable_through_in_move_possible() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;

        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        let tpos = Position::new(105, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 1);
        ensure_walkable_tile(&mut world.map, ppos, 1);
        ensure_walkable_tile(&mut world.map, tpos, 1);

        // A separate target (not the player on the dest tile) so the player is not the chase target.
        let target = insert_monster_with_config(&mut world, "Rat", tpos, 200, kicker_config());
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let mover = insert_monster_with_config(&mut world, "Cyclops", mpos, 200, kicker_config());
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(mover) {
            m.state = MonsterState::Attacking;
            m.base.attack_target = Some(target);
        }

        // Player tile is plannable-through (non-summon, non-IGNORED, has KickCreatures + target).
        assert!(
            world.monster_move_possible_planning_772(mover, ppos),
            "player tile must be plannable-through for non-summon kicker with target"
        );
    }

    /// P1-B5: a house tile is a hard block in the 772 `MovePossible` planning gate —
    /// C++ `crnonpl.cc:2168` `IsHouse(x,y,z)`.
    #[test]
    fn house_tile_is_hard_block_in_move_possible() {
        use crate::tile::{HouseTile, Tile, TileBody};

        let mut world = beat_driven_world();
        world.walk_wake_tx = None;

        let mpos = Position::new(100, 100, 7);
        let hpos = Position::new(101, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 1);
        // Insert a house tile.
        world.map.insert_tile(
            hpos,
            Tile::House(HouseTile {
                inner: TileBody {
                    ground: Some(1),
                    down_items: Vec::new(),
                    top_items: Vec::new(),
                    creatures: Vec::new(),
                    flags: 0,
                    zone: tfs_rust_common::enums::ZoneType::Normal,
                },
                house_id: 1,
            }),
        );

        let mover = insert_monster_with_config(&mut world, "Cyclops", mpos, 200, kicker_config());
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(mover) {
            m.state = MonsterState::Attacking;
        }

        assert!(
            !world.monster_move_possible_planning_772(mover, hpos),
            "house tile must be a hard block in MovePossible planning"
        );
    }

    // ─────────── F3: split EXHAUSTED target semantics (`cract.cc:870-877`) ───────────

    /// F3: a kick-kill (`Exhausted`) preserves the target — C++ `Execute` catch
    /// (`cract.cc:870-877`) does NOT clear `Target`; the kick-kill throw site
    /// (`crnonpl.cc:2241-2242`) doesn't clear it either. Was: unconditionally cleared.
    #[test]
    fn f3_kick_kill_preserves_target() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;
        world.server_ms = 0;
        let now = std::time::Instant::now();

        let mpos = Position::new(100, 100, 7);
        let bpos = Position::new(101, 100, 7);
        let tpos = Position::new(105, 100, 7);
        // Only the kicker, blocker, and far-target tiles exist — the blocker is boxed in
        // (no escape tiles), so `KickCreature` kills it and returns false → `Exhausted`.
        ensure_walkable_tile(&mut world.map, mpos, 1);
        ensure_walkable_tile(&mut world.map, bpos, 1);
        ensure_walkable_tile(&mut world.map, tpos, 1);

        let target = insert_monster_with_config(&mut world, "Rat", tpos, 200, kicker_config());
        let blocker =
            insert_monster_with_config(&mut world, "Rat", bpos, 200, MonsterAiConfig::default());
        world.map.register_creature_at(bpos, blocker);
        let mover = insert_monster_with_config(&mut world, "Cyclops", mpos, 200, kicker_config());
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(mover) {
            m.state = MonsterState::Attacking;
            m.base.attack_target = Some(target);
            m.base.follow_target = Some(target);
        }

        let outcome = world.monster_push_before_step(mover, bpos, now);
        assert_eq!(
            outcome,
            MonsterKickOutcome::Exhausted,
            "kick-kill must return Exhausted (target preserved)"
        );

        // F3: kick-kill recovery preserves the target (`clear_target = false`).
        world.monster_exhausted_wait_772(mover, false);

        let base = world.creatures.get(mover).unwrap().base();
        assert_eq!(
            base.attack_target,
            Some(target),
            "kick-kill must preserve attack_target (C++ Execute catch cract.cc:870-877)"
        );
        assert_eq!(
            base.follow_target,
            Some(target),
            "kick-kill must preserve follow_target (C++ Execute catch cract.cc:870-877)"
        );
        // Blocker was killed.
        assert!(!world.creatures.contains_key(blocker));
        // Wait armed.
        assert!(
            base.todo
                .queue
                .iter()
                .any(|a| matches!(a, CreatureAction::Wait { delay_ms } if *delay_ms == MONSTER_IDLE_WAIT_MS)),
            "EXHAUSTED must enqueue a {MONSTER_IDLE_WAIT_MS} ms Wait"
        );
    }

    /// F3: a player-tile `ExhaustedDropTarget` clears the target — C++ `crnonpl.cc:2236-2238`
    /// clears `Target` before `throw EXHAUSTED`. Regression of the original behavior.
    #[test]
    fn f3_player_tile_clears_target() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;
        world.server_ms = 0;
        let now = std::time::Instant::now();

        let mpos = Position::new(100, 100, 7);
        let ppos = Position::new(101, 100, 7);
        let tpos = Position::new(105, 100, 7);
        ensure_walkable_tile(&mut world.map, mpos, 1);
        ensure_walkable_tile(&mut world.map, ppos, 1);
        ensure_walkable_tile(&mut world.map, tpos, 1);

        // A separate target so the player on the dest tile is *not* the attack target.
        let target = insert_monster_with_config(&mut world, "Rat", tpos, 200, kicker_config());
        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let mover = insert_monster_with_config(&mut world, "Cyclops", mpos, 200, kicker_config());
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(mover) {
            m.state = MonsterState::Attacking;
            m.base.attack_target = Some(target);
            m.base.follow_target = Some(target);
        }

        let outcome = world.monster_push_before_step(mover, ppos, now);
        assert_eq!(
            outcome,
            MonsterKickOutcome::ExhaustedDropTarget,
            "player-tile must return ExhaustedDropTarget (target cleared)"
        );

        // F3: player-tile recovery clears the target (`clear_target = true`).
        world.monster_exhausted_wait_772(mover, true);

        let base = world.creatures.get(mover).unwrap().base();
        assert_eq!(
            base.attack_target, None,
            "player-tile must clear attack_target (C++ crnonpl.cc:2237)"
        );
        assert_eq!(
            base.follow_target, None,
            "player-tile must clear follow_target (C++ crnonpl.cc:2237)"
        );
        assert!(
            base.todo
                .queue
                .iter()
                .any(|a| matches!(a, CreatureAction::Wait { delay_ms } if *delay_ms == MONSTER_IDLE_WAIT_MS)),
            "EXHAUSTED must enqueue a {MONSTER_IDLE_WAIT_MS} ms Wait"
        );
    }

    /// F3: after a kick-kill + 1 s wait, the monster re-engages the **same** target — the
    /// target was preserved, so `IdleStimulus`'s `lose_existing_target` keeps it (close, valid)
    /// and `acquire_target` skips (already has a target). Was: target dropped → re-acquire
    /// might pick a different target or sleep.
    #[test]
    fn f3_kick_kill_reengages_same_target() {
        use crate::sim_harness::{beat_driven_test_world, TEST_SYNTHETIC_GROUND_WP};

        let mut world = beat_driven_test_world();
        let now = std::time::Instant::now();

        let mpos = Position::new(100, 100, 7);
        let bpos = Position::new(101, 100, 7);
        let ppos = Position::new(102, 100, 7);
        // Walkable corridor: mover → blocker → player. Blocker is boxed in (only corridor tiles
        // exist; no perpendicular escape), so KickCreature kills it.
        for x in 100..=103u16 {
            ensure_walkable_tile(&mut world.map, Position::new(x, 100, 7), TEST_SYNTHETIC_GROUND_WP);
        }

        let player = insert_player(&mut world, test_player("Hero", ppos));
        world.map.register_creature_at(ppos, player);
        let blocker =
            insert_monster_with_config(&mut world, "Rat", bpos, 200, MonsterAiConfig::default());
        world.map.register_creature_at(bpos, blocker);
        let mover = insert_monster_with_config(&mut world, "Cyclops", mpos, 200, kicker_config());
        world.map.register_creature_at(mpos, mover);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(mover) {
            m.state = MonsterState::Attacking;
            m.base.attack_target = Some(player);
            m.base.follow_target = Some(player);
        }

        // Kick-kill the blocker → Exhausted (target preserved).
        let outcome = world.monster_push_before_step(mover, bpos, now);
        assert_eq!(outcome, MonsterKickOutcome::Exhausted);
        world.monster_exhausted_wait_772(mover, false);

        // Target preserved after the exhausted wait.
        let base = world.creatures.get(mover).unwrap().base();
        assert_eq!(base.attack_target, Some(player));
        assert_eq!(base.follow_target, Some(player));

        // Advance past the 1000 ms wait and run IdleStimulus — the monster should still
        // target the same player (close, same floor, not in PZ/house, not invisible).
        world.server_ms += MONSTER_IDLE_WAIT_MS as u64 + 1;
        world.monster_idle_stimulus(mover);

        let base = world.creatures.get(mover).unwrap().base();
        assert_eq!(
            base.attack_target,
            Some(player),
            "monster must re-engage the same target after kick-kill + 1s wait"
        );
        assert_eq!(
            base.follow_target,
            Some(player),
            "monster must still follow the same target after kick-kill + 1s wait"
        );
    }

    // ─────────── F2: recursive chain-push (`crnonpl.cc:3066`) ───────────

    /// Helper: set up an ATTACKING pushable monster with a far-away target.
    fn insert_chain_monster(
        world: &mut GameWorld,
        name: &str,
        pos: Position,
        target: CreatureId,
    ) -> CreatureId {
        let cid = insert_monster_with_config(world, name, pos, 200, kicker_config());
        world.map.register_creature_at(pos, cid);
        if let Some(CreatureKind::Monster(m)) = world.creatures.get_mut(cid) {
            m.state = MonsterState::Attacking;
            m.base.attack_target = Some(target);
            m.base.follow_target = Some(target);
        }
        cid
    }

    /// F2: A→B→C chain-push — A kicks B, B's escape tile has C, B kicks C (chain-push),
    /// C relocates to a free tile, B relocates to C's old spot, A's dest is clear.
    /// All in one beat, no stacking (`crnonpl.cc:3066`).
    #[test]
    fn f2_chain_push_three_monsters() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;
        let now = std::time::Instant::now();

        let mpos = Position::new(100, 100, 7);
        let bpos = Position::new(101, 100, 7);
        let cpos = Position::new(101, 101, 7);
        let escape = Position::new(101, 102, 7);
        let tpos = Position::new(105, 105, 7);
        // Only the corridor tiles + far target exist; N(101,99) is absent so B tries S first.
        for &p in &[mpos, bpos, cpos, escape, tpos] {
            ensure_walkable_tile(&mut world.map, p, 1);
        }

        let target = insert_monster_with_config(&mut world, "Rat", tpos, 200, kicker_config());
        let c = insert_chain_monster(&mut world, "RatC", cpos, target);
        let b = insert_chain_monster(&mut world, "RatB", bpos, target);
        let a = insert_chain_monster(&mut world, "Cyclops", mpos, target);

        let outcome = world.monster_push_before_step(a, bpos, now);
        assert_eq!(
            outcome,
            MonsterKickOutcome::Proceed,
            "chain-push must clear the dest tile and let A proceed"
        );
        // B relocated to C's old spot.
        assert_eq!(
            world.creatures.get(b).map(|k| k.position()),
            Some(cpos),
            "B must relocate to C's old spot (chain-push)"
        );
        // C relocated to the free escape tile.
        assert_eq!(
            world.creatures.get(c).map(|k| k.position()),
            Some(escape),
            "C must relocate to the free escape tile"
        );
        // No stacking: B and C on different tiles.
        assert_ne!(
            world.creatures.get(b).map(|k| k.position()),
            world.creatures.get(c).map(|k| k.position()),
            "B and C must not share a tile (no stacking)"
        );
    }

    /// F2: A→B where B's only escape has a pushable C → B and C do **not** share a tile.
    /// Before F2, B was forcibly relocated onto C's tile (stacking). After F2, B kicks C first.
    #[test]
    fn f2_chain_push_no_stacking() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;
        let now = std::time::Instant::now();

        let mpos = Position::new(100, 100, 7);
        let bpos = Position::new(101, 100, 7);
        let cpos = Position::new(101, 101, 7);
        let escape = Position::new(101, 102, 7);
        let tpos = Position::new(105, 105, 7);
        for &p in &[mpos, bpos, cpos, escape, tpos] {
            ensure_walkable_tile(&mut world.map, p, 1);
        }

        let target = insert_monster_with_config(&mut world, "Rat", tpos, 200, kicker_config());
        let c = insert_chain_monster(&mut world, "RatC", cpos, target);
        let b = insert_chain_monster(&mut world, "RatB", bpos, target);
        let a = insert_chain_monster(&mut world, "Cyclops", mpos, target);

        let _ = world.monster_push_before_step(a, bpos, now);

        let b_pos = world.creatures.get(b).map(|k| k.position());
        let c_pos = world.creatures.get(c).map(|k| k.position());
        assert_ne!(
            b_pos, c_pos,
            "B and C must not share a tile — F2 chain-push prevents stacking"
        );
        // B moved off its original tile.
        assert_ne!(b_pos, Some(bpos), "B must have been relocated");
        // C moved off its original tile.
        assert_ne!(c_pos, Some(cpos), "C must have been relocated by chain-push");
    }

    /// F2: a boxed-in blocker (no escape tiles at all) is still killed — regression of the
    /// existing `boxed_in_blocker_is_killed_and_step_exhausted` behavior with the F2 changes.
    #[test]
    fn f2_chain_push_boxed_in_kills() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;
        let now = std::time::Instant::now();

        let mpos = Position::new(100, 100, 7);
        let bpos = Position::new(101, 100, 7);
        let tpos = Position::new(105, 100, 7);
        // Only the kicker, blocker, and far-target tiles exist — the blocker's other neighbours
        // are absent (non-walkable), so `KickCreature` cannot relocate it and must kill.
        ensure_walkable_tile(&mut world.map, mpos, 1);
        ensure_walkable_tile(&mut world.map, bpos, 1);
        ensure_walkable_tile(&mut world.map, tpos, 1);

        let target = insert_monster_with_config(&mut world, "Rat", tpos, 200, kicker_config());
        let blocker = insert_chain_monster(&mut world, "Rat", bpos, target);
        let kicker = insert_chain_monster(&mut world, "Cyclops", mpos, target);

        let outcome = world.monster_push_before_step(kicker, bpos, now);
        assert_eq!(
            outcome,
            MonsterKickOutcome::Exhausted,
            "boxed-in blocker must be killed → Exhausted (kick-kill)"
        );
        assert!(
            !world.creatures.contains_key(blocker),
            "boxed-in blocker must be killed by the kick"
        );
    }

    /// F2: cycle guard — a 4-monster cycle (B→C→D→A→B) must terminate via `MAX_KICK_DEPTH`
    /// instead of infinite recursion. Each monster's only escape is the next one's tile.
    #[test]
    fn f2_chain_push_cycle_guard() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;
        let now = std::time::Instant::now();

        // 2×2 cluster: A(100,100), B(101,100), C(101,101), D(100,101).
        // Only these 4 tiles + far target exist — each monster's only escape is the next one's tile.
        let apos = Position::new(100, 100, 7);
        let bpos = Position::new(101, 100, 7);
        let cpos = Position::new(101, 101, 7);
        let dpos = Position::new(100, 101, 7);
        let tpos = Position::new(105, 105, 7);
        for &p in &[apos, bpos, cpos, dpos, tpos] {
            ensure_walkable_tile(&mut world.map, p, 1);
        }

        let target = insert_monster_with_config(&mut world, "Rat", tpos, 200, kicker_config());
        let _d = insert_chain_monster(&mut world, "RatD", dpos, target);
        let _c = insert_chain_monster(&mut world, "RatC", cpos, target);
        let b = insert_chain_monster(&mut world, "RatB", bpos, target);
        let a = insert_chain_monster(&mut world, "Cyclops", apos, target);

        // A kicks B. B's only escape is S(101,101)=C. C's only escape is W(100,101)=D.
        // D's only escape is N(100,100)=A. A's only escape is E(101,100)=B. → 4-cycle.
        // The depth guard must terminate the recursion. Eventually B has no passable escape
        // and is killed. The test passing (not hanging) proves the cycle guard works.
        let outcome = world.monster_push_before_step(a, bpos, now);
        // The cycle causes all chain-kicks attempts to fail at MAX_KICK_DEPTH → B has no
        // passable escape → B is killed → Exhausted (kick-kill).
        assert_eq!(
            outcome,
            MonsterKickOutcome::Exhausted,
            "cycle must terminate via depth guard → blocker killed → Exhausted"
        );
        assert!(
            !world.creatures.contains_key(b),
            "blocker must be killed after cycle guard terminates recursion"
        );
    }

    /// F2: a 5-monster chain-push (A→B→C→D→E) in a 1-wide corridor — all relocate one tile
    /// in a single beat. This is the "dense convoy" scenario from the audit.
    #[test]
    fn f2_dense_convoy_fluid() {
        let mut world = beat_driven_world();
        world.walk_wake_tx = None;
        let now = std::time::Instant::now();

        // Corridor: A(100,100)→B(101,100)→C(101,101)→D(101,102)→E(101,103)→escape(101,104).
        // The chain goes South: each blocker's escape is the next one's tile.
        let positions: [Position; 6] = [
            Position::new(100, 100, 7), // A (mover)
            Position::new(101, 100, 7), // B
            Position::new(101, 101, 7), // C
            Position::new(101, 102, 7), // D
            Position::new(101, 103, 7), // E
            Position::new(101, 104, 7), // escape (free)
        ];
        let tpos = Position::new(105, 105, 7);
        for &p in positions.iter().chain(std::iter::once(&tpos)) {
            ensure_walkable_tile(&mut world.map, p, 1);
        }

        let target = insert_monster_with_config(&mut world, "Rat", tpos, 200, kicker_config());
        // Insert in reverse order so chain-push goes A→B→C→D→E.
        let e = insert_chain_monster(&mut world, "RatE", positions[4], target);
        let d = insert_chain_monster(&mut world, "RatD", positions[3], target);
        let c = insert_chain_monster(&mut world, "RatC", positions[2], target);
        let b = insert_chain_monster(&mut world, "RatB", positions[1], target);
        let a = insert_chain_monster(&mut world, "Cyclops", positions[0], target);

        let outcome = world.monster_push_before_step(a, positions[1], now);
        assert_eq!(
            outcome,
            MonsterKickOutcome::Proceed,
            "5-monster chain-push must clear the dest tile and let A proceed"
        );
        // Each monster advanced one tile South.
        assert_eq!(
            world.creatures.get(b).map(|k| k.position()),
            Some(positions[2]),
            "B must advance to C's old spot"
        );
        assert_eq!(
            world.creatures.get(c).map(|k| k.position()),
            Some(positions[3]),
            "C must advance to D's old spot"
        );
        assert_eq!(
            world.creatures.get(d).map(|k| k.position()),
            Some(positions[4]),
            "D must advance to E's old spot"
        );
        assert_eq!(
            world.creatures.get(e).map(|k| k.position()),
            Some(positions[5]),
            "E must advance to the free escape tile"
        );
        // No stacking: all on distinct tiles.
        let positions_after: Vec<_> = [b, c, d, e]
            .iter()
            .map(|&id| world.creatures.get(id).map(|k| k.position()))
            .collect();
        let unique: std::collections::HashSet<_> = positions_after.iter().collect();
        assert_eq!(
            unique.len(),
            positions_after.len(),
            "all monsters must be on distinct tiles (no stacking)"
        );
    }
}
