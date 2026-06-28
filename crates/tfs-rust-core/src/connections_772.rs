//! 772 `ProcessConnections` — idle kick and round tracking.
//!
//! C++ reference: `connections.cc:21` `TConnection::Process`, `connections.cc:53` `ResetTimer`.

use tfs_rust_common::ConnId;
use tfs_rust_common::GamePacket;
use tfs_rust_net::outgoing::send_text_message;

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;

/// C++ `TALK_ADMIN_MESSAGE` (`enums.hh`).
const TALK_ADMIN_MESSAGE: u8 = 18;

/// Rounds without action before idle warning — `connections.cc:29`.
const IDLE_WARN_ROUNDS: u32 = 900;
/// Rounds without action before forced logout — `connections.cc:35`.
const IDLE_KICK_ROUNDS: u32 = 960;

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

    /// C++ `ProcessConnections` idle kick path (`connections.cc:28-37`).
    pub(crate) fn process_connections_772(&mut self) -> Vec<ConnId> {
        if !self.beat_driven_loop {
            return Vec::new();
        }
        let round = self.round_nr_772;
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
            let last_action = round.saturating_sub(p.last_action_round);
            if last_action == IDLE_WARN_ROUNDS {
                let msg = "You have been idle for 15 minutes. You will be disconnected in one minute if you are still idle then.";
                self.enqueue_outgoing(conn_id, send_text_message(TALK_ADMIN_MESSAGE, msg).into_bytes());
            }
            if last_action >= IDLE_KICK_ROUNDS {
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
        world.conn_to_creature.insert(tfs_rust_common::ConnId(1), player);

        if let Some(CreatureKind::Player(p)) = world.creatures.get_mut(player) {
            p.last_action_round = 0;
        }
        world.round_nr_772 = 960;
        let kick = world.process_connections_772();
        assert_eq!(kick.len(), 1);
        assert_eq!(kick[0].0, 1);
    }
}