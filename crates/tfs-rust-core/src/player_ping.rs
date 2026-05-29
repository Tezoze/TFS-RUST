//! Connection keepalive — TFS `Player::sendPing` / `Game::playerReceivePing*`.
// C++ reference: `src/player.cpp` `Player::sendPing`, `receivePing`; `src/game.cpp` `playerReceivePing`, `playerReceivePingBack`.

use std::time::{Duration, Instant};

use tfs_rust_common::ConnId;
use tfs_rust_net::outgoing::{send_ping, send_ping_back};

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;

/// Server → client ping interval — `Player::sendPing` (`player.cpp` ~902).
const PING_INTERVAL: Duration = Duration::from_secs(5);

impl GameWorld {
    /// Client `0x1E` — `Game::playerReceivePing` → `Player::receivePing` + reply.
    pub(crate) fn player_receive_ping(
        &mut self,
        conn_id: ConnId,
        cid: CreatureId,
        now: Instant,
    ) {
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.last_pong_at = now;
        }
        self.enqueue_outgoing(conn_id, send_ping_back().into_bytes());
    }

    /// Client `0x1D` — `Game::playerReceivePingBack` → `ProtocolGame::sendPingBack`.
    pub(crate) fn player_receive_ping_back(&mut self, conn_id: ConnId, cid: CreatureId) {
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.last_pong_at = Instant::now();
        }
        self.enqueue_outgoing(conn_id, send_ping_back().into_bytes());
    }

    /// Called from `GameWorld::on_tick` — periodic `ProtocolGame::sendPing` (`0x1D`) per online player.
    pub(crate) fn tick_player_pings(&mut self, now: Instant) {
        let online: Vec<(ConnId, CreatureId)> = self
            .conn_to_creature
            .iter()
            .map(|(&conn, &cid)| (conn, cid))
            .collect();
        for (conn_id, cid) in online {
            let Some(CreatureKind::Player(p)) = self.creatures.get(cid) else {
                continue;
            };
            if now.duration_since(p.last_ping_sent) < PING_INTERVAL {
                continue;
            }
            if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
                p.last_ping_sent = now;
            }
            self.enqueue_outgoing(conn_id, send_ping().into_bytes());
        }
    }
}
