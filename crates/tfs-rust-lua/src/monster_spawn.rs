//! Dispatch `EventCallback(EVENT_CALLBACK_ONSPAWN)` after loot.
//!
//! Pack: TFS `Events::eventMonsterOnSpawn` mutate path (`events.cpp`).
//! `Monster:onSpawn` forwarder lived in deleted `data/events/scripts/monster.lua`;
//! Rust calls the EventCallback bus directly. Return value is ignored — spawn
//! already happened.

use mlua::{ObjectLike, Table, Value};

use crate::event_callback::EVENT_CALLBACK_ONSPAWN;
use crate::runtime::{LuaError, LuaRuntime};
use crate::userdata::monster::{MonsterRef, with_monster_spawn_inventory_scope};
use crate::userdata::position::PositionRef;

impl LuaRuntime {
    /// Call registered `onSpawn` callbacks with a spawn-scoped [`MonsterRef`].
    ///
    /// No-op when `hasEventCallback(ONSPAWN)` is false (rarity file not enabled).
    pub fn call_monster_on_spawned(
        &self,
        creature: u64,
        x: u16,
        y: u16,
        z: u8,
        startup: bool,
        artificial: bool,
    ) -> Result<(), LuaError> {
        if !self.has_event_callback(EVENT_CALLBACK_ONSPAWN) {
            return Ok(());
        }

        let globals = self.lua.globals();
        with_monster_spawn_inventory_scope(creature, |token| {
            let monster = self.lua.create_userdata(MonsterRef { creature, token })?;
            let pos = self.lua.create_userdata(PositionRef { x, y, z })?;
            if let Ok(ec) = globals.get::<Table>("EventCallback") {
                let _: Value =
                    ec.call((EVENT_CALLBACK_ONSPAWN, monster, pos, startup, artificial))?;
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::load_data_lib;
    use crate::event_callback::EVENT_CALLBACK_ONMOVEITEM;
    use std::path::PathBuf;

    fn workspace_data_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
    }

    #[test]
    fn spawn_noop_when_unregistered() {
        let data_root = workspace_data_root();
        if !data_root.join("scripts/lib/event_callbacks.lua").exists() {
            eprintln!("data pack not present — skipping");
            return;
        }
        let runtime = LuaRuntime::new().expect("runtime");
        load_data_lib(&runtime, &data_root).expect("data lib");
        runtime
            .call_monster_on_spawned(1, 100, 100, 7, false, false)
            .expect("onSpawn optional when data/events is gone");
    }

    /// Unregistered onSpawn must not invoke `EventCallback` (early return).
    #[test]
    fn spawn_does_not_call_lua_when_unregistered() {
        let data_root = workspace_data_root();
        if !data_root.join("scripts/lib/event_callbacks.lua").exists() {
            eprintln!("data pack not present — skipping");
            return;
        }
        let runtime = LuaRuntime::new().expect("runtime");
        load_data_lib(&runtime, &data_root).expect("data lib");
        assert!(
            !runtime.has_event_callback(EVENT_CALLBACK_ONSPAWN),
            "lib load must not register onSpawn"
        );
        // Call site exists, but no registration — dispatcher must not Lua-call EventCallback.
        assert!(
            !runtime.has_event_callback(EVENT_CALLBACK_ONMOVEITEM),
            "lib load only: ONMOVEITEM (16) stays unregistered without scripts interface"
        );
        runtime
            .exec_chunk(
                "spawn_sentinel",
                "SPAWN_FIRED = 0\n\
                 local mt = getmetatable(EventCallback)\n\
                 local old = mt.__call\n\
                 mt.__call = function(...)\n\
                   SPAWN_FIRED = SPAWN_FIRED + 1\n\
                   return old(...)\n\
                 end\n",
            )
            .expect("wrap EventCallback __call");
        runtime
            .call_monster_on_spawned(1, 100, 100, 7, false, false)
            .expect("unregistered onSpawn is a no-op");
        let fired: i32 = runtime
            .lua
            .load("return SPAWN_FIRED")
            .eval()
            .expect("SPAWN_FIRED");
        assert_eq!(fired, 0, "early return must not call EventCallback");
    }
}
