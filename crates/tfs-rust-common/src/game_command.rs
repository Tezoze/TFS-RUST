//! Game-thread commands: client protocol, scheduler, shutdown, async Lua results.
// C++ reference (this repo): `src/connection.cpp`, `src/game.cpp`, `src/tasks.cpp`.

use crate::conn_id::ConnId;
use crate::game_packet::GamePacket;

#[derive(Debug, Clone)]
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
    PlayerLogin {
        conn_id: ConnId,
        name: String,
        /// `OperatingSystem_t` from first game TCP message (`protocolgame.cpp` `onRecvFirstMessage`).
        operating_system: u16,
        /// `0` = not detected; else OTCv8 build (253, 260, …) after `"OTCv8"` probe.
        otclient_v8: u16,
    },
    /// One decoded client game packet.
    Game { conn_id: ConnId, packet: GamePacket },
}
