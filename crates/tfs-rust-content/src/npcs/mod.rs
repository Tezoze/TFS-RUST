//! Immutable NPC definition database — Lua `NpcType` / `NpcDialogue` registrations.
//!
//! Domain: TFS-style NPC content (`npc.cpp` / `NpcType`) under `data/npc/scripts/`.
//! 772 outcomes preserved later via importer → these types; runtime matching in NPC-4+.
//!
//! C++ reference (metadata shape): `tibia-game-master` `.npc` Name/Sex/Race/Outfit/Home/Radius/
//! GoStrength + `Behaviour`; TFS domain: `NpcType` appearance/movement/shop/parameters.

mod dialogue;
mod shop;
mod span;
mod validate;

pub use dialogue::{
    DialogueAction, DialogueExpr, DialoguePolicy, DialoguePredicate, DialogueProgram, DialogueProperty,
    DialogueRule, DialogueSituation, ExprOp, NpcCallbackId, SessionVar,
};
pub use shop::{NpcShopDefinition, NpcShopItem};
pub use span::SourceSpan;
pub use validate::{NpcValidateError, validate_pending_definitions};

use std::collections::HashMap;
use std::sync::Arc;

/// Typed NPC definition id (index into [`NpcDatabase::by_id`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NpcTypeId(pub u32);

impl NpcTypeId {
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// Outfit / look for an NPC (TFS `npc:appearance` / 772 Outfit tuple).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcAppearance {
    pub look_type: u16,
    pub look_head: u8,
    pub look_body: u8,
    pub look_legs: u8,
    pub look_feet: u8,
    pub look_addons: u8,
    pub look_type_ex: u16,
    pub look_mount: u16,
}

impl Default for NpcAppearance {
    fn default() -> Self {
        Self {
            look_type: 136,
            look_head: 0,
            look_body: 0,
            look_legs: 0,
            look_feet: 0,
            look_addons: 0,
            look_type_ex: 0,
            look_mount: 0,
        }
    }
}

/// Idle movement / home radius (772 Radius / GoStrength; TFS walk radius).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcMovement {
    pub radius: u16,
    pub speed: u16,
    /// 772 `GoStrength` — walk attempt budget / strength.
    pub go_strength: u16,
}

impl Default for NpcMovement {
    fn default() -> Self {
        Self {
            radius: 0,
            speed: 100,
            go_strength: 0,
        }
    }
}

/// Optional voice line for idle chatter (TFS voices).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcVoice {
    pub text: String,
    pub interval_ms: u32,
    pub chance: u32,
}

/// Named custom Lua callback slot (opaque id; RegistryKey on LuaRuntime).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcCallbackSlot {
    pub name: String,
    pub id: NpcCallbackId,
}

/// Immutable NPC type definition.
#[derive(Debug, Clone)]
pub struct NpcDefinition {
    pub id: NpcTypeId,
    pub name: String,
    pub appearance: NpcAppearance,
    pub health_max: u32,
    pub movement: NpcMovement,
    /// Speech bubble id (TFS); `0` = none.
    pub speech_bubble: u8,
    /// Sex: 0 female / 1 male (772 Sex); unused until spawn.
    pub sex: u8,
    /// Race id (772 Race).
    pub race: u16,
    pub parameters: HashMap<String, String>,
    pub voices: Vec<NpcVoice>,
    pub dialogue: Option<DialogueProgram>,
    pub shop: Option<NpcShopDefinition>,
    /// Custom predicate callbacks by name.
    pub custom_predicates: Vec<NpcCallbackSlot>,
    /// Custom action callbacks by name.
    pub custom_actions: Vec<NpcCallbackSlot>,
}

/// Frozen NPC registry: name → id → `Arc<NpcDefinition>`.
#[derive(Debug, Default, Clone)]
pub struct NpcDatabase {
    by_id: Vec<Arc<NpcDefinition>>,
    by_name: HashMap<String, NpcTypeId>,
}

impl NpcDatabase {
    /// Empty database.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of registered NPC types.
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    /// Lookup by spawn/definition name (case-insensitive).
    pub fn get_by_name(&self, name: &str) -> Option<&Arc<NpcDefinition>> {
        let id = *self.by_name.get(&name.to_ascii_lowercase())?;
        self.get(id)
    }

    /// Lookup by typed id.
    pub fn get(&self, id: NpcTypeId) -> Option<&Arc<NpcDefinition>> {
        self.by_id.get(id.index())
    }

    /// Iterate all definitions in id order.
    pub fn iter(&self) -> impl Iterator<Item = &Arc<NpcDefinition>> {
        self.by_id.iter()
    }

    /// Build from already-validated definitions (used by [`validate::validate_pending_definitions`]).
    pub(crate) fn from_validated(defs: Vec<NpcDefinition>) -> Self {
        let mut by_id = Vec::with_capacity(defs.len());
        let mut by_name = HashMap::with_capacity(defs.len());
        for def in defs {
            let key = def.name.to_ascii_lowercase();
            let id = def.id;
            by_name.insert(key, id);
            by_id.push(Arc::new(def));
        }
        Self { by_id, by_name }
    }
}

/// Intermediate definition before id assignment / freeze (Lua drain → validate).
#[derive(Debug, Clone, Default)]
pub struct PendingNpcDefinition {
    pub name: String,
    pub source_file: String,
    pub appearance: NpcAppearance,
    pub health_max: u32,
    pub movement: NpcMovement,
    pub speech_bubble: u8,
    pub sex: u8,
    pub race: u16,
    pub parameters: HashMap<String, String>,
    pub voices: Vec<NpcVoice>,
    pub dialogue: Option<DialogueProgram>,
    pub shop: Option<NpcShopDefinition>,
    pub custom_predicates: Vec<NpcCallbackSlot>,
    pub custom_actions: Vec<NpcCallbackSlot>,
}
