//! EventCallback bus — Rust-side registry synced from `EventCallbackData`.
//!
//! Pack: `data/scripts/lib/event_callbacks.lua`. Call sites live in focused
//! modules (`monster_spawn`, `player_move_item`, `player_report_bug`).

use mlua::{Function, Lua, MultiValue, Table, Value};
use rustc_hash::FxHashSet;
use std::collections::HashMap;

use crate::runtime::{CallbackRef, LuaError, LuaRuntime};

/// `EVENT_CALLBACK_ONAREACOMBAT` in `event_callbacks.lua`.
const EVENT_CALLBACK_ONAREACOMBAT: i32 = 2;
/// `EVENT_CALLBACK_ONTARGETCOMBAT`.
const EVENT_CALLBACK_ONTARGETCOMBAT: i32 = 3;
/// `EVENT_CALLBACK_ONMOVEITEM`.
pub const EVENT_CALLBACK_ONMOVEITEM: i32 = 16;
/// `EVENT_CALLBACK_ONITEMMOVED`.
pub const EVENT_CALLBACK_ONITEMMOVED: i32 = 17;
/// `EVENT_CALLBACK_ONREPORTBUG`.
pub const EVENT_CALLBACK_ONREPORTBUG: i32 = 19;
/// `EVENT_CALLBACK_ONSPAWN`.
pub const EVENT_CALLBACK_ONSPAWN: i32 = 25;
/// `EVENT_CALLBACK_LAST`.
pub const EVENT_CALLBACK_LAST: i32 = 25;

const RETURNVALUE_NOERROR: i32 = 0;

/// Callback types that have a Rust dispatch site after Phase 5.
const DISPATCHED_EVENT_CALLBACKS: &[i32] = &[
    EVENT_CALLBACK_ONMOVEITEM,
    EVENT_CALLBACK_ONITEMMOVED,
    EVENT_CALLBACK_ONREPORTBUG,
    EVENT_CALLBACK_ONSPAWN,
];

/// `auxargs` from `event_callbacks.lua` — arg slot ← ret slot (1-based Lua indices).
fn auxargs_for_type(callback_type: i32) -> &'static [(u8, u8)] {
    match callback_type {
        9 => &[(5, 1)],
        10 => &[(4, 1)],
        11 => &[(5, 1)],
        21 => &[(3, 1)],
        22 => &[(2, 1)],
        23 => &[(3, 1)],
        _ => &[],
    }
}

/// Rust-side mirror of `EventCallbackData` — `(priority, RegistryKey)` per type.
#[derive(Default)]
pub(crate) struct EventCallbackRegistry {
    registered_types: FxHashSet<i32>,
    by_type: HashMap<i32, Vec<(i32, CallbackRef)>>,
}

impl EventCallbackRegistry {
    fn clear(&mut self) {
        self.registered_types.clear();
        self.by_type.clear();
    }

    fn set_type(&mut self, callback_type: i32, entries: Vec<(i32, CallbackRef)>) {
        if entries.is_empty() {
            self.registered_types.remove(&callback_type);
            self.by_type.remove(&callback_type);
        } else {
            self.registered_types.insert(callback_type);
            self.by_type.insert(callback_type, entries);
        }
    }

    fn has(&self, callback_type: i32) -> bool {
        self.registered_types.contains(&callback_type)
    }

    fn registered_type_count(&self) -> usize {
        self.registered_types.len()
    }

    fn callback_count(&self, callback_type: i32) -> usize {
        self.by_type
            .get(&callback_type)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    fn callback_at(&self, callback_type: i32, index: usize) -> Option<&CallbackRef> {
        self.by_type
            .get(&callback_type)
            .and_then(|v| v.get(index))
            .map(|(_, cb)| cb)
    }
}

impl LuaRuntime {
    /// Drain `EventCallbackData` into the Rust registry after scripts-interface load.
    ///
    /// Pack: `EventCallback:register` populates Lua; Rust mirrors for direct dispatch.
    pub fn sync_event_callbacks_from_lua(&self) -> Result<(), LuaError> {
        let globals = self.lua.globals();
        let Ok(data): Result<Table, _> = globals.get("EventCallbackData") else {
            self.event_callbacks.borrow_mut().clear();
            return Ok(());
        };

        let mut registry = self.event_callbacks.borrow_mut();
        registry.clear();

        for callback_type in 1..=EVENT_CALLBACK_LAST {
            let Ok(event_table): Result<Table, _> = data.get(callback_type) else {
                continue;
            };
            let len = event_table.len().unwrap_or(0);
            let mut entries = Vec::with_capacity(len as usize);
            for i in 1..=len {
                let Ok(entry): Result<Table, _> = event_table.get(i) else {
                    continue;
                };
                let Ok(func): Result<Function, _> = entry.get(1) else {
                    continue;
                };
                let priority: i32 = entry.get(2).unwrap_or(0);
                let key = self.lua.create_registry_value(func)?;
                entries.push((priority, CallbackRef::from_registry_key(key)));
            }
            entries.sort_by_key(|(p, _)| *p);
            registry.set_type(callback_type, entries);
        }
        Ok(())
    }

    /// True when the Rust registry has at least one callback for `callback_type`.
    pub fn has_event_callback(&self, callback_type: i32) -> bool {
        self.event_callbacks.borrow().has(callback_type)
    }

    /// Count of EventCallback types with at least one registered handler (boot metrics).
    pub fn event_callback_registered_type_count(&self) -> usize {
        self.event_callbacks.borrow().registered_type_count()
    }

    /// Registered callback types that have no Rust call site (boot warn / tests).
    pub fn undispatched_event_callbacks(&self) -> Vec<i32> {
        (1..=EVENT_CALLBACK_LAST)
            .filter(|&ty| self.has_event_callback(ty) && !DISPATCHED_EVENT_CALLBACKS.contains(&ty))
            .collect()
    }

    /// Warn when a callback is registered for a type with no dispatcher.
    pub fn warn_undispatched_event_callbacks(&self) {
        for ty in self.undispatched_event_callbacks() {
            tracing::warn!(
                callback_type = ty,
                "EventCallback registered with no Rust call site"
            );
        }
    }

    /// Invoke registered callbacks with the same chain-stop rules as `EventCallback` `__call`.
    ///
    /// Returns `None` when nothing is registered; otherwise the last callback's return pack.
    pub(crate) fn dispatch_event_callbacks<F>(
        &self,
        callback_type: i32,
        build_args: F,
    ) -> Result<Option<MultiValue>, LuaError>
    where
        F: FnOnce(&Lua) -> Result<MultiValue, LuaError>,
    {
        let events = self.event_callbacks.borrow().callback_count(callback_type);
        if events == 0 {
            return Ok(None);
        }

        let mut args = build_args(&self.lua)?;
        let mut last_ret = MultiValue::new();

        for idx in 0..events {
            let func: Function = {
                let reg = self.event_callbacks.borrow();
                let cb = reg
                    .callback_at(callback_type, idx)
                    .expect("callback index in range");
                self.lua.registry_value(cb.registry_key())?
            };
            last_ret = func.call(args.clone())?;

            let is_last = idx + 1 == events;
            if is_last || chain_should_stop(callback_type, first_return_value(&last_ret)) {
                return Ok(Some(last_ret));
            }

            apply_auxargs(callback_type, &mut args, &last_ret);
        }
        Ok(Some(last_ret))
    }
}

fn first_return_value(ret: &MultiValue) -> Option<Value> {
    ret.iter().next().cloned()
}

fn chain_should_stop(callback_type: i32, first: Option<Value>) -> bool {
    let Some(v) = first else {
        return false;
    };
    if matches!(v, Value::Boolean(false)) {
        return true;
    }
    if callback_type == EVENT_CALLBACK_ONAREACOMBAT
        || callback_type == EVENT_CALLBACK_ONTARGETCOMBAT
    {
        return value_to_i32(v) != RETURNVALUE_NOERROR;
    }
    false
}

fn value_to_i32(v: Value) -> i32 {
    match v {
        Value::Integer(n) => i32::try_from(n).unwrap_or(0),
        Value::Number(n) => n as i32,
        Value::Boolean(true) => 1,
        Value::Boolean(false) => 0,
        _ => 0,
    }
}

fn apply_auxargs(callback_type: i32, args: &mut MultiValue, ret: &MultiValue) {
    let mapping = auxargs_for_type(callback_type);
    if mapping.is_empty() {
        return;
    }
    let mut arg_vec: Vec<Value> = args.clone().into_iter().collect();
    for &(arg_slot, ret_slot) in mapping {
        let arg_idx = (arg_slot as usize).saturating_sub(1);
        let ret_idx = (ret_slot as usize).saturating_sub(1);
        if let Some(val) = ret.iter().nth(ret_idx) {
            if arg_idx >= arg_vec.len() {
                arg_vec.resize(arg_idx + 1, Value::Nil);
            }
            arg_vec[arg_idx] = val.clone();
        }
    }
    *args = MultiValue::from_iter(arg_vec);
}

pub(crate) fn multivalue_to_return_int(ret: MultiValue) -> i32 {
    first_return_value(&ret).map(value_to_i32).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::load_data_lib;
    use std::path::PathBuf;

    fn workspace_data_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
    }

    #[test]
    fn undispatched_warn_lists_onlook_not_onreportbug() {
        let data_root = workspace_data_root();
        if !data_root.join("scripts/lib/event_callbacks.lua").exists() {
            eprintln!("data pack not present — skipping");
            return;
        }

        let runtime = LuaRuntime::new().expect("runtime");
        load_data_lib(&runtime, &data_root).expect("data lib");
        runtime
            .exec_chunk(
                "inject_onlook",
                "EventCallbackData[9][#EventCallbackData[9] + 1] = {function() end, 0}",
            )
            .expect("inject onLook");
        runtime
            .sync_event_callbacks_from_lua()
            .expect("sync after inject");
        let undispatched = runtime.undispatched_event_callbacks();
        assert!(
            undispatched.contains(&9),
            "onLook has no call site: {undispatched:?}"
        );
        assert!(
            !undispatched.contains(&EVENT_CALLBACK_ONSPAWN),
            "unregistered spawn must not warn"
        );
    }

    #[test]
    fn has_event_callback_uses_rust_registry_only() {
        let data_root = workspace_data_root();
        if !data_root.join("scripts/lib/event_callbacks.lua").exists() {
            eprintln!("data pack not present — skipping");
            return;
        }

        let runtime = LuaRuntime::new().expect("runtime");
        load_data_lib(&runtime, &data_root).expect("data lib");
        assert!(!runtime.has_event_callback(EVENT_CALLBACK_ONSPAWN));

        runtime
            .exec_chunk(
                "inject_spawn_lua_only",
                "EventCallbackData[25][#EventCallbackData[25] + 1] = {function() end, 0}",
            )
            .expect("inject onSpawn into Lua only");
        assert!(
            !runtime.has_event_callback(EVENT_CALLBACK_ONSPAWN),
            "has_event_callback must not consult Lua EventCallbackData"
        );

        runtime
            .sync_event_callbacks_from_lua()
            .expect("sync");
        assert!(runtime.has_event_callback(EVENT_CALLBACK_ONSPAWN));
    }
}
