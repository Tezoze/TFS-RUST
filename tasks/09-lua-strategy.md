# Lua Engine Strategy — Skeleton Now, Bindings Co-Developed

## Decision

Build the `tfs-rust-lua` crate infrastructure **now** (early Phase C), then
add Lua bindings **incrementally alongside each completed game system phase**.
Full Phase J work (J.2–J.7) is deferred until the underlying systems are stable.

---

## Why Not Defer to Phase J Entirely

TFS is fundamentally Lua-driven. Spells, actions, creature scripts, NPC
dialogue, move events, and global events are all Lua callbacks. Deferring Lua
entirely to Phase J means:

- **Phase H (Spells)** produces fake hardcoded spells with zero script
  compatibility — every TFS spell is a `.lua` file (e.g. `exura` calls
  `doCombat()` via `data/spells/scripts/`)
- **Phase D (Monster AI)** lacks the `onThink` / `onCreatureAppear` hooks that
  drive monster behaviour in `data/monster/scripts/`
- **Phase I (NPC Trade)** has no dialogue system without NPC Lua callbacks
- **Phase J** becomes a massive "retrofit everything" session against frozen APIs
  instead of incremental binding

**This violates the Compatibility Mandate** — TFS spell/script data files must
load and execute against real bindings, not stubs.

---

## Track 1 — Infrastructure (Build Now, ~1–2 days)

These are stable regardless of which game systems exist. Complete before Phase D.

| Task | C++ Ref | Description |
|------|---------|-------------|
| **J.1** | `luascript.cpp` init | `mlua` VM init, register global functions, load `data/lib/` |
| **J.4** | `baseevents.cpp` | Script loading pipeline — scan `data/scripts/`, `data/creaturescripts/`, `data/actions/` |
| **J.5** | `events.cpp` | Wire `EventDispatcher` trait — replace `NullEventDispatcher` |
| Proof-of-concept | `creaturescripts.cpp` | `onLogin` / `onLogout` hooks working end-to-end |

**Goal:** A player logging in triggers a real `onLogin` Lua callback from
`data/creaturescripts/scripts/`. This validates the full architecture.

---

## Track 2 — Bindings Co-Developed Per Phase

Add Lua API surface **as each system stabilises**, not before. The binding
pattern (metatable registration) is established in Track 1; these tasks just
fill in the methods.

### After Phase C (Inventory & Equipment)
- `Player:getItem(slot)`, `Player:addItem()`, `Player:removeItem()`
- `Item` metatable — `getId()`, `getCount()`, `getWeight()`, `setAttribute()`
- `Player:getCapacity()`, `Player:getFreeCapacity()`

### After Phase D (Monster/NPC Walk + AI)
- `Creature` metatable — `getPosition()`, `setPosition()`, `getHealth()`, `say()`
- `Monster` metatable — `getTarget()`, `selectTarget()`
- `onThink`, `onCreatureAppear`, `onCreatureDisappear` creature event hooks

### After Phase E (Combat)
- `doCombat()`, `doTargetCombat()`, `doAreaCombat()`
- `Combat` userdata — `setParameter()`, `setCallback()`, `execute()`
- `doSendMagicEffect()`, `doSendDistanceEffect()`

### After Phase F (Chat)
- `TalkAction` event binding — `onSay` handler
- `doPlayerSendMessage()`, `sendChannelMessage()`

### After Phase G (Conditions)
- `Condition` userdata — create, set ticks, set value
- `addCondition()`, `removeCondition()`, `hasCondition()`

### After Phase H (Spells)
- `Spell` metatable — `getName()`, `getManaCost()`, `isEnabled()`
- Full `data/spells/` script loading validated against live server

### After Phase I (NPC Trade)
- NPC Lua scripts — `onCreatureAppear`, `onCreatureSay`, keyword/dialogue callbacks
- `selfSay()`, `selfTurn()`, `selfMove()` NPC shorthand functions

---

## What Phase J Becomes

With Track 2 complete, Phase J is no longer a 5–8 day effort. Remaining work:

- **J.2** — Fill any missing `Game` / `Tile` / `Position` metatable methods
- **J.6** — Extended opcode handler for OTClient custom packets via Lua
- **J.7** — NPC dialogue system polish and keyword tree loading
- Integration testing: run full `data/` scripts against live server, fix gaps

Estimated residual effort: **2–3 days** instead of 5–8.

---

## Architecture Notes

### `LuaScriptInterface` pattern (C++ → Rust mapping)

| C++ | Rust equivalent |
|-----|-----------------|
| `LuaScriptInterface` singleton | `LuaRuntime` struct, owned by `GameWorld` |
| `lua_State*` | `mlua::Lua` (owns the VM) |
| `lua_pushstring` / `lua_getfield` | `mlua::Table`, `mlua::Function`, `lua.globals()` |
| `ScriptEnvironment` (per-call context) | `LuaEnv` struct passed via `mlua::UserData` |
| Virtual `configureEvent()` in baseevents | Trait method on `EventHandler` |

### Binding Pattern (establish in J.1, reuse for every system)

```rust
// Pattern: register a type as a Lua userdata with methods
impl mlua::UserData for PlayerRef {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("getPosition", |_, this, ()| {
            // resolve PlayerRef → &Player via GameWorld handle
            Ok(LuaPosition::from(this.position()))
        });
    }
}
```

All game object references passed to Lua use **ID handles** (not raw pointers or
`Arc`) — consistent with the `SlotMap<CreatureId, CreatureKind>` architecture.

### Event Dispatcher Replacement

`NullEventDispatcher` is replaced by `LuaEventDispatcher` which holds a
reference to the `LuaRuntime`. The `EventDispatcher` trait remains unchanged —
this is a pure drop-in.

---

## C++ Reference Files

| File | Purpose |
|------|---------|
| `src/luascript.cpp` | Full Lua API — 8000+ lines, the primary reference |
| `src/luascript.h` | All registered function signatures |
| `src/baseevents.cpp` | Script loading, event registration base |
| `src/creaturescripts.cpp` | `CreatureEvent` type — `onLogin`, `onDeath`, etc. |
| `src/actions.cpp` | `Action` event — item use scripts |
| `src/talkaction.cpp` | `TalkAction` event — chat command scripts |
| `src/globalevent.cpp` | `GlobalEvent` — server startup/shutdown/timer |
| `src/movement.cpp` | `MoveEvent` — step-in/step-out/equip scripts |
| `src/spells.cpp` | Spell script loading and execution |
| `src/npc.cpp` | NPC Lua state, dialogue system |
