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
  - [x] P2.5g Multi-monster `go_exec` drain order — `WakeupTiePolicy` (`HarnessAppearIdle` LIFO vs `HarnessGoStep`); cyclops lockstep **4/4**; battery **4/6**
  - Docs: divergence §26, trajectory Phase 2 closeout
