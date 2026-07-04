//! Walk-to-target then execute deferred player action (TFS `Player::walkTask`).
// C++ reference: `player.cpp` `setNextWalkActionTask`, `onWalkComplete`, `onWalkAborted`;
// `game.cpp` `playerMoveItem` (~970), `playerUseItem` (~2227), `playerUseItemEx` (~2151).

use std::time::Instant;

use tfs_rust_common::ConnId;
use tfs_rust_common::Position;

use crate::creature::CreatureKind;
use crate::creature::PlayerWalkAction;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;
use crate::return_value::ReturnValue;

/// TFS `createSchedulerTask(400, ...)` delay before walk-action fires (`game.cpp`), in **logical ms**
/// (audit Finding 1, Phase 4 — was a wall-clock `Duration`).
pub const WALK_ACTION_DELAY_MS: u64 = 400;

/// C++ two-object use exhaustion — `cract.cc:765` `EarliestMultiuseTime = ServerMilliseconds + 1000`.
pub const MULTIUSE_EXHAUST_MS: u64 = 1000;

impl GameWorld {
    /// TFS `Player::onWalkAborted` / `Game::playerMove` clearing `walkTask` (`player.cpp` ~3386, `game.cpp` ~1893).
    pub(crate) fn clear_player_walk_action(&mut self, cid: CreatureId) {
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.walk_action = None;
        }
    }

    fn set_next_walk_action_task(&mut self, cid: CreatureId, action: PlayerWalkAction) {
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.walk_action = Some(action);
        }
    }

    /// Reschedule a deferred walk-action while per-action timers are still active.
    pub(crate) fn defer_player_walk_action(&mut self, cid: CreatureId, action: PlayerWalkAction) {
        let now_ms = self.now_ms();
        let due = self
            .creatures
            .get(cid)
            .and_then(|k| k.base().earliest_action_block_ms(now_ms))
            .unwrap_or(now_ms);
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.walk_action = Some(action);
        }
        self.schedule_creature_wakeup(cid, due);
    }

    /// Start auto-walk toward `target` (within 1 tile) and defer `action` until walk completes.
    /// TFS `getPathTo(..., 0, 1, true, true)` + `playerAutoWalk` + `setNextWalkActionTask`.
    pub(crate) fn try_walk_to_and_action(
        &mut self,
        conn_id: ConnId,
        cid: CreatureId,
        target: Position,
        action: PlayerWalkAction,
        now: Instant,
    ) -> bool {
        let Some(path) = self.get_creature_path_to(cid, target, 0, 1) else {
            return false;
        };
        if path.is_empty() {
            self.set_next_walk_action_task(cid, action);
            return true;
        }
        // `player_auto_walk_path` → `player_todo_clear_with_snapback` → `player_todo_clear`
        // wipes any stale `walk_action` (C++ `ToDoClear`, audit #3). Set the new walk-action
        // **after** the clear so it survives until `on_player_walk_complete` fires.
        self.player_auto_walk_path(conn_id, cid, path, now);
        self.set_next_walk_action_task(cid, action);
        true
    }

    /// F8 S5 — Set up the walk queue for a `Go`-prepend walk-to-reach, **without** the
    /// `ToDoClear` + `Go` enqueue + `ToDoStart` that [`player_auto_walk_path`] performs.
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
        let Some(path) = self.get_creature_path_to(cid, target, 0, 1) else {
            return Err(ReturnValue::ThereIsNoWay);
        };
        if path.is_empty() {
            // Already adjacent — caller checks adjacency first, so this shouldn't happen.
            return Ok(());
        }
        // Check movement_blocked — mirrors `player_auto_walk_path` (`walk/mod.rs:776`).
        let blocked = self.creatures.get(cid).is_some_and(|k| {
            matches!(k, CreatureKind::Player(p) if p.base.movement_blocked)
        });
        if blocked {
            return Err(ReturnValue::ThereIsNoWay);
        }
        self.flush_deferred_turn_broadcast(cid);
        // Set up walk queue — mirrors `player_auto_walk_path` (`walk/mod.rs:803-810`)
        // but WITHOUT `player_todo_clear_with_snapback` (the ToDo queue is already correct).
        let cur_pos = self.creatures.get(cid).map(|k| k.position());
        if let (Some(CreatureKind::Player(pl)), Some(pos)) =
            (self.creatures.get_mut(cid), cur_pos)
        {
            pl.last_activity = now;
            for d in &path {
                pl.base.walk_queue.push_back(*d);
            }
            let mut acc = pos;
            for d in path.iter().rev() {
                acc = acc.offset(*d);
                pl.base.walk_destinations.push_front(acc);
            }
        }
        Ok(())
    }
}
