//! 772 `ProcessConnections` — idle kick and round tracking.
//!
//! C++ reference: `connections.cc:21` `TConnection::Process`, `connections.cc:53` `ResetTimer`.

use tfs_rust_common::ConnId;
use tfs_rust_common::GamePacket;
use tfs_rust_net::outgoing::{send_ping, send_text_message};

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;

/// C++ `TALK_ADMIN_MESSAGE` (`enums.hh`).
const TALK_ADMIN_MESSAGE: u8 = 18;

/// Rounds without any command before proactive ping #1 — `connections.cc:24`.
const PING_ROUND_1: u32 = 30;
/// Rounds without any command before proactive ping #2 — `connections.cc:24`.
const PING_ROUND_2: u32 = 60;
/// Rounds without any command before dead-connection logout — `connections.cc:37`.
const COMMAND_TIMEOUT_ROUNDS: u32 = 90;

/// C++ `TConnection::Process` — `connections.cc:21-51`.
///
/// All timing is **round-counter based** (`RoundNr`), never wallclock:
/// - Ping at `LastCommand == 30 || == 60` (`connections.cc:24`).
/// - Idle warn at `LastAction == 900` (`connections.cc:29`).
/// - Idle kick at `LastAction >= 960` (`connections.cc:35`).
/// - Connection timeout logout at `LastCommand >= 90` (`connections.cc:37`).
impl GameWorld {
    /// C++ `TConnection::ResetTimer` — update command/action round stamps on incoming packets.
    pub(crate) fn player_reset_connection_rounds(
        &mut self,
        cid: CreatureId,
        counts_as_action: bool,
    ) {
        if !self.beat_driven_loop {
            return;
        }
        let round = self.round_nr_772;
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.last_command_round = round;
            if counts_as_action {
                p.last_action_round = round;
            }
        }
    }

    /// C++ `ProcessConnections` idle kick + round-based ping + connection timeout
    /// (`connections.cc:21-51`). Returns conn IDs to disconnect.
    ///
    /// Idle warn/kick rounds are config-driven via `kickIdlePlayerAfterMinutes`
    /// (`config.lua`): warning at `N*60` rounds, kick at `(N+1)*60` rounds. The proactive
    /// ping cadence (30/60) and dead-connection timeout (90) are 772 engine constants.
    pub(crate) fn process_connections_772(&mut self) -> Vec<ConnId> {
        if !self.beat_driven_loop {
            return Vec::new();
        }
        let round = self.round_nr_772;
        let idle_warn_rounds = self.connection_config.idle_warn_rounds();
        let idle_kick_rounds = self.connection_config.idle_kick_rounds();
        let mut kick: Vec<ConnId> = Vec::new();

        let online: Vec<(ConnId, CreatureId)> = self
            .conn_to_creature
            .iter()
            .map(|(&conn, &cid)| (conn, cid))
            .collect();

        for (conn_id, cid) in online {
            let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
                continue;
            };
            let last_command = round.saturating_sub(p.last_command_round);
            let last_action = round.saturating_sub(p.last_action_round);

            // C++ `LastCommand == 30 || LastCommand == 60` → `SendPing` (`connections.cc:24`).
            if last_command == PING_ROUND_1 || last_command == PING_ROUND_2 {
                self.enqueue_outgoing(conn_id, send_ping().into_bytes());
            }

            if last_action == idle_warn_rounds {
                let msg = format!(
                    "You have been idle for {} minutes. You will be disconnected in one minute if you are still idle then.",
                    self.connection_config.kick_idle_after_minutes
                );
                self.enqueue_outgoing(
                    conn_id,
                    send_text_message(TALK_ADMIN_MESSAGE, &msg).into_bytes(),
                );
            }
            if last_action >= idle_kick_rounds {
                kick.push(conn_id);
            } else if last_command >= COMMAND_TIMEOUT_ROUNDS {
                // C++ `LastCommand >= 90` → logout (`connections.cc:37`).
                kick.push(conn_id);
            }
        }

        kick
    }

    /// C++ `SendAmbiente` on brightness change (`main.cc:361-372`).
    pub(crate) fn tick_ambiente_light_772(&mut self) {
        if !self.beat_driven_loop {
            return;
        }
        let wt = crate::world_light::world_time_from_local_clock();
        let brightness = crate::world_light::light_level_from_world_time(wt) as i8;
        if brightness == self.last_ambiente_brightness {
            return;
        }
        self.last_ambiente_brightness = brightness;
        let packet = tfs_rust_net::outgoing_extra::send_world_light(brightness as u8, 215, false)
            .into_bytes();
        let conns: Vec<ConnId> = self.conn_to_creature.keys().copied().collect();
        for conn_id in conns {
            self.enqueue_outgoing(conn_id, packet.clone());
        }
    }
}

/// Whether a client packet counts as player action for idle kick — `connections.cc:56-60`.
pub(crate) fn packet_counts_as_action_772(packet: &GamePacket) -> bool {
    !matches!(
        packet,
        GamePacket::Ping
            | GamePacket::PingBack
            | GamePacket::StopAutoWalk
            |         GamePacket::Turn(_)
    )
}

#[cfg(test)]
mod tests {
    use tfs_rust_common::Position;

    use crate::creature::CreatureKind;
    use crate::test_world::support::{
        beat_driven_test_world, ensure_walkable_tile, insert_player, test_player,
    };

    #[test]
    fn idle_kick_at_960_rounds() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 150);
        let player = insert_player(&mut world, test_player("Idle", pos));
        world.register_conn_mapping(tfs_rust_common::ConnId(1), player);

        // `last_command_round` stays recent so only the idle-kick path is exercised.
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.last_action_round = 0;
            p.last_command_round = 960;
        }
        world.round_nr_772 = 960;
        let kick = world.process_connections_772();
        assert_eq!(kick.len(), 1);
        assert_eq!(kick[0].0, 1);
    }

    /// Custom `kickIdlePlayerAfterMinutes = 10` → warn at 600, kick at 660.
    #[test]
    fn idle_kick_uses_config_minutes() {
        let mut world = beat_driven_test_world();
        world.connection_config.kick_idle_after_minutes = 10;
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 150);
        let player = insert_player(&mut world, test_player("Idle", pos));
        world.register_conn_mapping(tfs_rust_common::ConnId(1), player);

        // Keep `last_command_round` recent so the 90-round connection timeout
        // doesn't fire before the idle kick — we're testing idle timing only.
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.last_action_round = 0;
            p.last_command_round = 0;
        }
        // 600 rounds = 10 min → warning, not kick yet (command still recent).
        world.round_nr_772 = 600;
        // Refresh command round so connection-timeout (>=90) doesn't fire.
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.last_command_round = 600;
        }
        let kick = world.process_connections_772();
        assert!(kick.is_empty(), "warn at 600 must not kick");

        // 660 rounds = 11 min → kick (command still recent).
        world.round_nr_772 = 660;
        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.last_command_round = 660;
        }
        let kick = world.process_connections_772();
        assert_eq!(kick.len(), 1, "kick at 660 (10+1 min)");
    }

    /// C++ `LastCommand == 30` → `SendPing` (`connections.cc:24`).
    #[test]
    fn round_based_ping_at_30_rounds() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 150);
        let player = insert_player(&mut world, test_player("Pinged", pos));
        let conn = tfs_rust_common::ConnId(1);
        world.register_conn_mapping(conn, player);

        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.last_command_round = 0;
            p.last_action_round = 0;
        }
        world.round_nr_772 = 30;
        let kick = world.process_connections_772();
        assert!(kick.is_empty(), "ping round must not kick");
        let outgoing = world.pending_outgoing.get(&conn);
        assert!(outgoing.is_some_and(|q| !q.is_empty()), "ping must be enqueued");
    }

    /// C++ `LastCommand >= 90` → logout (`connections.cc:37`).
    #[test]
    fn connection_timeout_kick_at_90_rounds() {
        let mut world = beat_driven_test_world();
        let pos = Position::new(100, 100, 7);
        ensure_walkable_tile(&mut world.map, pos, 150);
        let player = insert_player(&mut world, test_player("Timeout", pos));
        world.register_conn_mapping(tfs_rust_common::ConnId(1), player);

        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.last_command_round = 0;
            p.last_action_round = 0;
        }
        world.round_nr_772 = 90;
        let kick = world.process_connections_772();
        assert_eq!(kick.len(), 1, "LastCommand >= 90 must trigger connection timeout");
    }
}