# Architecture — Lua API + loading system

Index: [README.md](README.md) · the class-global blocker this investigation found: [gap7-class-globals.md](gap7-class-globals.md)

## TVP revscript loading model (investigation 2026-08-10)

Investigated how TVP (`reference/tvp-772/gameserver/src/`) loads Lua scripts to verify our `load_data_lib` matches the engine's design.

### TVP load sequence

Source: `otserv.cpp:239-257` + `scriptmanager.cpp:45-110` + `script.cpp:24-83`.

```
1. C++:  loadFile("data/global.lua")                    ← only hardcoded C++ entry point
   └─ Lua: dofile('data/lib/lib.lua')                   ← inside global.lua:2
        └─ Lua: dofile('data/lib/core/core.lua')        ← inside lib.lua:2
             └─ Lua: dofile('data/lib/core/storages.lua')  ← inside core.lua
             └─ Lua: dofile('data/lib/core/achievements.lua')
             └─ ... (14 more dofiles — explicit list in core.lua)
        └─ Lua: dofile('data/lib/compat/compat.lua')    ← inside lib.lua:5
        └─ Lua: dofile('data/lib/debugging/dump.lua')   ← inside lib.lua:8-9
        └─ Lua: dofile('data/lib/debugging/lua_version.lua')

2. C++:  loadScripts("scripts/lib", true, false)        ← recursive scan, sorted
   └─ data/scripts/lib/**/*.lua (create_functions, event_callbacks, etc.)

3. C++:  XML systems load (weapons, spells, actions, etc.) — XML only, no Lua

4. C++:  loadScripts("scripts", false, false)           ← recursive scan, sorted
   └─ data/scripts/**/*.lua (skips lib/ and events/ subdirs)
       └─ functions.lua, scarab_tiles.lua, actions/**, spells/**, movements/**, etc.

5. C++:  loadScripts("monster", false, false)           ← recursive scan, sorted
   └─ data/monster/**/*.lua
```

**Key findings:**
- **Only `data/global.lua` is hardcoded in C++** (`scriptmanager.cpp:47`). Everything else is recursive directory scans (`script.cpp:24-83`) or Lua-side `dofile` chains.
- **The lib chain (`data/lib/**`) is entirely Lua-driven** via `dofile` from `global.lua`. The scripts stage (`data/scripts/**`, `data/monster/**`) is C++-driven recursive scan.
- **`Scripts::loadScripts`** (`script.cpp:24-83`): recursive `boost::filesystem` iterator, skips `lib/` subdir (when `isLib=false`), skips `events/` subdir always, skips files starting with `#`, sorts `PathBuf`, loads each via `scriptInterface.loadFile`. No per-file logic — pure filesystem walk.
- **TVP does NOT have separate per-subsystem Lua loaders.** One single recursive scan of `data/scripts/**` picks up `functions.lua`, `scarab_tiles.lua`, AND all revscripts in `actions/`, `spells/`, `movements/`, `weapons/`, `talkactions/`, `creaturescripts/`, `chatchannels/`, `eventcallbacks/`. The `PendingAction` / `PendingSpell` / etc. drains happen after that one pass.

### Our current architecture (deviation from TVP)

We have separate per-subsystem loaders: `load_action_scripts` (scans `actions/`), `load_weapon_scripts` (scans `weapons/`), `load_spell_scripts` (scans `spells/`), `load_move_event_scripts` (scans `movements/`), etc. Each is a separate recursive scan of its own subdir, with per-subsystem pending drains after each.

`load_data_lib` handles the lib stage + top-level `data/scripts/*.lua` (the files no per-subsystem loader covers). It uses recursive scans matching TVP's `loadScripts` — no hardcoded file lists.

### `dofile` and `os.time` availability

**Both work in our mlua LuaJIT VM by default.** The comment at `runtime.rs:1294` ("`dofile`/`os.time` dependencies not yet wired") is **stale** — verified by probe test:

- `os.time()` returns epoch seconds (mlua LuaJIT exposes `os` stdlib by default).
- `dofile(path)` resolves relative to process CWD (same as TVP). When CWD is the repo root, `dofile('data/global.lua')` → `dofile('data/lib/lib.lua')` → `dofile('data/lib/core/core.lua')` → all core files resolve correctly.

## Target architecture — Lua API + loading system

The gaps in [gaps-load.md](gaps-load.md), [gaps-lua-api.md](gaps-lua-api.md) and [gap7-class-globals.md](gap7-class-globals.md) are symptoms. This is the end state they should converge on; each gap is a step toward it, and no gap should be implemented in a way that moves away from it.

### Principle: one owner per concern

| Concern | Single owner | Today |
|---|---|---|
| Class globals (`Tile`, `Combat`, …) | `register_class` (Gap 7a+7c) ✅ | 3 competing mechanisms, order-dependent |
| Userdata → class-table method lookup | shared `__index` chain helper (Gap 7b) ✅ | one hardcoded `"Player"` fallback; every other userdata had none |
| Lib-stage load + error policy | `load_data_lib`, **fatal** | warn-and-continue, hides 9 failures |
| Content-stage load + error policy | per-subsystem loaders, **warn** | same as lib — no distinction |
| Test VM construction | `LuaRuntime::new_for_test()` | 4 hand-assembled copies |

### Three phases with distinct error policies

The current `run_server.rs` sequence is a flat list of calls whose ordering constraints are implicit. Make the phases explicit, because **their error handling genuinely differs**:

```
Phase 1 — Bootstrap (Rust)      register_class for every engine class; constants; enums
                                 → any failure is fatal (programming error)
Phase 2 — Lib (data pack)        data/lib/**, data/scripts/lib/**, data/scripts/*.lua
                                 → FATAL, aggregated: the data pack ships with this repo,
                                   a lib file that does not load is a boot-blocking defect
Phase 3 — Content (revscripts)   data/scripts/{actions,spells,movements,…}/**
                                 → WARN and continue: a broken shard script must not
                                   brick the server; report the file and keep going
```

Phase 2 being fatal is the real fix for Gap 5a — not a longer allowlist. Phase 3 staying lenient is deliberate: shard operators edit content, not lib.

### Prefer `data/global.lua` over hand-rolled substitutes

Once `register_class` lands, the dofile chain works (CWD resolution already verified). At that point these hand-rolled stand-ins should be **deleted**, not maintained:

- `inject_door_tables_from_global` — substring-extraction hack over `global.lua`
- the inline `string.trim` / `string.splitTrimmed` chunk and its now-stale "dofile not yet wired" comment (`runtime.rs:1329-1332`; the same stale claim is repeated at `actions.rs:87`)
- the `data/lib/core/**` scan in `load_data_lib` (the Lua `dofile` chain replaces it)

Phase 2 then reduces to: `exec_chunk("global.lua")` → `data/scripts/lib/**` scan → top-level `data/scripts/*.lua`. One source of truth, matching TVP's `scriptmanager.cpp:47`.

### Keep the per-subsystem loaders

Resolves former open question 3 ([decisions.md](decisions.md#open-questions)): **do not** merge `load_action_scripts` / `load_spell_scripts` / … into one recursive scan. TVP uses a single scan because C++ made typed pending-drains awkward; the per-subsystem split is cleaner in Rust and produces an identical set of loaded scripts. Loader structure is *implementation* layer, where idiomatic Rust wins over C++ fidelity (`TFS-Core`). Only the phase boundary (lib vs content) needs to match.

### Test VM parity

`userdata/combat.rs` hand-assembles a VM in **eight** places (`Lua::new()` at `1082`, `1122`, `1198`, `1226`, `1316`, `1446`, `1565`, `1650` — the doc previously said four), each registering a different subset. Tests therefore validate a VM that is not the one shipped — a contributing reason the Gap 5 assertion passed against a half-loaded lib. Add `LuaRuntime::new_for_test()` that runs the real Phase 1 + Phase 2 and route all tests through it.
