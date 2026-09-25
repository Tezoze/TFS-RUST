//! Five-minute online load log.
//!
//! C++ reference: `crplayer.cc` `CreatePlayerList`, `main.cc:384-386`.
//! The query-manager player list has no client here. The 15-minute offline
//! slot save stays a logout save.

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;

impl GameWorld {
    /// `Log("load", "%d %d")` — unix time and every player (`FirstFreePlayer`).
    pub(crate) fn log_online_player_list(&self, unix_time: i64) {
        let players = self
            .creatures
            .iter()
            .filter(|(_, k)| matches!(k, CreatureKind::Player(_)))
            .count();
        tracing::info!(target: "load", unix_time, players, "{unix_time} {players}");
    }
}
