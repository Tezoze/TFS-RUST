//! Walk-to-target then execute deferred player action (TFS `Player::walkTask`).
// C++ reference: `player.cpp` `setNextWalkActionTask`, `onWalkComplete`, `onWalkAborted`;
// `game.cpp` `playerMoveItem` (~970), `playerUseItem` (~2227), `playerUseItemEx` (~2151).

use std::time::{Duration, Instant};

use tfs_rust_common::ConnId;
use tfs_rust_common::Position;

use crate::creature::PlayerWalkAction;
use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;

/// TFS `createSchedulerTask(400, ...)` delay before walk-action fires (`game.cpp`).
pub const WALK_ACTION_DELAY: Duration = Duration::from_millis(400);

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

    /// TFS `Player::onWalkComplete` — schedule stored `walkTask` (`player.cpp` ~3390–3395).
    pub(crate) fn on_player_walk_complete(&mut self, cid: CreatureId, now: Instant) {
        let should_schedule = self.creatures.get(cid).is_some_and(|k| {
            matches!(k, CreatureKind::Player(p) if p.walk_action.is_some())
        });
        if should_schedule {
            if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
                p.walk_action_due = Some(now + WALK_ACTION_DELAY);
            }
        }
    }

    /// Drain due walk-action tasks (scheduler tick equivalent).
    pub(crate) fn process_walk_action_tasks(&mut self, now: Instant) {
        let due: Vec<(CreatureId, PlayerWalkAction)> = self
            .creatures
            .iter()
            .filter_map(|(cid, k)| {
                let CreatureKind::Player(p) = k else {
                    return None;
                };
                let action = p.walk_action.clone()?;
                let due_at = p.walk_action_due?;
                (now >= due_at).then_some((cid, action))
            })
            .collect();

        for (cid, action) in due {
            self.run_player_walk_action(cid, action, now);
        }
    }

    /// Reschedule a deferred walk-action when `nextAction` is still active (`game.cpp` ~908–913).
    fn defer_player_walk_action(&mut self, cid: CreatureId, action: PlayerWalkAction, now: Instant) {
        let due = match self.creatures.get(cid) {
            Some(CreatureKind::Player(p)) => p.next_action_until.filter(|t| *t > now),
            _ => None,
        };
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.walk_action = Some(action);
            p.walk_action_due = Some(due.unwrap_or(now));
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
            return false;
        }
        self.set_next_walk_action_task(cid, action);
        self.player_auto_walk_path(conn_id, cid, path, now);
        true
    }

    fn run_player_walk_action(&mut self, cid: CreatureId, action: PlayerWalkAction, now: Instant) {
        if !self.player_timed_action_ready(cid, now) {
            self.defer_player_walk_action(cid, action, now);
            return;
        }
        self.clear_player_walk_action(cid);
        let Some(conn_id) = self.conn_for_creature(cid) else {
            return;
        };
        match action {
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
                self.player_use_item(conn_id, cid, payload, now);
            }
            PlayerWalkAction::UseItemEx(payload) => {
                self.player_use_item_ex(conn_id, cid, payload, now);
            }
        }
    }
}
