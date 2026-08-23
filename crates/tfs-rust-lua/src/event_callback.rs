//! Shared EventCallback bus helpers (`hasEventCallback` / undispatched warn).
//!
//! Pack: `data/scripts/lib/event_callbacks.lua`. Call sites live in focused
//! modules (`monster_spawn`, `player_move_item`, `player_report_bug`).

use mlua::Function;

use crate::runtime::LuaRuntime;

/// `EVENT_CALLBACK_ONMOVEITEM` in `event_callbacks.lua`.
pub const EVENT_CALLBACK_ONMOVEITEM: i32 = 16;
/// `EVENT_CALLBACK_ONITEMMOVED`.
pub const EVENT_CALLBACK_ONITEMMOVED: i32 = 17;
/// `EVENT_CALLBACK_ONREPORTBUG`.
pub const EVENT_CALLBACK_ONREPORTBUG: i32 = 19;
/// `EVENT_CALLBACK_ONSPAWN`.
pub const EVENT_CALLBACK_ONSPAWN: i32 = 25;
/// `EVENT_CALLBACK_LAST`.
pub const EVENT_CALLBACK_LAST: i32 = 25;

/// Callback types that have a Rust dispatch site after Phase 5.
const DISPATCHED_EVENT_CALLBACKS: &[i32] = &[
    EVENT_CALLBACK_ONMOVEITEM,
    EVENT_CALLBACK_ONITEMMOVED,
    EVENT_CALLBACK_ONREPORTBUG,
    EVENT_CALLBACK_ONSPAWN,
];

impl LuaRuntime {
    /// `hasEventCallback(type)` — false when the global is missing or the bus is empty.
    pub fn has_event_callback(&self, callback_type: i32) -> bool {
        let globals = self.lua.globals();
        let Ok(func) = globals.get::<Function>("hasEventCallback") else {
            return false;
        };
        self.call_lua::<bool>(&func, callback_type).unwrap_or(false)
    }

    /// Registered callback types that have no Rust call site (boot warn / tests).
    pub fn undispatched_event_callbacks(&self) -> Vec<i32> {
        (1..=EVENT_CALLBACK_LAST)
            .filter(|&ty| {
                self.has_event_callback(ty) && !DISPATCHED_EVENT_CALLBACKS.contains(&ty)
            })
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
}
