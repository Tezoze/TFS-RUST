//! Five-minute online load log.
//!
//! C++ reference: `crplayer.cc` `CreatePlayerList`, `main.cc:384-386`.
//! The query-manager player list has no client here. The 15-minute offline
//! slot save stays a logout save. The corpus line is unix time plus a count;
//! this log prints the host's local 24-hour clock instead.

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;

impl GameWorld {
    /// Host-local `HH:MM` and the number of players online (`FirstFreePlayer`).
    ///
    /// `chrono::Local` is the machine timezone (`TZ` / `/etc/localtime`), not a
    /// fixed offset.
    pub(crate) fn log_online_player_list(&self, unix_time: i64) {
        let players = self
            .creatures
            .iter()
            .filter(|(_, k)| matches!(k, CreatureKind::Player(_)))
            .count();
        let stamp = host_local_hhmm(unix_time);
        tracing::info!(
            target: "load",
            "{stamp}: There is currently {players} players online"
        );
    }
}

/// `HH:MM` in the hosting machine's timezone.
fn host_local_hhmm(unix_time: i64) -> String {
    chrono::DateTime::from_timestamp(unix_time, 0)
        .map(|utc| {
            utc.with_timezone(&chrono::Local)
                .format("%H:%M")
                .to_string()
        })
        .unwrap_or_else(|| "--:--".to_string())
}
