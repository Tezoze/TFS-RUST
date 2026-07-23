# NPC system — 2026-07-22

**Plan:** `tasks/npc-system-plan.md`
**Goal:** exact 772 NPC outcomes, TFS-style Lua content/API flexibility, idiomatic game-thread Rust.

## Audit and design

- [x] Audit current NPC spawn, chat, Lua, movement, shop, persistence, and data-pack surfaces
- [x] Trace 772 parser, matching, focus/queue, timing, actions, and movement outcomes
- [x] Compare runtime interpreter, generated handler Lua, typed registry, and Lua-only options
- [x] Select canonical design: declarative Lua → typed registry → native runtime + Lua hooks
- [x] Define one-way legacy importer; no runtime `.npc`/`.ndb` engine

## Implementation phases

- [x] NPC-0 — Freeze corpus inventory and differential parity traces
  - Inventory: `scripts/npc_corpus_inventory.py` → `tasks/npc-corpus-inventory.{json,md}` (337 `.npc` + 39 `.ndb` + 165 includes; unsupported: `String`/`Bless`/`Town`/`Promote` + 4 non-utf8 files)
  - Black-box fixtures: `tests/fixtures/npc/` + `scripts/validate_npc_fixtures.py` (no live C++ harness)
- [x] NPC-1 — Add typed definitions and `NpcType` / `NpcDialogue` Lua registration
  - Content: `crates/tfs-rust-content/src/npcs/` (`NpcDatabase`, dialogue enums, validate)
  - Lua: `npc_type.rs` / `npc_dialogue.rs` / `npc_loader.rs`; smoke `data/npc/scripts/definitions/greeting.lua`
- [ ] NPC-2 — Add offline legacy importer and full-corpus validation
- [ ] NPC-3 — Wire NPC definitions into spawn/type initialization
- [ ] NPC-4 — Implement speech stimulus, focus, queue, and rule matching
- [ ] NPC-5 — Implement standard immediate actions
- [ ] NPC-6 — Implement ToDo reply timing, movement, sleep/wake, and NPC speech
- [ ] NPC-7 — Add custom Lua callbacks and migrate compatibility scripts
- [ ] NPC-8 — Add opt-in shop-window subsystem
- [ ] NPC-9 — Add atomic reload, diagnostics, and rollout cleanup

## Prior completed plan: monster combat audit

- [x] B1 — Extract physical mitigate helper; wire CASTING Damage Physical + reuse from aoe.rs
- [x] Parse poison→Earth, manadrain→ManaDrain, knife/rock/stone shooteffects
- [x] B2 — Speed MDAct% + Haste/Paralyze + duration rounds
- [x] B3 — Drunk Power=drunkness/20≤6 + duration timer
- [x] Outfit + Invisible SpellImpact + ConditionOutfit look_type_ex + ProcessSkills
- [x] Fist-only Attack distance=1; cast target = follow_target only
- [x] Tests + lessons / todo

### Deferred

- [ ] IMPACT_STRENGTH (no TFS XML name=strength)
