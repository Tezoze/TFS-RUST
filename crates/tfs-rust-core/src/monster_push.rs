//! TFS monster push-before-step — `Monster::pushCreature`, `pushCreatures` (`monster.cpp` ~1174–1221).
//!
//! Called from [`crate::walk::GameWorld::on_walk`] before the mover steps, matching
//! `Monster::getNextStep` (~1260–1271).

use std::time::Instant;

use rand::seq::SliceRandom;
use tfs_rust_common::enums::Direction;
use tfs_rust_common::Position;

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::tile::flags as tilestate;

impl GameWorld {
    /// TFS `Monster::getNextStep` — push blocking creatures off the destination tile.
    pub(crate) fn monster_push_before_step(
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
    fn monster_push_creatures_on_tile(
        &mut self,
        dest: Position,
        mover: CreatureId,
        now: Instant,
    ) {
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
            if last_pushed != Some(blocker)
                && self.monster_push_creature(blocker, now)
            {
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
