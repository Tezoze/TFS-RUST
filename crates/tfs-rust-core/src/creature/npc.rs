//! NPCs and script event dispatch surface (Lua wired in Phase 8 / NPC-7).
//!
//! Domain: TFS-style `Npc` instance; definitions live in [`tfs_rust_content::npcs::NpcDatabase`].
//! C++ reference: `Npc` (`npc.h`); 772 runtime state shaped after `TNonPlayer` behaviour focus.

use std::collections::{HashMap, VecDeque};

use tfs_rust_common::Position;
use tfs_rust_content::npcs::{DialoguePolicy, NpcTypeId};

use crate::creature::base::CreatureBase;
use crate::ids::CreatureId;

/// Per-instance NPC activity (772 sleep/wake / talk / leave).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NpcActivity {
    Sleeping,
    #[default]
    Idle,
    Talking,
    Leaving,
}

/// One FIFO wait-queue entry (`QueuedPlayers` + `QueuedAddresses`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuedNpcAddress {
    pub player: CreatureId,
    pub text: String,
}

/// Per-player session vars when [`DialoguePolicy::PerPlayer`] is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NpcPlayerSession {
    pub topic: i32,
    pub price: i32,
    pub amount: i32,
    pub item_type: i32,
    pub data: i32,
    pub last_talk_round: u32,
    pub active: bool,
}

/// Game-thread-only conversation / roam state.
#[derive(Debug, Clone)]
pub struct NpcRuntimeState {
    pub activity: NpcActivity,
    pub policy: DialoguePolicy,
    pub topic: i32,
    pub price: i32,
    pub amount: i32,
    pub item_type: i32,
    pub data: i32,
    pub last_talk_round: u32,
    pub home_position: Position,
    pub radius: u16,
    /// Next roam attempt deadline in `server_ms` (NPC-6).
    pub next_walk: Option<u64>,
    pub focus: Option<CreatureId>,
    pub queue: VecDeque<QueuedNpcAddress>,
    /// Opt-in per-player sessions ([`DialoguePolicy::PerPlayer`]).
    pub player_sessions: HashMap<CreatureId, NpcPlayerSession>,
}

impl NpcRuntimeState {
    /// Fresh idle state at `home` with definition radius and conversation policy.
    pub fn at_home(home: Position, radius: u16, policy: DialoguePolicy) -> Self {
        Self {
            activity: NpcActivity::Idle,
            policy,
            topic: 0,
            price: 0,
            amount: 0,
            item_type: 0,
            data: 0,
            last_talk_round: 0,
            home_position: home,
            radius,
            next_walk: None,
            focus: None,
            queue: VecDeque::new(),
            player_sessions: HashMap::new(),
        }
    }

    /// Whether the NPC is currently holding a conversation (or has waiters).
    pub fn is_engaged(&self) -> bool {
        matches!(self.activity, NpcActivity::Talking | NpcActivity::Leaving)
            || !self.queue.is_empty()
            || self.focus.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct Npc {
    pub base: CreatureBase,
    /// Typed definition id into [`tfs_rust_content::npcs::NpcDatabase`].
    pub definition: NpcTypeId,
    /// Copied from definition at spawn for wire encoding without DB re-lookup.
    pub speech_bubble: u8,
    /// Auto-incrementing wire id (same scheme as `Monster::wire_id`).
    /// C++ `Npc::setID` uses the same `monsterAutoID` counter (`monster.h:168`).
    pub wire_id: u32,
    pub runtime: NpcRuntimeState,
}

impl Npc {
    /// Ad-hoc test/sim NPC with no definition database entry.
    ///
    /// Production spawns must go through [`crate::game_world::GameWorld::spawn_npc`].
    pub fn placeholder(base: CreatureBase) -> Self {
        let home = base.position;
        Self {
            base,
            definition: NpcTypeId(0),
            speech_bubble: 0,
            wire_id: 0,
            runtime: NpcRuntimeState::at_home(home, 0, DialoguePolicy::QueuedSingleFocus),
        }
    }
}

/// Hooks for NPC Lua — implemented by `tfs-rust-lua` later; core stays trait-only.
/// Retired in NPC-7 in favour of game-thread `EventDispatcher` fire helpers.
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
