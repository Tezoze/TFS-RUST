//! Load `data/npc/scripts/**/*.lua` into an immutable [`NpcDatabase`].
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

use crate::context::CreatureRef;
use crate::npc_type::PendingNpc;
use crate::runtime::{LuaError, LuaRuntime};

impl LuaRuntime {
    /// Load NPC definition scripts from `data_dir/npc/scripts/**/*.lua`.
    ///
    /// Hard-fails on script exec errors and on validation errors after drain.
    /// Does not require `GameWorld`.
    pub fn load_npc_definitions(
        &mut self,
        data_dir: &Path,
        items: &ItemDatabase,
    ) -> Result<NpcDatabase, String> {
        let defs_dir = data_dir.join("npc/scripts");
        self.load_npc_definitions_dir(&defs_dir, items)
    }

    /// Load NPC definition scripts from an explicit scripts directory.
    ///
    /// Used by the offline `import-npcs` CLI to validate a temp tree before
    /// atomically replacing `data/npc/scripts`.
    pub fn load_npc_definitions_dir(
        &mut self,
        defs_dir: &Path,
        items: &ItemDatabase,
    ) -> Result<NpcDatabase, String> {
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
        self.lua
            .globals()
            .set(
                "_pending_npc_lifecycle_callbacks",
                self.lua.create_table().map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;

        let mut lua_files: Vec<PathBuf> = Vec::new();
        collect_lua_files(defs_dir, &mut lua_files);
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
        let life_cbs = self
            .lua
            .globals()
            .get::<mlua::Table>("_pending_npc_lifecycle_callbacks")
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

            // Resolve lifecycle callbacks.
            if let Ok(map) = life_cbs.get::<mlua::Table>(idx) {
                for pair in map.pairs::<String, mlua::Function>() {
                    let (name, func) = pair.map_err(|e| e.to_string())?;
                    let id = NpcCallbackId(next_callback_id);
                    next_callback_id += 1;
                    let reg_key = self
                        .lua
                        .create_registry_value(func)
                        .map_err(|e| e.to_string())?;
                    self.register_npc_callback(id, reg_key);
                    match name.as_str() {
                        "think" => def.on_think = Some(id),
                        "appear" => def.on_appear = Some(id),
                        "disappear" => def.on_disappear = Some(id),
                        "move" => def.on_move = Some(id),
                        "say" => def.on_say = Some(id),
                        other => {
                            return Err(format!(
                                "NPC {:?}: unknown lifecycle callback {other:?}",
                                def.name
                            ));
                        }
                    }
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
        let _ = self.lua.globals().set(
            "_pending_npc_lifecycle_callbacks",
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

    /// Invoke an NPC lifecycle/custom callback with `(NpcRef[, PlayerRef[, ...]])`.
    ///
    /// Returns `Ok(true)` if the callback returned true or nil (treated as success for
    /// actions); `Ok(false)` if it returned false. Errors are mapped to `Err`.
    pub fn call_npc_callback_npc_only(
        &self,
        id: tfs_rust_content::npcs::NpcCallbackId,
        npc: crate::context::CreatureId,
    ) -> Result<bool, LuaError> {
        let Some(key) = self.npc_callbacks.get(&id) else {
            return Ok(false);
        };
        let function: mlua::Function = self.lua.registry_value(key).map_err(LuaError::Init)?;
        let npc_ud = self
            .lua
            .create_userdata(crate::userdata::NpcRef(npc))
            .map_err(LuaError::Init)?;
        match function.call::<mlua::Value>(npc_ud) {
            Ok(mlua::Value::Boolean(b)) => Ok(b),
            Ok(mlua::Value::Nil) => Ok(true),
            Ok(_) => Ok(true),
            Err(e) => Err(LuaError::Init(e)),
        }
    }

    pub fn call_npc_callback_with_player(
        &self,
        id: tfs_rust_content::npcs::NpcCallbackId,
        npc: crate::context::CreatureId,
        player: crate::context::CreatureId,
    ) -> Result<bool, LuaError> {
        let Some(key) = self.npc_callbacks.get(&id) else {
            return Ok(false);
        };
        let function: mlua::Function = self.lua.registry_value(key).map_err(LuaError::Init)?;
        let npc_ud = self
            .lua
            .create_userdata(crate::userdata::NpcRef(npc))
            .map_err(LuaError::Init)?;
        let player_ud = self
            .lua
            .create_userdata(CreatureRef(player))
            .map_err(LuaError::Init)?;
        match function.call::<mlua::Value>((npc_ud, player_ud)) {
            Ok(mlua::Value::Boolean(b)) => Ok(b),
            Ok(mlua::Value::Nil) => Ok(true),
            Ok(_) => Ok(true),
            Err(e) => Err(LuaError::Init(e)),
        }
    }

    pub fn call_npc_callback_say(
        &self,
        id: tfs_rust_content::npcs::NpcCallbackId,
        npc: crate::context::CreatureId,
        speaker: crate::context::CreatureId,
        text: &str,
    ) -> Result<bool, LuaError> {
        let Some(key) = self.npc_callbacks.get(&id) else {
            return Ok(false);
        };
        let function: mlua::Function = self.lua.registry_value(key).map_err(LuaError::Init)?;
        let npc_ud = self
            .lua
            .create_userdata(crate::userdata::NpcRef(npc))
            .map_err(LuaError::Init)?;
        let speaker_ud = self
            .lua
            .create_userdata(CreatureRef(speaker))
            .map_err(LuaError::Init)?;
        match function.call::<mlua::Value>((npc_ud, speaker_ud, text)) {
            Ok(mlua::Value::Boolean(b)) => Ok(b),
            Ok(mlua::Value::Nil) => Ok(true),
            Ok(_) => Ok(true),
            Err(e) => Err(LuaError::Init(e)),
        }
    }

    pub fn call_npc_callback_think(
        &self,
        id: tfs_rust_content::npcs::NpcCallbackId,
        npc: crate::context::CreatureId,
        interval_ms: u32,
    ) -> Result<bool, LuaError> {
        let Some(key) = self.npc_callbacks.get(&id) else {
            return Ok(false);
        };
        let function: mlua::Function = self.lua.registry_value(key).map_err(LuaError::Init)?;
        let npc_ud = self
            .lua
            .create_userdata(crate::userdata::NpcRef(npc))
            .map_err(LuaError::Init)?;
        match function.call::<mlua::Value>((npc_ud, interval_ms)) {
            Ok(mlua::Value::Boolean(b)) => Ok(b),
            Ok(mlua::Value::Nil) => Ok(true),
            Ok(_) => Ok(true),
            Err(e) => Err(LuaError::Init(e)),
        }
    }

    pub fn call_npc_callback_move(
        &self,
        id: tfs_rust_content::npcs::NpcCallbackId,
        npc: crate::context::CreatureId,
        from: (u16, u16, u8),
        to: (u16, u16, u8),
    ) -> Result<bool, LuaError> {
        let Some(key) = self.npc_callbacks.get(&id) else {
            return Ok(false);
        };
        let function: mlua::Function = self.lua.registry_value(key).map_err(LuaError::Init)?;
        let npc_ud = self
            .lua
            .create_userdata(crate::userdata::NpcRef(npc))
            .map_err(LuaError::Init)?;
        let from_ud = self
            .lua
            .create_userdata(crate::userdata::PositionRef {
                x: from.0,
                y: from.1,
                z: from.2,
            })
            .map_err(LuaError::Init)?;
        let to_ud = self
            .lua
            .create_userdata(crate::userdata::PositionRef {
                x: to.0,
                y: to.1,
                z: to.2,
            })
            .map_err(LuaError::Init)?;
        match function.call::<mlua::Value>((npc_ud, from_ud, to_ud)) {
            Ok(mlua::Value::Boolean(b)) => Ok(b),
            Ok(mlua::Value::Nil) => Ok(true),
            Ok(_) => Ok(true),
            Err(e) => Err(LuaError::Init(e)),
        }
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

    /// NPC-2 gate: imported Quentin definition loads without GameWorld.
    #[test]
    fn quentin_definition_loads_stable_snapshot() {
        let data_root = workspace_data_root();
        let quentin = data_root.join("npc/scripts/quentin.lua");
        if !quentin.exists() {
            panic!("missing imported definition: {}", quentin.display());
        }

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
            .globals()
            .set(
                "_pending_npc_lifecycle_callbacks",
                runtime.lua.create_table().unwrap(),
            )
            .unwrap();

        let source = std::fs::read_to_string(&quentin).expect("read quentin");
        runtime
            .lua
            .load(&source)
            .set_name(quentin.to_str().unwrap_or("quentin.lua"))
            .exec()
            .expect("exec quentin");

        let db = runtime.drain_pending_npcs(None).expect("drain");
        let def = db.get_by_name("Quentin").expect("Quentin registered");
        assert_eq!(def.name, "Quentin");
        assert_eq!(def.appearance.look_type, 57);
        assert_eq!(def.movement.radius, 4);

        let dialogue = def.dialogue.as_ref().expect("dialogue");
        assert_eq!(
            dialogue.policy,
            tfs_rust_content::npcs::DialoguePolicy::QueuedSingleFocus
        );
        assert!(!dialogue.rules.is_empty());

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
                    text.to_ascii_lowercase().contains("welcome"),
                    "unexpected say: {text}"
                );
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
            .globals()
            .set(
                "_pending_npc_lifecycle_callbacks",
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
            .globals()
            .set(
                "_pending_npc_lifecycle_callbacks",
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

    #[test]
    fn import_emit_roundtrip_albert_via_lua() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../reference/cipsoft-772/runtime/npc");
        if !root.exists() {
            return;
        }
        let file = tfs_rust_content::npc_import::parse_npc_file(&root, &root.join("albert.npc"))
            .expect("parse");
        let pending = tfs_rust_content::npc_import::lower_npc(file).expect("lower");
        let rule_count = pending
            .dialogue
            .as_ref()
            .map(|d| d.rules.len())
            .unwrap_or(0);
        let lua_src = tfs_rust_content::npc_import::emit_npc_lua(&pending);

        let mut runtime = LuaRuntime::new().expect("rt");
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
            .globals()
            .set(
                "_pending_npc_lifecycle_callbacks",
                runtime.lua.create_table().unwrap(),
            )
            .unwrap();
        runtime
            .lua
            .load(&lua_src)
            .set_name("albert_import.lua")
            .exec()
            .expect("exec emitted lua");
        let db = runtime.drain_pending_npcs(None).expect("drain");
        let def = db.get_by_name("Albert").expect("albert");
        assert_eq!(
            def.dialogue.as_ref().map(|d| d.rules.len()),
            Some(rule_count)
        );
    }

    #[test]
    fn loads_custom_smoke_with_callbacks() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/npc/scripts/custom_smoke.lua");
        if !path.exists() {
            eprintln!("skip: custom_smoke.lua missing");
            return;
        }
        let mut runtime = LuaRuntime::new().expect("rt");
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
            .globals()
            .set(
                "_pending_npc_lifecycle_callbacks",
                runtime.lua.create_table().unwrap(),
            )
            .unwrap();
        let src = std::fs::read_to_string(&path).expect("read");
        runtime
            .lua
            .load(&src)
            .set_name("custom_smoke.lua")
            .exec()
            .expect("exec");
        let db = runtime.drain_pending_npcs(None).expect("drain");
        let def = db.get_by_name("CustomSmoke").expect("CustomSmoke");
        assert!(!def.custom_actions.is_empty());
        assert!(!def.custom_predicates.is_empty());
        assert!(runtime.npc_callbacks.len() >= 3);
    }

    #[test]
    fn loads_migrated_captain_and_banker() {
        for stem in ["captain.lua", "banker.lua"] {
            let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../data/npc/scripts")
                .join(stem);
            if !path.exists() {
                eprintln!("skip: {stem} missing");
                continue;
            }
            let mut runtime = LuaRuntime::new().expect("rt");
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
                .globals()
                .set(
                    "_pending_npc_lifecycle_callbacks",
                    runtime.lua.create_table().unwrap(),
                )
                .unwrap();
            let src = std::fs::read_to_string(&path).expect("read");
            runtime
                .lua
                .load(&src)
                .set_name(stem)
                .exec()
                .unwrap_or_else(|e| panic!("exec {stem}: {e}"));
            let db = runtime
                .drain_pending_npcs(None)
                .unwrap_or_else(|e| panic!("drain {stem}: {e}"));
            assert!(!db.is_empty(), "{stem} registered no NPC");
        }
    }

    #[test]
    fn npc_lib_does_not_require_npcsystem() {
        let lib = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/npc/lib/npc.lua");
        let src = std::fs::read_to_string(&lib).expect("npc.lua");
        assert!(
            !src.contains("npcsystem"),
            "npc.lua must not load KeywordHandler library"
        );
    }
}
