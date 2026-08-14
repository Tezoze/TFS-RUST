# Gap 1: `Action:allowFarUse` — 2026-08-14

Load-time: `fishing_rod.lua` calls `action:allowFarUse(true)` and fails (`nil` method).
Runtime: far-use range must honor the flag (`Actions::canExecuteAction` → `canUseFar`).

772 fishing rod (`objects.srv` TypeID 3483) has **no** `DistUse` — only `Take`. Without this flag the ToDo Use arm walks to water.

- [x] Lua `Action:allowFarUse(bool)` stores `_allow_far_use` (C++ `luaActionAllowFarUse`)
- [x] Drain `PendingAction` → `ActionDef` → `ActionEntry`
- [x] `EventDispatcher::action_allows_far_use` + ToDo Obj2 arm (`areInRange<7,5>`, same as rune `allowFarUse`)
- [x] `tools_scripts_load_and_register` asserts 9 files / id 2580; focused drain test
- [x] Docs + lessons

# Gap 5a: Lib-stage load is fatal and aggregated — 2026-08-13

Phase 2 (`load_data_lib`) returns one `LuaError` listing every broken lib file; boot aborts. Content-stage loaders stay warn-and-continue. Skip `core.lua`/`lib.lua` dispatchers (double-load + CWD-relative `dofile`).

- [x] `LuaError::LibStageFailures` — aggregated `(path, error)` list
- [x] `load_data_lib` collects failures instead of `tracing::warn!` + continue
- [x] Skip `core.lua` / `lib.lua` in the `data/lib/core` scan
- [x] `run_server.rs` `anyhow::bail!` on lib-stage Err (match Gap 5 globals assertion)
- [x] `load_spell_scripts` propagates lib-stage Err (do not swallow)
- [x] Tests: `lib_stage_loads_with_zero_failures` + aggregation/skip-dispatcher guard
- [x] Docs: README / gaps-load / architecture / lessons

# Gap 7c: Revscript ctor globals through `register_class` — 2026-08-13

Unblock `data/scripts/lib` load (3 of 5 files) so Gap 5a can flip the lib stage to fatal.

- [x] Route `Action`/`TalkAction`/`MoveEvent`/`Channel`/`Condition`/`Variant`/`MonsterType` through `register_class`
- [x] Add `CreatureEvent`/`GlobalEvent` with `__call` (plain tables, Action pattern)
- [x] Port `createFunctions` into `data/lib/core/` (do not load `compat.lua`)
- [x] No `MonsterTypeRef` `__index` chain (`#example.lua` only)
- [x] `all_class_globals_are_tables` + `scripts_lib_files_load_with_zero_failures`
- [x] `cargo test -p tfs-rust-lua --lib` (77 passed)

# P2: Player Combat Fidelity Polish — 2026-08-07

From player-combat-audit canvas / docs/772_PLAYER_COMBAT_AUDIT.md §12.
772 outcomes; TFS-shaped domain; idiomatic Rust.

- [x] P2-1 SendMarkCreature on damage taken (G2) — S
- [x] P2-2 Mana-drain effects + shield attacker name (G4) — S
- [x] P2-3 Amulet-of-loss full slot scan + exact lethal (G3) — M
- [x] P2-4 BANK/UNLAY legs on ammo drop (G5) — S
- [x] P2-5 Native burst arrow + typed AMMO*/THROW* attrs (G6/G7) — M
- [x] P2-6 GetExpForLevel guards + Decrease abort quirk (G12) — S
- [x] P2-7 Gate formula.melee_damage / dist_damage to Modern/1098 (N1) — S
- [x] P2-8 Cite CheckMana; close N2 as verified — S
- [x] Verify cargo check/clippy/test; update lessons + canvas

## Prior (done)

# P1: Player Combat Parity — 2026-08-07
- [x] P1-1 Periodic damage types + origins + poison gate (B7)
- [x] P1-2 Ring-buffer CombatList + 60-round window (B8)
- [x] P1-3 Soul regen on exp gain (G1)
- [x] P1-4 Per-beat invisibility re-check (G8)
- [x] P1-5 Ranged ToDoWait(100) (G9)
- [x] P1-6 CheckCombatValues on level Jump (G10)
