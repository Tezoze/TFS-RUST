//! Ordered dialogue/state events for NPC-0 fixture parity and diagnostics.

use crate::ids::CreatureId;

/// Runtime situation passed into the matcher (`SITUATION` in `crnonpl.cc`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogueSituationKind {
    Address,
    Default,
    Busy,
    Vanish,
    AddressQueue,
}

impl DialogueSituationKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Address => "ADDRESS",
            Self::Default => "DEFAULT",
            Self::Busy => "BUSY",
            Self::Vanish => "VANISH",
            Self::AddressQueue => "ADDRESSQUEUE",
        }
    }
}

/// World mutation recorded in order for fixture / differential tests (NPC-5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MutateOp {
    CreateItem {
        item_id: i32,
        count: i32,
    },
    DeleteItem {
        item_id: i32,
        count: i32,
    },
    CreateMoney {
        amount: i32,
    },
    DeleteMoney {
        amount: i32,
    },
    SetCondition {
        condition: &'static str,
        value: i32,
    },
    Effect {
        effect_id: u16,
        on_npc: bool,
    },
    SetQuestValue {
        id: u32,
        value: i32,
    },
    SetHp {
        value: i32,
    },
    Profession {
        vocation: i32,
    },
    TeachSpell {
        spell: i32,
    },
    Summon {
        monster: String,
    },
    Teleport {
        x: i32,
        y: i32,
        z: i32,
    },
    StartPosition {
        x: i32,
        y: i32,
        z: i32,
    },
    /// NPC-7 custom Lua action completed.
    CustomAction,
}

/// Observable dialogue/state event (NPC-0 fixture `expected[]` kinds).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogueEvent {
    Situation {
        name: &'static str,
    },
    MatchRule {
        index: usize,
    },
    State {
        value: &'static str,
    },
    Focus {
        player: Option<CreatureId>,
        temporary: bool,
    },
    TurnTo {
        player: CreatureId,
    },
    Queue {
        op: QueueOp,
        player: CreatureId,
        text: String,
    },
    Set {
        var: &'static str,
        value: i32,
    },
    Say {
        text: String,
        delay_ms: u32,
        byte_len: usize,
    },
    Todo {
        op: TodoOp,
        delay_ms: Option<u32>,
    },
    /// Immediate world mutation applied left-to-right (NPC-5).
    Mutate {
        player: CreatureId,
        op: MutateOp,
    },
    /// Custom Lua action — not applied until NPC-7.
    DeferredAction {
        kind: &'static str,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueOp {
    Push,
    Pop,
    DedupeSkip,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TodoOp {
    Wait,
    Talk,
    Start,
}

/// Accumulator used by react / focus and fixture harnesses.
#[derive(Debug, Default, Clone)]
pub struct DialogueTrace {
    pub events: Vec<DialogueEvent>,
}

impl DialogueTrace {
    pub fn push(&mut self, event: DialogueEvent) {
        self.events.push(event);
    }

    pub fn clear(&mut self) {
        self.events.clear();
    }
}
