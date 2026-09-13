# Loadgen --progress ticker + session drop counters (2026-09-13)

Stderr one-line ticker and first-class disconnect/reconnect counts. Do **not** auto-reconnect during the measurement window. Do not edit frozen `mixed_300.ron`.

- [x] `progress.rs`: atomics + `--progress` 1 Hz stderr line; `SessionLease` for connected/in_world
- [x] `session.rs`: count EOF/I/O drop after game connect as `disconnects` (return outcome, do not fail the join); never reconnect
- [x] `RunReport` JSON `disconnects` / `reconnects`; `check_equivalence.py` drop-rate mismatch; `run_comparison.py` passes `--progress`
- [x] `docs/BENCHMARK.md` §5/§7
- [x] Verify: loadgen test/clippy; equivalence `--self-test`

# Fix 5-bot cluster equivalence gate (2026-09-13)

Inbound `0xA7`/`0xD3`/`0xD4` skips + `unknown_opcodes`; isolation RONs 500 ms + cyclops loop; skip `damage_sum` when both `unique_creatures` are 0. Do not edit frozen `mixed_300.ron`.

- [x] `inbound.rs`: length-skip fight modes / VIP status; count unknown opcodes
- [x] Isolation `bench/scenarios/{walker,melee,caster,rune,aoe_rune,noise}.ron`
- [x] `check_equivalence.py` skip field-noise damage; keep ME check
- [x] `docs/BENCHMARK.md` §7/§8 cluster seed required
- [x] Verify: loadgen test/clippy; equivalence `--self-test`; seed `--layout cluster`; live `--mode equivalence` → **FAIL** `results/20260913T062814Z` (combat now visible; residual damage/ME/creature deltas)

# Remaining bench harness §8.2–7 (2026-09-13)

Waypoints, cluster layout, 500 ms walk note, named game thread, dual headline load-curve, vocation-cycle. Do not edit frozen `mixed_300.ron`. Do not run live A/B in this change.

- [x] `waypoints.rs` expander (phase / lane / jitter); `Scenario.waypoint_file`; `roles.rs` call site
- [x] `bench/waypoints/thais_depot_loop.csv` + temple ring + cyclops hunt loop; `walker_loop.ron`; `clustered_hunt.ron` (not frozen)
- [x] `seed_bench_accounts.py --layout cluster` + `--cluster-*` + `--self-test` disk packing
- [x] Document 500 ms LinearGo step; new RON files use 500
- [x] Named `game` OS thread in `run_server.rs`; `plot_results.py` diagnosis overlay
- [x] Default `run_comparison.py --mode load-curve` runs mixed_300 + clustered_hunt
- [x] `--vocation-cycle` on the seeder
- [x] Verify: loadgen test/clippy; seeder `--self-test`; equivalence `--self-test`; `run_comparison.py --dry-run --mode load-curve`

# Walk-ack retire on 0xB5 (2026-09-13)

Publication blocker `docs/BENCHMARK.md` §8.1 / §6.2.1. Cancel-walk pops the outstanding-walk FIFO head as a rejection (not a latency sample) so a later `0x6D` cannot inherit a rejected timestamp.

- [x] `inbound.rs`: emit `InboundEvent::CancelWalk` after the direction byte
- [x] `latency.rs`: `on_walk_cancel` + `walk.rejections`; merge across bots; JSON next to `samples`
- [x] `session.rs`: `CancelWalk` → `on_walk_cancel`
- [x] `check_equivalence.py`: `walk.rejections / walk.samples` ≤ 25% (skip if both 0)
- [x] Unit test: send, cancel, send, ack → one sample, one rejection
- [x] Verify: `rtk cargo test -p tfs-loadgen`; `rtk cargo clippy -p tfs-loadgen --all-targets -- -D warnings`; `python3 scripts/bench/check_equivalence.py --self-test`

# Loadgen TVP inbound skips + rune seed (2026-09-13)

TVP frames include `0x86` square / `0xA3` cancel-target / `0xAA` say (and `0x8E`/`0x8F`/`0x90`/`0x91`); unknown opcodes discarded the rest of the payload so walk-acks and effects after them were lost. Rune/caster roles also had no inventory rune, vocation 0, and `needLearnSpells`. Do not edit frozen `mixed_300.ron`.

- [x] `inbound.rs`: length-skip `0x86`/`0x8E`/`0x8F`/`0x90`/`0x91`/`0xA3`/`0xAA`; tests (square+walk-ack, cancel-target+ME, say+ME)
- [x] `item_extra.rs`: OTB server-id → client-id; Rune/AoeRune look up sprite at send; target `last_other_creature` tile
- [x] `seed_bench_accounts.py`: vocation 1, premium, `player_spells` Energy Strike, SD 2268 slot 10, GFB 2304 slot 6
- [x] `check_equivalence.py` + `docs/BENCHMARK.md` §7: `bytes_discarded / bytes_in` ≤ 25%
- [x] Verify: `rtk cargo test -p tfs-loadgen`; `rtk cargo clippy -p tfs-loadgen --all-targets -- -D warnings`; `python3 scripts/seed_bench_accounts.py --self-test`; `python3 scripts/bench/check_equivalence.py --self-test`

# Benchmark doc consolidation (2026-09-13)

- [x] `docs/BENCHMARK.md` — single source of truth (method, fairness, two headline scenarios, walk-ack defect, remaining work)
- [x] Archive `PERF_BENCHMARK_PLAN.md`, `PERF_BENCHMARK_METHODOLOGY.md`, `LOADGEN_SWARM.md` → `docs/archive/` with banners; repoint references
- [x] §8.1 walk-ack retire on `0xB5` + `walk.rejections`
- [x] §8.2–7 harness (waypoints, cluster, 500 ms note, game `comm`, dual headline, vocation-cycle); `clustered_hunt.ron` not frozen

# Loadgen swarm — Canary ideas, open-loop 772 (`docs/archive/LOADGEN_SWARM.md`, now `docs/BENCHMARK.md` §4.3/§8) (2026-09-13)

Steal **ideas only** from local `canary-bots-with-cast-main/` (GPL-2.0, 15.00 in-process bots). Do not copy C++/CSV/coords. Do not edit frozen `mixed_300.ron`. Loadgen stays the 772 TCP client; Python stays the orchestrator.

- [x] Design doc (archived into `docs/BENCHMARK.md`)
- [x] Waypoint CSV expander in `tools/loadgen` (`waypoint_file` on `Scenario`; random cardinals if absent)
- [x] Phase desync + lane offset in the expander (two seeds ≠ identical opcode prefix)
- [x] One authored 772 loop (`bench/waypoints/thais_depot_loop.csv`) + `walker_loop.ron`
- [x] `seed_bench_accounts.py --layout cluster` + `clustered_hunt.ron`
- [x] Name Rust game thread; diagnosis plot of that tid’s CPU (not publication charts)
- [ ] Verify 10-bot `walker_loop` against a live server (not this change)

# Perf Phase E — Tier 4 A/B vs TVP (2026-09-12)

A/B comparison harness against TVP C++ 7.72 (`docs/archive/PERF_BENCHMARK_PLAN.md` Phase E; current spec `docs/BENCHMARK.md`). Same 772 loadgen drives both servers; orchestrator + `/proc` sampler in `scripts/bench/`. Do **not** grow `tools/loadgen` into an orchestrator; keep loadgen as the client, Python as the runner. No `tfs-rust-core` changes.

- [x] Bulk account/char generator (`scripts/seed_bench_accounts.py`) for Rust `schema.sql` and TVP `schema.sql`
- [x] `scripts/build_tvp.sh` / `scripts/run_tvp.sh` (CMake Release + IPO from `reference/tvp-772/gameserver/`)
- [x] Content-equivalence gate at 5 bots (`scripts/bench/check_equivalence.py`) — damage, creatures, packets/action
- [x] `scripts/bench/sample_proc.py`, `run_comparison.py`, `plot_results.py`
- [x] `docs/PERF_BENCHMARK_METHODOLOGY.md` (fairness controls) — now archived, folded into `docs/BENCHMARK.md` §3
- [x] Orchestrator modes: load curve, steady state, overload, soak, equivalence (execute is a pinned-host run, not CI)
- [x] Loadgen: `--bots`/`--duration-s` override scenario; warmup load is recorded after `warmup_s`; inbound `0x84`/`0x85`/`0x8C` counters in JSON
- [x] Verify: clippy/check on loadgen; Python `--help` / self-test; methodology doc exists
- [x] TVP on dedicated MariaDB `test_tvp` (SHA1); 10-player login smoke
- [x] Disable TVP `Ban::acceptConnection`; walker-5 + equivalence gate on host
- [x] Scatter bench chars from `data/world/spawns.xml` (not temple stack)
- [x] Pinned-host `run_comparison.py --mode equivalence --reps 1` → **FAIL** (`results/20260912T230932Z`)
  - TVP `magic_effects=0` on every role; rust walkers already 25 (login/step FX). Do not run load-curve until this is explained.
  - OTBMs differ: rust `forgotten.otbm` vs TVP `map.otbm` (same size, different sha256).
  - Walk p50 5–15 s at 5 bots; hundreds outstanding — `0x6D` correlation is starving (walls + inbound `break` on `0x64` / unknown opcodes).
- [x] TVP OTBM: `scripts/sync_tvp_world.sh` hardlinks `forgotten.otbm` → TVP `map.otbm`; `run_tvp.sh` + `run_comparison.py` always call it; `enableMapDataFiles=false`
- [x] Loadgen inbound: skip 772 map bodies (`0x64` / `0x65`–`0x68` / `0xBE`/`0xBF`) plus login trailers (`0xB4`/`0xB5`/stats/inventory) so TVP teleport `0x83` in the same payload is counted. Walker-5: rust ME=18, TVP ME=5 (was 0). Do not run load-curve until `check_equivalence.py` prints `equivalence ok`.
