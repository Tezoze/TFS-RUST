use crate::pending_login::PendingLogin;

pub use crate::game_command::GameCommand;

/// TCP + protocol state. `PendingLogin` holds the oneshot receiver for async DB/login (task 1.6b).
pub enum ConnectionState {
    Handshake,
    Login(ProtocolLogin),
    Status(ProtocolStatus),
    Game(ProtocolGame),
    /// Awaiting `LoginPendingResult` from the game/DB side; chat may be queued, movement/attack dropped.
    PendingLogin(PendingLogin),
    Closed,
}

pub struct ProtocolLogin {}
pub struct ProtocolStatus {}
pub struct ProtocolGame {}
