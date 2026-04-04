//! Buffered requests from Lua scripts to mutate game state on the game thread.
// C++ reference: mutations performed inside `LuaScriptInterface` calls — batched here for async safety.

use crate::ids::{CreatureId, ItemId};
use tfs_rust_common::Position;

#[derive(Debug, Clone)]
pub enum LuaCommand {
    Teleport {
        creature: CreatureId,
        position: Position,
    },
    SetHealth {
        creature: CreatureId,
        value: i32,
    },
    SetMana {
        creature: CreatureId,
        value: i32,
    },
    SendCreatureSay {
        creature: CreatureId,
        text: String,
    },
    RemoveItem {
        item: ItemId,
    },
    AddItem {
        item_type: u16,
        position: Position,
        count: u16,
    },
}
