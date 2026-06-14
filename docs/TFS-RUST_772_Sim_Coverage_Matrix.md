# 772 Sim Harness — Coverage Matrix (Movement + Combat E0–E6)

**Date:** 2026-06-14 (§19 combat-complete)  
**Harness:** `chase_kite_sim` (Rust synthetic/OTBM) + `build/game chase-scenario` (C++ synthetic/`.sec`)  
**Logs:** `log/chase_ai.jsonl` / `log/chase_path_rust.log` / `log/chase_path_cip.log`  
**Compare:** `scripts/compare_chase_live_logs.py`, `scripts/summarize_chase_gaps.py` (`--lockstep`)  
**Battery:** `TFS_SIM_SEED=772 python3 scripts/run_sim_battery.py --synthetic`  
**Divergence report:** [`TFS-RUST_772_Sim_Divergence_Report.md`](TFS-RUST_772_Sim_Divergence_Report.md) — §19 full battery

Enable: `TFS_CHASE_PATH_DEBUG=1` / `TIBIA_CHASE_PATH_DEBUG=1`

---

## Implemented phases vs sim coverage

| Phase | What shipped | Sim event(s) | Compared? |
|-------|--------------|--------------|-----------|
| **Movement** (pre-E0) | IdleStimulus walk arms, TShortway, Go | `branch`, `todo_go`, `shortway`, `go_exec` | **Yes** |
| **E0** | Combat data on `Monster` + data-pack spawn | `monster_load_type` scenario verb | **Yes** (spawn path) |
| **E1** | `MonsterState` (Attacking/Panic/…) | `combat_state` | **Yes** |
| **E2** | Melee execute + `earliest_attack_ms` cadence | `melee_hit`, `attack_enqueue` | **Yes** |
| **E3** | ATTACKING walk gating, `attack_close_chase` | `combat_state.chase_mode`, `todo_go.arm` | **Partial** |
| **E4** | Spells / ranged | `spell_cast`, `ranged_hit` | **Partial** (plumbed; cobra scenario 0 casts) |
| **E5** | DamageStimulus / PANIC flee | `damage_stimulus` | **Yes** (`kite_rat_panic`, `kite_rat_kill`) |
| **E6** | Death / exp / loot | `creature_death` | **Yes** (`kite_rat_kill`) |

---

## JSONL event schema (both stacks)

| `evt` | Fields | C++ hook | Rust hook |
|-------|--------|----------|-----------|
| `branch` | `branch`, `from`, `dest`, `must`, `max`, `cheb` | `crnonpl.cc` IdleStimulus WALKING | `idle_stimulus.rs` |
| `todo_go` | `via`, optional `arm`, `from`, `dest` | `cract.cc` `ToDoGo` | `creature_todo.rs`, `monster_ai.rs` |
| `shortway` | `steps`, `ok`, `visible`, `min_wp` | `cract.cc` `TShortway` | `monster_ai.rs` repath |
| `go_exec` | `from`, `to`, `diag` | `cract.cc` `Go` | `walk/mod.rs` |
| `combat_state` | `monster_state`, `chase_mode`, `attack_target` | `crnonpl.cc` ATTACKING block | `idle_stimulus.rs` |
| `attack_enqueue` | `wait_ms`, `needs_close_step`, `close_chase` | `cract.cc` `ToDoAttack` | `idle_stimulus.rs` |
| `melee_hit` | `attack`, `defense`, `armor`, `damage`, `hp_*`, `earliest_attack_ms` | `crcombat.cc` `CloseAttack` | `monster_ai.rs` |
| `ranged_hit` | same shape as `melee_hit` | — (not hooked) | `monster_ai.rs` DistanceAttack |
| `spell_cast` | `spell`, `target_id`, `shape`, `range` | `crnonpl.cc` CASTING block | `idle_stimulus.rs` |
| `damage_stimulus` | `old_state`, `new_state`, `attacker_id`, `damage`, `had_target` | `crnonpl.cc` `DamageStimulus` | `idle_stimulus.rs` |
| `creature_death` | `killer_id`, `experience`, `corpse_id` | `crnonpl.cc` `~TMonster` | `game_world_lifecycle.rs` |
| `rng_trace` | `call_index`, `value` | — | Rust sim only |
| `parked` | scheduler dead-end | — | Rust only |

---

## Scenario verbs

| Verb | Purpose |
|------|---------|
| `monster_load_type 1` | Spawn from `data/monster/` (default on) |
| `monster_state sleeping\|idle\|…` | Initial 772 posture |
| `player_damage <n>` | E5/E6 player strike on all scenario monsters |
| `player_damage_monster <idx> <n>` | Targeted strike |

---

## Scenarios

| File | Exercises |
|------|-----------|
| `kite_rat_stand_melee.scenario` | E2/E3 stand melee (data-pack rat) |
| `kite_rat_melee.scenario` | Movement + combat while kiting |
| `kite_cyclops_quad_chase.scenario` | Multi-monster chase |
| `kite_cobra_poison.scenario` | **E4** spell cast at range (synthetic) |
| `kite_rat_panic.scenario` | **E5** sleeping rat + `player_damage` → PANIC |
| `kite_rat_kill.scenario` | **E6** `creature_death` + race XP |

```bash
TFS_SIM_SEED=772 python3 scripts/run_sim_battery.py --synthetic
python3 scripts/summarize_chase_gaps.py \
  --ref log/chase_path_cip_kill.log --rust log/chase_path_rust_kill.log \
  --monster rat --max-tick 2000 --lockstep
```

---

## Known compare limitations (post-§19)

1. **Creature ID encoding** — C++ internal IDs vs Rust slotmap keys break `attacker_id`/`killer_id` lockstep (kill/panic scenarios).
2. **Corpse ID** — C++ race `.mon` corpse type vs XML `corpse=` on Rust (kill scenario: 3994 vs 2813).
3. **E4 delay gate** — cobra closes to melee before `spell_cast` fires on either stack.
4. **Post-panic walk** — C++ dance vs Rust flee after `damage_stimulus` panic (panic scenario).
5. **Legacy movement** — §18 RNG/path/tick blockers unchanged on stand/kite/cyclops.

**Resolved (§19):** E5/E6 events in compare gate; data-pack spawn; kite time budget; 6-scenario battery.

---

## Changelog

| Date | Change |
|------|--------|
| 2026-06-14 | Matrix + combat trace events (`combat_state`, `attack_enqueue`, `melee_hit`); aligned `todo_go.via` |
| 2026-06-14 | §15 lockstep: synthetic arena, glibc RNG, `--lockstep` gate, updated limitations |
| 2026-06-14 | §16 A/B: stand/kite/cyclops quad rerun; kite C++ needs time budget; cyclops `combat_state` 4/4 |
| 2026-06-14 | §19: E0–E6 coverage, new scenarios, `creature_death`/`ranged_hit`, battery runner |
