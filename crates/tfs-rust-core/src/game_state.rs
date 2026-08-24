//! Server game-state bits used by the daily save clock.
//!
//! Pack surface: TFS `GAME_STATE_NORMAL` / `GAME_STATE_CLOSED` / `GAME_STATE_SHUTDOWN`
//! (`game.h`). Closed blocks new logins; shutdown is the save-then-exit path.
// C++ reference: `Game::setGameState` — `game.cpp`.

/// TFS `GameState_t` subset we actually drive from Rust (no Lua `setGameState`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GameState {
    #[default]
    Normal,
    Closed,
    Shutdown,
}
