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
//!   `EXHAUSTED` case → [`MonsterKickOutcome::Exhausted`] (the caller clears the target and waits
//!   1000 ms; `crnonpl.cc:2890-2898`).
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

/// Outcome of the 772 pre-step kick gate — mirrors the `Execute=true` side of
/// `TMonster::MovePossible` (`crnonpl.cc:2225-2244`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MonsterKickOutcome {
    /// Not a 772 kick situation, or the destination was cleared — proceed with the normal step.
    Proceed,
    /// 772 `EXHAUSTED` — a player blocker or a `KickCreature` kill. The mover must **not** step
    /// this beat; the caller runs `Target=0; ToDoClear; Wait(1000); ToDoStart`.
    Exhausted,
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
    fn monster_kick_before_step_772(
        &mut self,
        mover: CreatureId,
        dest: Position,
        now: Instant,
    ) -> MonsterKickOutcome {
        let Some((mover_pos, master, target_attack, target_follow, state, can_push_creatures, is_summon)) = ({
            match self.creatures.get(mover) {
                Some(CreatureKind::Monster(m)) => Some((
                    m.base.position,
                    m.base.master,
                    m.base.attack_target,
                    m.base.follow_target,
                    m.state,
                    m.can_push_creatures,
                    m.base.is_summon(),
                )),
                _ => return MonsterKickOutcome::Proceed,
            }
        }) else {
            return MonsterKickOutcome::Proceed;
        };

        // C++ creature-block gate: only an ATTACKING/PANIC monster with a target and the
        // `KickCreatures` race flag (and no master) ever kicks a blocking creature.
        let has_target = target_attack.is_some() || target_follow.is_some();
        let posture = matches!(state, MonsterState::Attacking | MonsterState::Panic);
        let creature_kicker = can_push_creatures && !is_summon && posture && has_target;

        // C++ box-block gate (`CanKickBoxes`) is independent of attack posture.
        let can_kick_boxes = self.monster_can_kick_boxes_772(mover);

        // Creatures first (chain tail in the Rust tile model). A player tile or a forced kill is
        // the `EXHAUSTED` short-circuit — stop and report it (C++ `throw EXHAUSTED`).
        if creature_kicker {
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

            for blocker in blockers {
                // C++ `MovePossible` creature gate (`crnonpl.cc:2207-2210`): never kick the mover's
                // own target or master — these are hard blocks, not `EXHAUSTED`.
                if Some(blocker) == target_attack
                    || Some(blocker) == target_follow
                    || Some(blocker) == master
                {
                    continue;
                }
                match self.creatures.get(blocker) {
                    // C++ `crnonpl.cc:2236-2238`: a player blocker clears `Target` and throws
                    // `EXHAUSTED`. (NOTE(parity): `IGNORED_BY_MONSTERS` GM flag not modeled — such
                    // a player is a hard block in C++, not `EXHAUSTED`.)
                    Some(CreatureKind::Player(_)) => return MonsterKickOutcome::Exhausted,
                    // NPC / unpushable monster → hard block (`crnonpl.cc:2216,2228`), not kicked.
                    Some(CreatureKind::Npc(_)) => continue,
                    Some(CreatureKind::Monster(m)) if !m.is_pushable() => continue,
                    Some(CreatureKind::Monster(_)) => {
                        // C++ `crnonpl.cc:2240-2242`: kick the blocker; a forced kill (no free
                        // adjacent tile) still throws `EXHAUSTED`.
                        if !self.monster_kick_creature_772(mover, blocker, mover_pos, now) {
                            return MonsterKickOutcome::Exhausted;
                        }
                    }
                    None => continue,
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
        let immune = matches!(
            self.creatures.get(mover),
            Some(CreatureKind::Monster(m)) if m.immunity_poison
        );

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
                    .filter(|&iid| self.item_is_kickable_box_772(iid, state, immune))
                    .collect()
            })
            .unwrap_or_default();

        for item_id in to_kick {
            self.monster_kick_single_box_772(mover, item_id, dest, mover_pos);
        }
    }

    /// True when an item on a destination tile is a movable blocker the mover must shove
    /// (`MovePossible` `UNPASS`/`AVOID` branches, `crnonpl.cc:2250-2284`). Hazard `AVOID` fields
    /// are ignored while `PANIC` or for a poison-immune mover.
    fn item_is_kickable_box_772(&self, item_id: ItemId, state: MonsterState, immune: bool) -> bool {
        let Some(item) = self.items.get(item_id) else {
            return false;
        };
        let server_id = item.item_type;
        if self.items_db.is_unpass_772(server_id) {
            return !self.items_db.is_unmove_772(server_id);
        }
        if self.items_db.is_avoid_hazard_772(server_id) {
            let ignore_hazard = state == MonsterState::Panic || immune;
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
    fn monster_kick_creature_772(
        &mut self,
        kicker: CreatureId,
        blocker: CreatureId,
        mover_pos: Position,
        now: Instant,
    ) -> bool {
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
            // `Creature->MovePossible(Dest, Execute=true)` is enforced inside the move
            // (`tile_query_add_creature`); a kick is a forced relocate (no walk-timer gate).
            if self.try_creature_walk_step(blocker, dir, now) {
                return true;
            }
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
    /// tile (not its target) is the `EXHAUSTED` case — `crnonpl.cc:2236-2238`.
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
        assert_eq!(outcome, MonsterKickOutcome::Exhausted);
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

    /// `EXHAUSTED` recovery clears the target and arms a 1000 ms wait (`crnonpl.cc:2890-2898`).
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

        world.monster_exhausted_wait_772(mover);

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
}
