//! Lua `NpcType(name)` builder — pending registration into `_pending_npcs`.
//!
//! Domain: TFS-style `NpcType` content API; callbacks are opaque ids in definitions
//! while `mlua::RegistryKey`s stay on [`crate::runtime::LuaRuntime`].
//!
//! 772: no Lua NPC types — metadata + Behaviour in `.npc` files (imported later).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use mlua::{Lua, RegistryKey, UserData, UserDataMethods, Value};

use tfs_rust_content::npcs::{NpcVoice, PendingNpcDefinition};

use crate::npc_dialogue::NpcDialogueProgram;

/// Pending NPC registration snapshot (UserData for `_pending_npcs`).
#[derive(Debug, Clone, Default)]
pub struct PendingNpc {
    pub def: PendingNpcDefinition,
}

impl UserData for PendingNpc {}

/// Builder returned by `NpcType(name)`.
#[derive(Clone)]
pub struct NpcTypeBuilder {
    pub pending: Rc<RefCell<PendingNpc>>,
    /// Named custom action callbacks captured before `:register()`.
    pub action_fns: Rc<RefCell<HashMap<String, RegistryKey>>>,
    /// Named custom predicate callbacks.
    pub predicate_fns: Rc<RefCell<HashMap<String, RegistryKey>>>,
    /// Lifecycle callbacks: think/appear/disappear/move/say.
    pub lifecycle_fns: Rc<RefCell<HashMap<String, RegistryKey>>>,
}

/// Register `NpcType` constructor and metatable methods.
pub fn register_npc_type(lua: &Lua) -> Result<(), mlua::Error> {
    lua.register_userdata_type::<NpcTypeBuilder>(|_registry| {})?;
    lua.register_userdata_type::<PendingNpc>(|_registry| {})?;

    let ctor = lua.create_function(|lua, name: String| {
        if name.trim().is_empty() {
            return Err(mlua::Error::runtime("NpcType: name must not be empty"));
        }
        let file = current_file(lua);
        Ok(NpcTypeBuilder {
            pending: Rc::new(RefCell::new(PendingNpc {
                def: PendingNpcDefinition {
                    name,
                    source_file: file,
                    health_max: 100,
                    ..Default::default()
                },
            })),
            action_fns: Rc::new(RefCell::new(HashMap::new())),
            predicate_fns: Rc::new(RefCell::new(HashMap::new())),
            lifecycle_fns: Rc::new(RefCell::new(HashMap::new())),
        })
    })?;
    lua.globals().set("NpcType", ctor)?;
    Ok(())
}

fn current_file(lua: &Lua) -> String {
    let Ok(debug): Result<mlua::Table, _> = lua.globals().get("debug") else {
        return "<lua>".into();
    };
    let Ok(getinfo): Result<mlua::Function, _> = debug.get("getinfo") else {
        return "<lua>".into();
    };
    let Ok(info): Result<Value, _> = getinfo.call((2i32, "S")) else {
        return "<lua>".into();
    };
    if let Value::Table(t) = info
        && let Ok(source) = t.get::<String>("source")
    {
        let trimmed = source.trim_start_matches('@');
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    "<lua>".into()
}

impl UserData for NpcTypeBuilder {
    fn register(registry: &mut mlua::UserDataRegistry<Self>) {
        crate::class_registry::register_with_recording(registry, "NpcType");
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("appearance", |_, this, table: mlua::Table| {
            let mut def = this.pending.borrow_mut();
            let a = &mut def.def.appearance;
            if let Ok(v) = table.get::<u16>("lookType") {
                a.look_type = v;
            }
            if let Ok(v) = table.get::<u8>("lookHead") {
                a.look_head = v;
            }
            if let Ok(v) = table.get::<u8>("lookBody") {
                a.look_body = v;
            }
            if let Ok(v) = table.get::<u8>("lookLegs") {
                a.look_legs = v;
            }
            if let Ok(v) = table.get::<u8>("lookFeet") {
                a.look_feet = v;
            }
            if let Ok(v) = table.get::<u8>("lookAddons") {
                a.look_addons = v;
            }
            if let Ok(v) = table.get::<u16>("lookTypeEx") {
                a.look_type_ex = v;
            }
            if let Ok(v) = table.get::<u16>("lookMount") {
                a.look_mount = v;
            }
            Ok(true)
        });

        methods.add_method_mut("movement", |_, this, table: mlua::Table| {
            let mut def = this.pending.borrow_mut();
            let m = &mut def.def.movement;
            if let Ok(v) = table.get::<u16>("radius") {
                m.radius = v;
            }
            if let Ok(v) = table.get::<u16>("speed") {
                m.speed = v;
            }
            Ok(true)
        });

        methods.add_method_mut("health", |_, this, max: u32| {
            this.pending.borrow_mut().def.health_max = max;
            Ok(true)
        });

        methods.add_method_mut("speechBubble", |_, this, bubble: u8| {
            this.pending.borrow_mut().def.speech_bubble = bubble;
            Ok(true)
        });

        methods.add_method_mut("sex", |_, this, sex: u8| {
            this.pending.borrow_mut().def.sex = sex;
            Ok(true)
        });

        methods.add_method_mut("race", |_, this, race: u16| {
            this.pending.borrow_mut().def.race = race;
            Ok(true)
        });

        methods.add_method_mut("parameter", |_, this, (key, value): (String, String)| {
            this.pending.borrow_mut().def.parameters.insert(key, value);
            Ok(true)
        });

        methods.add_method_mut("voice", |_, this, table: mlua::Table| {
            let text: String = table.get("text")?;
            let interval_ms: u32 = table.get("interval").unwrap_or(0);
            let chance: u32 = table.get("chance").unwrap_or(0);
            this.pending.borrow_mut().def.voices.push(NpcVoice {
                text,
                interval_ms,
                chance,
            });
            Ok(true)
        });

        methods.add_method_mut("dialogue", |_, this, prog: mlua::AnyUserData| {
            let program = prog.borrow::<NpcDialogueProgram>()?.0.clone();
            this.pending.borrow_mut().def.dialogue = Some(program);
            Ok(true)
        });

        methods.add_method_mut(
            "onCustomAction",
            |lua, this, (name, func): (String, mlua::Function)| {
                if name.trim().is_empty() {
                    return Err(mlua::Error::runtime(
                        "onCustomAction: name must not be empty",
                    ));
                }
                let key = lua.create_registry_value(func)?;
                this.action_fns.borrow_mut().insert(name, key);
                Ok(true)
            },
        );

        methods.add_method_mut(
            "onCustomPredicate",
            |lua, this, (name, func): (String, mlua::Function)| {
                if name.trim().is_empty() {
                    return Err(mlua::Error::runtime(
                        "onCustomPredicate: name must not be empty",
                    ));
                }
                let key = lua.create_registry_value(func)?;
                this.predicate_fns.borrow_mut().insert(name, key);
                Ok(true)
            },
        );

        methods.add_method_mut("onThink", |lua, this, func: mlua::Function| {
            let key = lua.create_registry_value(func)?;
            this.lifecycle_fns.borrow_mut().insert("think".into(), key);
            Ok(true)
        });
        methods.add_method_mut("onAppear", |lua, this, func: mlua::Function| {
            let key = lua.create_registry_value(func)?;
            this.lifecycle_fns.borrow_mut().insert("appear".into(), key);
            Ok(true)
        });
        methods.add_method_mut("onDisappear", |lua, this, func: mlua::Function| {
            let key = lua.create_registry_value(func)?;
            this.lifecycle_fns
                .borrow_mut()
                .insert("disappear".into(), key);
            Ok(true)
        });
        methods.add_method_mut("onMove", |lua, this, func: mlua::Function| {
            let key = lua.create_registry_value(func)?;
            this.lifecycle_fns.borrow_mut().insert("move".into(), key);
            Ok(true)
        });
        methods.add_method_mut("onSay", |lua, this, func: mlua::Function| {
            let key = lua.create_registry_value(func)?;
            this.lifecycle_fns.borrow_mut().insert("say".into(), key);
            Ok(true)
        });

        methods.add_method("register", |lua, this, ()| {
            let globals = lua.globals();
            let pending: mlua::Table = globals.get("_pending_npcs").map_err(|_| {
                mlua::Error::runtime(
                    "NpcType:register: _pending_npcs missing (call load_npc_definitions)",
                )
            })?;
            let idx = pending.len()? + 1;

            let snapshot = this.pending.borrow().clone();
            pending.set(idx, snapshot)?;

            // Parallel callback tables: idx → { name → function }
            let action_cbs: mlua::Table = globals
                .get("_pending_npc_action_callbacks")
                .map_err(|_| mlua::Error::runtime("_pending_npc_action_callbacks missing"))?;
            let pred_cbs: mlua::Table = globals
                .get("_pending_npc_predicate_callbacks")
                .map_err(|_| mlua::Error::runtime("_pending_npc_predicate_callbacks missing"))?;
            let life_cbs: mlua::Table = globals
                .get("_pending_npc_lifecycle_callbacks")
                .map_err(|_| mlua::Error::runtime("_pending_npc_lifecycle_callbacks missing"))?;

            let actions_map = lua.create_table()?;
            for (name, key) in this.action_fns.borrow_mut().drain() {
                let func: mlua::Function = lua.registry_value(&key)?;
                actions_map.set(name, func)?;
            }
            action_cbs.set(idx, actions_map)?;

            let preds_map = lua.create_table()?;
            for (name, key) in this.predicate_fns.borrow_mut().drain() {
                let func: mlua::Function = lua.registry_value(&key)?;
                preds_map.set(name, func)?;
            }
            pred_cbs.set(idx, preds_map)?;

            let life_map = lua.create_table()?;
            for (name, key) in this.lifecycle_fns.borrow_mut().drain() {
                let func: mlua::Function = lua.registry_value(&key)?;
                life_map.set(name, func)?;
            }
            life_cbs.set(idx, life_map)?;

            Ok(true)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::npc_dialogue::register_npc_dialogue;

    fn setup() -> Lua {
        let lua = Lua::new();
        register_npc_dialogue(&lua).unwrap();
        register_npc_type(&lua).unwrap();
        lua.globals()
            .set("_pending_npcs", lua.create_table().unwrap())
            .unwrap();
        lua.globals()
            .set("_pending_npc_action_callbacks", lua.create_table().unwrap())
            .unwrap();
        lua.globals()
            .set(
                "_pending_npc_predicate_callbacks",
                lua.create_table().unwrap(),
            )
            .unwrap();
        lua.globals()
            .set(
                "_pending_npc_lifecycle_callbacks",
                lua.create_table().unwrap(),
            )
            .unwrap();
        lua
    }

    #[test]
    fn ctor_and_fluent_setters() {
        let lua = setup();
        let ud: mlua::AnyUserData = lua
            .load(
                r#"
                local npc = NpcType("Quentin")
                npc:appearance({ lookType = 57 })
                npc:movement({ radius = 4, speed = 10 })
                npc:health(150)
                return npc
                "#,
            )
            .eval()
            .unwrap();
        let b = ud.borrow::<NpcTypeBuilder>().unwrap();
        let d = &b.pending.borrow().def;
        assert_eq!(d.name, "Quentin");
        assert_eq!(d.appearance.look_type, 57);
        assert_eq!(d.movement.radius, 4);
        assert_eq!(d.movement.speed, 10);
        assert_eq!(d.health_max, 150);
    }

    #[test]
    fn register_pushes_pending() {
        let lua = setup();
        lua.load(
            r#"
            local npc = NpcType("Guard")
            npc:dialogue(NpcDialogue({
                policy = "queued_single_focus",
                rules = {
                    {
                        when = { { situation = "address" }, { words = { "hi$" } } },
                        actions = { { say = "Halt!" } }
                    }
                }
            }))
            npc:register()
            "#,
        )
        .exec()
        .unwrap();
        let pending: mlua::Table = lua.globals().get("_pending_npcs").unwrap();
        assert_eq!(pending.len().unwrap(), 1);
        let ud: mlua::AnyUserData = pending.get(1).unwrap();
        let p = ud.borrow::<PendingNpc>().unwrap();
        assert_eq!(p.def.name, "Guard");
        assert!(p.def.dialogue.is_some());
    }
}
