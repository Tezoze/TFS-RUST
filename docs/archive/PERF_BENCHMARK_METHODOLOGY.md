# Performance benchmark methodology (Tier 4)

> **Archived 2026-09-13.** Superseded by [`docs/BENCHMARK.md`](../BENCHMARK.md).
> Kept for history; do not update.

Status: harness ready (Phase E). Publication numbers come from a **pinned-host**
run of `scripts/bench/run_comparison.py`, not from CI.

This is the fairness contract for A/B comparison of the Rust 772 server against
TVP (`reference/tvp-772/gameserver`). The same `tfs-loadgen` binary drives both
sides on the 772 wire. If a control below is skipped, the headline numbers are
not publishable.

## What is being compared

| Axis | Rust | TVP |
|------|------|-----|
| Wire | 772 (`communication.cc` / `connections.cc` corpus) | 772 |
| Mechanics | decompile corpus via `MechanicsProfile` | TVP C++ |
| Load | `tools/loadgen` open-loop, intended-time latency | same binary, same scenario |
| Persistence | MariaDB `TFS` (bcrypt after login) | MariaDB `test_tvp` (SHA1) |

`GameObs` (`RUST_LOG=tfs_obs=info`, `beat_wall_ms`) is captured in server logs
for **diagnosis only**. It never appears on a head-to-head chart.

## Host and scheduling

- Same machine. **Alternating** rust/tvp cells (`A/B/A/B`), never concurrent.
- `--reps 3` or more; report **median** and spread (min/max or IQR).
- Pin the server with `--cpuset-server` and the loadgen with `--cpuset-loadgen`
  (disjoint cpusets). Optionally put loadgen on a second machine; then record
  loadgen CPU to prove it is not the bottleneck.
- CPU governor `performance`. Document CPU model, kernel, RAM (`meta.json`
  written by `run_comparison.py`).
- Client timestamps are `CLOCK_MONOTONIC` (`std::time::Instant` in loadgen;
  sampler uses `clock_gettime(CLOCK_MONOTONIC)`).

## Build flags

- Rust: `cargo build --release --bin tfs-rust` (and `-p tfs-loadgen`).
- TVP: `./scripts/build_tvp.sh` → `cmake -DCMAKE_BUILD_TYPE=Release -DSKIP_GIT=ON`
  plus CMake IPO/LTO when `check_ipo_supported` succeeds
  (`gameserver/CMakeLists.txt`). `build_tvp.sh` sets `TMPDIR` to
  `.tmp/tvp-build` so LTO does not fill a 16G `/tmp` tmpfs.

**Asymmetric flag (disclose):** TVP `CMakeLists.txt` non-Win32
`add_compile_options(-ggdb3 -Og …)` applies even under Release, so `-Og` can
win over `-O3`. Default publication builds leave that as TVP wrote it.
`./scripts/build_tvp.sh --o3` rewrites generated `-Og` → `-O3` for a sensitivity
run — label those results separately.

## Workload

Frozen scenario: `bench/scenarios/mixed_300.ron` (do not edit between
publication runs). Isolation roles: `walker`, `melee`, `caster`, `rune`,
`aoe_rune`, `noise`. Extra swarm scenarios (waypoint loops, clustered hunt)
are specified in `docs/LOADGEN_SWARM.md` and must not replace this mix on a
publication chart.

Open-loop scheduling: latency is ack time minus **intended** send time, not
actual send time. Warmup seconds generate load; histograms start after
`warmup_s` (discarded). Login ramp is ≤ 8/s (`MAX_CONCURRENT_LOGIN_LOADS`).
Bots respect `say_period_ms ≥ 2500` (RecordTalk) and `walk_period_ms` (default
200). Cooldowns constrain the *schedule*; they never suppress a due action's
timestamp.

Walk-ack SLO: self `0x6D`. Spell/rune SLO: `0x83` at the target tile.

## Persistence

Default: TVP dist save flags (`enablePlayerDataFiles = true`). Rust saves
players to MariaDB on logout. That residual is disclosed, not hidden.

`--disable-saves` / `TFS_BENCH_DISABLE_SAVES=1` turns off TVP player/map data
files for a sensitivity run. Rust has no matching “no DB save” switch today.

TVP `ServerSave` globalevent at 04:30 **shuts the process down**. Do not soak
across that wall clock.

## Accounts and map

`python3 scripts/seed_bench_accounts.py --count 600 --apply --target rust`
(and `--target tvp` against MariaDB `test_tvp`)
inserts `accounts.id` 1..N (772 account number) and characters `Test`,
`Test1`, … at **scattered** login tiles from `data/world/spawns.xml` (NPC
stands, nudged one tile off the NPC, then spawn centers) with Chebyshev
gap 16 so they do not share a spectator set. Same `(posx,posy,posz)` on
Rust `TFS` and TVP `test_tvp`. `--stack-temple` restores the old single
Thais temple tile (32369,32241,7). One character per account
(`onePlayerOnlinePerAccount` on TVP).

Rust and TVP use **separate** databases: Rust `TFS` (bcrypt after first
login), TVP `test_tvp` (SHA1 `char(40)`). Do not point TVP at `TFS`.
`./scripts/setup_tvp_db.sh` creates `test_tvp` (MariaDB PUBLIC `test_%` grant;
`tfs` cannot `CREATE DATABASE TVP`) and imports `gameserver/schema.sql`.

TVP `Ban::acceptConnection` is **disabled** in the local `reference/tvp-772`
tree (always returns true). Stock TVP FORCE_CLOSEs >5 accepts/5 s with
≤500 ms gaps; login+game is two accepts per bot, which made loadgen ramps
look like a flood. Disclose this vs an unmodified TVP binary.

Shared OTBM: `./scripts/sync_tvp_world.sh` hardlinks Rust
`data/world/forgotten.otbm` onto TVP `gameserver/data/world/map.otbm` (both
OTBMs already name `spawns.xml` / `houses.xml`). `run_tvp.sh` and
`run_comparison.py` call it before a TVP process starts. TVP
`enableMapDataFiles` stays **false** so `gamedata/map.tvpm` cannot overlay a
different tile set. Spawns stay TVP `tvpspawn` vs Rust nested XML (same 9950
zones; some radii differ). `items.otb` and Lua trees still differ — see the
equivalence gate.

## Content-equivalence gate (validity, not a footnote)

Before any load-curve chart:

```
python3 scripts/bench/run_comparison.py --mode equivalence --reps 1
```

This runs 5 bots of each isolation role against rust then tvp and fails
`check_equivalence.py` when relative delta exceeds:

| Observable | Threshold |
|------------|-----------|
| `damage_sum` (numeric `0x84` text) | 20% (skip if both 0) |
| `unique_creatures` | 15% (skip if both 0) |
| `bytes_in` per send | 25% |
| `magic_effects` | 25% (skip if both 0) |
| `sends` | 25% |

If the two servers do not match at 5 bots, the 300-bot chart is not measuring
the claimed workload. Document any residual delta that survives the gate
(script-tree AoE is the expected hotspot).

Server-side combat/loot RNG is not locked across processes. Headline metrics
must be insensitive to it (median of ≥3 reps) or the gate fails.

## Sampling

`scripts/bench/sample_proc.py` at 1 Hz from `/proc/<pid>`:

- `utime`/`stime` (CLK_TCK → CPU-seconds and CPU%)
- RSS (stat pages) and PSS (`smaps_rollup`)
- thread count; per-thread CPU (`--threads-out`)
- voluntary / nonvoluntary ctx switches
- `/proc/<pid>/io`
- bytes/packets from `/proc/<pid>/net/dev` (**netns-wide**, including `lo`)

Work-normalized CPU: CPU-seconds / delivered actions (`walk.samples +
spell_rune.samples`). Report raw CPU% *and* this ratio — TVP is
asio-multithreaded; Rust is one game thread plus Tokio I/O.

## Modes

| Mode | Bots | Duration | Purpose |
|------|------|----------|---------|
| `load-curve` | 25/50/100/200/300/400/600 | scenario | knee |
| `steady` | 300 | scenario | 1 Hz time series |
| `overload` | 600 (or `--bots`) | scenario | collapse vs degrade |
| `soak` | 200 | 3600 s | RSS growth |
| `equivalence` | 5 × each role | 30 s | validity gate |

Headline: (a) max concurrent bots with walk-ack p99 &lt; 100 ms on frozen
`mixed_300`; (b) CPU-seconds per delivered action.

## Reproduce

```
./scripts/build_tvp.sh
python3 scripts/seed_bench_accounts.py --count 600 --apply --target both
python3 scripts/bench/run_comparison.py --mode equivalence --reps 1
python3 scripts/bench/run_comparison.py --mode load-curve --reps 3 \
  --cpuset-server 0-7 --cpuset-loadgen 8-15
python3 scripts/bench/plot_results.py results/<timestamp>
```

Publish raw CSVs plus this harness (`scripts/bench/`, `tools/loadgen`, frozen
`bench/scenarios/*.ron`).
