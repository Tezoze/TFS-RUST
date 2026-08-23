# Active plans

- **Data-pack Lua (772 corpus):** [docs/DATA_PACK_LUA.md](../docs/DATA_PACK_LUA.md). **Phases 1–6 done.** Phases 7–8 remain (globalevents; extra tests). Plan: [data-pack-lua-implementation-plan.md](data-pack-lua-implementation-plan.md).
- **Monsters XML → Lua:** [monsters-lua-plan.md](monsters-lua-plan.md) (Lua-as-data, not TFS `createMonsterType`).
  - **Done.** Converter shipped 157 files; runtime `load_dir` is Lua; XML pack + parser + `export-monsters-lua` removed; `lua/#example.lua` deleted. Lessons 366–367.
  - Left as-is: `data/scripts/lib/register_monster_type.lua` stub.

# Phase 6 — Retire XML script trees

**Status:** done 2026-08-24. Lesson 372.

# Monster combat 772 parity — audit fixes

**Status:** done except deferred tie-break. Lesson 363.
