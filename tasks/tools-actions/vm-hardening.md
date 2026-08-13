# VM hardening — the parts of a "modern sandboxed API" we can adopt

Index: [README.md](README.md)

**Measured 2026-08-10.** A modern sandboxed scripting design has five pillars. Three are **orthogonal to the TFS contract** and should be adopted; two are the contract itself and are rejected (see [*Strategic decision*](decisions.md#strategic-decision--keep-the-tfs-lua-facing-contract)).

| Pillar | Breaks the data pack? | Verdict |
|---|---|---|
| 4. Resource limits (instruction + memory) | No | **Adopt** |
| 5. Typed contracts (LuaLS) | No | **Adopt** |
| 1. Stdlib allowlist | Barely — 2 runtime call sites | **Adopt** |
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

**Two caveats to decide, not skip:**
- **No rollback.** Mutations apply immediately so scripts can read them back mid-callback (`TFS-lua-boundaries`, Mutation Path). Aborting mid-script leaves *partially applied* effects — failure isolation, not atomicity. Document the semantic per callback.
- **LuaJIT + active hooks probably forces interpreter fallback** (LuaJIT does not call count hooks from compiled traces), which cuts against choosing LuaJIT for speed. **Measure before enabling globally.** Fallbacks: generous budget; hooks on content-stage scripts only; or ship `set_memory_limit` alone first (no JIT impact).

Choose the budget by measuring the heaviest legitimate callback (large loot loops, map-wide iteration), then ~10×.

### Pillar 5 — typed contracts (LuaLS)

Emit `.d.lua` definitions for every registered class, method, and constant; `lua-language-server` gives editor autocomplete and a headless CI check over `data/`.

**Why it matters here:** every Gap 3 / Gap 4 defect is a "name doesn't exist" error that is **statically detectable** — `SKILL_FISHING` undefined, `actionIds.destroyableStone` nil, `Tile:relocateTo` missing. Note that the Gap 3 table in this document is a hand-maintained inventory of which methods exist, and it was **wrong** (over-counted by five). Generated definitions are correct by construction; hand-maintained inventories rot.

Generate the **union** of two sources: methods registered from Rust (enabled by `register_class` being the single owner — Gap 7a ✅), plus methods the data pack defines in Lua (`Tile.relocateTo` lives in `lib/core/tile.lua`), which LuaLS infers from the workspace. The `__index` chains (Gap 7b ✅) make the Lua-defined methods faithfully reachable at runtime, so the generated types match actual call resolution.

### Pillar 1 — stdlib allowlist (isolation)

Replace `Lua::new()` (mlua `ALL_SAFE`, which includes `io`, `os`, `package`) with an explicit `Lua::new_with(StdLib::STRING | TABLE | MATH | BIT | COROUTINE, …)`. Probed current VM — all of these are live today:

```
io.open = function   os.execute = function   os.remove = function
package.loadlib = function   loadstring = function   debug = nil
```

Any data-pack file can shell out, delete files, or load a native `.so`.

**Measured cost across the whole data pack:**

| Symbol | Uses | Where |
|---|---|---|
| `io.*` | 14 | `functions.lua:287-294` (command log), `default_onReportBug.lua`, `migrations/11.lua`, `migrations/14.lua` |
| `os.time` / `os.date` | 52 | pure time reads |
| `require` | 0 | (2 hits are the English word in NPC dialogue) |
| `loadstring`, `package.*` | 0 | — |

So: **two runtime call sites** become a `tfs.appendLog(kind, text)` capability constrained to `data/logs/`; migrations are one-off tooling and can run in a separate unrestricted VM; keep a minimal `os` shim with `time`/`date`/`clock`. Nothing uses `require`/`loadstring`/`package`, so those drop free.

**Value depends on threat model.** If we are the only script authors, `os.execute` is not a vulnerability — we already have a shell. It becomes real with community scripts, outside content contributions, or hosting shards for others. Cheap enough to do on principle, but lowest urgency of the three.

### When to implement

| Pillar | When | Gate |
|---|---|---|
| 4 — `set_memory_limit` | ✅ done (2026-08-10) — independent of everything else | none; no JIT impact |
| 5 — LuaLS generation | Immediately after **Gap 7a+7b** (both done) | needs `register_class` as single owner + `__index` chains so method resolution is faithful |
| 4 — instruction hook | After Gaps 1-6 (tools running end-to-end); **before any production or third-party exposure** | needs a JIT-cost measurement + a chosen budget |
| 1 — stdlib allowlist | After Gaps 1-6, alongside or after the instruction hook | needs the `tfs.appendLog` capability first |

Rationale for the ordering: the memory limit is free and prevents a whole-process kill. LuaLS is gated on Gap 7a+7b (both done) and pays for itself immediately by replacing hand-maintained inventories. The instruction hook needs a real measurement first, so it should not block the tools work. The allowlist is cheap but addresses a threat we may not have yet.
