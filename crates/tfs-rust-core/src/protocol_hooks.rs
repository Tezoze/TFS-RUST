//! Hooks for OTC extended opcode and async Lua/DB results (game thread).
// C++ reference (this repo): `src/game.cpp` `Game::parsePlayerExtendedOpcode`, `connection.cpp` async paths.
// Phase 8 replaces `NullProtocolHooks` with Lua `PacketHandler` dispatch.

use std::sync::Arc;

use tfs_rust_common::ConnId;

/// Extended opcode (`0x32`) and `LuaAsyncResult` delivery — implemented by Lua layer in Phase 8.
pub trait ProtocolHooks: Send + Sync {
    /// Client → server extended opcode payload (after `parseExtendedOpcode`).
    fn extended_opcode(&self, conn_id: ConnId, opcode: u8, buffer: String);

    /// Result of `db.asyncQuery` / async work (next tick), OTCv8 flows.
    fn lua_async_result(&self, conn_id: ConnId, request_id: u64, payload: &[u8], success: bool);
}

/// Default no-op until Lua wiring.
pub struct NullProtocolHooks;

impl ProtocolHooks for NullProtocolHooks {
    fn extended_opcode(&self, _conn_id: ConnId, _opcode: u8, _buffer: String) {}

    fn lua_async_result(
        &self,
        _conn_id: ConnId,
        _request_id: u64,
        _payload: &[u8],
        _success: bool,
    ) {
    }
}

pub type SharedProtocolHooks = Arc<dyn ProtocolHooks>;
