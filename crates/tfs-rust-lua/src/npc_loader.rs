//! Load `data/npc/scripts/definitions/**/*.lua` into an immutable [`NpcDatabase`].
//!
//! Domain: TFS-style Lua `NpcType` / `NpcDialogue` registrations.
//! 772: definitions come from offline import of `.npc`/`.ndb` (NPC-2); this loader
//! only executes declarative Lua and freezes typed content.
//!
//! Callbacks: opaque [`NpcCallbackId`] on definitions; `RegistryKey`s on
//! [`LuaRuntime::npc_callbacks`].

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use mlua::RegistryKey;

use tfs_rust_content::items::ItemDatabase;
use tfs_rust_content::npcs::{
    DialogueAction, DialoguePredicate, NpcCallbackId, NpcCallbackSlot, NpcDatabase,
    PendingNpcDefinition, validate_pending_definitions,
};

use crate::npc_type::PendingNpc;
use crate::runtime::LuaRuntime;

impl LuaRuntime {
    /// Load NPC definition scripts from `data_dir/npc/scripts/definitions/**/*.lua`.
    ///
    /// Hard-fails on script exec errors and on validation errors after drain.
    /// Does not require `GameWorld`.
    pub fn load_npc_definitions(
        &mut self,
        data_dir: &Path,
        items: &ItemDatabase,
    ) -> Result<NpcDatabase, String> {
        let defs_dir = data_dir.join("npc/scripts/definitions");
        if !defs_dir.exists() {
            tracing::warn!(
                "NPC definitions dir not found: {}",
                defs_dir.display()
            );
            return Ok(NpcDatabase::new());
        }

        self.lua
            .globals()
            .set(
                "_pending_npcs",
                self.lua.create_table().map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
        self.lua
            .globals()
            .set(
                "_pending_npc_action_callbacks",
                self.lua.create_table().map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
        self.lua
            .globals()
            .set(
                "_pending_npc_predicate_callbacks",
                self.lua.create_table().map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;

        let mut lua_files: Vec<PathBuf> = Vec::new();
        collect_lua_files(&defs_dir, &mut lua_files);
        lua_files.retain(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| !n.starts_with('#'))
        });
        lua_files.sort();

        for path in &lua_files {
            let path_str = path.display().to_string();
            let source = std::fs::read_to_string(path).map_err(|e| {
                format!("failed to read NPC definition {path_str}: {e}")
            })?;
            self.lua
                .load(&source)
                .set_name(&path_str)
                .exec()
                .map_err(|e| format!("failed to load NPC definition {path_str}: {e}"))?;
        }

        let db = self.drain_pending_npcs(Some(items))?;

        tracing::info!(
            "Loaded {} NPC definition scripts → {} types",
            lua_files.len(),
            db.len()
        );

        Ok(db)
    }

    /// Drain `_pending_npcs` (+ callback tables) into a validated [`NpcDatabase`].
    ///
    /// Used by the directory loader and by unit tests that register inline.
    pub fn drain_pending_npcs(
        &mut self,
        items: Option<&ItemDatabase>,
    ) -> Result<NpcDatabase, String> {
        let pending = self
            .lua
            .globals()
            .get::<mlua::Table>("_pending_npcs")
            .map_err(|e| e.to_string())?;
        let action_cbs = self
            .lua
            .globals()
            .get::<mlua::Table>("_pending_npc_action_callbacks")
            .map_err(|e| e.to_string())?;
        let pred_cbs = self
            .lua
            .globals()
            .get::<mlua::Table>("_pending_npc_predicate_callbacks")
            .map_err(|e| e.to_string())?;

        let mut next_callback_id = 1u32;
        let mut defs: Vec<PendingNpcDefinition> = Vec::new();

        for pair in pending.pairs::<i64, mlua::AnyUserData>() {
            let (idx, ud) = pair.map_err(|e| e.to_string())?;
            let pending_npc = ud.borrow::<PendingNpc>().map_err(|e| e.to_string())?;
            let mut def = pending_npc.def.clone();

            // Resolve custom action callbacks.
            let mut action_name_to_id: HashMap<String, NpcCallbackId> = HashMap::new();
            if let Ok(map) = action_cbs.get::<mlua::Table>(idx) {
                for pair in map.pairs::<String, mlua::Function>() {
                    let (name, func) = pair.map_err(|e| e.to_string())?;
                    let id = NpcCallbackId(next_callback_id);
                    next_callback_id += 1;
                    let reg_key = self
                        .lua
                        .create_registry_value(func)
                        .map_err(|e| e.to_string())?;
                    self.register_npc_callback(id, reg_key);
                    action_name_to_id.insert(name.clone(), id);
                    def.custom_actions.push(NpcCallbackSlot {
                        name: name.clone(),
                        id,
                    });
                }
            }

            // Resolve custom predicate callbacks.
            let mut pred_name_to_id: HashMap<String, NpcCallbackId> = HashMap::new();
            if let Ok(map) = pred_cbs.get::<mlua::Table>(idx) {
                for pair in map.pairs::<String, mlua::Function>() {
                    let (name, func) = pair.map_err(|e| e.to_string())?;
                    let id = NpcCallbackId(next_callback_id);
                    next_callback_id += 1;
                    let reg_key = self
                        .lua
                        .create_registry_value(func)
                        .map_err(|e| e.to_string())?;
                    self.register_npc_callback(id, reg_key);
                    pred_name_to_id.insert(name.clone(), id);
                    def.custom_predicates.push(NpcCallbackSlot {
                        name: name.clone(),
                        id,
                    });
                }
            }

            // Patch Custom predicate/action placeholders with resolved ids.
            if let Some(ref mut dialogue) = def.dialogue {
                for rule in &mut dialogue.rules {
                    for pred in &mut rule.predicates {
                        if let DialoguePredicate::Custom {
                            callback_id, name, ..
                        } = pred
                        {
                            let id = pred_name_to_id.get(name).copied().ok_or_else(|| {
                                format!(
                                    "NPC {:?}: custom predicate {name:?} referenced but not registered via onCustomPredicate",
                                    def.name
                                )
                            })?;
                            *callback_id = id;
                        }
                    }
                    for action in &mut rule.actions {
                        if let DialogueAction::Custom {
                            callback_id, name, ..
                        } = action
                        {
                            let id = action_name_to_id.get(name).copied().ok_or_else(|| {
                                format!(
                                    "NPC {:?}: custom action {name:?} referenced but not registered via onCustomAction",
                                    def.name
                                )
                            })?;
                            *callback_id = id;
                        }
                    }
                }
            }

            defs.push(def);
        }

        // Clear pending buffers.
        let _ = self
            .lua
            .globals()
            .set("_pending_npcs", self.lua.create_table().unwrap());
        let _ = self.lua.globals().set(
            "_pending_npc_action_callbacks",
            self.lua.create_table().unwrap(),
        );
        let _ = self.lua.globals().set(
            "_pending_npc_predicate_callbacks",
            self.lua.create_table().unwrap(),
        );

        validate_pending_definitions(defs, items).map_err(|e| e.to_string())
    }

    /// Store an NPC custom callback RegistryKey (game-thread only).
    pub fn register_npc_callback(&mut self, id: NpcCallbackId, key: RegistryKey) {
        self.npc_callbacks.insert(id, key);
    }

    /// Lookup an NPC custom callback by opaque id.
    pub fn npc_callback(&self, id: NpcCallbackId) -> Option<&RegistryKey> {
        self.npc_callbacks.get(&id)
    }
}

fn collect_lua_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_lua_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "lua") {
            out.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn workspace_data_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
    }

    /// NPC-1 gate: handwritten greeting.lua loads without GameWorld.
    #[test]
    fn greeting_definition_loads_stable_snapshot() {
        let data_root = workspace_data_root();
        let greeting = data_root.join("npc/scripts/definitions/greeting.lua");
        if !greeting.exists() {
            panic!("missing smoke definition: {}", greeting.display());
        }

        // Minimal empty ItemDatabase — greeting.lua has no create/delete.
        let items = ItemDatabase {
            items: Default::default(),
            client_to_server: Default::default(),
        };

        let mut runtime = LuaRuntime::new().expect("runtime");
        let db = runtime
            .load_npc_definitions(&data_root, &items)
            .expect("load npc definitions");

        let def = db
            .get_by_name("Quentin")
            .expect("Quentin registered");
        assert_eq!(def.name, "Quentin");
        assert_eq!(def.appearance.look_type, 57);
        assert_eq!(def.movement.radius, 4);

        let dialogue = def.dialogue.as_ref().expect("dialogue");
        assert_eq!(
            dialogue.policy,
            tfs_rust_content::npcs::DialoguePolicy::QueuedSingleFocus
        );
        assert_eq!(dialogue.rules.len(), 2);

        // First rule ADDRESS + hi$ → welcome say
        assert!(matches!(
            &dialogue.rules[0].predicates[0],
            DialoguePredicate::Situation {
                kind: tfs_rust_content::npcs::DialogueSituation::Address,
                ..
            }
        ));
        match &dialogue.rules[0].actions[0] {
            DialogueAction::Say { text, .. } => {
                assert!(
                    text.contains("Welcome, adventurer %N"),
                    "unexpected say: {text}"
                );
            }
            other => panic!("expected Say, got {other:?}"),
        }

        // Second rule DEFAULT + bye → farewell
        match &dialogue.rules[1].actions[0] {
            DialogueAction::Say { text, .. } => {
                assert!(text.contains("Good bye"), "unexpected say: {text}");
            }
            other => panic!("expected Say, got {other:?}"),
        }
    }

    #[test]
    fn duplicate_name_fails_validation() {
        let items = ItemDatabase {
            items: Default::default(),
            client_to_server: Default::default(),
        };
        let mut runtime = LuaRuntime::new().expect("runtime");
        runtime
            .lua
            .globals()
            .set("_pending_npcs", runtime.lua.create_table().unwrap())
            .unwrap();
        runtime
            .lua
            .globals()
            .set(
                "_pending_npc_action_callbacks",
                runtime.lua.create_table().unwrap(),
            )
            .unwrap();
        runtime
            .lua
            .globals()
            .set(
                "_pending_npc_predicate_callbacks",
                runtime.lua.create_table().unwrap(),
            )
            .unwrap();

        runtime
            .lua
            .load(
                r#"
                NpcType("Dup"):register()
                NpcType("dup"):register()
                "#,
            )
            .exec()
            .expect("register");

        let err = runtime
            .drain_pending_npcs(Some(&items))
            .expect_err("duplicate");
        assert!(err.contains("duplicate"), "{err}");
    }

    #[test]
    fn unknown_item_id_in_create_fails() {
        let items = ItemDatabase {
            items: Default::default(),
            client_to_server: Default::default(),
        };
        let mut runtime = LuaRuntime::new().expect("runtime");
        runtime
            .lua
            .globals()
            .set("_pending_npcs", runtime.lua.create_table().unwrap())
            .unwrap();
        runtime
            .lua
            .globals()
            .set(
                "_pending_npc_action_callbacks",
                runtime.lua.create_table().unwrap(),
            )
            .unwrap();
        runtime
            .lua
            .globals()
            .set(
                "_pending_npc_predicate_callbacks",
                runtime.lua.create_table().unwrap(),
            )
            .unwrap();

        runtime
            .lua
            .load(
                r#"
                local npc = NpcType("Shopper")
                npc:dialogue(NpcDialogue({
                    rules = {
                        {
                            when = { { situation = "address" } },
                            actions = { { create = { item = 1, count = 1 } } }
                        }
                    }
                }))
                npc:register()
                "#,
            )
            .exec()
            .expect("register");

        let err = runtime
            .drain_pending_npcs(Some(&items))
            .expect_err("bad item");
        assert!(err.contains("unknown item"), "{err}");
    }
}
