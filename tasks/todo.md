## Gameloop lag fixes — parent + cron heap + dense TShortway (2026-07-19)
- [x] `Item.parent: Option<Cylinder>`; remove unused `ItemPosition`
- [x] Hub writers: tile / inventory / container + OTBM + load resync + depot link + inv↔inv swap
- [x] `resolve_item_parent_cylinder` / Lua parent+position O(1); demote map scan
- [x] `DecayManager` deadline min-heap (`CronCheck` shape)
- [x] Dense `TShortway` scratch (±Visible+1 indices)
- [x] Tests: decay + tshortway/cyclops; lessons.md §205

## Item decay / expire system (2026-07-18) — Phase 5 done
- [x] Audit decompile Expire/Cron vs TFS duration/decayto/stopduration vs Rust
- [x] Plan doc: `tasks/decay-system-plan.md` (Phase 0 decisions locked)
- [x] Phase 0: corpse = XML duration only; Empty required; depot config default false; 1098 shared
- [x] Phase 1: typed ItemType decay fields + `decay_deadline_ms` (fix `duration * 50`)
- [x] Phase 2: `start_decay` / `stop_decay` / `can_decay` / `internal_decay_item` / `process_decay_expiry`; Lua `item:decay()`; cron applies all cylinders
- [x] Phase 3: `change_item_type` stopduration; cylinder add→`start_decay`; destroy→cancel; equip/lua transform wired
- [x] Phase 4: login `Pending` → `start_decay`; save writes `DecayManager` remaining ms
- [x] Phase 5: Empty before expire; showduration look; actionId/depot `can_decay`; field cron smoke
- [ ] Phase 6: fold corpse/splash ad-hoc schedulers; remove `corpse_decay_offset_ms`

## Monster XML summons — 772 CASTING / ToDo (2026-07-18) — done
- [x] Parse `<summons>` / `SummonBlock` into `MonsterType`
- [x] Merge into CASTING as `SpellImpact::Summon` (Origin r=0) via `combat_from_monster_type`
- [x] `search_summon_field` (`info.cc`) + `monster_create_summon` (`TSummonImpact` / `CreateMonster`)
- [x] Origin/Destination CASTING `handleField` path; Master==0 + SummonedCreatures < max
- [x] Tests: parse, giant spider spell merge, create+master link
- [x] Fix summon rebind: monster masters are never `Following` — inherit `AttackDest` (lesson 194)
- [x] Own-summon kick = decompile push-first (lesson 195; always-trample reverted)

## PC-3a Gaps 5–6 — summons + utilities (2026-07-18) — done
- [x] Phase 9a: VARIANT_STRING param plumbing; Variant ctor; getString/getNumber/getPosition
- [x] Phase 9b–9c: Position fields/ctor/getNextPosition/moveUpstairs; Tile hasFlag/getGround/getItems/getItemByType/getTopDownItem
- [x] Phase 9d: ropeSpots / Fields / corpseIds in functions.lua
- [x] Phase 10: summonable/convinceable/manaCost; Game.createMonster; addSummon/getSummons/isMonster/getType; ItemType isCorpse/isMovable
- [x] Phase 11a: rune:{id} callbacks; use-with → fire_on_cast_rune + charge consume
- [x] Phase 11b: sendTextMessage, getDirection, move/teleportTo, CanSummonAll/CanConvinceAll; docs

## PC-3a Phases 6–8 + MonsterType (2026-07-18) — done
- [x] Phase 8: CREATEITEM / NODAMAGE / DISTANCEEFFECT on request → `aoe.rs`
- [x] Minimal `Tile(pos):hasProperty(BLOCKSOLID)`
- [x] Phase 7: `extArea` diagonal orientation
- [x] Phase 6: TARGETCREATURE/TILE; `Game.getWorldType`; `doChallengeCreature`
- [x] `MonsterType:getOutfit` / `isIllusionable`; parse illusionable/challengeable
- [x] Docs in `pc3a-spell-gaps.md`; unit tests (remap, diagonal, WORLD_TYPE_*)

## PC-3a Phase 5 — conjure helpers (2026-07-18) — done
- [x] `getMana` / `addMana` / `addManaSpent` on `CreatureRef` (+ ScriptContext / LuaMutation)
- [x] `ItemType:getCharges`, `item:hasAttribute`, `item:transform`
- [x] Unblockers: `Group:hasFlag`, `PlayerFlag_HasInfiniteMana`, `ITEM_ATTRIBUTE_*`
- [x] Docs in `pc3a-spell-gaps.md`; `player_flags` + constants tests

## PC-3a Phase 4b — condition client updates (2026-07-18) — done
- [x] `Player::internal_light` + `player_creature_light` max(internal, items)
- [x] `on_condition_started` / `on_condition_ended` (icons, speed, light, invis, outfit)
- [x] Wire AoE apply/dispel, Lua add/remove/setInFight, ProcessSkills expiry
- [x] Docs in `pc3a-spell-gaps.md`; light max unit smoke

## Equip item abilities (2026-07-18) — done
- [x] Native `MoveEvent::EquipItem` / `DeEquipItem` abilities (`movement.cpp`) — speed, skills, flat/percent stats, invisible, mana shield, regen, suppressdrunk
- [x] `CreatureBase::var_speed` + `Player::{var_skills,var_stats,condition_suppressions}`; walk/combat/stats use effective values
- [x] `transformEquipTo` / `transformDeEquipTo` + decay schedule (`duration` sec × 1000); cron expiry strips abilities
- [x] `ConditionType::Regeneration` + `process_equipment_regeneration` (life ring / soft boots)
- [x] `sendIcons` (0xA2) + stealth-ring empty-outfit visibility announce
- [x] Wire via `fire_on_player_equip` / `fire_on_player_deequip` + login hydrate
- [x] Tests: BoH, sword ring, time ring transform, life/energy/stealth/dwarven rings

## Live spider / name-bar / cast turn bugs (2026-07-13) — in progress
- [x] **Monster fire attacks not casting** (2026-07-16): `length`+`spread` fire wave fell through to `SpellShape::Actor` (self-cast, invisible on fire-immune dragons); `target+radius` fireball mapped to `Victim` (single target) instead of `Destination` (area); `areaeffect` parsed but never broadcast (`parse_area_effect_name` stub). Fixed shape detection, 772 `AngleShapeSpell` cone by facing direction (`spread*10`→Angle, `length`→Range), Destination circle, and `CONST_ME_*` area-effect broadcast. See lesson 176.
- [x] Rock soil OTB: classic IDs (107, 170+) match objects.srv; items.xml misnames shallow water 861–864 / mountains 4411+ as "rock soil"
- [x] **Rock soil live bug:** OTBM "rock soil" 4411–4421 = srv mountains; blanket `blockSolid` blocked players ("not enough room") — cleared Bank+Unpass+wp0 solid; non-Bank Unpass stay solid; Clip speed0→150; FillMap wp0→−1
- [x] Name/HP flicker: both-visible `!fully_sent` was appear-without-remove → remove+appear; always `mark_creature_fully_sent` on appear
- [x] Cast turn-dance: `after_creature_move` skips CASTING; Destination/Victim cast Rotate gated on `walk_timer_idle`
- [x] Idle Rotate 0x6B spam: Attack then Rotate; suppress wire turn when Go pending (lesson 169)
- [ ] Retest live: giant spider diagonals / run-turn-face / rock-soil feel after **server restart** (reload patched `items.otb`)
- [x] Follow-up: `SpellImpact::Field` — poisonfield/firefield/energyfield parse + `CreateField` via `internal_add_item_to_tile` (772 `TFieldImpact` / `magic.cc:167`)
- [x] Follow-up: summon-of-summon reparent + `SearchFreeField` nudge + aggressive PZ skip (CASTING Origin/Destination/Angle)
- [ ] Follow-up: Unmove→`!moveable` still 835 mismatches (offline patch similar to Unpass)

## OTB-only item load (no objects.srv at runtime) — done
- [x] Remove `overlay_*_from_objects_srv` from `pipeline.rs` + `sim_harness::load_items_db_for`
- [x] Confirm grass/dirt/sand WAYPOINTS from patched OTB without srv
- [x] Offline Unpass→`FLAG_BLOCK_SOLID` via `patch-otb-waypoints` (default + `--flags-only`)
- [ ] Follow-up: DistUse has no OTB bit — offline DistUse content path (was srv overlay)

## Terrain-weighted TW-1..TW-3 fixes (2026-07-13) — done
- [x] TW-1 — `truncate_tshortway_go_queue` stops at cheb≤1 only; dist band via MaxSteps
- [x] TW-2 — FillMap keeps `MinWaypoints=1000` when viewport has no positive wp
- [x] TW-3 — dist budget `(cheb−td).max(0)`; empty truncate after reachable path = OK
- [x] Rename `truncate_cipsoft_chase_queue` → `truncate_tshortway_go_queue`
- [x] Tests + `rtk cargo test -p tfs-rust-core --lib`

## Monster movement audit pass 4 — done
- [x] P4-1 — cornered `Flee` (`SearchFlightField` fail) falls through to roam (`crnonpl.cc:2754-2759` → `:2902`)
- [x] P4-2 — master-follow Manhattan bands: dist≤1→roam, dist==2 Wait-only, dist==3 Wait+Go (`crnonpl.cc:2760-2777`)
- [x] P4-3 — after `IdleStimulus`, defer all fresh batches to armed wakeup, not only `Go`-fronted (`cract.cc:789-793`)
- [x] P4-4 — remove `flee_opening_melee_dance_done` / X3 (no decompile support; `IsFleeing` checked first)
- [x] Tests + `rtk cargo test -p tfs-rust-core --lib` (622 passed)
- [ ] P4-5 deferred VERIFY — `ATTACKING` promotion gate vs `Skills[FIST]>0` (`crnonpl.cc:2779`)
- [ ] P4-6 INFO — per-type `targetDistance` vs decompile literal 4 (intentional TFS domain)

## Scheduler parity fixes (audit pass 2–3) — done
- [x] Dist: no-op `monster_on_follow_creature_moved` mid-batch wipe (`crmain.cc:920` CLOSE-only)
- [x] Wait: `CreatureAction::Wait { deadline_ms }` absolute at enqueue (`cract.cc:1033`)
- [x] LockToDo: set on ToDoStart, hold until batch drain / ToDoClear (`cract.cc:1010`)
- [x] Tests: dist mid-walk_queue + Wait absolute + full `tfs-rust-core --lib` green

## Monster combat E0 — runtime combat data on `Monster` — done
- [x] `creature/monster_combat.rs`: `MonsterSpell`, `SpellShape`, `SpellImpact`, `combat_from_monster_type`
- [x] Extend `MonsterAiConfig` + `Monster`; `from_monster_type`; wire `spawn_monster`
- [x] Runtime lookups: `monster_has_melee_attack_spell`, `monster_can_use_attack` use on-monster data
- [x] Tests: `test_e0_rat/cobra/spawn/unknown_spell/runtime_spell_range`

## Monster combat E1 — combat state machine — done
- [x] `MonsterState` enum + `state` field on `Monster` (772-only transitions)
- [x] `monster_idle_reset_combat_state` (`crnonpl.cc:2387`) + `monster_idle_maybe_enter_attacking` (`crnonpl.cc:2705`)
- [x] `monster_idle_is_attacking_posture` reads `state == Attacking|Panic`
- [x] `monster_set_idle` bridge: `Sleeping` ↔ `Idle` on 772
- [x] Tests: `test_e1_melee_monster_enters_attacking_on_idle`, reset/under_attack_promoted/no_melee/panic

## Monster combat E5 — DamageStimulus / PANIC flee — done
- [x] `combat_execute_with_stimulus` → `monster_damage_stimulus` on HP loss (`crnonpl.cc:2278`)
- [x] PANIC/UNDERATTACK state rules + storm-guarded `creature_todo_yield` (`cract.cc:1001`)
- [x] ~~`is_fleeing()` includes `MonsterState::Panic`~~ **Corrected §20:** PANIC ≠ flee; `IsFleeing()` is HP-threshold only (`crnonpl.cc:3136`); panic → melee_dance + PANIC→ATTACKING after dance
- [x] `chase_debug::log_damage_stimulus` sim event
- [x] Tests: `test_e5_idle_with_target_hit_becomes_under_attack`, sleeping→panic+yield, `test_e5_panic_dances_without_low_health`, rehit storm guard

## Monster combat E6 — loot-on-spawn / death drop / equipment stats / race exp — done
- [x] `MonsterOutfit.corpse_id` parse from `<look corpse=>` ([monsters.rs](../crates/tfs-rust-content/src/monsters.rs))
- [x] `monster_inventory.rs`: spawn loot roll, bag/equip routing, `effective_monster_combat_stats`, `drop_monster_corpse_772`
- [x] `spawn_monster`: copy `experience`/`corpse_id`, roll loot + recompute stats (772, no master)
- [x] `combat_execute_with_stimulus`: HP≤0 → `apply_creature_death`
- [x] `death.rs`: race `experience` + `distribute_experience`; generic corpse only on 1098
- [x] Tests: `test_e6_rat_experience_on_death`, corpse loot, armor/weapon stats, summon no-loot

## Combat-complete sim harness (E0–E6 battery) — done
- [x] `monster_load_type` data-pack spawn + loot roll in `chase_kite_sim` / `sim_harness.rs`
- [x] Scenario verbs: `monster_state`, `player_damage`, `player_damage_monster`
- [x] JSONL: `creature_death`, `ranged_hit`; C++ `spell_cast`/`damage_stimulus`/`creature_death` hooks
- [x] Scenarios: `kite_cobra_poison`, `kite_rat_panic`, `kite_rat_kill` + `monster_load_type` on legacy three
- [x] `scripts/run_sim_battery.py`; `summarize_chase_gaps.py` E4–E6 events
- [x] Full A/B battery §19 — logs under `log/summary_*.txt`; lockstep 0/6 (E5/E6 events fire on kill)

## Phase 1 — Stand + panic lockstep (3/6) — done
- [x] P1.1 Trace appear→first branch; document §22.1
- [x] P1.2 `kite_monsters_appear_batch` + `harness_defer_appear_idle` + 2000 ms idle defer
- [x] P1.3 `sim_melee_defense=5`, fist skill defense probe, signed `sim_rand_mod`
- [x] P1.4 PANIC→ATTACKING on first melee hit; panic lockstep PASS
- [x] P1.5 Battery 3/6 (kill + stand + panic); §22 + trajectory §22 + lesson 53

## Phase 2 — Cyclops quad lockstep (4/6) — **DONE**
- [x] P2.1 Trace cyclops appear→chase; extend `harness_defer_appear_idle` through kite window; §23.1
- [x] P2.2 Cadence verify: 0 branch@0, 4× todo_go@2000, 20/20; battery 3/6 PASS
- [x] P2.3 Cyclops geometry path tests + `compare_chase_pathfinding.py` scenarios
- [x] P2.4 TShortway expand (`cract.cc:158-202`), remove goal-band trim, walk_queue LIFO push, `monster_tshortway_fill_walkable`
- [x] P2.5 Lockstep — **shortway 4/4** + **go_exec 4/4** (Rust trace); battery **4/6** pending C++ A/B (P2.5f)
  - [x] P2.5a Rust dump — `dump_tshortway_fill_walkable_viewport` + `cyclops_quad_nw_fill_walkable_dump_at_tick_2000`
  - [x] P2.5b C++ dump — `ChasePathLogFillMap` @tick=2000 (`chase_path_debug.cc`, local `cract.cc`)
  - [x] P2.5c `scripts/compare_fill_walkable.py`; priority tiles match after OTBM overlay fix
  - [x] P2.5d OTBM+synthetic overlay (`overlay_synthetic_ground_in_arena` / `beat_driven_world_for_kite_synthetic`); `monster_tshortway_fill_walkable` state/KickCreatures/self-tile gates; `cyclops_quad_nw_and_far_n_shortway_match_live_ref` unignored
  - [x] P2.5e NW `go_exec` @4000 — `earliest_walk_server_ms` + `ToDoStart` min-1ms; appear-defer remaining-ms; `cyclops_quad_nw_go_exec_at_tick_4000`
  - [x] P2.5f Full battery C++ A/B rerun — **3/6** (stand/panic/kill PASS); cyclops **FAIL** — `go_exec` 4/4 counts but **2/4** pairwise (E vs far-N drain order swap @4000); kite/cobra unchanged FAIL

## Phase 3 — Kite rat (5/6) — DONE

- [x] P3.1 `advance_ms 2000` in `kite_rat_melee.scenario` (`wall_ms=6000`)
- [x] P3.2 `CreatureMoveStimulus` `LockToDo` gate; harness drain-before-teleport; `TDGo` segment pacing; attack defers when `todo.has_go()`
- [x] P3.3 Kite lockstep PASS + battery **5/6** (cobra still open)
  - [x] P2.5g Multi-monster `go_exec` drain order — `WakeupTiePolicy` (`HarnessAppearIdle` LIFO vs `HarnessGoStep`); cyclops lockstep **4/4**; battery **4/6**
  - Docs: divergence §26, trajectory Phase 2 closeout

## Extended battery — hunter + dragon flee (non-gating) — done
- [x] Harness: preserve XML combat stats on `monster_load_type` unless scenario line overrides (`chase_kite_sim.rs`)
- [x] `kite_hunter_dist_chase.scenario` — dist_chase + spell_cast while player kites east
- [x] `kite_hunter_dist_flee.scenario` — dist_flee when player closes inside cheb &lt; 4
- [x] `kite_dragon_lowhp_flee.scenario` — `player_damage 725` → HP 300 → `flee` branch (`runonhealth`)
- [x] `run_sim_battery.py --extended` (9 scenarios); C++ races verified: `runtime/mon/hunter.mon`, `dragon.mon`
- [ ] Lockstep PASS on extended scenarios (follow-on parity work)

## Extended battery follow-on (X1)
- [x] X1 — prevent synchronous `Go` execute on the same idle-drain tick when `IdleStimulus` already armed a future `next_wakeup`; keep first dist_chase `go_exec` deferred to the next beat (`walk/mod.rs` `process_creature_todo`).
- [x] X2 — align 772 `todo_go` trace contract with the actual chase/flee branch budget: dist-chase logs `max = cheb - target_distance`, while flee logs single-step `must=true, max=INT_MAX` (`creature_todo.rs`).

## Real-map pilot — P5 combat tail lockstep (2026-06-26) — **DONE**

- [x] P4 verify — unit tests + rust-only `one_real` smoke (`go_exec` @ 400/2000/4000).
- [x] `attack_enqueue` — `idle_tail` when `skip_idle_melee_chase` + close chase skipped (`idle_stimulus.rs`).
- [x] `melee_hit` — harness glibc RNG realign before first strike (`monster_ai.rs`, `sim_glibc_rand.rs`).
- [x] Harness drain order — `drain_todo_queue_once` before walk, `run_sim_tick` after (`chase_kite_sim.rs`).
- [x] Lockstep **PASS** on `kite_cyclops_one_real` + battery (`run_realmap_sim_battery.py`).
- [x] Docs — trajectory §14, divergence §33, archived `log/realmap_pilot_20260626_*`.

## Real-map P6 — expanded trace gate (2026-06-27)

- [x] Expanded JSONL: `idle_stimulus`, `todo_wait`, `rotate`, `creature_move_stimulus`, `todo_label` (Rust + C++ hooks).
- [x] Chase/face inline repath on target flee (`monster_events.rs`, `idle_stimulus.rs`).
- [x] Lockstep compare registry — 15 event types (`compare_chase_live_logs.py`, `summarize_chase_gaps.py`).
- [x] Real-map battery re-run; gap tables in `log/summary_realmap_cyclops_*.txt`, trajectory §15, divergence §34.
- [x] **G1** — Stop per-walk inline repath during harness U-loop (`monster_close_chase_batch_in_flight`; `todo_go` 1 vs C++ 1).
- [x] **G2/G3** — Restore `go_exec` @2000 and `melee_hit` @4000 tick buckets.
- [x] **G4** — Normalize `creature_move_stimulus` kind in compare; Rust logs `move_stimulus` on follow-target move.
- [x] **G5** — Split gate: `--movement-core` includes scheduler trace; exclude `todo_label` from lockstep.
- [x] **G6** — Scheduler trace parity on `one_real` (idle_stimulus/todo_wait/rotate @ fresh A/B).

### P6 — Real-map ramp (deferred)

- [x] **`kite_cyclops_two_real`** — scenario + first A/B baseline; AI parity plan in trajectory §16.4 (T1–T6).
- [ ] Ramp `kite_cyclops_six_real` to 6 monsters; verify branch/roam under load.
- [ ] Second real-map scenario (Thais flat control).
- [ ] Optional: live repro + `compare_chase_live_logs.py`.
- [ ] Real-map rows in CI gate (after six-monster validation).

## Audit Phase 1 — Push / collision rewrite (772) — in progress

Source: `docs/MONSTER_AI_772_AUDIT.md` "Phase 1". C++ ref: `crnonpl.cc:2141` `MovePossible`,
`:2994` `KickBoxes`, `:2984` `CanKickBoxes`, `:3036` `KickCreature`, `:2890-2898` IdleStimulus
`EXHAUSTED` catch.

- [x] Step 1 — gate creature kick on `State∈{ATTACKING,PANIC}` + `Target` + `KickCreatures`, never kick target/master/NPC/unpushable (pre-existing uncommitted work in `monster_push.rs`).
- [x] Step 2 — deterministic N,S,W,E `KickCreature`, kill on all-offsets-fail with full-HP damage attributed to the kicker (kill credit/corpse/loot/exp via `combat::execute` → `apply_creature_death`) (pre-existing gate; kill-attribution added).
- [x] Step 5 — `monster_tshortway_fill_walkable` plans through kickable creatures, hard-blocks for non-`KickCreatures` movers, handles UNPASS/AVOID boxes (verified `monster_ai.rs:2642`).
- [x] Step 3 — `KickBoxes` (UNPASS/AVOID movable items) + `CanKickBoxes()` master inheritance; deterministic N,S,W,E to `BANK && !UNPASS`, delete on failure.
- [x] Step 4 — push returns `MonsterKickOutcome`; player tile (clear `Target`) or kick-kill → `EXHAUSTED` → `Target=0; ToDoClear; Wait(1000); ToDoStart` instead of clear-queue+replan.
- [x] Tests — `kicker_onto_player_tile_is_exhausted`, `non_kicker_onto_player_tile_proceeds`, `kicker_onto_own_target_tile_proceeds`, `exhausted_wait_clears_target_and_waits_1000`, `can_kick_boxes_inherits_from_master`, `boxed_in_blocker_is_killed_and_step_exhausted` (box-move itself needs real item data — covered by real-map battery + `fillmap_movepossible_blocks_unpass_under_grass`).
- [x] Gate — `rtk cargo check` + `clippy` clean; `monster_push` 5/5. (Pre-existing unrelated failures: `test_e4_spell_delay_gate`, `test_e4_cobra_poison_at_range`, `test_772_dist_target_flee_inline_chase_after_goal_wait` — glibc-RNG harness, audit Finding 15; fail on HEAD without these changes.)


## Audit Phase 2 — 772 line-of-sight (`ThrowPossible`) — DONE

C++ ref: `info.cc:1154` `ThrowPossible` (Power=0 for all monster/combat callers, `crnonpl.cc:2798`).
- [x] `tile.rs` — add `UNTHROW`/`HOOKEAST`/`HOOKSOUTH` flag bits (24–26).
- [x] `map/mod.rs` — aggregate `UNTHROW` from `block_projectile()`, hooks from `is_hangable()`+`is_horizontal/vertical()`.
- [x] `map/los.rs` — `Map::throw_possible(orig,dest,power)`: major-axis interpolation + `UNTHROW` + multi-floor `MinZ` step + HOOK `StartT=0`.
- [x] `GameWorld::monster_sight_clear` dispatcher (772 → `throw_possible`, else Bresenham `is_sight_clear`); rerouted monster/combat sight checks (monster_targets, monster_ai dist-branch + ranged, idle_stimulus spell + trace, creature_think).
- [x] Tests — `tests/map_los.rs`: open clear, UNTHROW blocks, solid-but-throwable passes (16b), adjacent clear.
- [x] Gate — `rtk cargo check` + `clippy` clean; map_los 7/7.

## Audit Phase 3 — chase leash + roam bounds — DONE

C++ ref: `crnonpl.cc:2148-2167` `MovePossible` radius block; `:1515` `MonsterhomeInRange`; `:2407` despawn.
- [x] Finding 17 — `monster_can_occupy_chase_tile` + `monster_tshortway_fill_walkable` skip the leash for ATTACKING/PANIC (chase out of range; despawn via existing out-of-range path).
- [x] Finding 17b — per-monster `home_radius` (from spawn zone `radius` in `spawn_monster`); roam (non-attacking) leash uses it via `monster_roam_leash_radius` (falls back to global despawn radius when unset / on 1098).
- [x] Tests — `chase_leash_skipped_when_attacking_bounded_when_roaming`, `roam_leash_falls_back_to_despawn_radius_when_home_unset`.
- [x] Gate — `rtk cargo check` + `clippy` clean; new tests pass.

> Note: `test_772_dist_dance_enqueues_go_and_wait` (and `test_e4_*`, `test_772_dist_target_flee_*`) are
> **pre-existing non-deterministic tests** — they draw from `ai_rng = StdRng::from_entropy()` when
> `TFS_SIM_SEED` is unset (plain `cargo test`), so they pass/fail at random regardless of these
> changes (audit Findings 8/15; fixed by Phase 5 RNG unification). Verified flaky in isolation both
> ways; all new P2/P3 tests are deterministic and pass.


## Audit Phase 4 — decision-tree constants — scope revised

- Finding 1 (distance band) — **no code change.** Shipped data is uniform (`targetdistance` 4 for all
  distance monsters, 1 for melee), so `DistanceKeep::PerType` already matches the 772 hardcoded-4
  behavior; pinning `Fixed(4)` would mis-classify melee monsters carrying a ranged spell. Optional
  warn-only load guard for `targetdistance != 4` distance-fighters (not required).
- [x] Finding 2 — remove `if cast_any { break; }` in `monster_idle_try_casting` (`idle_stimulus.rs`);
  evaluate + cast every spell whose gates pass per idle. Fixes multi-spell-per-idle + glibc RNG desync.

## Audit Phase 5 — RNG unification (sim parity) — partial (9/10/14/19 done)

C++ ref: `common.hh:206` `RandomShuffle`; `info.cc:1030` `SearchFlightField`; `magic.cc:776` `ComputeDamage`; `info.cc` `SearchSpawnField`.
- [x] Finding 9 — `sim_glibc_rand::parity_random_shuffle` (forward Fisher-Yates over `parity_random`); used for both `SearchFlightField` sub-slices. `search_flight_field` no longer takes `rng`.
- [x] Finding 10 — roam draws `parity_rand_mod(4)` (was `ai_rng.gen_range(0..4)`).
- [x] Finding 14 — monster spell damage/heal/speed variation draws `parity_random` on 772 (Condition arm already did).
- [x] Finding 19 — spawn tile tie-break draws `parity_random(0,99)` (was `thread_rng`).
- [x] Tests — `parity_random_shuffle_is_permutation`; flight_field/roam/spawn suites green; clippy clean.
- [ ] Finding 8/15 (deferred) — retire `ai_rng` from the 772 path, delete `TFS_SIM_MELEE_REALIGN` hack,
  per-`GameWorld` glibc generator. Requires re-baselining the C++-oracle golden RNG traces
  (`run_sim_battery.py`), which needs the CipSoft harness — out of band here. Hack is inert outside
  seeded harness runs, left in place until re-baseline.

## Audit Phase 6 — Lifecycle polish (respawn / summon / talk) — DONE

C++ ref: `crnonpl.cc:1296` `StartMonsterhomeTimer`; `:2359–2405` summon despawn/re-bind; `:2442–2458` Talk.
- [x] Finding 18 — `RespawnModel::Monsterhome772` in `MechanicsProfile`; `compute_respawn_delay_ms`
      in `spawn_lifecycle.rs` — `random(regen/2, regen)` + player-count scaling (`>800 → *2/5`,
      `>200 → *200/(n/2+100)`). 1098 stays `Fixed`. Lua keys in `data/formulas/{772,1098}.lua`.
- [x] Finding 20 — `monster_idle_summon_lifecycle_772` in `idle_stimulus.rs` — master gone / floor
      change / >30 tiles → despawn; re-bind `Following ? Target=0 : Target=AttackDest`, fallback
      `Target=Master`. Runs before sleeping/idle check (C++ ordering).
- [x] Finding 3 — `monster_idle_try_talk` emits `0xAA` via era-aware `Codec::encode_creature_say`
      (772 omits `level`). `#y `/`#Y ` prefix → `TALKTYPE_MONSTER_YELL=0x10`, else `MONSTER_SAY=0x11`.
      Talk texts from `<voices><voice sentence="…"/></voices>` in monster XML (`monsters.rs`).
- [x] `CreatureSayWire` + `ProtocolCodec::encode_creature_say` in `tfs-rust-net` codec (1098 with
      `level`, 772 without). `broadcast_creature_say_viewport` refactored to codec path + monster
      speaker support.
- [x] Tests — 10 new: respawn band/scaling/fixed (4), summon despawn/rebind (5), talk emit/no-emit (2).
      399 existing tests still pass. Clippy clean.

## 772 Monster AI audit — Phase 8: CreatureAction::Rotate + atomic Execute drain — done
- [x] `creature_todo.rs`: added `CreatureAction::Rotate { target_id }` (mirrors C++ `TDRotate`
      `cract.cc:818`), `CreatureTodo::has_rotate`, `GameWorld::enqueue_creature_rotate`.
- [x] `idle_stimulus.rs`: added `TodoExecuteKind::Rotate`; `CreatureAction::Rotate` arm in
      `execute_creature_todo_action` → `monster_execute_rotate_toward` (NO `walk_timer_idle`
      gate — matches C++ unconditional `Rotate(Target)` `cract.cc:452`); routed `Rotate` through
      the `Go|Attack` dispatch in `run_monster_todo_execute` (atomic drain via
      `finish_creature_todo_execute` tail-recursion — semantically equivalent to C++ `while(true)`
      `Execute` `cract.cc:783-898`, bounded by the `+1` clamp).
- [x] `idle_stimulus.rs`: `monster_idle_rotate_toward_attack_target` now enqueues
      `CreatureAction::Rotate { target_id }` instead of calling the gated
      `monster_update_look_direction` (AI#22 fix — rotate no longer skipped when walk armed).
- [x] `monster_ai.rs`: doc note on `monster_update_look_direction` — ATTACKING/PANIC rotate path
      now uses the enqueued action; this function retains its `walk_timer_idle` gate for the
      casting turn + 1098 `onThink` path.
- [x] Tests — 3 new: `test_phase8_rotate_then_attack_fires_in_one_beat` (direction + HP + no
      server_ms advance in one call), `test_phase8_rotate_enqueued_when_walk_armed`,
      `test_phase8_idle_tail_enqueues_rotate_before_attack`. 408 tests pass, clippy clean.

## 772 Monster AI audit — Phase 9: Z-level target clear + move-stimulus fixes — done
- [x] `monster_events.rs`: AI#24 — gated Z-level target clear on `!beat_driven_loop` (1098 only);
      772 monsters keep targets across ramp drops (C++ `CreatureMoveStimulus` `crmain.cc:920` does
      NOT clear on Z-change). `!target_visible` clear stays for both eras.
- [x] `idle_stimulus.rs`: AI#25 — added `IsHouse(target_pos)` (`crnonpl.cc:2427`, via
      `matches!(tile, Tile::House(_))`) and `target.is_invisible() && !see_invisible`
      (`crnonpl.cc:2429`) to `monster_idle_772_should_lose_target`, after the Protection zone
      check, matching C++ order.
- [x] `monster_events.rs`: AI#26 — replaced `!has_attack || has_go` (whole-queue scan) with
      head-only check `todo.queue.front() == Some(&CreatureAction::Attack)` in
      `monster_combat_creature_move_stimulus`, matching C++ `crmain.cc:931-932`
      `ToDoList.at(ActToDo)->Code == TDAttack`. `[Attack, Go]` now correctly fires combat re-arm.
- [x] `monster_events.rs`: GL#24 — added `NOTE(parity)` comment documenting 16×16 (C++
      `TFindCreatures`) vs 64×64 (Rust `CHUNK_SIZE`) chunk granularity divergence; creation-order
      sort kept as deterministic fallback (audit's explicit fallback path).
- [x] Tests — 7 new: `test_phase9_772_keeps_target_across_z_change`,
      `test_phase9_1098_clears_target_across_z_change`,
      `test_phase9_772_loses_target_entering_house`,
      `test_phase9_772_loses_invisible_target_without_see_invisible`,
      `test_phase9_772_keeps_invisible_target_with_see_invisible`,
      `test_phase9_move_stimulus_fires_when_head_is_attack`,
      `test_phase9_move_stimulus_skips_when_head_is_go`. 446 tests pass, clippy clean.

## Walk-engine unification Phase 1 (1.4–1.7) — 772 player parity — in progress

Source: `tasks/walk-engine-unification.md` Phase 1.4–1.7. C++ ref (mechanics):
`tibia-game-master/src/` `crcombat.cc:357-522` (`SetAttackDest`/`CanToDoAttack`/`StopAttack`),
`receiving.cc:1133-1155` (`CAttack`), `crplayer.cc:388-405` (`IdleStimulus`),
`cract.cc:392-413` (drunk stagger), `cract.cc:953-1008` (`ToDoClear`/`ToDoStop`).
Wire ref (772): `gameserver/src/protocolgame.cpp:1485-1490` `sendCancelTarget` (`0xA3`).

- [x] 1.4a — `encode_clear_target` (`0xA3`) on `ProtocolCodec` + `Codec772`/`Codec1098` + delegate.
- [x] 1.4b — `player_combat.rs`: `player_set_attack_dest` / `player_stop_attack` /
      `player_cancel_attack_and_follow` / `player_can_to_do_attack_chase` (close chase via
      `get_creature_path_to`; strike deferred — no player weapon damage yet).
- [x] 1.4c — `game_loop.rs`: `Attack`/`Follow`/`CancelAttackAndFollow`/`FightModes` arms
      (FightModes sets `base.chase_mode` from `raw_chase_mode`: 0=None, 1=Close).
- [x] 1.4d — `idle_stimulus.rs`: player `Attack` execute via `player_can_to_do_attack_chase`;
      `player_idle_stimulus` thrown-`RESULT` path (`ToDoClear` + `SendResult` + `ToDoWait(1000)`).
- [x] 1.5 — CipSoft drunk formula (`max(7-DrunkLevel,1)`, `rand%chance==0`) in `walk/mod.rs`;
      `CreatureAction::Talk` variant + execute arm; stagger → `ToDoClear` + snapback +
      `ToDoTalk("Hicks!")` + `ToDoStart` + random cardinal step.
- [x] 1.6 — Delete "floor ×2" comment in `walk_timing.rs:198`; re-point 772 mechanics comments
      to decompile refs; verify `ground_speed_for_item` returns BANK `WAYPOINTS` on 772.
- [x] 1.7 — Player ToDo/idle tests: walk, autowalk, blocked step, attack/follow chase, cancel,
      drunk stagger. 11 new `test_phase1_*` tests; 468 total pass.
- [x] Verify — `rtk cargo check` / `clippy` / `test -p tfs-rust-core` (468 pass) /
      `-p tfs-rust-net` (95 pass); no new clippy warnings in changed files.

## 772 Monster Audit Verification — Step 4 (M5) + Step 5 (N2)  [2026-07-01]

Source: `docs/TFS-RUST_772_Monster_Audit_Verification.md` §7 Step 4 & Step 5.

- [x] Step 4 — M5: align AI Z-visibility with 772 `TConnection::IsVisible` (`connections.cc:357-378`).
      Added `beat_driven_loop: bool` param to `creature_can_see`; 772 drops the `tz < 8`
      underground→surface rejection (only `abs(dz) > 2` rejects). 1098/TFS `canSee` path unchanged.
      Updated 8 call sites (monster_events ×4, monster_ai ×1, monster_targets ×3) to pass
      `self.beat_driven_loop`. `protocol_can_see` (client viewport) untouched.
      Test: `creature_can_see_underground_to_surface`.
- [x] Step 5 — N2: F1 regression coverage. `hard_block_reruns_idle_next_beat` (monster) +
      `hard_block_player_stops_cleanly` (player) drive `on_walk` directly to assert the Err-arm
      contract: chase `Go` cleared, `locked=false`, `Wait{0}` enqueued, `next_wakeup == server_ms+1`,
      `walk_queue` empty. Optional direct-`creature_todo_yield` hardening deferred (guards traced OK).
- [x] Verify — `rtk cargo test -p tfs-rust-core` (482 pass, 0 fail); `rtk cargo clippy` no new
      warnings in changed files. Audit doc §6/§7 + cross-cutting checkboxes ticked; lessons #93/#94
      appended to `tasks/lessons.md`.

## Refactor Audit Phase 1 — Extract inline tests from mega-files  [2026-07-01]

Source: `docs/REFACTOR_AUDIT.md` §"Phase 1". Pure code-movement; no logic/renames.

- [x] Baseline captured: `rtk cargo test -p tfs-rust-core` → 482 passed, 2 ignored, 12 suites.
- [x] `idle_stimulus.rs` (6809→2511): `mod tests` body → `idle_stimulus_tests.rs` via
      `#[cfg(test)] #[path] mod tests;`. Module path `crate::idle_stimulus::tests::*` preserved.
- [x] `monster_ai.rs` (4758→2843): two test mods → `monster_ai_tests.rs` (`mod tests`) +
      `monster_ai_world_tests.rs` (`mod world_tests`), both via `#[cfg(test)] #[path]`.
      Kept separate to preserve `crate::monster_ai::{tests,world_tests}::*` filter paths.
- [x] `pathfinding.rs` (2261→1230): `mod tests` → `pathfinding_tests.rs`.
- [x] `sim_harness.rs` (2441→1788): `mod harness_tests` → `sim_harness_tests.rs`.
- [x] `>50%-test` sweep: `todo_queue.rs` (293→124), `monster_push.rs` (1545→639),
      `spell.rs` (299→137), `creature_think.rs` (539→256) — each `mod tests` → `*_tests.rs`.
      `test_world.rs` already fully `#[cfg(test)]` — left alone.
- [x] Verify — `rtk cargo test -p tfs-rust-core` → 482 passed, 2 ignored, 12 suites (identical).
- [x] Verify — `cargo clippy --all-targets` full `^warning:` set byte-identical before/after
      (46 warning lines / 44 unique in both). **Zero regression.**
      NOTE: `rtk cargo clippy` aggregated output shows a non-deterministic subset per run
      (files never touched appear/disappear) — do NOT diff rtk summaries; use raw
      `cargo clippy ... | grep '^warning:' | sort` for reliable before/after comparison.
- [x] Exit criteria: `idle_stimulus.rs` ≤~2500 (2511), `monster_ai.rs` at audit-measured prod
      LOC ~2835 (2843). Test pass count identical.

## Refactor Audit Phase 2 — Quarantine simulation/debug code  [2026-07-02]

Source: `docs/REFACTOR_AUDIT.md` §"Phase 2". Gate diagnostic/sim modules behind `sim` cargo feature.

- [x] Add `sim = []` cargo feature to `tfs-rust-core/Cargo.toml` (default off).
      `chase_kite_sim` bin: `required-features = ["sim"]`. `path_compare` bin: no feature needed.
- [x] Gate `pub mod sim_harness` in `lib.rs` with `#[cfg(any(test, feature = "sim"))]`.
- [x] `chase_debug.rs` — split into no-op stubs (`#[cfg(not(any(test, feature = "sim")))]`)
      + full implementation (`#[cfg(any(test, feature = "sim"))]`). Stubs have identical
      signatures; `chase_path_debug_enabled() → false` eliminates all log call branches.
- [x] `sim_glibc_rand.rs` — always-compiled: `GlibcRngState`, `DANCE_DIR_ORDER`,
      `parity_random/rand_mod/random_shuffle` (sim branch gated), `sim_glibc_rng_enabled`
      (returns `false` in production), `SimRngTraceSiteGuard` + `sim_rng_trace_site` (no-op).
      Feature-gated: `sim_random`, `sim_rand_mod`, `SimGlibcRng`, `enable_sim_glibc_rng`,
      `resync_harness_glibc_rng_from_env`, trace functions, `sim_probe_random_factor`,
      `harness_melee_realign_*`, `draw_rand`.
- [x] Production call sites — `#[cfg(any(test, feature = "sim"))]` on sim-only branches:
      `combat/math.rs` (2: probe factor + armor roll), `creature/monster_combat.rs` (1: poison),
      `monster_ai.rs` (1: melee realign block), `game_world.rs` (3: init_sim_rng, resync, dance).
- [x] Exposed dead code gated: `spawn_placement.rs` (`search_login_field`, `spiral_login_positions`,
      `harness_place_creature_login`), `game_world.rs` (`player_apply_spell_exhaust`,
      `parity_random_shuffle` method), `sim_glibc_rand.rs` (`parity_random_shuffle` free fn).
- [x] Verify — `cargo check -p tfs-rust-core` (default) → 0 errors, no sim code compiled.
- [x] Verify — `cargo test -p tfs-rust-core --lib` → 481 passed, 2 ignored (identical to baseline).
- [x] Verify — `cargo check -p tfs-rust-core --features sim` → 0 errors.
- [x] Verify — `cargo check --bin chase_kite_sim --features sim` → 0 errors.
- [x] Verify — `cargo check --bin path_compare` (no sim) → 0 errors.
- [x] Verify — `cargo check --bin chase_kite_sim` (no sim) → correctly fails (feature required).
- [x] Verify — clippy net warnings **decreased**: lib 44→29, test 76→75. No new unique warnings.
- [x] Lessons: appended #100 to `tasks/lessons.md`.

## Map System Audit — Phase 2: Storage invariants (findings #3, #7)

Source: `docs/MAP_SYSTEM_AUDIT.md` §Phase 2. Targets MED #3 (void-tile registration loss)
and LOW #7 (dual creature-list desync).

- [x] **#3 Void-tile registration.** `Map::register_creature_at` currently silently drops a
      creature when the target tile/chunk is absent (the `if let Some(t)` skips the tile list,
      and `grid.register_creature` no-ops on a missing chunk). Fix: detect the void case and
      `tracing::error!` (release) + `debug_assert!` (debug/test) — never panic in release per
      `tfs-packets.md` validation rules. Same treatment for `unregister_creature_at` at
      `warn` level (less harmful but still indicates untracked state).
- [x] **#7 Single placement path.** Restrict `SparseGrid::register_creature` /
      `unregister_creature` from `pub` to `pub(super)` so only `map/mod.rs` (the `*_at`
      wrappers) can call them. The grid's own `#[cfg(test)] mod tests` retains access as a
      child module. Add `SparseGrid::debug_assert_creature_lists_agree` (delegated through
      `Map`) that checks every `Chunk.creatures` entry is on some tile in the chunk and vice
      versa — `debug_assert!`-gated so it compiles out in release.
- [x] **Tests** (`tests/map_storage.rs` + `map/grid.rs` `mod tests`):
  - `register_creature_at_on_void_tile_is_noop` — register on a position in an absent chunk;
    debug panics, release logs+drops; creature absent from both lists.
  - `unregister_creature_at_on_void_tile_is_noop` — no panic, no state change.
  - `creature_lists_agree_after_batch_of_moves` — register/move/unregister batch + consistency check.
  - `debug_assert_catches_chunk_list_creature_not_on_tile` / `debug_assert_catches_tile_list_creature_not_in_chunk`
    (grid unit tests, `#[cfg(debug_assertions)]`) — confirm the checker trips on each desync direction.
  - `debug_assert_passes_on_clean_grid` — clean grid does not trip.
- [x] **Harness invariant fix.** Added `ensure_walkable_tile_if_absent` to central harness
      `insert_*` / `player_walk` paths — 10 pre-existing tests silently relied on the old
      silent-drop. Does NOT overwrite intentional tiles.
- [x] **Verify:** `cargo test -p tfs-rust-core --lib` → 468 passed, 2 ignored (was 458+10
      failed); all 9 integration binaries pass; `cargo test -p tfs-rust-net` → 98 passed;
      clippy clean on changed files.
- [x] **Lessons:** appended #98 to `tasks/lessons.md`.
- [x] **Docs:** marked Phase 2 ✅ in `docs/MAP_SYSTEM_AUDIT.md` summary table + Phase 2 header.

## F8 D3 — `Turn` walk-to-reach (decompile parity audit) — done
- [x] `idle_stimulus.rs` `CreatureAction::Turn` execute arm: mirror the S5 `Go`-prepend
      pattern from the `Move` arm — if `obj.pos.x != 0xFFFF` and `dx>1 || dy>1`,
      `setup_player_walk_to_target` + push `[Go, Turn]` + `todo_start_go_delay`/schedule;
      on `Err(rv)` apply `apply_todo_result_catch`. C++ ref: `cract.cc:1340-1341`
      `ObjectInRange(1)` → `ToDoGo(...)`.
- [x] Update `enqueue_player_turn` doc comment to note walk-to-reach is in the execute arm
      (same shape as `Use`/`Move`).
- [x] Tests: `s5_turn_not_adjacent_prepends_go_and_re_enqueues_turn`,
      `s5_turn_adjacent_does_not_go_prepend`,
      `s5_turn_no_path_to_target_applies_result_catch`.
- [x] Mark D3 Fixed in `tasks/f8-decompile-parity-audit.md`.
- [x] `cargo check`/`clippy`/`test -p tfs-rust-core --lib` → 533 passed, 2 ignored
      (was 530); clippy clean on changed files (1 pre-existing warning in
      `game_world_player_throw.rs:197`, unrelated).

## 772 Splash/Pool Layer Mismatch — Runtime override (Option A) — done
- [x] **Phase 1: OTB loader override.** `crates/tfs-rust-content/src/otb.rs` `parse_node`:
      before `db.insert`, if `item.group == ItemType::GROUP_SPLASH`, clear
      `FLAG_ALWAYSONTOP` from `item.flags`. Unconditional (correct for both eras).
- [x] **Phase 2: Rewrite `create_liquid_splash_772`.** Scan `down_items` (the pool's
      layer, mirroring decompile `CreatePool` `BOTTOM` scan): existing splash → collect
      for replacement; non-splash down item (corpse/drop) → silent abort (NOROOM). Delete
      old pools, then `internal_add_item_to_tile` places new splash → routes to
      `down_items`. `top_items` (ladders/signs/borders) not scanned → blood on ladders
      works.
- [x] **Phase 3: Remove the "no splash in ladders" guard.** Delete the
      `splash_order` / `alwaysOnTopOrder` conflict check (dead code with splashes in
      `down_items`).
- [x] **Phase 4: Verify tile description / stackpos.** `get_item_stack_pos` already
      counts `top_items` before `down_items`; `map_description.rs` iterates top →
      creatures → down. Splashes moving to `down_items` renders them after creatures =
      772 `BOTTOM` order. No code change needed — confirm by reading.
- [x] **Phase 5: Tests.** Blood-on-ladder, pool replacement, NOROOM on corpse,
      death-pool-before-corpse coexistence.
- [x] **Phase 6: Audit call sites.** Confirm no regression in `top_items` iterators
      expecting splashes (`login_out.rs`, `monster_push.rs`).
- [x] **Verify:** `cargo check`/`clippy`/`test -p tfs-rust-core --lib`.
- [x] **Lessons:** append to `tasks/lessons.md`.

## Chat system CH-0 — missing outgoing wire (prerequisite) — done
- [x] Server opcodes added to `protocol_opcodes.rs::server`: `CHANNELS_DIALOG=0xAB`,
      `CHANNEL_OPEN=0xAC`, `OPEN_PRIVATE_CHANNEL=0xAD`, `CREATE_PRIVATE_CHANNEL=0xB2`,
      `CLOSE_PRIVATE=0xB3`.
- [x] Neutral wire structs in `codec/wire.rs`: `ChannelsDialogWire`, `ChannelOpenWire`,
      `CreatePrivateChannelWire` (max-width fields; 772 ignores user/invited lists).
- [x] `ProtocolCodec` trait methods: `encode_channels_dialog`, `encode_channel_open`,
      `encode_create_private_channel` + `Codec772`/`Codec1098` impls.
- [x] Wired through `Codec` enum `delegate_codec!` + `ProtocolCodec for Codec` impl.
- [x] `outgoing_extra.rs`: `send_open_private_channel`/`send_close_private` now use opcode
      constants (era-identical, kept as free fns); `send_channel_open`/`send_create_private_channel`
      marked `#[deprecated]` (1098-shaped, retained for legacy test pinning); full
      `send_channels_dialog` available via `Codec::encode_channels_dialog`.
- [x] §4.5 resolved: 1098 parity confirmed — `sendChannelsDialog`/`sendClosePrivate`/
      `sendOpenPrivateChannel` are era-identical; `sendChannel`/`sendCreatePrivateChannel`
      diverge (1098 appends user/invited name lists, 772 omits them). Divergence isolated to
      `codec::v772`/`codec::v1098` — no `if version == 772` in core.
- [x] Golden-byte tests in `tests/protocol_compat.rs`: 6 new (3 per era) covering all 5
      functions; 74 total pass.
- [x] **Verify:** `cargo check --workspace` (0 errors), `cargo clippy -p tfs-rust-net -p
      tfs-rust-common` (clean), `cargo test -p tfs-rust-net` (123 passed).
- [x] **Lessons:** append to `tasks/lessons.md`.


## Player combat PC-2 — The strike (`CloseAttack`) — melee first
**Plan:** `tasks/player-combat-plan.md` §3 Phase PC-2. 772 mechanics, TVP data shape. Reuses
`combat::math::{weapon_damage, defense_value, armor_reduction, melee_damage_after_defense_and_armor}`
+ `roll_target_defense` + `combat_execute_with_stimulus` — no parallel player combat math module.

- [x] `creature/base.rs`: add `learning_points: i32` to `CreatureBase` (C++ `TCombat::LearningPoints`,
      `crcombat.cc:526` `ActivateLearning` sets 30; `ProbeValue` decrements + `Increase(1)`).
- [x] `creature/monster_combat.rs`: add `GameWorld::melee_defense_snapshot_for(target_id)` — for
      player targets use `player_get_defend_value` (shield/weapon defend + shielding skill) +
      `player_get_armor_strength`; monsters/NPCs delegate to existing `melee_defense_snapshot`.
- [x] `monster_ai.rs`: switch both strike call sites to `melee_defense_snapshot_for` so player
      targets defend with shield/weapon/armor (was fist-only stub).
- [x] `player/combat/strike.rs` (new): `CloseAttack` body —
      `attack = weapon_damage(...) × vocation formula.melee_damage (floor)`;
      `defense = roll_target_defense(target, snapshot)`;
      `armor = armor_reduction(profile, hooks, rng, snap.armor)`;
      `dmg = melee_damage_after_defense_and_armor(attack, defense, armor)`;
      `combat_execute_with_stimulus(Some(cid), target, PHYSICAL, -dmg)`;
      `if damage_done>0: ActivateLearning (learning_points=30)`;
      `ProbeValue` side-effects: `if learning_points>0 { learning_points-=1; Increase(1) }`;
      weapon wearout (`ItemType.charges>0` → decrement);
      cadence `DelayAttack(200)` before, `DelayAttack(attack_speed_ms)` after, re-arm `TDAttack`;
      `if target dead: StopAttack`.
- [x] `player/combat/mod.rs`: `mod strike;` + replace `Adjacent`/`NoPath` re-arm arm with
      `player_close_attack_strike` dispatch (melee range `GetDistance()==1`).
- [x] `sim_harness.rs`: init `learning_points: 0` in `test_player_base`.
- [x] Tests: strike damage range, learning activation on damage_done>0, defense gate 2000ms,
      vocation `melee_damage` multiplier, target death → StopAttack, fist fallback.
- [x] **Verify:** `cargo check -p tfs-rust-core`, `cargo clippy -p tfs-rust-core --all-targets`,
      `cargo test -p tfs-rust-core`. **Lessons:** append to `tasks/lessons.md`.

## LUA-2 — Player read methods + account-type backing + Vocation object
Plan: `tasks/lua-api-plan.md` §LUA-2. Unblocks the active (uncommented) channel-hook
gating logic (`getAccountType`/`getLevel`/`getVocation():getId()`/`hasFlag`).

- [ ] `Player.account_type: u8` field (default `ACCOUNT_TYPE_NORMAL = 1`, `enums.h:80`).
      Update all 5 `Player { … }` construction sites (login.rs, sim_harness.rs,
      spell_tests.rs, tests/arena.rs, player/inventory/notifications.rs).
- [ ] DB plumb: add `account_type: u8` to `LoadedPlayerData`; load `accounts.type` in
      `load_player_full` (single extra `SELECT type FROM accounts WHERE id = ?` — fold
      into the existing `premium_ends_at` query to avoid a second round-trip); set in
      `player_from_loaded`. C++ ref: `iologindata.cpp` `gameworldAuthentication`
      `SELECT … type … FROM accounts`.
- [ ] `ScriptContext` trait: add `get_player_level`, `get_player_account_type`,
      `get_player_vocation_id`, `player_has_flag` — all default-`None`/`false` so
      `NullEventDispatcher`/tests need no change.
- [ ] `GameWorld` impl in `game_world_script.rs`: reuse `player_has_flag` (stats.rs),
      `Player.level`, `Player.vocation_id`, new `Player.account_type`.
- [ ] `Vocation` userdata (`userdata/vocation.rs`) wrapping `vocation_id: i32`, method
      `getId()` (§1.4 option a — extensible for `getName`/`getPromotion` later).
      Register metatable from `LuaRuntime::new`.
- [ ] `userdata/player.rs` bindings: `getLevel`, `getAccountType`, `getVocation`,
      `hasFlag`.
- [ ] Tests: `ScriptContext` fake-backed unit test (GM player) asserting
      `getAccountType`/`hasFlag`/`getLevel`/`getVocation():getId()` read through.
- [ ] **Verify:** `cargo check`, `cargo clippy --all-targets`, `cargo test -p tfs-rust-lua
      -p tfs-rust-core -p tfs-rust-db`. **Lessons:** append to `tasks/lessons.md`.

## Refactor Phase 3 — Rename `_772` core functions — DONE (2026-07-10)

**Goal:** comply with the `TFS-Core` naming rule (no version suffix on core fns / public APIs).

The audit's original 10 example functions (`advance_beat_772`, `process_creatures_772`, etc.)
were already renamed in prior work. The remaining `_772`-suffixed production functions across
all crates were renamed to behavior-based names:

- [x] **core:** `condition_type_from_lua_772` → `condition_type_from_lua`
      (`game_world_chat.rs`, `game_world_script.rs`)
- [x] **content:** `is_terrain_bank_772` → `is_terrain_bank`, `is_unpass_772` → `is_unpassable`,
      `is_unmove_772` → `is_immovable`, `is_avoid_hazard_772` → `is_avoid_hazard`,
      `avoid_damage_type_772` → `avoid_damage_type`, `waypoints_raw_772` → `waypoints_raw`
      (`otb.rs`, `items.rs`; call sites in `monster_ai.rs`, `monster_push.rs`)
- [x] **content:** `reference_772_objects_srv_under` → `reference_objects_srv_under`
      (`objects_srv.rs`)
- [x] **lua:** `return_value_message_772` → `return_value_message` (`userdata/player.rs`)
- [x] **net:** `send_icons_772` → `send_icons_classic` (`outgoing_extra.rs`, `login_out.rs`),
      `liquid_color_772` → `liquid_color` (`codec/v772.rs`),
      `build_login_success_772` → `build_login_success_classic` (`protocol_login_out.rs`),
      `premium_days_left_772` → `premium_days_left` (`protocol_login_out.rs`)

**Exceptions kept:** test fns (`test_772_*`, `*_reads_772`), config (`protocol_version_reads_772`),
data constants (`OTB_MAJOR_772`, `LOGIN_ERR_772`, `REF_772_DIR_NAMES`), local variables
(`is_772`, `codec_772`).

**Verification:** `cargo test -p tfs-rust-core --lib` → 585 passed, 2 ignored (identical to
baseline). Clippy 26 warnings (identical to baseline). `grep -rn "fn .*_772" crates/tfs-rust-core/src`
returns only test/config items. **Lessons:** appended to `tasks/lessons.md`.


## Player combat PC-4 — Fight/chase/secure mode + PVP gating — done
**Plan:** `tasks/player-combat-plan.md` §3 Phase PC-4. 772 mechanics (`crcombat.cc:325-593`,
`crmain.cc:433-453,536-538`). Scope decision (Q5): defer all skulls/`RecordAttack`/aggressor to a
dedicated PvP phase; implement M1 (INVULNERABLE) + M6 (BlockLogout) + M8 (SecureMode gate) +
fight/secure mode storage. M1 uses the TFS group-flag system (`PlayerFlag_CannotBeAttacked` bit 3),
not a 772 `CharacterRights` DB table.

- [x] `config.rs`: `PvpConfig { world_type: WorldType, protection_level: u32 }` with
      `from_config` parsing `config.lua` `worldType` (`pvp`/`no-pvp`/`pvp-enforced`) +
      `protectionLevel` (default 1). Wired onto `GameWorld.pvp_config`.
- [x] `creature/player.rs`: add `secure_mode: bool` + `earliest_protection_zone_round: u32`
      fields (PC-4). Updated all 4 `Player` construction sites (login, sim_harness, spell_tests,
      inventory/notifications) + `tests/arena.rs`.
- [x] `player/flags.rs`: add `PLAYER_FLAG_CANNOT_USE_COMBAT = 1 << 0` (772 `NO_ATTACK` right) +
      `PLAYER_FLAG_CANNOT_BE_ATTACKED = 1 << 3` (772 `INVULNERABLE` right); map in
      `flag_name_to_bit` (`cannotusecombat`, `cannotbeattacked`).
- [x] `player/combat/fight_mode.rs` (new): `player_set_fight_modes` (attack_mode with
      `DelayAttack(2000)` on change, chase_mode without overriding follow-forced `Close`,
      secure_mode), `player_block_logout` (EarliestLogoutRound + EarliestProtectionZoneRound,
      NoPvp clears PZ block), `player_is_attack_justified` (stub `false`), 
      `player_secure_mode_blocks_attack` (WorldType==Pvp + both players + secure + !justified),
      `player_attack_blocked_by_right` (`NO_ATTACK` flag), `player_is_invulnerable`
      (`CANNOT_BE_ATTACKED` flag). 16 unit tests.
- [x] `game_loop.rs`: `FightModes` arm now calls `player_set_fight_modes` (was chase-only).
- [x] `player/combat/mod.rs`: `validate_player_attack_target` now enforces `SecureMode` +
      `AttackNotAllowed` branches (was NPC/PZ/distance/invis only). `player_execute_attack`
      re-checks secure mode + NO_ATTACK at strike time + calls `BlockLogout(60)` on attacker +
      target. `player_set_attack_dest` `!Follow` path calls `BlockLogout(60)` on attacker.
      `CombatResult::SecureMode` no longer `#[allow(dead_code)]`.
- [x] `idle_stimulus.rs`: M1 INVULNERABLE check at top of `combat_execute_with_stimulus` —
      zeroes incoming damage + broadcasts `EFFECT_POFF` (3) when target has
      `PLAYER_FLAG_CANNOT_BE_ATTACKED`. Gated on `primary.1 < 0 || secondary.1 < 0` to avoid
      blocking heals.
- [x] `sim_harness.rs`: added `minimal_player()` + `minimal_creature_base()` test helpers.
- [x] **Verify:** `cargo check -p tfs-rust-core -p tfs-rust-content -p tfs-rust-net
      --all-targets` (0 errors), `cargo clippy -p tfs-rust-core --all-targets` (0 new warnings),
      `cargo test -p tfs-rust-core --lib` (600 passed, 2 ignored), `cargo test -p tfs-rust-core
      --test arena` (1 passed). **Lessons:** appended to `tasks/lessons.md` (#151–154).

## PC-3a — AoE spell modeling + spell-casting mechanics — done
- [x] `combat/circles.rs`: `DISC_RINGS` (rings 0–7, 101 tiles) + `disc_offsets(radius)` +
      `disc_tile_count(radius)` from `circles.dat` (772 `magic.cc:InitCircles`) /
      `combat.cpp:setupArea` (1098). Both eras share the same 772 disc-ring model —
      no era variance. 8 unit tests (ring counts, radius clamping, 1098 grid parity).
- [x] **Reverted** `AreaShapeModel` enum + `area_shape_model` field from `MechanicsProfile`
      (both eras use the same model — no selector needed). Removed from `formulas.rs`,
      `772.lua`, `1098.lua`, and all tests.
- [x] `lua_mutation.rs`: `CombatExecuteRequest` struct + `LuaMutation::CombatExecute` variant
      + `call_combat_execute` helper. Re-exported in `tfs-rust-lua/src/lib.rs`.
- [x] `lua_scope.rs`: `apply_lua_mutation` handles `LuaMutation::CombatExecute` →
      `combat_execute_from_lua`. Added `fire_on_cast_spell` helper (read context + mutation
      scope → `dispatch_on_cast_spell`).
- [x] `combat/aoe.rs`: `combat_execute_from_lua` — iterates area offsets, checks PZ per tile
      (772 `magic.cc:475`), checks `throw_possible` per tile (772 `magic.cc:479`), collects
      creatures, rolls damage via `uniform_random`, applies via `combat_execute_with_stimulus`.
      Caster self-damage skip for aggressive spells.
- [x] `userdata/combat.rs`: `Combat:execute()` method — resolves creature ID + variant
      (POSITION/TARGETPOSITION → table x/y/z, NUMBER → ctx lookup, nil → caster pos),
      builds `CombatExecuteRequest` with area offsets from `AreaCombat::affected_offsets`,
      resolves damage min/max from formula (Damage literal, LevelMagic via
      `level*2+magic*3`), dispatches via `call_combat_execute`.
- [x] `userdata/spell.rs`: `SpellBuilder` now holds `on_cast_fn: Rc<RefCell<Option<RegistryKey>>>`
      for `__newindex` callback capture. `__newindex` metamethod captures
      `spell.onCastSpell = function(...)` (the `data/scripts/spells/` pattern).
      `:register()` stores the callback in `_pending_spell_callbacks` at the same index.
- [x] `runtime.rs`: `LuaRuntime` holds `spell_callbacks: HashMap<String, RegistryKey>` keyed
      by spell words (lowercased). `call_on_cast_spell(words, creature)` looks up + invokes
      the callback with `(creature, nil)`. `register_spell_callback` for population.
- [x] `combat_scripts.rs`: `load_spell_scripts` initializes + drains
      `_pending_spell_callbacks` in parallel with `_pending_spells`, storing each
      callback's `RegistryKey` on the `LuaRuntime` via `register_spell_callback`.
- [x] `event_dispatcher.rs`: `dispatch_on_cast_spell` trait method (default `false`).
- [x] `lua_event_dispatcher.rs`: `dispatch_on_cast_spell` impl → `runtime.call_on_cast_spell`.
- [x] `game_world_chat.rs`: `player_say_spell` now enforces:
      - Exhaustion check (772 `magic.cc:3399` `EarliestSpellTime > ServerMilliseconds`)
      - Aggressive spell in PZ block (772 `magic.cc:3403-3407`)
      - Mana/soul deduction (772 `CheckMana` `magic.cc:762-763`)
      - Spell exhaustion set (772 `magic.cc:770-773` `delay_spell_ms`)
      - PZ lock for aggressive spells (772 `BlockLogout` `crmain.cc:433-457`, 50 rounds)
      - `onCastSpell` Lua callback dispatch via `fire_on_cast_spell`
- [x] **Tests:** 3 spell tests (constructor, register, `__newindex` callback capture),
      8 circles tests (ring counts, offsets, 1098 parity, clamping).
- [x] **Verify:** `cargo check` (0 errors), `cargo clippy` (0 warnings),
      `cargo test --workspace` (617+ passed, 1 pre-existing failure unrelated to PC-3a).
      **Lessons:** appended to `tasks/lessons.md`.

## PC-3a spell-gaps doc audit — done
- [x] Corrected `tasks/pc3a-spell-gaps.md` against tree (wild_growth, area names, DISPEL/CREATEITEM counts, conjureItem via functions.lua, direct addCondition vs callbacks, existing CreatureRef APIs).

## PC-5 — Skill tries / learning / death (2026-07-18) — done
- [x] SkillTriesTuning + skillTuning in 772/1098.lua
- [x] PlayerSkills tries + manaspent + blessings; login/save
- [x] Increase(1) on attack/defend probe + magic_increase on mana spend
- [x] M12 shield learning; M13 level-up current vitals; M7 AoL/SOME drop/bless
- [x] experience_for_level_poly consolidate; fed_regen no hardcoded fallback
- [x] Tests: req_skill_tries goldens, skill_increase, vitals Advance
- [x] Wire `rateSkill`/`rateMagic` from config.lua onto try gains (`scale_tries`; exp stages already done)
- [x] Client refresh: `send_player_stats` after kill XP; `send_player_skills` + real skill/magic % bars
- [x] Exp popup (`TEXTCOLOR_WHITE_EXP` animated text) + `MESSAGE_EVENT_ADVANCE` skill/magic/level lines

