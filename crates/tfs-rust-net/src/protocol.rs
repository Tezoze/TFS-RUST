use tfs_rust_common::Position;

pub enum ConnectionState {
    Handshake,
    Login(ProtocolLogin),
    Status(ProtocolStatus),
    Game(ProtocolGame),
    Closed,
}

pub struct ProtocolLogin {}
pub struct ProtocolStatus {}
pub struct ProtocolGame {}

#[derive(Debug, Clone)]
pub enum GameCommand {
    // TODO(Phase 7): Add `conn_id: u32, account: AccountData, char_name: String`
    PlayerLogin,
    // TODO(Phase 7): Add `conn_id: u32`
    PlayerLogout,
    PlayerMove(tfs_rust_common::enums::Direction),
    PlayerSay(String),
    PlayerUseItem(Position),
    PlayerAttack(u32),
    ExtendedOpcode(u8, String),
    // ... we will fill the rest of the 50+ opcodes progressively as we build out handlers
    Unknown(u8),
}
