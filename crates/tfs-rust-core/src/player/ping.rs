//! Connection keepalive — TFS `Player::sendPing` / `Game::playerReceivePing*`.
// C++ reference: `src/player.cpp` `Player::sendPing`, `receivePing`; `src/game.cpp` `playerReceivePing`, `playerReceivePingBack`.
// 772 reference: `reference/tvp-772/gameserver/src/protocolgame.cpp` `sendPing`/`sendPingBack`,
//   `player.cpp` `Player::sendPing` (onThink → 5000ms wallclock), `player.h` `receivePing`.

use std::time::{Duration, Instant};

use tfs_rust_common::{CLIENTOS_OTCLIENT_LINUX, ConnId};
use tfs_rust_net::outgoing::send_ping_back;

use crate::creature::CreatureKind;
use crate::game_world::GameWorld;
use crate::ids::CreatureId;

/// Server → client ping interval — `Player::sendPing` (`player.cpp` ~902, both eras).
const PING_INTERVAL: Duration = Duration::from_secs(5);

impl GameWorld {
    /// Client `0x1E` — `Game::playerReceivePing` → `Player::receivePing` (`player.h:912`).
    //
    // TVP 772: just records `lastPong`, **no reply**. The 1098 C++ also does not reply here
    // (`playerReceivePing` → `receivePing` only). Our previous code incorrectly sent `0x1E` back.
    pub(crate) fn player_receive_ping(&mut self, _conn_id: ConnId, cid: CreatureId, now: Instant) {
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.last_pong_at = now;
        }
        // No reply — matches TVP `playerReceivePing` and 1098 `playerReceivePing`.
    }

    /// Client `0x1D` — `Game::playerReceivePingBack` → `ProtocolGame::sendPingBack` (`0x1E`).
    pub(crate) fn player_receive_ping_back(&mut self, conn_id: ConnId, cid: CreatureId) {
        if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
            p.last_pong_at = Instant::now();
        }
        self.enqueue_outgoing(conn_id, send_ping_back().into_bytes());
    }

    /// Periodic `ProtocolGame::sendPing` per online player — called from both 1098 `on_tick` and
    /// 772 `run_other_subsystems`.
    //
    // C++ 1098 (`src/protocolgame.cpp:2530`): always `0x1D`.
    // TVP 772 (`protocolgame.cpp:1516`): `0x1D` for OTClient (`OS >= CLIENTOS_OTCLIENT_LINUX`),
    //   `0x1E` for non-OTClient (real Tibia / Forgotten Client). The 772 `Player::sendPing`
    //   (`player.cpp:754`) is wallclock-based (5000ms from `onThink`) and runs alongside the
    //   round-based `ProcessConnections` idle ping — both eras use the 5s wallclock ping.
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
            // TVP 772: OTClient gets 0x1D, non-OTClient gets 0x1E.
            // 1098: always 0x1D (send_ping).
            let is_otclient = p.operating_system >= CLIENTOS_OTCLIENT_LINUX;
            if let Some(CreatureKind::Player(p)) = self.creatures.get_mut(cid) {
                p.last_ping_sent = now;
            }
            // X1: opcode selection pushed to the codec — core only signals "send periodic ping".
            let pkt = self.codec.periodic_ping_packet(is_otclient);
            self.enqueue_outgoing(conn_id, pkt.into_bytes());
        }
    }
}
