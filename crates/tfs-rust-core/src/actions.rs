//! Action registry — `data/scripts/actions/**` self-registering `onUse` scripts.
//!
//! C++ reference: `actions.h` / `actions.cpp` `Actions::{registerLuaEvent,getAction,useItem,canUseFar}`.

use std::collections::HashMap;
use std::sync::Arc;

use tfs_rust_lua::ActionDef;

/// One registered action callback (shared across multiple item/action ids).
#[derive(Debug, Clone)]
pub struct ActionEntry {
    pub on_use: Arc<mlua::RegistryKey>,
    /// C++ `Action::allowFarUse` — `actions.h`. ToDo Use Obj2 uses `canUseFar`.
    pub allow_far_use: bool,
}

/// Indexed action registry — materialized at startup on the game thread.
#[derive(Debug, Default)]
pub struct ActionRegistry {
    /// Server item type id → callback (`useItemMap`).
    pub by_item_id: HashMap<u16, ActionEntry>,
    /// Action id → callback (`actionItemMap`).
    pub by_action_id: HashMap<u16, ActionEntry>,
}

impl ActionRegistry {
    /// Build from defs loaded by `load_action_scripts`.
    ///
    /// C++ `Actions::registerLuaEvent` — duplicate keys warn but last wins.
    pub fn from_defs(defs: Vec<ActionDef>) -> Self {
        let mut by_item_id = HashMap::new();
        let mut by_action_id = HashMap::new();
        for def in defs {
            let Some(on_use) = def.on_use else {
                tracing::warn!(
                    item_ids = ?def.item_ids,
                    action_ids = ?def.action_ids,
                    "action has no onUse callback; skipping"
                );
                continue;
            };
            let entry = ActionEntry {
                on_use: Arc::clone(&on_use),
                allow_far_use: def.allow_far_use,
            };
            for id in def.item_ids {
                if by_item_id.insert(id, entry.clone()).is_some() {
                    tracing::warn!(item_id = id, "duplicate Action item id; overwriting");
                }
            }
            for aid in def.action_ids {
                if by_action_id.insert(aid, entry.clone()).is_some() {
                    tracing::warn!(action_id = aid, "duplicate Action aid; overwriting");
                }
            }
        }
        Self {
            by_item_id,
            by_action_id,
        }
    }

    /// Lookup order without uniqueid: action id, then item type.
    ///
    /// C++ `Actions::getAction` — `uniqueItemMap` → `actionItemMap` → `useItemMap`
    /// (uniqueid deferred to a later phase).
    pub fn get(&self, item_type: u16, action_id: u16) -> Option<&ActionEntry> {
        if action_id != 0
            && let Some(entry) = self.by_action_id.get(&action_id)
        {
            return Some(entry);
        }
        self.by_item_id.get(&item_type)
    }

    pub fn len(&self) -> usize {
        self.by_item_id.len() + self.by_action_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_item_id.is_empty() && self.by_action_id.is_empty()
    }
}
