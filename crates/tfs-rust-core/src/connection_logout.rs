//! `CONNECTION_LOGOUT` — body logout now, TCP close on the stored round.
//!
//! C++ reference: `connections.cc` `TConnection::Logout` / `Process` logout arm;
//! `crmain.cc` `~TCreature` `Logout(30, true)`.

use tfs_rust_common::ConnId;

use crate::game_state::GameState;
use crate::game_world::GameWorld;

/// Destructor delay — `crmain.cc:300` `Connection->Logout(30, true)`.
pub(crate) const DEATH_LOGOUT_DELAY_ROUNDS: u32 = 30;

impl GameWorld {
    /// Sockets whose logout round has arrived, plus every logout socket on Shutdown.
    ///
    /// Checked at the start of `Process` so a `Logout(0)` scheduled later in the same
    /// call is not closed until the next round (`connections.cc:44-49`).
    pub(crate) fn take_due_connection_logouts(&mut self) -> Vec<(ConnId, bool)> {
        let shutdown = self.game_state == GameState::Shutdown;
        let round = self.round_nr;
        let due: Vec<ConnId> = self
            .logout_at_round
            .iter()
            .filter(|(_, at)| shutdown || **at <= round)
            .map(|(&conn, _)| conn)
            .collect();
        let mut out = Vec::with_capacity(due.len());
        for conn in due {
            self.logout_at_round.remove(&conn);
            // StopFight already ran when the logout was scheduled.
            out.push((conn, false));
        }
        out
    }

    /// In-game `Logout(delay, stop_fight)` — `StartLogout` now, TCP on a later `Process`.
    pub(crate) fn schedule_connection_logout(
        &mut self,
        conn: ConnId,
        delay_rounds: u32,
        stop_fight: bool,
        display_effect: bool,
    ) {
        if !self.logout_at_round.contains_key(&conn) {
            if let Some(cid) = self.conn_to_creature.get(&conn).copied() {
                if display_effect {
                    self.broadcast_player_logout_poff(cid);
                }
                // ClearConnection, then StartLogout (`connections.cc:281-285`).
                self.unregister_conn_mapping(conn);
                self.forget_known_creatures_for_conn(conn);
                self.creature_begin_logout(cid, false, stop_fight);
            }
            self.dead_connections.remove(&conn);
            self.dead_conn_state.remove(&conn);
        }
        self.logout_at_round
            .insert(conn, self.round_nr.saturating_add(delay_rounds));
    }

    /// TCP close with no `StartLogout` (takeover old conn, death after the body is gone).
    pub(crate) fn schedule_tcp_close_after(&mut self, conn: ConnId, delay_rounds: u32) {
        self.dead_connections.remove(&conn);
        self.dead_conn_state.remove(&conn);
        self.logout_at_round
            .insert(conn, self.round_nr.saturating_add(delay_rounds));
    }
}
