# Spell runes — look + conjure — 2026-07-26

- [x] `Item:getId()` returns SlotMap key instead of server type — breaks `conjureItem` blank-rune check
- [x] Port C++ rune look (`item.cpp` ~951–1003) + ItemType patch on `spell:register()` (`luascript.cpp:15889–15895`)
- [x] Parse `runespellname`; `rune:runeMagicLevel` → `magic_level` (C++ `setMagicLevel`)
- [x] Nested Lua mutation scope cleared mid-`conjureItem` (`item:remove` → inventory hook)
- [x] `UseWithCreature` packet was dropped — needTarget runes did nothing
- [x] Far-use runes: do not walk to Obj2 — `allowFarUse` / DistUse fire from standing tile
- [x] Floor/AoE runes (GFB): `needTarget` false → position variant at aimed tile
- [x] needTarget miss (SD on empty tile): cancel + `CONST_ME_POFF` on caster

# Signs / blackboards look — 2026-07-26

- [x] Unserialize OTBM `ItemNodeProps` attrs (`ATTR_TEXT`, etc.) on map load
- [x] Port `allowDistRead` look arm (`item.cpp` ~1422–1449)
- [x] Remere OTBM attrs 23–28 (key/door) — not TFS NAME/WEIGHT; Latin-1 prop strings

# Wall spawn placement — 2026-07-26

- [x] Root cause: `forced = !startup` + `login_possible = forced || …` accepted walls on respawn
- [x] `probe_spawn_tile` matches 772 `SearchSpawnField` object-flag loop (UNPASS+UNMOVE)
- [x] TFS `forced` = `startup` only; never override failed `queryAdd`
- [x] Regression: `probe_rejects_immovable_unpass_wall`, `classic772_spawn_skips_wall_home_picks_neighbor`
- [x] Gorn spider sewer leak: Bank+wp0 dirt walls must count as Unpass for spawn BFS (`item_is_unpass_for_spawn_field`)
- [x] Same Bank+wp0 Unpass for monster MovePossible + monster/NPC queryAdd (players keep cliff walk)
- [x] Narrow OTB clear-solid to srv `"a mountain"` only — dirt walls block player ladder pathfind

# Spell fail messages + GM cast — 2026-07-26

- [x] Spell fails use `failure_message_type` (`TALK_FAILURE_MESSAGE` / cancel bar), not status/console
- [x] Wording via `ReturnValue` — mana / learn match 772 `sending.cc`
- [x] `need_learn` gate against `persist.spells`
- [x] Wire `IgnoreSpellCheck` / `HasInfiniteMana` / `HasInfiniteSoul` / `HasNoExhaustion` / `CannotUseSpells`
- [x] Gamemaster group: can cast, infinite mana/soul (was `cannotusespells`)

## Prior: NPC system — 2026-07-22

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
  - Lua: `npc_type.rs` / `npc_dialogue.rs` / `npc_loader.rs`; smoke `data/npc/scripts/greeting.lua`
- [x] NPC-2 — Add offline legacy importer and full-corpus validation
  - Importer: `crates/tfs-rust-content/src/npc_import/` (reference `cipsoft-772/runtime/npc` authority; reject `String`/`Bless`/`Town`/`Promote`)
  - CLI: `cargo run -p tfs-rust-lua --bin import-npcs -- --root … --out … --validate-data-dir data`
  - Tests: parse-all 337, goldens albert/quentin/suzy(+bank), Lua round-trip
  - Generated: 337 Lua defs under `data/npc/scripts/`
  - Archived: `data/npc/archive/{xml,behavior}/` (old `behavior=` pack); 9 `script=` XMLs stay live for NPC-7
- [x] NPC-3 — Wire NPC definitions into spawn/type initialization
- [x] NPC-4 — Implement speech stimulus, focus, queue, and rule matching
- [x] NPC-5 — Implement standard immediate actions
- [x] NPC-6 — Implement ToDo reply timing, movement, sleep/wake, and NPC speech
- [x] NPC-7 — Add custom Lua callbacks and migrate compatibility scripts
  - EventDispatcher `on_npc_*` + `fire_npc_*`; `NpcRef` userdata; custom pred/action wired
  - Migrated Captain + Banker; archived bless/promote/shop/oracle-handler under `data/npc/archive/script-compat/`
  - Stopped loading `data/npc/lib/npcsystem/` from `npc.lua`
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
