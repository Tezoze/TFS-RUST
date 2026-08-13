---
trigger: model_decision
description: Lua integration patterns using trait dispatch, mlua+LuaJIT, and scoped mutation appliers.
globs: ["crates/tfs-rust-core/**/*.rs", "crates/tfs-rust-lua/**/*.rs"]
---

# Lua Integration (Trait Dispatch + mlua + LuaJIT)

Lua integration uses trait dispatch to avoid circular dependencies between `tfs-rust-core` and `tfs-rust-lua`.

**Engine choice (mandatory):** mlua with **LuaJIT** (`Cargo.toml`: `features = ["luajit", "vendored"]`). Do **not** swap to Rhai or plain Lua 5.4 — TFS script parity requires LuaJIT + incremental port of `luascript.cpp`.

**Lua-facing contract (mandatory):** keep the TFS surface — global class tables, global constants, global helper functions, self-registering revscripts (`Action():register()`), `data/` layout. Do **not** replace it with a bespoke/namespaced API: the data pack is the **772 parity oracle**, and running reference scripts unmodified is how mechanics outcomes are verified. Improve it *within* the contract — `register_class` (below), fail-fast load phases (below), and generated LuaLS type definitions — never by changing what `data/` sees. Rationale and the revisit condition: `tasks/tools-actions/decisions.md` § *Strategic decision*.

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

## Class Registration (Mandatory)

Every engine class exposed to Lua (`Tile`, `Item`, `Creature`, `Combat`, `Position`, `ItemType`, `Party`, …) goes through the shared `register_class` helper. **Never** `globals().set("Tile", ctor_fn)` — a bare function is callable but not indexable, so `function Tile.relocateTo(…)` in `data/lib/core/*.lua` fails to load.

`registerClass` in `luascript.cpp` does **four** jobs. Partially implementing it is the root cause of a recurring bug class (see `tasks/tools-actions/gap7-class-globals.md`):

| # | Job | Skipping it breaks |
|---|-----|--------------------|
| 1 | Class table as the global | `function Tile.method(…)` — load-time error |
| 2 | `__call` metamethod → constructor | `Tile(pos)` stops working |
| 3 | Userdata `__index` fallback → class table | `tile:method()` — **call-time** `nil`, loads fine |
| 4 | Chain to base class | subclass userdata can't see base methods |

Jobs 1 and 3 are **independent**. A class table alone makes the data pack *load* while every Lua-defined method is still unreachable at call time — a green load test over a broken feature.

### Rules

- `register_class(lua, name, ctor)` is the **only** way a class global is created. Idempotent, get-or-create, never replaces an existing table — so registration order in `LuaRuntime::new` is not load-bearing.
- Each userdata type declares its `__index` chain (first hit wins, checked after native Rust methods):

  | Userdata | Chain |
  |---|---|
  | `CreatureRef` | `Player` → `Creature` |
  | `TileRef` | `Tile` |
  | `ItemRef` | `Item` |
  | `ItemTypeRef` | `ItemType` |
  | `PositionRef` | `Position` |
  | `CombatRef` | `Combat` |

- Native Rust methods keep priority: mlua invokes `MetaMethod::Index` only when the registered-method lookup misses, so a data-pack method cannot silently shadow an engine method. Assert this, don't assume it.
- Classes the data pack only extends (no constructor) still need registering: `register_class(lua, "Party", None)`.
- Do **not** maintain a hardcoded list of class names in a bootstrap function. A class exists because something registered it.

### Verification (non-negotiable)

Test through a **live userdata instance**, never a load test:

```rust
// Not sufficient: asserting the global is a table, or that the lib file loaded.
lua.load("function Tile.probe(self) return 'ok' end").exec()?;
let ud = lua.create_userdata(TileRef { x: 1, y: 2, z: 7 })?;
// This is the assertion that matters:
assert_eq!(lua.load("return t:probe()").eval::<String>()?, "ok");
```

## Script Loading Phases (Mandatory)

Three phases with **different error policies**. Do not collapse them:

| Phase | Scope | On error |
|-------|-------|----------|
| 1. Bootstrap | `register_class`, constants, enums (Rust) | **Fatal** — programming error |
| 2. Lib | `data/lib/**`, `data/scripts/lib/**`, `data/scripts/*.lua` | **Fatal, aggregated** — the data pack ships with this repo; a lib file that does not load is a build defect |
| 3. Content | `data/scripts/<subsystem>/**` revscripts | **Warn and continue** — a broken shard script must not brick the server |

Phase 2 being lenient hides real breakage: an allowlist of "required globals" does not scale to the data pack, so **the load itself is the guard**. Prefer loading `data/global.lua` and letting its `dofile` chain drive `data/lib/**` over hand-rolled substitutes (substring extraction, inlined Lua chunks, parallel directory scans).

Tests must construct the VM through the real init path (`LuaRuntime::new_for_test()`), not hand-assembled subsets — otherwise tests validate a VM that is never shipped.

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

## Script Execution Limits

Game simulation is single-threaded (`TFS-threading`), so an unbounded script is an **availability bug, not a script bug**: one `while true do end` in `data/scripts/**` hangs ticks, packets, logins, and saves until `kill -9`. No attacker required.

- `lua.set_memory_limit(bytes)` — turns a runaway allocation into `Error::MemoryError` instead of an OOM-killed process. No JIT cost; safe to enable unconditionally.
- `lua.set_hook(HookTriggers::new().every_nth_instruction(n), …)` — errors out a runaway script so the server keeps ticking. **Measure first:** LuaJIT does not call count hooks from compiled traces, so an always-on hook may force interpreter fallback and negate the reason for choosing LuaJIT.
- **Aborting a script does not roll back mutations.** Mutations apply immediately (Mutation Path above), so a killed script leaves partial effects — this is failure isolation, not atomicity. Document the semantic per callback.

Do not add new script entry points that can block the game thread indefinitely (unbounded loops over map/creature sets, blocking I/O in a callback).

## Full API Port Plan (luascript.cpp)

Port incrementally; community scripts need breadth before depth on hot paths:

1. **`data/lib/*.lua` metatables** — `Game`, `Player`, `Creature`, `Item`, `Tile`, `Position`, `Condition`. Finish this **uniformly** via `register_class` (see Class Registration) before porting more methods — per-class ad-hoc registration is what produced the Gap 7 bug class.
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
9. **Class registration:** `register_class` only — class table + `__call` + userdata `__index` chain + base chain. Never `globals().set(Name, ctor_fn)`
10. **Verify classes through a live userdata instance** — a load test passing proves nothing about `obj:method()`
11. **Load phases:** bootstrap fatal, lib fatal (aggregated), content warn-and-continue — never one uniform policy
12. **Execution limits:** memory limit always on; instruction hook measured against LuaJIT cost first. An unbounded script is an availability bug
