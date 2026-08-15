# VM hardening — the parts of a "modern sandboxed API" we can adopt

Index: [README.md](README.md)

**Measured 2026-08-10.** A modern sandboxed scripting design has five pillars. Three are **orthogonal to the TFS contract** and should be adopted; two are the contract itself and are rejected (see [*Strategic decision*](decisions.md#strategic-decision--keep-the-tfs-lua-facing-contract)).

| Pillar | Breaks the data pack? | Verdict |
|---|---|---|
| 4. Resource limits (instruction + memory) | No | **Adopt** |
| 5. Typed contracts (LuaLS) | No | **Adopt** |
| 1. Stdlib allowlist | Barely — 2 runtime call sites | ✅ **Adopt** (done 2026-08-15) |
| 2. Per-script `_ENV` | Yes — contract *is* shared globals | Reject |
| 3. Modules / returned descriptors | Yes — replaces `Action():register()` | Reject |

### Pillar 4 — resource limits (reliability)

```rust
lua.set_hook(HookTriggers::new().every_nth_instruction(N), |_, _| {
    Err(mlua::Error::runtime("script exceeded instruction budget"))
})?;
lua.set_memory_limit(BYTES)?;
```

Both exist in mlua 0.12.0; `set_hook` is available on non-Luau builds (our LuaJIT) and is documented for exactly this use.

**Why it matters here:** game simulation is single-threaded (`TFS-threading`). One `while true do end` anywhere in `data/scripts/**` hangs the whole server — no ticks, no packets, no saves, recoverable only by `kill -9`, every player losing state since last save. A runaway allocation OOMs the process. No attacker required; an accidental loop in a quest script is the normal case. The guard turns a total outage into one failed script call.

✅ **`set_memory_limit` done 2026-08-10.** `DEFAULT_LUA_MEMORY_LIMIT_BYTES` (512 MiB) in `LuaRuntime::new`; `config.lua` `luaMemoryLimit` (MB).

✅ **Instruction hook done 2026-08-15.** Per-invocation count hook in `instruction_budget.rs` (`DEFAULT_LUA_INSTRUCTION_BUDGET` = 10_000_000, ~10× a 500×20 loot-style loop — `default_budget_covers_heavy_loot_loop_with_headroom`). Armed at every Rust→Lua entry (`exec_chunk`, `call_*`, timers, combat value/event callbacks, NPC callbacks). Nested Lua→Rust→Lua shares the outer budget. `config.lua` `luaInstructionBudget`; `0` disables the hook and re-enables LuaJIT.

**Caveats (decided, not skipped):**
- **No rollback.** Mutations apply immediately so scripts can read them back mid-callback (`TFS-lua-boundaries`, Mutation Path). Aborting mid-script leaves *partially applied* effects — failure isolation, not atomicity. Same semantic for every callback: the dispatcher logs the error and continues; world/Lua state already written stays written (`abort_does_not_roll_back_prior_lua_side_effects`).
- **LuaJIT compiled traces skip count hooks** (mlua's own test calls `jit.off()`). While the budget is > 0 we `jit.off()` on first armed entry so the hook actually fires. Operators who need JIT back set `luaInstructionBudget = 0` (and accept that a runaway script can hang the game thread).

A lifetime hook (install once in `LuaRuntime::new`) would spend the budget across the whole process and then start killing legitimate scripts. The hook is therefore re-armed at each outermost invocation (`budget_resets_per_invocation`).

### Pillar 5 — typed contracts (LuaLS)

✅ **done 2026-08-14.** `cargo run -p tfs-rust-lua --bin emit-lua-defs` boots a live `LuaRuntime`, records native userdata methods via `RecordingRegistry`, walks class tables / ctor instances / constants / free functions, and writes `lua-defs/{engine,constants,globals}.d.lua`. Lua-defined methods (`Tile.relocateTo` in `lib/core/tile.lua`) stay in the data pack — LuaLS infers them from the workspace. `.luarc.json` points the editor at `lua-defs/`. `lua_defs_snapshot_covers_engine_surface` + `lua_defs_committed_files_are_current` keep the stubs honest.

CI runs `lua-language-server --check=. --configpath=.luarc.json --checklevel=Warning` from the **repo root**. `--check=data/lib` would make the workspace `data/lib`, so `./lua-defs` would not load and every engine global would look undefined. `.luarc.json` `ignoreDir` keeps `data/npc`, `data/monster`, `data/lib/compat`, and the unused legacy `data/{actions,talkactions,weapons,…}` trees out of the baseline. `diagnostics.globals` lists TFS names the live VM does not register yet (`db`, `ITEM_GOLD_COIN`, …) so existing scripts stay green; a new missing global (the `SKILL_FISHING` class of bug) still fails CI. Two scripts with undeclared locals (`Obj2`, `creature`) are ignored until those files are fixed.

Generate the **union** of two sources: methods registered from Rust (enabled by `register_class` being the single owner — Gap 7a ✅), plus methods the data pack defines in Lua (`Tile.relocateTo` lives in `lib/core/tile.lua`), which LuaLS infers from the workspace. The `__index` chains (Gap 7b ✅) make the Lua-defined methods faithfully reachable at runtime, so the generated types match actual call resolution.

### Pillar 1 — stdlib allowlist (isolation)

✅ **done 2026-08-15.** `stdlib_allowlist.rs` builds the game VM with `Lua::new_with(STRING | TABLE | MATH | BIT | JIT | OS)` — not `Lua::new()` / `ALL_SAFE`. `OS` is loaded only so LuaJIT's `os.time` / `os.date` / `os.clock` stay exact (strftime, `*t` tables, `os.time{year=…}`); the table is then replaced with those three functions. `io`, `package`, `require`, `loadstring`, `loadfile`, `load` are nil. `JIT` stays so `luaInstructionBudget = 0` can still `jit.on()`. LuaJIT has no `StdLib::COROUTINE` flag — `coroutine` is in the base library.

`tfs.appendLog(kind, text)` is the only script write path, rooted at `data/logs/`. `kind` is a relative filename (no `..`, no absolute, `[A-Za-z0-9._\- ]` per path component). Returns `false` on rejection or IO error (same as the old `if not file then return`).

**Call sites:**

| Old | New |
|---|---|
| `functions.lua` `io.open("data/logs/" .. name .. " commands.log")` | `tfs.appendLog(name .. " commands.log", line)` — same path |
| `default_onReportBug.lua` `io.open("data/reports/bugs/" .. name .. " report.txt")` | `tfs.appendLog("bugs/" .. name .. " report.txt", text)` — now `data/logs/bugs/` |
| `data/migrations/11.lua` / `14.lua` | not loaded on the game VM (SQLx migrations); leftover C++ tooling |

`dofile` stays for the TFS `global.lua` load chain. Nothing uses `require` / `loadstring` / `package`.

Probed before the change — all of these were live on `ALL_SAFE`:

```
io.open = function   os.execute = function   os.remove = function
package.loadlib = function   loadstring = function   debug = nil
```

Any data-pack file could shell out, delete files, or load a native `.so`.

**Measured cost across the whole data pack:**

| Symbol | Uses | Where |
|---|---|---|
| `io.*` | 14 | `functions.lua` (command log), `default_onReportBug.lua`, `migrations/11.lua`, `migrations/14.lua` |
| `os.time` / `os.date` | 52 | pure time reads |
| `require` | 0 | (2 hits are the English word in NPC dialogue) |
| `loadstring`, `package.*` | 0 | — |

**Value depends on threat model.** If we are the only script authors, `os.execute` is not a vulnerability — we already have a shell. It becomes real with community scripts, outside content contributions, or hosting shards for others. Done on principle now that the instruction hook landed.

### When to implement

| Pillar | When | Gate |
|---|---|---|
| 4 — `set_memory_limit` | ✅ done (2026-08-10) — independent of everything else | none; no JIT impact |
| 5 — LuaLS generation | ✅ done (2026-08-14) — `emit-lua-defs` + committed `lua-defs/` + CI `--check` | needs `register_class` as single owner + `__index` chains so method resolution is faithful |
| 4 — instruction hook | ✅ done (2026-08-15) — per-invocation `set_hook` + `jit.off()` while budget > 0 | Gaps 1-6 done; budget = 10× loot-loop measurement; `luaInstructionBudget = 0` restores JIT |
| 1 — stdlib allowlist | ✅ done (2026-08-15) — `new_with` allowlist + `tfs.appendLog` + os time shim | Gaps 1-6 done; `tfs.appendLog` first; instruction hook already landed |

Rationale for the ordering: the memory limit is free and prevents a whole-process kill. LuaLS is gated on Gap 7a+7b (both done) and pays for itself immediately by replacing hand-maintained inventories. The instruction hook needs a real measurement first, so it should not block the tools work. The allowlist is cheap and closes `os.execute` / `io.open` / `package.loadlib` on the game VM now that Gaps 1-6 and the instruction hook are done.
