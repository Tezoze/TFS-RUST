//! NPCs and script event dispatch surface (Lua wired in Phase 8).
// C++ reference: `Npc` (`npc.h`), `NpcScriptInterface`.

use crate::creature::base::CreatureBase;
use crate::ids::CreatureId;

#[derive(Debug, Clone)]
pub struct Npc {
    pub base: CreatureBase,
    pub npc_type_id: u32,
}

/// Hooks for NPC Lua — implemented by `tfs-rust-lua` later; core stays trait-only.
pub trait NpcEventsHandler: Send + Sync + 'static {
    fn on_appear(&self, _npc: CreatureId) {}
    fn on_disappear(&self, _npc: CreatureId) {}
    fn on_say(&self, _npc: CreatureId, _speaker: CreatureId, _words: &str) {}
    fn on_buy(&self, _npc: CreatureId, _buyer: CreatureId, _item_type: u16, _amount: u16) {}
    fn on_sell(&self, _npc: CreatureId, _buyer: CreatureId, _item_type: u16, _amount: u16) {}
    fn on_check_item(&self, _npc: CreatureId, _player: CreatureId, _item_type: u16) -> bool {
        true
    }
    fn on_close_channel(&self, _npc: CreatureId, _player: CreatureId) {}
}

#[derive(Debug, Default)]
pub struct NullNpcHandler;

impl NpcEventsHandler for NullNpcHandler {}
