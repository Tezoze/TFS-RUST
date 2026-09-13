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
- [ ] Publication blockers per `BENCHMARK.md` §8 items 1–3: walk-ack retire on `0xB5` + `rejections` counter; waypoint expander; `--layout cluster` + `clustered_hunt.ron`

# Loadgen swarm — Canary ideas, open-loop 772 (`docs/archive/LOADGEN_SWARM.md`, now `docs/BENCHMARK.md` §4.3/§8) (2026-09-13)

Steal **ideas only** from local `canary-bots-with-cast-main/` (GPL-2.0, 15.00 in-process bots). Do not copy C++/CSV/coords. Do not edit frozen `mixed_300.ron`. Loadgen stays the 772 TCP client; Python stays the orchestrator.

- [x] Design doc (archived into `docs/BENCHMARK.md`)
- [ ] Waypoint CSV expander in `tools/loadgen` (`waypoint_file` on `Scenario`; random cardinals if absent)
- [ ] Phase desync + lane offset in the expander (two seeds ≠ identical opcode prefix)
- [ ] One authored 772 loop (`bench/waypoints/thais_depot_loop.csv`) + `walker_loop.ron`
- [ ] `seed_bench_accounts.py --layout cluster` + `clustered_hunt.ron`
- [ ] Name Rust game thread; diagnosis plot of that tid’s CPU (not publication charts)
- [ ] Verify: `rtk cargo test -p tfs-loadgen`; seeder `--self-test`; 10-bot `walker_loop` against a live server

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
