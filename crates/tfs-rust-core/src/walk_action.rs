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
            p.walk_action_due = None;
        }
    }

    fn set_next_walk_action_task(&mut self, cid: CreatureId, action: PlayerWalkAction) {
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.walk_action = Some(action);
            p.walk_action_due = None;
        }
    }

    /// TFS `Player::onWalkComplete` — schedule stored `walkTask` on the logical clock.
    /// 772: `ToDoQueue` wakeup (`schedule_creature_wakeup`); 1098: `walk_action_due` poll.
    pub(crate) fn on_player_walk_complete(&mut self, cid: CreatureId) {
        let should_schedule = self
            .creatures
            .get(cid)
            .is_some_and(|k| matches!(k, CreatureKind::Player(p) if p.walk_action.is_some()));
        if !should_schedule {
            return;
        }
        let due = self.now_ms().saturating_add(WALK_ACTION_DELAY_MS);
        if self.beat_driven_loop {
            if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
                p.walk_action_due = Some(due);
            }
            self.schedule_creature_wakeup(cid, due);
        } else if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.walk_action_due = Some(due);
        }
    }

    /// Drain due walk-action tasks — **1098 only** (`on_tick`). 772 uses `ToDoQueue` drain.
    pub(crate) fn process_walk_action_tasks(&mut self) {
        if self.beat_driven_loop {
            return;
        }
        let now_ms = self.now_ms();
        let due: Vec<(CreatureId, PlayerWalkAction)> = self
            .creatures
            .iter()
            .filter_map(|(cid, k)| {
                let CreatureKind::Player(p) = k else {
                    return None;
                };
                let action = p.walk_action.clone()?;
                let due_at = p.walk_action_due?;
                (now_ms >= due_at).then_some((cid, action))
            })
            .collect();

        let now = Instant::now();
        for (cid, action) in due {
            self.run_player_walk_action(cid, action, now);
        }
    }

    /// 772 `ToDoQueue` drain hook — run deferred walk-action when its wakeup fires.
    pub(crate) fn try_run_player_walk_action_from_todo(&mut self, cid: CreatureId, now: Instant) {
        let (action, due_ok) = match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => {
                let Some(due) = p.walk_action_due else {
                    return;
                };
                let Some(action) = p.walk_action.clone() else {
                    return;
                };
                (action, self.now_ms() >= due)
            }
            _ => return,
        };
        if !due_ok {
            return;
        }
        self.run_player_walk_action(cid, action, now);
    }

    /// Reschedule a deferred walk-action while per-action timers are still active.
    pub(crate) fn defer_player_walk_action(&mut self, cid: CreatureId, action: PlayerWalkAction) {
        let now_ms = self.now_ms();
        let due = if self.beat_driven_loop {
            self.creatures
                .get(cid)
                .and_then(|k| k.base().earliest_action_block_ms(now_ms))
                .unwrap_or(now_ms)
        } else {
            match self.creatures.get(cid) {
                Some(CreatureKind::Player(p)) => p.next_action_until.filter(|t| *t > now_ms),
                _ => None,
            }
            .unwrap_or(now_ms)
        };
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.walk_action = Some(action);
            p.walk_action_due = Some(due);
        }
        if self.beat_driven_loop {
            self.schedule_creature_wakeup(cid, due);
        }
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
            self.on_player_walk_complete(cid);
            return true;
        }
        // `player_auto_walk_path` → `player_todo_clear_with_snapback` → `player_todo_clear`
        // wipes any stale `walk_action` (C++ `ToDoClear`, audit #3). Set the new walk-action
        // **after** the clear so it survives until `on_player_walk_complete` fires.
        self.player_auto_walk_path(conn_id, cid, path, now);
        self.set_next_walk_action_task(cid, action);
        true
    }

    pub(crate) fn run_player_walk_action(
        &mut self,
        cid: CreatureId,
        action: PlayerWalkAction,
        now: Instant,
    ) {
        if !self.player_walk_action_ready(cid, &action) {
            self.defer_player_walk_action(cid, action);
            return;
        }
        self.clear_player_walk_action(cid);
        let Some(conn_id) = self.conn_for_creature(cid) else {
            return;
        };
        let result = match action {
            PlayerWalkAction::MoveItem {
                from_pos,
                sprite_id,
                from_stack_pos,
                to_pos,
                count,
            } => self.player_move_thing(
                conn_id,
                cid,
                from_pos,
                sprite_id,
                from_stack_pos,
                to_pos,
                count,
                now,
            ),
            PlayerWalkAction::UseItem(payload) => {
                self.player_use_item(conn_id, cid, payload, now)
            }
            PlayerWalkAction::UseItemEx(payload) => {
                self.player_use_item_ex(conn_id, cid, payload, now)
            }
        };
        if let Err(rv) = result {
            self.send_cancel_message(conn_id, rv);
        }
    }
}
