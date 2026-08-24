//! Game-thread commands: client protocol, scheduler, shutdown, async Lua results.
// C++ reference (this repo): `src/connection.cpp`, `src/game.cpp`, `src/tasks.cpp`.

use crate::conn_id::ConnId;
use crate::game_packet::GamePacket;
use crate::owned_player_load::OwnedPlayerLoad;

#[derive(Debug)]
pub enum GameCommand {
    /// Stop the game loop.
    Shutdown,
    /// `addEvent` / scheduler wake (Phase 4 `Scheduler`).
    LuaCallback { event_id: u64 },
    /// Result of `db.asyncQuery` / async work delivered on the next tick (OTCv8 / extended flows).
    LuaAsyncResult {
        conn_id: ConnId,
        request_id: u64,
        /// Opaque success payload (Lua or JSON); empty on failure.
        payload: Vec<u8>,
        success: bool,
    },
    /// Character selected — enter world (may originate outside game opcode stream).
    /// Game thread must **not** await DB I/O: spawn load and wait for [`Self::PlayerLoaded`].
    PlayerLogin {
        conn_id: ConnId,
        name: String,
        /// `OperatingSystem_t` from first game TCP message (`protocolgame.cpp` `onRecvFirstMessage`).
        operating_system: u16,
        /// `0` = not detected; else OTCv8 build (253, 260, …) after `"OTCv8"` probe.
        otclient_v8: u16,
        /// TCP peer IPv4 packed for `luaPlayerGetIp` (`0` if unknown / non-v4).
        peer_ip: u32,
    },
    /// Async character load finished — apply on the game thread only if `conn_id` is still current.
    PlayerLoaded {
        conn_id: ConnId,
        name: String,
        operating_system: u16,
        otclient_v8: u16,
        /// TCP peer IPv4 packed for `luaPlayerGetIp`.
        peer_ip: u32,
        data: OwnedPlayerLoad,
    },
    /// Async character load failed (not found / DB error / overload reject).
    PlayerLoadFailed {
        conn_id: ConnId,
        name: String,
        reason: String,
    },
    /// Close connection and clean up player session (logout / kick).
    // C++ reference: `ProtocolGame::disconnect()` (`src/protocolgame.cpp`).
    PlayerDisconnect {
        conn_id: ConnId,
        /// Send logout effect (poff) before closing.
        display_effect: bool,
    },
    /// I/O thread registered a bounded outbound writer — mirror into game-thread sink map (GL-3).
    RegisterOutputSink { conn_id: ConnId },
    /// I/O thread removed outbound writer (TCP closed / writer task ended).
    UnregisterOutputSink { conn_id: ConnId },
    /// One decoded client game packet.
    Game { conn_id: ConnId, packet: GamePacket },
}
