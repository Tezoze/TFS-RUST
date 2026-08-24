//! Drain `_pending_global_events` and dispatch revscript GlobalEvents.
//!
//! Pack surface: TFS `GlobalEvents::registerLuaEvent` / `startup` / `execute(SHUTDOWN)`
//! / `Game::checkPlayersRecord` (`globalevent.cpp`, `game.cpp`).
//! Reload stance (a): the name map is replaced wholesale on each scripts-interface scan.
//!
//! Dispatched types: startup, shutdown, record.
//! `:time` / `:interval` / `onTime` / `onThink` are **not** dispatched — daily save is engine.

use std::collections::HashMap;

use mlua::{Function, Table, Value};

use crate::runtime::{CallbackRef, LuaError, LuaRuntime};

/// GlobalEvent kinds this phase actually fires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobalEventKind {
    Startup,
    Shutdown,
    Record,
}

pub struct RegisteredGlobalEvent {
    pub kind: GlobalEventKind,
    pub callback: CallbackRef,
}

impl LuaRuntime {
    pub fn has_global_event(&self, name: &str) -> bool {
        self.global_events.borrow().contains_key(name)
    }

    /// Drain `_pending_global_events` into a replaceable name map.
    ///
    /// C++: `GlobalEvents::registerLuaEvent` overwrites by name.
    pub fn install_pending_global_events(&self) -> Result<(), LuaError> {
        let pending: Table = self.lua.globals().get("_pending_global_events")?;
        let mut map = HashMap::new();
        let len = pending.len()?;
        for i in 1..=len {
            let table: Table = pending.get(i)?;
            let name: String = table.get("name")?;
            match classify_global_event(&table, &name)? {
                Classify::Dispatch(kind, field) => {
                    let func: Function = table.get(field)?;
                    let key = self.lua.create_registry_value(func)?;
                    map.insert(
                        name,
                        RegisteredGlobalEvent {
                            kind,
                            callback: CallbackRef::from_registry_key(key),
                        },
                    );
                }
                Classify::TimerUnsupported => {
                    tracing::warn!(
                        name = %name,
                        "GlobalEvent :time/:interval/onTime/onThink has no dispatch site; save is engine"
                    );
                }
                Classify::Unknown => {
                    tracing::warn!(
                        name = %name,
                        "GlobalEvent has no onStartup/onShutdown/onRecord (and is not a timer)"
                    );
                }
            }
        }

        *self.global_events.borrow_mut() = map;
        Ok(())
    }

    pub fn fire_global_startup(&self) -> Result<(), LuaError> {
        self.fire_global_kind(GlobalEventKind::Startup, None)
    }

    pub fn fire_global_shutdown(&self) -> Result<(), LuaError> {
        self.fire_global_kind(GlobalEventKind::Shutdown, None)
    }

    pub fn fire_global_record(&self, current: u32, old: u32) -> Result<(), LuaError> {
        self.fire_global_kind(GlobalEventKind::Record, Some((current, old)))
    }

    fn fire_global_kind(
        &self,
        kind: GlobalEventKind,
        record: Option<(u32, u32)>,
    ) -> Result<(), LuaError> {
        let names: Vec<String> = self
            .global_events
            .borrow()
            .iter()
            .filter(|(_, ev)| ev.kind == kind)
            .map(|(name, _)| name.clone())
            .collect();
        for name in names {
            let callback = {
                let map = self.global_events.borrow();
                let Some(ev) = map.get(&name) else {
                    continue;
                };
                // Registry key lives as long as the map; clone the Function first.
                let function: Function = self
                    .lua
                    .registry_value(ev.callback.registry_key())
                    .map_err(LuaError::Init)?;
                function
            };
            let value: Value = match record {
                Some((current, old)) => self.call_lua(&callback, (current, old))?,
                None => self.call_lua(&callback, ())?,
            };
            let _ = value;
        }
        Ok(())
    }
}

enum Classify {
    Dispatch(GlobalEventKind, &'static str),
    TimerUnsupported,
    Unknown,
}

fn classify_global_event(table: &Table, _name: &str) -> Result<Classify, LuaError> {
    let type_name: Option<String> = table.get("_type")?;
    let time: Option<String> = table.get("_time")?;
    let interval: Option<u32> = table.get("_interval")?;
    let on_time: Option<Function> = table.get("onTime")?;
    let on_think: Option<Function> = table.get("onThink")?;
    let on_startup: Option<Function> = table.get("onStartup")?;
    let on_shutdown: Option<Function> = table.get("onShutdown")?;
    let on_record: Option<Function> = table.get("onRecord")?;

    if time.is_some() || interval.is_some() || on_time.is_some() || on_think.is_some() {
        return Ok(Classify::TimerUnsupported);
    }

    let lowered = type_name
        .as_deref()
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    if lowered == "startup" || on_startup.is_some() {
        return Ok(Classify::Dispatch(GlobalEventKind::Startup, "onStartup"));
    }
    if lowered == "shutdown" || on_shutdown.is_some() {
        return Ok(Classify::Dispatch(GlobalEventKind::Shutdown, "onShutdown"));
    }
    if lowered == "record" || on_record.is_some() {
        return Ok(Classify::Dispatch(GlobalEventKind::Record, "onRecord"));
    }
    Ok(Classify::Unknown)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runtime_with(src: &str) -> LuaRuntime {
        let runtime = LuaRuntime::new().expect("runtime");
        runtime.exec_chunk("ge_test", src).expect("exec");
        runtime.install_pending_global_events().expect("install");
        runtime
    }

    #[test]
    fn drain_registers_record_type() {
        let runtime = runtime_with(
            r#"
            local e = GlobalEvent("PlayerRecord")
            function e.onRecord(current, old) return true end
            e:type("record")
            e:register()
            "#,
        );
        assert!(runtime.has_global_event("PlayerRecord"));
    }

    #[test]
    fn timer_type_is_not_registered() {
        let runtime = runtime_with(
            r#"
            local e = GlobalEvent("ServerSave")
            function e.onTime(interval) return true end
            e:time("04:30:00")
            e:register()
            "#,
        );
        assert!(!runtime.has_global_event("ServerSave"));
    }

    #[test]
    fn drain_is_name_keyed_and_replaceable() {
        let runtime = runtime_with(
            r#"
            local e = GlobalEvent("Alpha")
            function e.onStartup() return true end
            e:type("startup")
            e:register()
            "#,
        );
        assert!(runtime.has_global_event("Alpha"));

        runtime.reset_pending_script_event_tables().expect("reset");
        runtime
            .exec_chunk(
                "ge_replace",
                r#"
                local e = GlobalEvent("Beta")
                function e.onStartup() return true end
                e:type("startup")
                e:register()
                "#,
            )
            .expect("exec");
        runtime.install_pending_global_events().expect("reinstall");
        assert!(!runtime.has_global_event("Alpha"));
        assert!(runtime.has_global_event("Beta"));
    }

    #[test]
    fn game_get_players_returns_online_ids() {
        use crate::context::with_lua_context;
        use tfs_rust_common::{
            ScriptContext, ScriptCreatureData, ScriptCreatureId, ScriptItemId, ScriptItemRef,
        };

        struct PlayersCtx;
        impl ScriptContext for PlayersCtx {
            fn get_creature(&self, _: ScriptCreatureId) -> Option<ScriptCreatureData> {
                Some(ScriptCreatureData {
                    name: "P".into(),
                    guid: 1,
                })
            }
            fn get_item(&self, _: ScriptItemId) -> Option<ScriptItemRef> {
                None
            }
            fn get_config_string(&self, _: &str) -> Option<String> {
                None
            }
            fn online_player_ids(&self) -> Vec<ScriptCreatureId> {
                vec![1, 2, 3]
            }
        }

        let runtime = LuaRuntime::new().expect("runtime");
        with_lua_context(&PlayersCtx, || {
            runtime
                .exec_chunk("get_players", "COUNT = #Game.getPlayers()")
                .expect("exec");
        });
        let count: i64 = runtime.lua.globals().get("COUNT").expect("COUNT");
        assert_eq!(count, 3);
    }
}
