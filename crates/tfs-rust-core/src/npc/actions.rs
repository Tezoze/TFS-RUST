//! Immediate NPC dialogue mutation host (NPC-5).
//!
//! Domain: TFS-style `Npc` actions on the game thread.
//! 772 outcomes: left-to-right `TBehaviourDatabase::react` mutations (`crnonpl.cc:1085-1291`)
//! with no rollback on partial failure.

use crate::ids::CreatureId;

/// Game-thread host for standard dialogue mutations.
///
/// Implemented by [`crate::game_world::GameWorld`]. Failures return `Err` and are
/// logged by the caller; already-applied actions remain applied.
pub trait NpcActionHost {
    fn create_item(&mut self, player: CreatureId, item_id: i32, count: i32) -> Result<(), String>;
    fn delete_item(&mut self, player: CreatureId, item_id: i32, count: i32) -> Result<(), String>;
    fn create_money(&mut self, player: CreatureId, amount: i32) -> Result<(), String>;
    fn delete_money(&mut self, player: CreatureId, amount: i32) -> Result<(), String>;
    fn set_hp(&mut self, player: CreatureId, value: i32) -> Result<(), String>;
    fn set_poison(&mut self, player: CreatureId, cycles: i32, param: i32) -> Result<(), String>;
    fn set_burning(&mut self, player: CreatureId, cycles: i32, param: i32) -> Result<(), String>;
    fn effect_me(&mut self, npc: CreatureId, effect_id: u16) -> Result<(), String>;
    fn effect_opp(&mut self, player: CreatureId, effect_id: u16) -> Result<(), String>;
    fn set_quest_value(&mut self, player: CreatureId, id: u32, value: i32) -> Result<(), String>;
    fn set_profession(&mut self, player: CreatureId, vocation: i32) -> Result<(), String>;
    fn teach_spell(&mut self, player: CreatureId, spell: i32) -> Result<(), String>;
    fn summon(&mut self, npc: CreatureId, monster: &str) -> Result<(), String>;
    fn teleport(&mut self, player: CreatureId, x: i32, y: i32, z: i32) -> Result<(), String>;
    /// `pos = None` → use NPC home coordinates (772 bare `StartPosition`).
    fn set_start_position(
        &mut self,
        player: CreatureId,
        npc: CreatureId,
        pos: Option<(i32, i32, i32)>,
    ) -> Result<(i32, i32, i32), String>;

    /// NPC-7: invoke a custom Lua action callback.
    fn invoke_custom_action(
        &mut self,
        npc: CreatureId,
        player: CreatureId,
        callback_id: tfs_rust_content::npcs::NpcCallbackId,
    ) -> Result<(), String>;
}

/// Logging context for a failed action.
#[derive(Debug, Clone)]
pub struct ActionFailCtx<'a> {
    pub npc_name: &'a str,
    pub player: CreatureId,
    pub rule_span: &'a str,
    pub action_index: usize,
}

pub fn log_action_failure(ctx: &ActionFailCtx<'_>, err: &str) {
    tracing::warn!(
        npc = %ctx.npc_name,
        player = ?ctx.player,
        rule = %ctx.rule_span,
        action = ctx.action_index,
        "{err}"
    );
}
