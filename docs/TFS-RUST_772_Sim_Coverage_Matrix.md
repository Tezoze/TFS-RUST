# 772 Sim Harness — Coverage Matrix (Movement + Combat E0–E3)

**Date:** 2026-06-14  
**Harness:** `chase_kite_sim` (Rust synthetic/OTBM) + `build/game chase-scenario` (C++ synthetic/`.sec`)  
**Logs:** `log/chase_ai.jsonl` / `log/chase_path_rust.log` / `log/chase_path_cip.log`  
**Compare:** `scripts/compare_chase_live_logs.py`, `scripts/summarize_chase_gaps.py` (`--lockstep`)  
**Divergence report:** [`TFS-RUST_772_Sim_Divergence_Report.md`](TFS-RUST_772_Sim_Divergence_Report.md) — full gap analysis from stand + kite runs

Enable: `TFS_CHASE_PATH_DEBUG=1` / `TIBIA_CHASE_PATH_DEBUG=1`

Lockstep A/B: `TFS_SIM_SEED=772 python3 scripts/run_kite_scenario.py --synthetic scripts/scenarios/kite_rat_stand_melee.scenario`

---

## Implemented phases vs sim coverage

| Phase | What shipped | Sim event(s) | Compared? |
|-------|--------------|--------------|-----------|
| **Movement** (pre-E0) | IdleStimulus walk arms, TShortway, Go | `branch`, `todo_go`, `shortway`, `go_exec` | **Yes** |
| **E0** | Combat data on `Monster` (melee_skill, armor, spells…) | *(spawn only — no dedicated event)* | No |
| **E1** | `MonsterState` (Attacking/Panic/…) | `combat_state` | **Yes** |
| **E2** | Melee execute + `earliest_attack_ms` cadence | `melee_hit`, `attack_enqueue` | **Yes** |
| **E3** | ATTACKING walk gating, `attack_close_chase` | `combat_state.chase_mode`, `todo_go.arm=attack_close_chase`, skip idle `melee_chase` | **Partial** (via branch absence + arm tag) |
| **E4** | Spells / ranged | `spell_cast` (plumbed) | **Not yet** |
| **E5** | DamageStimulus / PANIC flee | — | **Not yet** |
| **E6** | Death / exp / loot | — | **Not yet** |

---

## JSONL event schema (both stacks)

| `evt` | Fields | C++ hook | Rust hook |
|-------|--------|----------|-----------|
| `branch` | `branch`, `from`, `dest`, `must`, `max`, `cheb` | `crnonpl.cc` IdleStimulus WALKING | `idle_stimulus.rs` |
| `todo_go` | `via` (`enter`/`single`/`noway`), optional `arm`, `from`, `dest` | `cract.cc` `ToDoGo` | `creature_todo.rs`, `monster_ai.rs` |
| `shortway` | `steps`, `ok`, `visible`, `min_wp` | `cract.cc` `TShortway` | `monster_ai.rs` repath |
| `go_exec` | `from`, `to`, `diag` | `cract.cc` `Go` | `walk/mod.rs` |
| `combat_state` | `monster_state`, `chase_mode`, `attack_target` | `crnonpl.cc` ATTACKING block | `idle_stimulus.rs` |
| `attack_enqueue` | `wait_ms`, `needs_close_step`, `close_chase` | `cract.cc` `ToDoAttack` | `idle_stimulus.rs` |
| `melee_hit` | `attack`, `defense`, `armor`, `damage`, `hp_before`, `hp_after`, `earliest_attack_ms` | `crcombat.cc` `CloseAttack` | `monster_ai.rs` `monster_do_attacking` |
| `spell_cast` | `spell`, `target_id`, `shape`, `range` | `crnonpl.cc` CASTING block | `idle_stimulus.rs` `monster_idle_apply_spell_impact` |
| `rng_trace` | `call_index`, `value` | — | Rust sim only (`TFS_SIM_RNG_TRACE=1`) |
| `parked` | scheduler dead-end | — | Rust only |

---

## Scenarios

| File | Exercises |
|------|-----------|
| `scripts/scenarios/kite_rat_melee.scenario` | Movement: melee_dance while kiting (synthetic arena) |
| `scripts/scenarios/kite_rat_stand_melee.scenario` | Combat: adjacent rat, ~2s cadence melee hits (synthetic arena) |
| `scripts/scenarios/kite_rat_dist4.scenario` | Dist chase/dance (when dist monster configured) |
| `scripts/scenarios/kite_cyclops_quad_chase.scenario` | Multi-monster: 4 cyclops chase kiting player (synthetic arena) |

```bash
TFS_SIM_SEED=772 python3 scripts/run_kite_scenario.py --synthetic scripts/scenarios/kite_rat_stand_melee.scenario
TFS_SIM_SEED=772 python3 scripts/run_kite_scenario.py --synthetic scripts/scenarios/kite_cyclops_quad_chase.scenario
python3 scripts/summarize_chase_gaps.py \
  --ref log/chase_path_cip.log --rust log/chase_path_rust.log --monster rat --max-tick 6000 --lockstep
```

---

## Known compare limitations (post-§15 lockstep)

1. **Pre-dance RNG stream** — glibc combat/dance rolls are wired, but C++ may consume extra `rand()` before the first logged dance (talk gate, spawn path). Use `TFS_SIM_RNG_TRACE=1` to find first divergence.
2. **Appear-drain cadence** — Rust may log `branch`/`go_exec` at tick=0 on `monster_appear`; C++ often defers movement logs to later ticks.
3. **Armor field** — C++ `CloseAttack` logs `armor=0` (applied inside `Damage`); Rust logs explicit armor roll.
4. **E4+** — spells, DamageStimulus, death not asserted in lockstep gate yet.
5. **Kite rat C++ silent** — `kite_rat_melee.scenario` has `wall_ms=0`; C++ writes no `chase_ai.jsonl` until scenario advances `ServerMilliseconds` (add `advance_ms` between steps).
6. **Multi-monster** — cyclops quad: `combat_state` 4/4 match; movement still diverges at tick=0 Rust appear drain.

**Resolved (§15):** OTBM vs `.sec` map asymmetry (use `--synthetic` / `arena_synthetic 1`); unseeded RNG; C++ `todo_go` duplicate `enter`+`single` pairs; uncapped scenario wall time.

---

## Changelog

| Date | Change |
|------|--------|
| 2026-06-14 | Matrix + combat trace events (`combat_state`, `attack_enqueue`, `melee_hit`); aligned `todo_go.via` |
| 2026-06-14 | §15 lockstep: synthetic arena, glibc RNG, `--lockstep` gate, updated limitations |
| 2026-06-14 | §16 A/B: stand/kite/cyclops quad rerun; kite C++ needs time budget; cyclops `combat_state` 4/4 |
