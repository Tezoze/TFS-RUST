//! Walk-to-target helpers for the ToDo `Go`-prepend pattern.
// C++ reference: `cract.cc:600-760` `Use` executor walk-to-reach; `player.cpp` `onWalkAborted`.

use std::time::Instant;

use tfs_rust_common::Position;

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::return_value::ReturnValue;

/// C++ two-object use exhaustion — `cract.cc:765` `EarliestMultiuseTime = ServerMilliseconds + 1000`.
pub const MULTIUSE_EXHAUST_MS: u64 = 1000;

impl GameWorld {
    /// TFS `Player::onWalkAborted` / `Game::playerMove` clearing `walkTask` (`player.cpp` ~3386, `game.cpp` ~1893).
    pub(crate) fn clear_player_walk_action(&mut self, cid: CreatureId) {
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.walk_action = None;
        }
    }

    /// F8 S5 — Set up the walk queue for a `Go`-prepend walk-to-reach, **without** the
    /// `ToDoClear` + `Go` enqueue + `ToDoStart` that `player_auto_walk_path` performs.
    /// The caller (the ToDo execute arm) enqueues `Go` + the re-enqueued action itself,
    /// then calls `todo_start_go_delay` + `schedule_immediate_todo_wakeup`.
    ///
    /// Mirrors the walk-queue setup in `player_auto_walk_path` (`walk/mod.rs`) minus the
    /// clear — the ToDo queue is already in the right state (the action was just popped,
    /// queue is empty or has only the caller's re-enqueue left). C++ ref: `cract.cc:600-760`
    /// `Use` executor — if the target isn't reachable, it prepends `ToDoGo(dest)` +
    /// re-enqueues the action + `ToDoStart` and returns.
    ///
    /// Returns `Err(ThereIsNoWay)` if no path exists (C++ `Use` executor's "no path" case)
    /// or the player is movement-blocked. `Ok(())` if the path is set up (or empty — already
    /// adjacent, caller should have checked adjacency first).
    pub(crate) fn setup_player_walk_to_target(
        &mut self,
        cid: CreatureId,
        target: Position,
        now: Instant,
    ) -> Result<(), ReturnValue> {
        let Some(path) = self.get_creature_path_to(cid, target, 0, 1, usize::MAX) else {
            return Err(ReturnValue::ThereIsNoWay);
        };
        if path.is_empty() {
            // Already adjacent — caller checks adjacency first, so this shouldn't happen.
            return Ok(());
        }
        // Check movement_blocked — mirrors `player_auto_walk_path` (`walk/mod.rs:776`).
        let blocked = self
            .creatures
            .get(cid)
            .is_some_and(|k| matches!(k, CreatureKind::Player(p) if p.base.movement_blocked));
        if blocked {
            return Err(ReturnValue::ThereIsNoWay);
        }
        self.flush_deferred_turn_broadcast(cid);
        // Set up walk queue — mirrors `player/combat/mod.rs` chase pattern
        // (`walk_queue` uses `push_back` in rev order + `pop_back` (LIFO) so `pop_back`
        // yields the first step). `get_creature_path_to` returns forward execution order
        // (first step first), so we must reverse before pushing — same as monster AI
        // (`monster_ai.rs`) and player combat chase (`player/combat/mod.rs`).
        // WITHOUT `player_todo_clear_with_snapback` (the ToDo queue is already correct).
        let cur_pos = self.creatures.get(cid).map(|k| k.position());
        if let (Some(CreatureKind::Player(pl)), Some(pos)) = (self.creatures.get_mut(cid), cur_pos)
        {
            pl.last_activity = now;
            for d in path.iter().rev() {
                pl.base.walk_queue.push_back(*d);
            }
            let mut acc = pos;
            for &d in &path {
                acc = acc.offset(d);
                pl.base.walk_destinations.push_front(acc);
            }
        }
        Ok(())
    }
}
