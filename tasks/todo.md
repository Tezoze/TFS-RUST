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
