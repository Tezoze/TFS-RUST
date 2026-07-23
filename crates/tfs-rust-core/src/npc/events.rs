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
    /// Mutating / deferred action recorded but not applied in NPC-4.
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
