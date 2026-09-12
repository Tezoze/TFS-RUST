# Loadgen swarm — richer bots without breaking the A/B contract

Status: design (not implemented). Publication still uses frozen
`bench/scenarios/mixed_300.ron` and `docs/PERF_BENCHMARK_METHODOLOGY.md`.

Canary-bots-with-cast (local tree `canary-bots-with-cast-main/`) is a **15.00 /
protocol 13+** in-process `Player(nullptr)` engine plus a `@cast` spectator
login. It is **GPL-2.0**. Do not copy C++, Lua, hunt CSVs, or map coordinates
from that tree. This document takes **ideas only** and maps them onto
`tools/loadgen` (real 772 TCP, open-loop) so both Rust and TVP see the same
client.

In-process population stays in `crates/tfs-rust-sim` (`population.rs`). That is
the analog of Canary’s dummy players. It is Tier 2, not Tier 4.

## Invariants (do not violate)

1. **Open-loop.** Actions fire at intended `Instant`s. Latency = ack − intended
   send time. Cooldowns shape the *schedule*; they never drop a due timestamp.
   Closed-loop AI (wait for walk-ack before the next step, hibernate when
   unobserved, pathfind on the server) is out of scope for loadgen.
2. **`mixed_300.ron` is frozen.** New scenarios are new files. Headline A/B
   charts stay on the frozen mix unless the methodology is explicitly revised.
3. **Same binary, both servers.** Anything loadgen sends must be valid 772 wire
   against Rust *and* TVP.
4. **No GPL in this repo.** Authored waypoints are 772 tiles we write ourselves.

## What we steal vs reject

| Idea | Canary | Our mapping | Take? |
|------|--------|-------------|-------|
| Authored hunt / city polylines | CSV waypoints, server A* between nodes | Pre-expand 772 polylines to cardinal `Walk` opcodes at `walk_period_ms` | **Yes** |
| Path jitter / lane offset / phase desync | Live planner | Per-bot start index + lateral tile offset in the expander | **Yes** |
| Activity mix tables (dwell / POI / hunt / travel) | Closed-loop state machine | Extra RON roles that *select* a waypoint file; still open-loop | Later |
| Stratified roster (town / vocation / level) | DB seed 997 bots | Optional seeder columns; login names stay `Test` / `Test1` / … | Optional |
| Cluster around a hunt | Density / proximity wake | `--layout cluster` seed + `clustered_hunt.ron` | **Yes** (new scenario) |
| Tick wall vs thread CPU | `CLOCK_THREAD_CPUTIME_ID` on dispatcher | Attribute `threads.csv` `comm` to the game thread (diagnosis only) | **Yes** |
| Hibernation / density caps | Hide unobserved bots | Opposite of stress | **No** |
| In-process `Player` + `g_spells()` | No TCP | Would not hit TVP | **No** |
| Cast / `@cast` login | ProtocolGame viewers | Not 772 corpus | **No** |
| Server-side A*, z-graph, doors | 53k-line `.so` | Loadgen must not pathfind; expand offline | **No** |
| Market / houses / chat corpus / PK gangs | World-feel product | Out of scope | **No** |

Melee already binds `Attack` to `InboundState.last_other_creature_id` at send
time (`tools/loadgen/src/session.rs`). Random cardinal walks plus Chebyshev-16
scatter make that id usually empty. Clustered hunt + waypoint loops are what
make combat roles actually hit creatures.

## 1. Authored waypoint loops

### File format

`bench/waypoints/<name>.csv` — UTF-8, `#` comments, header `x,y,z`. Tiles are
**772 OTBM** (same map as Rust/TVP), not Canary otservbr.

```
# thais_depot_loop — walkable surface tiles only
x,y,z
32369,32241,7
32364,32241,7
32364,32236,7
32369,32236,7
```

A loop is implied: last tile reconnects to the first. No `travel_to` /
`hunt_patrol` phases, no shovel/rope extras — 772 loadgen has no use-item-on-ground
tool path yet. Keep loops on one floor until a later scenario adds `UseItem`
for ropes.

Author from `data/world/` (OTBM / `spawns.xml` centers). Verify tiles are
walkable in-game once; do not import Canary CSVs (wrong world).

Starter set (names only until tiles are walked):

| File | Intent |
|------|--------|
| `thais_depot_loop.csv` | Temple ↔ depot street churn (spectator recalc, no combat) |
| `thais_temple_ring.csv` | Tight ring on walkable tiles around 32369,32241,7 |
| One hunt loop per clustered scenario | Spawn-center from `spawns.xml` (e.g. a rotworm box) |

### Expander (loadgen)

New helper in `tools/loadgen` (focused module, not a dump into `roles.rs`):

- Input: polyline, `walk_period_ms`, bot RNG.
- Output: sequence of 772 cardinal move opcodes (`MOVE_NORTH` / `EAST` /
  `SOUTH` / `WEST`).
- Between two tiles, emit axis-aligned steps (horizontal then vertical, or
  reverse if the bot’s jitter bit is set). Diagonal tiles become two steps.
- If `dx`/`dz` are huge, still emit; a bad CSV is a data bug, not a runtime
  pathfinder.
- Repeat the loop until `duration_s + warmup_s` is filled.

`RoleKind::Walker` (and later a dedicated hunt role) uses this when the
scenario sets `waypoint_file`. If the field is absent, keep today’s random
cardinals so existing RON files stay valid.

Open-loop: a rejected walk still consumes its intended timestamp. Inbound
`pos` is **not** used to replan.

### Scenario field

Add optional `waypoint_file: Option<String>` on `Scenario` (default `None`).
Path is relative to repo root. Isolation scenarios that omit it are unchanged.

New files (do not edit `mixed_300.ron`):

- `bench/scenarios/walker_loop.ron` — 25–50 walkers, `thais_depot_loop.csv`
- `bench/scenarios/clustered_hunt.ron` — see §3

## 2. Phase desync and lane jitter

Canary avoids a conga line with per-bot path jitter, walking lanes, and route
phase offset. Open-loop equivalent:

- **Phase:** bot `i` starts at waypoint index `i % n` (or a hash of
  `bot_seed`).
- **Lane:** optional ±1 tile offset on the axis perpendicular to the current
  segment, clamped so we do not walk into a known wall (document the clamp in
  the CSV comment; do not query the map at runtime).
- **Jitter bit:** swap “horizontal then vertical” vs the reverse so two bots
  on the same segment desynchronize corners.

All of this is folded into the precomputed opcode list. No extra packets.

## 3. Clustered hunt spawn

Default seeder (`scripts/seed_bench_accounts.py`) scatters with Chebyshev gap
16 so logins do not share a spectator set. That is the **quiet / fair** layout
for `mixed_300`.

Canary packs bots near activity. For **worst-case spectator fan-out + combat**,
add a second layout — it is a different experiment, not a replacement.

### Seeder

```
python3 scripts/seed_bench_accounts.py --count 50 --apply --target both \
  --layout cluster --cluster-x X --cluster-y Y --cluster-z 7 --cluster-radius 6
```

- Place characters on a Chebyshev disk around `(X,Y,Z)` with gap 1–2 (they
  **should** share spectator sets).
- `--layout scatter` remains default (gap 16).
- `--stack-temple` stays as the third, already-documented, mode.
- Login names/accounts unchanged (`Test` / `Test1` / …, ids 1..N) so loadgen
  needs no rename.

Pick `(X,Y,Z)` from a `spawns.xml` monster center that actually has creatures
on both servers (equivalence gate still applies at 5 bots before scaling).

### Scenario

`bench/scenarios/clustered_hunt.ron`: melee-heavy mix, optional hunt waypoint
file, `bots: 50` (orchestrator `--bots` still overrides). Run as a **sensitivity**
cell in `run_comparison.py` later; do not add it to the default load-curve
points until methodology says so.

## 4. Game-thread CPU vs wall (diagnosis only)

Canary’s perf harness compares tick **wall** to **thread** CPU so preemption
is not mistaken for compute. Our methodology already forbids `GameObs` on
head-to-head charts. Same rule here.

`scripts/bench/sample_proc.py --threads-out` already writes per-tid `comm`,
`utime_ticks`, `stime_ticks`. Follow-on:

- Name the Rust game thread (`std::thread::Builder::name("game")` or
  equivalent) so `comm` is stable. TVP: document which `comm` is the
  dispatcher vs asio workers.
- `plot_results.py`: optional overlay of that tid’s CPU% next to process
  CPU% — **diagnosis plots**, never the publication CPU-seconds / action
  chart.
- Do not add `CLOCK_THREAD_CPUTIME_ID` sampling inside the game loop for
  A/B; `/proc` is the language-agnostic instrument.

## 5. Stratified roster (optional, last)

Canary samples evenly across vocation/town/level. Our seeder already strides
NPC-adjacent tiles then spawn centers. If combat mix is too uniform:

- Optional `--vocation-cycle 1,2,3,4` on the seeder (knight/paladin/sorcerer/
  druid ids as in schema).
- Do not invent a second character-name scheme; loadgen keys off `Test{i}`.

Skip until waypoint + cluster land. Scatter-vs-cluster dominates vocation.

## Implementation order

Parent implements; no sub-agent edits.

1. **Waypoint CSV + expander + `waypoint_file`** — `tools/loadgen` new module
   (e.g. `waypoints.rs`), `scenario.rs` field, `roles.rs` call site, unit test
   that a 4-tile square yields 4× cardinals × loops. One real Thais loop CSV.
2. **Phase / lane** — same expander; test two bot seeds do not emit identical
   opcode prefixes.
3. **`--layout cluster`** on `seed_bench_accounts.py` + `--self-test` disk
   packing. `clustered_hunt.ron`.
4. **Game-thread `comm`** + plot overlay (diagnosis).
5. Orchestrator hook: optional `--scenario` other than `mixed_300` for
   sensitivity cells. Default load-curve stays frozen.

Crate placement: all wire work in `tools/loadgen` and `bench/`. Seeder stays
Python. Do not grow `game_world.rs`. Do not add in-process Canary-style bots
to core.

## Verify (when implemented)

```
rtk cargo test -p tfs-loadgen
rtk cargo clippy -p tfs-loadgen --all-targets -- -D warnings
python3 scripts/seed_bench_accounts.py --self-test
# Rust server up:
./target/release/tfs-loadgen --scenario bench/scenarios/walker_loop.ron --bots 10
```

Equivalence gate still uses isolation roles at 5 bots. Waypoint/cluster
scenarios are extra cells, not a replacement for that gate.
