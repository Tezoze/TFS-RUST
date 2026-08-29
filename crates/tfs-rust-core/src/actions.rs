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
    /// Unique id → callback (`uniqueItemMap`).
    pub by_unique_id: HashMap<u16, ActionEntry>,
}

impl ActionRegistry {
    /// Build from defs loaded by `load_action_scripts`.
    ///
    /// C++ `Actions::registerLuaEvent` — duplicate keys warn but last wins.
    pub fn from_defs(defs: Vec<ActionDef>) -> Self {
        let mut by_item_id = HashMap::new();
        let mut by_action_id = HashMap::new();
        let mut by_unique_id = HashMap::new();
        for def in defs {
            let Some(on_use) = def.on_use else {
                tracing::warn!(
                    item_ids = ?def.item_ids,
                    action_ids = ?def.action_ids,
                    unique_ids = ?def.unique_ids,
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
            for uid in def.unique_ids {
                if by_unique_id.insert(uid, entry.clone()).is_some() {
                    tracing::warn!(unique_id = uid, "duplicate Action unique id; overwriting");
                }
            }
        }
        Self {
            by_item_id,
            by_action_id,
            by_unique_id,
        }
    }

    /// C++ `Actions::getAction` — `uniqueItemMap` → `actionItemMap` → `useItemMap`.
    pub fn get(&self, item_type: u16, action_id: u16, unique_id: u16) -> Option<&ActionEntry> {
        if unique_id != 0
            && let Some(entry) = self.by_unique_id.get(&unique_id)
        {
            return Some(entry);
        }
        if action_id != 0
            && let Some(entry) = self.by_action_id.get(&action_id)
        {
            return Some(entry);
        }
        self.by_item_id.get(&item_type)
    }

    /// O(1) membership matching [`Self::get`] (no Lua, no userdata).
    #[inline]
    pub fn has_event(&self, item_type: u16, action_id: u16, unique_id: u16) -> bool {
        (unique_id != 0 && self.by_unique_id.contains_key(&unique_id))
            || (action_id != 0 && self.by_action_id.contains_key(&action_id))
            || self.by_item_id.contains_key(&item_type)
    }

    pub fn len(&self) -> usize {
        self.by_item_id.len() + self.by_action_id.len() + self.by_unique_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_item_id.is_empty() && self.by_action_id.is_empty() && self.by_unique_id.is_empty()
    }
}
