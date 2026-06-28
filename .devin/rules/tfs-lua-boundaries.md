---
inclusion: conditional
name: tfs-lua-boundaries
description: Lua integration patterns using trait dispatch, mlua+LuaJIT, and scoped mutation appliers. Applies to core and lua crates.
globs: ["crates/tfs-rust-core/**/*.rs", "crates/tfs-rust-lua/**/*.rs"]
---

# Lua Integration (Trait Dispatch + mlua + LuaJIT)

Lua integration uses trait dispatch to avoid circular dependencies between `tfs-rust-core` and `tfs-rust-lua`.

**Engine choice (mandatory):** mlua with **LuaJIT** (`Cargo.toml`: `features = ["luajit", "vendored"]`). Do **not** swap to Rhai or plain Lua 5.4 — TFS script parity requires LuaJIT + incremental port of `luascript.cpp`.

**Threading:** `LuaRuntime` is `!Send` and lives on the **game thread only** (`LocalSet` + `spawn_local`). I/O threads never touch Lua or `GameWorld`.

## Architecture Constraint

**Problem:**
- `tfs-rust-core` needs to call Lua scripts (events, hooks)
- `tfs-rust-lua` needs to resolve entities and apply mutations
- Circular dependency: `core` → `lua` → `core` ❌

**Solution:**
- `tfs-rust-common` defines `ScriptContext` (read trait) — **no lua dependency**
- `core` defines `EventDispatcher` using `&dyn ScriptContext` — **no lua import in event trait**
- `tfs-rust-lua` re-exports `ScriptContext as LuaContext` + mlua userdata bindings
- `lua` does **not** depend on `core` — one-way: `core` → `lua` at wiring only

## Dependency Graph (Mandatory)

```
tfs-rust-common  ← ScriptContext, ScriptCreatureData, …
       ↑                    ↑
       │                    │
tfs-rust-core ────────────► tfs-rust-lua
  EventDispatcher            mlua bindings, with_lua_context
  GameWorld impl ScriptContext
```

**Never:** `tfs-rust-lua` → `tfs-rust-core` (would create a cycle).

## Trait Definition (in `tfs-rust-core`)

```rust
// crates/tfs-rust-core/src/event_dispatcher.rs
use tfs_rust_common::ScriptContext;

pub trait EventDispatcher {
    fn on_login(&self, creature: CreatureId, ctx: &dyn ScriptContext) {}
    fn on_logout(&self, creature: CreatureId, ctx: &dyn ScriptContext) {}
    // ...
}
```

**Critical:** `event_dispatcher.rs` must **not** import `tfs-rust-lua`.

## Read Path — `ScriptContext` + `with_lua_context`

Read-only userdata methods resolve IDs through `tfs_rust_common::ScriptContext`:

```rust
// crates/tfs-rust-common/src/script_context.rs
pub trait ScriptContext {
    fn get_creature(&self, id: ScriptCreatureId) -> Option<ScriptCreatureData>;
    fn get_item(&self, id: ScriptItemId) -> Option<ScriptItemRef>;
    // ...
}

// crates/tfs-rust-lua/src/context.rs — re-exports ScriptContext as LuaContext
pub fn with_lua_context<F, R>(ctx: &dyn LuaContext, f: F) -> R { /* CURRENT_CTX */ }
```

- Userdata stores **typed IDs only** (`CreatureRef(u64)`, `ItemRef(u64)`) — never references or pointers to Rust entities
- `GameWorld` implements `ScriptContext` in `tfs-rust-core`

## Mutation Path — `LuaMutation` + Immediate Apply (Mandatory for TFS Parity)

**Rule:** If C++ applies the mutation before the Lua call returns, Rust must too. Scripts often read world state in the **same callback** after a mutation (`addItem`, `teleport`, `setMaxHealth`, etc.).

```rust
// crates/tfs-rust-lua/src/lua_mutation.rs
pub enum LuaMutation {
    PlayerAddItem { creature_id: u64, item_type: u16, count: u16 },
    PlayerRemoveItem { creature_id: u64, item_type: u16, count: u32 },
    // extend as luascript.cpp methods are ported
}

pub fn call_lua_add_item(...) -> Result<(), String> {
    apply_mutation(...)  // synchronous — NOT queued to end of tick
}
```

```rust
// crates/tfs-rust-core/src/lua_scope.rs — ONLY place for re-entrant &mut GameWorld unsafe
pub fn fire_on_login(world: &mut GameWorld, cid: CreatureId) {
    with_lua_mutation_scope(/* world ptr */, || {
        with_lua_context(world, || { world.events.on_login(cid, world); });
    });
}
```

**When adding events:** add matching `fire_on_*` helpers in `lua_scope.rs` (`fire_on_logout`, `fire_on_death`, `fire_on_think`, …). Do **not** scatter raw world pointers or cookie patterns at call sites.

**Do NOT defer these to a tick-end buffer** — that breaks scripts like:
```lua
if player:addItem(2160, 100) then
    -- expects backpack state updated HERE
end
```

### Deferred mutations (tick-end) — only when safe

`LuaCommand` in `lua_command.rs` may be used for mutations that:
- C++ also defers, OR
- scripts never read back in the same callback, OR
- only affect outbound packets (client visibility), not Lua-visible game state

**Network flush deferral is separate.** Batching `flush_output_buffers` to tick end (except walk/login/disconnect) does **not** change what Lua reads from `GameWorld` during a callback.

## Userdata Pattern

```rust
// crates/tfs-rust-lua/src/userdata/player.rs
impl UserData for CreatureRef {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("getName", |_, this, ()| {
            // read via CURRENT_CTX + LuaContext
        });
        methods.add_method("addItem", |_, this, (ty, count): (u16, Option<u16>)| {
            call_lua_add_item(this.0, ty, count.unwrap_or(1))  // immediate mutation
                .map_err(|e| mlua::Error::runtime(e))
        });
    }
}
```

- Reads: `CURRENT_CTX` + `LuaContext` trait
- Mutations: `call_lua_*` → `LuaMutation` → `apply_lua_mutation` in `lua_scope.rs`
- Never pass `&mut GameWorld` into mlua closures

## Startup Wiring

```rust
// run_server.rs
register_lua_mutation_hooks();  // once at startup — registers apply_lua_mutation

// login.rs
fire_on_login(world, cid);    // not manual cookie / hook setup
```

## Error Handling

Lua errors must not crash the server — log and continue (see `LuaEventDispatcher`).

```rust
match self.runtime.call_creature_callback(callback, creature_id) {
    Ok(true) => {}
    Ok(false) => tracing::warn!("Lua callback returned false"),
    Err(e) => tracing::error!("Lua callback failed: {}", e),
}
```

## Full API Port Plan (luascript.cpp)

Port incrementally; community scripts need breadth before depth on hot paths:

1. **`data/lib/*.lua` metatables** — `Game`, `Player`, `Creature`, `Item`, `Tile`, `Position`, `Condition`
2. **Creature events** — think, death, preparedeath, advance (not just login/logout)
3. **Move events, talk actions, globalevents, actions**
4. **`addEvent` / `stopEvent`** — wire to `Scheduler` + unbounded `GameCommand` channel
5. **Combat / hot callbacks** — port last; **profile callback volume before optimizing**

Each new `luascript.cpp` method: classify as **read** (`ScriptContext`) or **mutation** (`LuaMutation` + immediate apply if script-visible).

## Performance at 2000+ Players

Architecture is correct; Lua time on the game thread is the ceiling:

- LuaJIT via mlua (already configured) — not the bottleneck vs plain 5.4, but mlua FFI per call adds cost
- Cache `CallbackRef` / registry keys for hot creaturescripts (already done for login/logout)
- Spread `lua_gc_step` across ticks
- Batch spectator/map updates in Rust, not Lua
- Skip or gate Lua dispatch for events most servers never register
- **Profile before micro-optimizing** — measure callbacks per tick under load first

## Testing Without Lua

```rust
let mut world = GameWorld {
    events: Box::new(NullEventDispatcher),
    // ...
};
// No Lua runtime required for core tests
```

## Summary (Mandatory)

1. **mlua + LuaJIT** — full TFS API port target; no Rhai bridge
2. **Game thread only** — `!Send` LuaRuntime on `LocalSet`
3. **Reads:** `ScriptContext` (common) + `with_lua_context` (lua crate)
4. **Mutations scripts observe mid-callback:** `LuaMutation` + **immediate** `apply_lua_mutation`
5. **Scoped dispatch:** `fire_on_*` in `lua_scope.rs` — all re-entrant unsafe confined there
6. **New events:** extend `fire_on_*`, not new cookie/hook patterns
7. **Deferred tick buffer:** only for mutations safe to delay; never for addItem-class APIs
8. **Profile hot paths** before optimizing Lua at scale
