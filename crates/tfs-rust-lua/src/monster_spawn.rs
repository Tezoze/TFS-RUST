//! Dispatch `Monster:onSpawn` / `EventCallback(EVENT_CALLBACK_ONSPAWN)` after loot.
//!
//! Pack: TFS `Events::eventMonsterOnSpawn` mutate path (`events.cpp`) via
//! `data/events/scripts/monster.lua`. Return value is ignored — spawn already happened.

use mlua::{Function, ObjectLike, Table, Value};

use crate::runtime::{LuaError, LuaRuntime};
use crate::userdata::monster::{MonsterRef, with_monster_spawn_inventory_scope};
use crate::userdata::position::PositionRef;

/// `EVENT_CALLBACK_ONSPAWN` in `data/scripts/lib/event_callbacks.lua`.
const EVENT_CALLBACK_ONSPAWN: i32 = 25;

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
        let globals = self.lua.globals();
        let has = match globals.get::<Function>("hasEventCallback") {
            Ok(func) => self.call_lua::<bool>(&func, EVENT_CALLBACK_ONSPAWN)?,
            Err(_) => false,
        };
        if !has {
            return Ok(());
        }

        with_monster_spawn_inventory_scope(creature, |token| {
            let monster = self.lua.create_userdata(MonsterRef { creature, token })?;
            let pos = self.lua.create_userdata(PositionRef { x, y, z })?;
            if let Ok(tbl) = globals.get::<Table>("Monster")
                && let Ok(func) = tbl.get::<Function>("onSpawn")
            {
                let _: Value =
                    self.call_lua(&func, (monster.clone(), pos.clone(), startup, artificial))?;
                return Ok(());
            }
            if let Ok(ec) = globals.get::<Table>("EventCallback") {
                let _: Value =
                    ec.call((EVENT_CALLBACK_ONSPAWN, monster, pos, startup, artificial))?;
            }
            Ok(())
        })
    }
}
