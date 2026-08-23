# Active plans

- **Data-pack Lua (772 corpus):** [docs/DATA_PACK_LUA.md](../docs/DATA_PACK_LUA.md) — spawn-only loot; Lua vs native. **Phases 1–2 done** (scripts-interface allowlist + CreatureEvent registry/dispatch). Phases 3–8 not implemented. Plan: [data-pack-lua-implementation-plan.md](data-pack-lua-implementation-plan.md).
- **Monsters XML → Lua:** [monsters-lua-plan.md](monsters-lua-plan.md) (Lua-as-data, not TFS `createMonsterType`).
  - **Done.** Converter shipped 157 files; runtime `load_dir` is Lua; XML pack + parser + `export-monsters-lua` removed; `lua/#example.lua` deleted. Lessons 366–367.
  - Left as-is: `data/scripts/lib/register_monster_type.lua` stub.

# Monster combat 772 parity — audit fixes

**Status:** done except deferred tie-break. Lesson 363.
