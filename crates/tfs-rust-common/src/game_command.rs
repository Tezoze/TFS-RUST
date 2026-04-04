//! Game-thread commands (client input, scheduler, shutdown).
// C++ reference: distributed across `connection.cpp`, `game.cpp`, scheduler hooks.

use crate::enums::Direction;
use crate::Position;

#[derive(Debug, Clone)]
pub enum GameCommand {
    /// Enter game with a resolved character name (after DB auth in net layer).
    PlayerLogin {
        name: String,
    },
    PlayerLogout,
    PlayerMove(Direction),
    PlayerSay(String),
    PlayerUseItem(Position),
    PlayerAttack(u32),
    ExtendedOpcode(u8, String),
    /// `addEvent` / scheduler wake (Phase 4 `Scheduler`).
    LuaCallback {
        event_id: u64,
    },
    /// Graceful shutdown (SIGINT/SIGTERM or admin).
    Shutdown,
    Unknown(u8),
}
