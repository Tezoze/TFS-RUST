//! Drain `_pending_creature_events` and dispatch revscript CreatureEvents by name.
//!
//! Pack surface: TFS `CreatureEvents::registerLuaEvent` / `playerLogin` / `playerLogout`
//! / `Creature::onDeath` / `Creature::onKill` (`creatureevent.cpp`, `creature.cpp`).
//! Reload stance (a): the name map is replaced wholesale on each scripts-interface scan.

use std::collections::HashMap;

use mlua::{Function, Table, Value};

use crate::context::CreatureRef;
use crate::runtime::{CallbackRef, LuaError, LuaRuntime};

/// Event kinds this phase dispatches. Other TFS types (think, prepareDeath, …) are ignored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureEventKind {
    Login,
    Logout,
    Death,
    Kill,
}

/// Names that must never enter the registry (Phase 2.4).
/// `login.lua` still calls `registerEvent("DropLoot")` — warn-and-ignore.
const BLOCKED_CREATURE_EVENT_NAMES: &[&str] = &["DropLoot", "RegenerateStamina"];

pub fn is_blocked_creature_event_name(name: &str) -> bool {
    BLOCKED_CREATURE_EVENT_NAMES
        .iter()
        .any(|blocked| *blocked == name)
}

pub struct RegisteredCreatureEvent {
    pub kind: CreatureEventKind,
    pub callback: CallbackRef,
}

impl LuaRuntime {
    pub fn has_creature_event(&self, name: &str) -> bool {
        self.creature_events.borrow().contains_key(name)
    }

    /// Drain `_pending_creature_events` into a replaceable name map.
    ///
    /// C++: `CreatureEvents::registerLuaEvent` overwrites by name.
    pub fn install_pending_creature_events(&self) -> Result<(), LuaError> {
        let pending: Table = self.lua.globals().get("_pending_creature_events")?;
        let mut map = HashMap::new();
        let len = pending.len()?;
        for i in 1..=len {
            let table: Table = pending.get(i)?;
            let name: String = table.get("name")?;
            if is_blocked_creature_event_name(&name) {
                tracing::warn!(
                    name = %name,
                    "CreatureEvent not registrable (native death/loot owns this hook)"
                );
                continue;
            }
            let Some(kind) = infer_creature_event_kind(&table)? else {
                tracing::warn!(
                    name = %name,
                    "CreatureEvent has no onLogin/onLogout/onDeath/onKill"
                );
                continue;
            };
            let field = kind.lua_field();
            let func: Function = table.get(field)?;
            let key = self.lua.create_registry_value(func)?;
            map.insert(
                name,
                RegisteredCreatureEvent {
                    kind,
                    callback: CallbackRef::from_registry_key(key),
                },
            );
        }

        let names = self.lua.create_table()?;
        for name in map.keys() {
            names.set(name.as_str(), true)?;
        }
        self.lua.globals().set("_creature_event_registry", names)?;
        *self.creature_events.borrow_mut() = map;
        Ok(())
    }

    /// Every registered event of `kind` — TFS `playerLogin` / `playerLogout` iterate the global map.
    pub fn fire_creature_events_of_kind(
        &self,
        kind: CreatureEventKind,
        creature: crate::context::CreatureId,
    ) -> Result<bool, LuaError> {
        let names: Vec<String> = self
            .creature_events
            .borrow()
            .iter()
            .filter(|(_, ev)| ev.kind == kind)
            .map(|(name, _)| name.clone())
            .collect();
        let mut allow = true;
        for name in names {
            if !self.invoke_creature_event(&name, kind, creature, None)? {
                allow = false;
            }
        }
        Ok(allow)
    }

    /// Fire named events of `kind` that the player registered (`Player:registerEvent`).
    pub fn fire_registered_creature_events(
        &self,
        kind: CreatureEventKind,
        creature: crate::context::CreatureId,
        other: Option<crate::context::CreatureId>,
        names: &[String],
    ) -> Result<(), LuaError> {
        for name in names {
            let matches = self
                .creature_events
                .borrow()
                .get(name)
                .is_some_and(|ev| ev.kind == kind);
            if !matches {
                continue;
            }
            let _ = self.invoke_creature_event(name, kind, creature, other)?;
        }
        Ok(())
    }

    fn invoke_creature_event(
        &self,
        name: &str,
        kind: CreatureEventKind,
        creature: crate::context::CreatureId,
        other: Option<crate::context::CreatureId>,
    ) -> Result<bool, LuaError> {
        let map = self.creature_events.borrow();
        let Some(ev) = map.get(name) else {
            return Ok(true);
        };
        if ev.kind != kind {
            return Ok(true);
        }
        self.call_creature_event_callback(&ev.callback, kind, creature, other)
    }

    fn call_creature_event_callback(
        &self,
        callback: &CallbackRef,
        kind: CreatureEventKind,
        creature: crate::context::CreatureId,
        other: Option<crate::context::CreatureId>,
    ) -> Result<bool, LuaError> {
        let function: Function = self
            .lua
            .registry_value(callback.registry_key())
            .map_err(LuaError::Init)?;
        let actor = self
            .lua
            .create_userdata(CreatureRef(creature))
            .map_err(LuaError::Init)?;
        let value: Value = match kind {
            CreatureEventKind::Login | CreatureEventKind::Logout => {
                self.call_lua(&function, actor)?
            }
            CreatureEventKind::Kill => {
                let target = match other {
                    Some(id) => Value::UserData(
                        self.lua
                            .create_userdata(CreatureRef(id))
                            .map_err(LuaError::Init)?,
                    ),
                    None => Value::Nil,
                };
                self.call_lua(&function, (actor, target))?
            }
            CreatureEventKind::Death => {
                let killer = match other {
                    Some(id) => Value::UserData(
                        self.lua
                            .create_userdata(CreatureRef(id))
                            .map_err(LuaError::Init)?,
                    ),
                    None => Value::Nil,
                };
                // TFS `executeOnDeath(creature, corpse, killer, mostDamage, unjust, mostUnjust)`.
                self.call_lua(
                    &function,
                    (actor, Value::Nil, killer, Value::Nil, false, false),
                )?
            }
        };
        Ok(lua_truthy(value))
    }
}

fn lua_truthy(value: Value) -> bool {
    match value {
        Value::Nil => true,
        Value::Boolean(b) => b,
        _ => true,
    }
}

fn infer_creature_event_kind(table: &Table) -> Result<Option<CreatureEventKind>, LuaError> {
    if has_function(table, "onLogin")? {
        return Ok(Some(CreatureEventKind::Login));
    }
    if has_function(table, "onLogout")? {
        return Ok(Some(CreatureEventKind::Logout));
    }
    if has_function(table, "onDeath")? {
        return Ok(Some(CreatureEventKind::Death));
    }
    if has_function(table, "onKill")? {
        return Ok(Some(CreatureEventKind::Kill));
    }
    let type_name: Option<String> = match table.get::<Value>("_type")? {
        Value::String(s) => Some(s.to_str()?.to_owned()),
        _ => None,
    };
    Ok(type_name.as_deref().and_then(kind_from_type_name))
}

fn has_function(table: &Table, key: &str) -> Result<bool, LuaError> {
    match table.get::<Value>(key)? {
        Value::Function(_) => Ok(true),
        _ => Ok(false),
    }
}

fn kind_from_type_name(name: &str) -> Option<CreatureEventKind> {
    match name.to_ascii_lowercase().as_str() {
        "login" => Some(CreatureEventKind::Login),
        "logout" => Some(CreatureEventKind::Logout),
        "death" => Some(CreatureEventKind::Death),
        "kill" => Some(CreatureEventKind::Kill),
        _ => None,
    }
}

impl CreatureEventKind {
    fn lua_field(self) -> &'static str {
        match self {
            Self::Login => "onLogin",
            Self::Logout => "onLogout",
            Self::Death => "onDeath",
            Self::Kill => "onKill",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runtime_with_event(src: &str) -> LuaRuntime {
        let runtime = LuaRuntime::new().expect("runtime");
        runtime.exec_chunk("ce_test", src).expect("exec");
        runtime.install_pending_creature_events().expect("install");
        runtime
    }

    #[test]
    fn drain_is_name_keyed_and_replaceable() {
        let runtime = runtime_with_event(
            r#"
            local e = CreatureEvent("Alpha")
            function e.onLogin(player) return true end
            e:register()
            "#,
        );
        assert!(runtime.has_creature_event("Alpha"));

        runtime
            .reset_pending_script_event_tables()
            .expect("reset pending");
        runtime
            .exec_chunk(
                "ce_replace",
                r#"
                local e = CreatureEvent("Beta")
                function e.onLogin(player) return true end
                e:register()
                "#,
            )
            .expect("exec replace");
        runtime
            .install_pending_creature_events()
            .expect("install replace");
        assert!(!runtime.has_creature_event("Alpha"));
        assert!(runtime.has_creature_event("Beta"));
    }

    #[test]
    fn droploot_and_regeneratestamina_are_not_registered() {
        let runtime = runtime_with_event(
            r#"
            local drop = CreatureEvent("DropLoot")
            function drop.onDeath(player) return true end
            drop:register()
            local stam = CreatureEvent("RegenerateStamina")
            function stam.onLogin(player) return true end
            stam:register()
            local ok = CreatureEvent("PlayerDeath")
            function ok.onDeath(player) return true end
            ok:register()
            "#,
        );
        assert!(!runtime.has_creature_event("DropLoot"));
        assert!(!runtime.has_creature_event("RegenerateStamina"));
        assert!(runtime.has_creature_event("PlayerDeath"));
    }

    #[test]
    fn login_kind_invokes_callback() {
        let runtime = runtime_with_event(
            r#"
            LOGIN_FIRED = 0
            local e = CreatureEvent("PlayerLogin")
            function e.onLogin(player)
                LOGIN_FIRED = LOGIN_FIRED + 1
                return true
            end
            e:register()
            "#,
        );
        runtime
            .fire_creature_events_of_kind(CreatureEventKind::Login, 1)
            .expect("fire login");
        let fired: i32 = runtime
            .lua
            .globals()
            .get("LOGIN_FIRED")
            .expect("LOGIN_FIRED");
        assert_eq!(fired, 1);
    }

    #[test]
    fn logout_false_propagates() {
        let runtime = runtime_with_event(
            r#"
            local e = CreatureEvent("PlayerLogout")
            function e.onLogout(player) return false end
            e:register()
            "#,
        );
        let allow = runtime
            .fire_creature_events_of_kind(CreatureEventKind::Logout, 1)
            .expect("fire logout");
        assert!(!allow);
    }

    #[test]
    fn death_only_fires_registered_names() {
        let runtime = runtime_with_event(
            r#"
            DEATH_FIRED = 0
            local e = CreatureEvent("PlayerDeath")
            function e.onDeath(player)
                DEATH_FIRED = DEATH_FIRED + 1
            end
            e:register()
            "#,
        );
        runtime
            .fire_registered_creature_events(
                CreatureEventKind::Death,
                1,
                None,
                &["NotRegistered".into()],
            )
            .expect("fire none");
        let fired: i32 = runtime.lua.globals().get("DEATH_FIRED").expect("counter");
        assert_eq!(fired, 0);
        runtime
            .fire_registered_creature_events(
                CreatureEventKind::Death,
                1,
                None,
                &["PlayerDeath".into()],
            )
            .expect("fire death");
        let fired: i32 = runtime.lua.globals().get("DEATH_FIRED").expect("counter");
        assert_eq!(fired, 1);
    }
}
