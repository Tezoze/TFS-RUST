# Doors Phase 6 — Auto-close MoveEvents — 2026-08-01

- [x] 6.1 MoveEvent() table + `:id`/`:aid`/`:type`/`:register` + onStepIn/onStepOut
- [x] 6.2 Load `data/scripts/movements/**` (warn-on-fail); merge into MoveEventsRegistry
- [x] 6.3 Tile: getCreatureCount, queryAdd, getThing, getThingCount, getItemByGroup + ITEM_GROUP_*
- [x] 6.4 Global doRelocate; item:getType()→ItemTypeRef; item.uid
- [x] Fire StepIn/Out from move_creature_on_map; call_move_step fromPosition
- [x] 6.6 level_doors.lua step-in in same pass
- [x] Unit: closing/level doors register; constants; getThing ground index
- [x] Update doors-actions-plan.md Phase 6 + lessons.md

# Doors Phase 5 — house doors (native gate + Lua transform) — 2026-08-01

- [x] 5.1 Door identity: `ATTR_HOUSEDOORID` → `set_door_id`; serialize on write; house tiles from OTBM
- [x] 5.2 Native before Lua: `house_door_can_use_or_deny` in `player_use_item_core` (`Door::canUse`)
- [x] 5.3 Wire DB `houses.owner` + `house_lists` into `HouseManager` at boot (name→guid resolve)
- [x] 5.4 Deny → `ReturnValue::NotPossible` ("Sorry, not possible.")
- [x] 5.5 Allow → existing `doors.lua` ±1 transform
- [x] Unit: AccessList / door_can_use; HouseDoorId round-trip; CanEditHouses flag
- [x] Update `tasks/doors-actions-plan.md` Phase 5 status + lessons.md

# Doors Phase 4 — quest + level doors — 2026-08-01

- [x] 4.1–4.2 Authority: Remere custom attrs (`doorquestnumber` / `doorquestvalue` / `doorlevel`) for `doors.lua`; legacy `data/movements/scripts/` actionid paths unchanged (Phase 6 MoveEvent)
- [x] 4.3 `player:getStorageValue` / `setStorageValue` via ScriptContext + LuaMutation (reuse `player_get/set_storage`)
- [x] 4.4 Confirm `getGroup():getAccess()` + `getLevel()` already work (GM / level gate)
- [x] 4.5 Success path already in doors.lua (`transform(+1)` + `teleportTo`); fail messages already present
- [x] Unit: storage round-trip; doors.lua registers quest/level IDs; Remere door attr constants
- [x] Update `tasks/doors-actions-plan.md` Phase 4 status + lessons.md

# Doors Phase 3 — keys (use-with) — 2026-08-01

- [x] 3.1 Use-with already reaches Action (`player_use_item_ex_core` → `fire_on_use_action`); keys registered via doors.lua
- [x] 3.2 `Item:isItem()` / `Creature:isItem()` (doors.lua key branch)
- [x] 3.3 `Tile:getTopVisibleThing([creature])` → Item/Creature userdata
- [x] 3.4–3.5 `getAttribute` / `hasAttribute` / `setAttribute` for Remere custom-attr aliases (`keynumber`, `keyholenumber`, …)
- [x] Register `ITEM_ATTRIBUTE_KEYNUMBER` / `KEYHOLENUMBER` (+ door quest/level aliases) as string constants
- [x] Unit: attr round-trip; getTopVisibleThing prefers door item; keys id in doors register
- [x] Update `tasks/doors-actions-plan.md` Phase 3 status

# 772 known-creature table overflow (blank name/HP + crash) — 2026-08-01

- [x] Root cause: `check_creature_known` used TFS 1098 limit 1300; CipSoft/TVP 772 is 150
- [x] `ProtocolCaps::known_creature_limit` — 772=150, 1098=1300
- [x] Thread limit through map description + `send_creature_appear_to_conn`
- [x] Unit/caps tests; lesson 280

# Doors Phase 2 — basic open/close/locked — 2026-08-01

- [x] 2.1 `item:transform` updates tile flags (reset old + apply new) so doors block/unblock walk
- [x] 2.2 `Tile:getCreatures()` — shove occupants before close
- [x] 2.3 `item:getPosition()` → `Position` userdata; `Position + offset` signed
- [x] 2.4 Verify `getItemByType(ITEM_TYPE_MAGICFIELD)` + `remove` on close; reset flags on remove
- [x] Fix `TILESTATE_*` Lua globals to match `tile.h` (BLOCKSOLID = 1<<17)
- [x] Unit: transform clears BLOCKSOLID; Position −1 offset; doors register open/closed/locked
- [x] Update `tasks/doors-actions-plan.md` Phase 2 status

# Doors Phase 1 — Action pipeline — 2026-08-01

- [x] `Action()` table + `:id`/`:aid`/`:register` + `_pending_actions` (TalkAction pattern)
- [x] `load_action_scripts` recursive; inject door tables from `global.lua` (no full lib)
- [x] `ActionRegistry` + `fire_on_use_action`; hook use / use-with before native
- [x] `item.itemid` field; unit smoke `food.lua` (meat 2666) + `doors.lua` register
- [x] Update `tasks/doors-actions-plan.md` Phase 1 status

# Doors Phase 0 — data-pack hygiene — 2026-08-01

- [x] Uncomment `openQuestDoors` / `openLevelDoors` in `data/global.lua` (open = closed +1)
- [x] Spot-check vs `forgotten.otbm`: closed/locked/house/quest/level IDs present; no real table gaps (1215–1218 = buttresses)
- [x] Update `tasks/doors-actions-plan.md` Phase 0 status

# 772 client crash on hole/stairs down (z=7→8) — bug0000013 rz=-1 — 2026-08-01

- [x] Root cause: `send_notify_go` led with `0x6D` for surface→underground; client FloorDown then double-applies z → `Map.cpp` `rz=-1`
- [x] Fix: `0x6C` remove for `orig.z==7 && dest.z>=8` (match TVP / `send_move_creature_player`)
- [x] Tests: `hole_down_self_packet_then_floors`, `surface_to_underground_stairs_uses_remove_not_move`
- [x] Lesson 277

# Field runes (firebomb) — no damage + delayed 772 client crash — 2026-08-01

- [x] Root cause crash: `is_cip_priority_bottom` treated MagicField as BOTTOM; 772 `objects.srv` fields are LOW → creature stackpos desync after `0x6A` when monster moves
- [x] Root cause no damage: field runes have no instant formula; `onStepInField`/`onAddField` are C++ natives (movements.xml load fails); never applied conditions
- [x] Fix: magic fields = LOW only; splash/pool remain BOTTOM
- [x] `internal_add_item_to_tile`: replace MAGICFIELD, set tile flags, `AddItemField` damage
- [x] Walk land: `apply_magic_fields_under_creature` (StepInField)
- [x] Tests: `magic_field_place_damages_creature_on_tile`, `magic_fields_are_cip_priority_low_not_bottom`, `firefield_xml_nested_attrs`
- [x] Lesson 276

# Aggressive spell/rune PZ lock only vs players — 2026-08-01

- [x] Spoken aggressive: `BlockLogout(60, false)` not `true` (`magic.cc:3636-3638`)
- [x] Rune aggressive: same (`magic.cc:4304-4306`); player-hit still PZ-locks via combat
- [x] Field fire/poison/energy keep `BlockLogout(..., true)`
- [x] Test + lesson 275

- [x] `condition_blob` serialize/deserialize TFS PropStream (`players.conditions`)
- [x] Save path writes blob; login loads into `active_conditions`
- [x] Login: `send_player_icons` + `reapply_persisted_condition_effects` (speed/invis/light)
- [x] Roundtrip test for mana shield / haste / invis; lesson 274

# Client stats (0xA0) after mana/HP change — 2026-08-01

- [x] Spell cast always `notify_magic_tries_gained` → `send_player_stats` (772 `TSkillMana::Set`)
- [x] Lua combat healing → `notify_creature_healed` (not damage_done==0 early-out)
- [x] Condition DoT → `notify_player_combat_damage` after apply
- [x] Test: cast enqueues `0xA0`; lesson 273

# Exhaust: decompile clocks + TFS Lua knobs — 2026-08-01

- [x] `EarliestSpellTime` / `EarliestMultiuseTime` are the live gates (not ConditionExhaust)
- [x] Instant: `:cooldown` / world-type CheckMana fallback via `spell_exhaust_delay_ms`
- [x] Instant: unset `PendingSpell.cooldown = 0` → open PvP **2000** (not TFS hard 1000)
- [x] Instant: `:group` / `:groupCooldown` → `spell_group_cooldown_end` (additive)
- [x] Rune: always multiuse +1000; `cooldownSpellTime(true)` also bumps spell clock (default false)
- [x] `player_apply_spell_exhaust_ms` respects `HasNoExhaustion`; multiuse does not
- [x] Lesson 272

# UseItem on bare ground — 2026-08-01

- [x] `resolve_ground_use_type` — TypeID match only (772 `GetObject`; no empty-list / stackpos-0 shortcut)
- [x] `validate_use_object_ref` for Use enqueue (Turn stays item-only)
- [x] `execute_player_use` → `player_use_ground_core` (teleport / `CannotUseThisObject`)
- [x] Tests: walk-to bare ground; wrong TypeID no walk; plain → NOTUSABLE; type-430 teleport
- [x] Lesson 270

# Protection zone gaps (772) — 2026-08-01

- [x] Walk-time PZ-entry lock: `earliest_protection_zone_round` in `tile_query_add_player` → `PlayerIsPzLocked`
- [x] Unconditional snapback for `PlayerIsPzLocked` (772 `ENTERPROTECTIONZONE`)
- [x] NoLogout logout check uses `tilestate::NOLOGOUT` flag (not never-set `ZoneType::NoLogout`)
- [x] Tests: PZ entry lock / within-PZ / expired; NoLogout flag; snapback sibling
- [x] Lessons: walk lock + flag-only NoLogout; houses not auto-PZ under 772

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

# NPC Phase 2 correctness follow-ups — 2026-07-28 ✅

Source: `docs/772_NPC_AUDIT.md` Phase 2.

## 2.1 Thread `Npc->Data` through Create / Delete / `Count(...)`

- Add `data: i32` parameter to `NpcActionHost::create_item` / `delete_item`.
- Pass `ctx.data` from `react.rs` `DialogueAction::Create` / `Delete`.
- `host.rs` `npc_give_to` / `npc_get_from` already accept `data`; forward it instead of `-1`.
- `focus.rs` `inventory_count` closure: capture session `data` and pass as `sub_type` when the type is a fluid container or key (`ItemDatabase::is_fluid_container`); otherwise `-1`.
- C++ quirk: `CountObjects` does **not** propagate `Value` into nested containers (`info.cc:553`); preserve by only applying `sub_type` at top-level inventory slots.

## 2.2 `%T` uses 772 game time and fixes am/pm

- `focus.rs`: replace wall-clock `chrono::Local::now()` hour/minute with `world_time_from_local_clock()`.
- Compute `game_minutes = world_time_from_local_clock()`; `game_hour = game_minutes / 60`; `game_minute = game_minutes % 60`.
- `expr.rs::format_game_time`: restore C++ branch verbatim so PM prints `pm`:
  `if hour < 12 { "{hour}:{minute:02} am" } else { "{}:{minute:02} pm", hour - 12 }`.

## 2.3 `ToDoYield` on VANISH / null-interlocutor Idle transitions

- `focus.rs::npc_creature_move_stimulus`: after the VANISH `Idle` state flip, call `self.creature_todo_yield(npc_id)`.
- Also yield in the `focus.is_none()` / `Interlocutor == NULL` arm when transitioning away from Talking/Leaving.

## 2.4 `Idle` action skips `StartToDo` under `ADDRESSQUEUE`

- `react.rs::apply_dialogue_plan`: in `DialogueAction::Idle`, only set `plan.start_todo = true` when `situation != DialogueSituationKind::AddressQueue`.
- On `AddressQueue` + no prior speech, emit a `tracing::warn!` matching the decompile error log.

## 2.5 `Delete` failure aborts remaining actions

- `react.rs::apply_dialogue_plan`: change `Delete` arm to return a sentinel/break out of the action loop on `Err`.
- Ensure `Create` and other subsequent actions in the same rule are skipped, matching C++ `throw ERROR` unwinding.

## Tests to add

- `format_time_pm` — game hour 13 → `"1:00 pm"`.
- `create_respects_data_subtype` — fluid container with `Data=11` creates vial with fluid type 11.
- `delete_failure_aborts_remaining_actions` — `Delete` fails and a following `Say` does not fire.
- `vanish_yields_idle_loop` — VANISH transition arms `ToDoWait(0)` + start.
- `idle_address_queue_no_trailing_starttodo` — ADDRESSQUEUE Idle-only rule does not schedule trailing Wait/Start.

## Verification

- `rtk cargo check --workspace`
- `rtk cargo clippy --workspace --all-targets -- -D warnings`
- `rtk cargo test -p tfs-rust-core npc::`
- `rtk cargo test -p tfs-rust-content npc_import::`
- `rtk cargo test --workspace`

## Risks

- `data` threading touches `npc/actions.rs` trait and `npc/host.rs` impl; ensure all call sites updated.
- `Delete` failure abort changes economy edge cases; covered by new test.
- `ToDoYield` on VANISH must not double-yield when `creature_todo_clear` already ran inside `npc_react`.

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
