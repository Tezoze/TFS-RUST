# Performance benchmark — Rust 772 server vs TVP

Status: harness built (Tiers 1–4); **not yet publishable**. Two blockers must
land before any head-to-head chart is trusted — see §6.2 and §7.

Supersedes `docs/archive/PERF_BENCHMARK_PLAN.md`,
`docs/archive/PERF_BENCHMARK_METHODOLOGY.md`, and
`docs/archive/LOADGEN_SWARM.md`. Those are kept for history only; this file is
the single source of truth for method, fairness contract, workload, and the
remaining work.

## 1. Goal

Two deliverables, deliberately separate:

- **Engineering loop** — catch regressions per commit and find what to
  optimize next (Tiers 1–2, CI).
- **Publishable comparison** — defensible evidence against the TVP C++ 7.72
  server (`reference/tvp-772/gameserver`) (Tiers 3–4, pinned host).

| Tier | Instrument | Where | Answers |
|---|---|---|---|
| 1 | Criterion microbenches | `crates/tfs-rust-core/benches/hot_paths.rs` | Did this commit slow pathfinding / spectators / conditions / ToDo heap? |
| 2 | In-process scaling sweep | `crates/tfs-rust-sim` (`population.rs`, `sweep.rs`, bin `scale_sweep`) | How does the single game thread scale with monsters / players / spectators? |
| 3 | 772 wire loadgen vs Rust | `tools/loadgen` (bin `tfs-loadgen`) | What do network, serialization, fan-out, login cost? |
| 4 | A/B vs TVP | `scripts/bench/` | How do we compare to the C++ reference? |

Tiers 1–2 cannot touch TVP and never appear on a head-to-head chart. Tier 2
flamegraphs (`scripts/profile_sim.sh`) are the optimization instrument, not a
comparison artifact.

## 2. What is compared

| Axis | Rust | TVP |
|------|------|-----|
| Wire | 772 (`communication.cc` / `connections.cc` corpus) | 772 |
| Mechanics | decompile corpus via `MechanicsProfile` / `772.lua` | TVP C++ |
| Load | `tfs-loadgen`, open-loop, intended-time latency | **same binary, same scenario** |
| Persistence | MariaDB `TFS` (bcrypt after first login) | MariaDB `test_tvp` (SHA1) + player data files |

One synthetic client on one wire driving both servers is what makes the
comparison defensible. Anything that only exercises one side (in-process bots,
`GameObs`, `tfs-rust-sim`) is diagnosis.

`GameObs` (`RUST_LOG=tfs_obs=info`, `beat_wall_ms`) is captured in Rust server
logs for diagnosis only.

## 3. Fairness contract

If any control here is skipped, the numbers are not publishable.

### 3.1 Host and scheduling

- Same machine. Alternating cells `rust/tvp/rust/tvp`, never concurrent.
- `--reps 3` minimum; report **median** and spread (min/max or IQR).
- Server on `--cpuset-server`, loadgen on a disjoint `--cpuset-loadgen`.
  Record loadgen CPU to prove it is not the bottleneck. A second machine for
  loadgen is acceptable if its CPU is still recorded.
- CPU governor `performance`. `run_comparison.py` writes CPU model, kernel,
  RAM to `meta.json`.
- All client-side timing is `CLOCK_MONOTONIC` (`std::time::Instant`;
  `sample_proc.py` uses `clock_gettime(CLOCK_MONOTONIC)`).
- Warmup (`warmup_s`, default 30 s) generates load but is discarded from
  histograms.

### 3.2 Build flags

- Rust: `cargo build --release --bin tfs-rust` and `-p tfs-loadgen`.
- TVP: `./scripts/build_tvp.sh` → `cmake -DCMAKE_BUILD_TYPE=Release
  -DSKIP_GIT=ON` plus IPO/LTO when supported. `TMPDIR=.tmp/tvp-build` so LTO
  does not fill a tmpfs `/tmp`.

**Asymmetric flag — must appear on the chart, not in a footnote.** TVP's
`CMakeLists.txt` adds `-ggdb3 -Og` unconditionally, so a stock Release build
is effectively `-Og`. Publication runs **both** cells:

| Label | Build |
|---|---|
| `tvp` | as TVP ships (`-Og` wins) |
| `tvp-o3` | `./scripts/build_tvp.sh --o3` (`CMAKE_CXX_FLAGS_RELEASE="-O3 -DNDEBUG"`) |

Lead with `tvp-o3` on headline charts; `tvp` shows what operators actually
run. Never publish only one.

### 3.3 Persistence

- Default: TVP `enablePlayerDataFiles = true` (dist config). Rust saves
  players to MariaDB on logout. Logouts only happen at teardown, so the
  residual is small; disclose it.
- `--disable-saves` / `TFS_BENCH_DISABLE_SAVES=1` turns off TVP player/map
  files for a sensitivity cell. Rust has no matching switch.
- TVP `ServerSave` globalevent at 04:30 **shuts the process down**. Never soak
  across that wall clock.
- **Auth (disclose).** `seed_bench_accounts.py` stores SHA1 (`transformToSHA1`
  of plaintext `1`). TVP verifies SHA1 and stops. Rust default
  `upgradeSha1OnLogin = true` re-hashes to bcrypt cost 12 on first login
  (`spawn_blocking`, ~200 ms × bots) — that was the 700%+ post-listen spike
  at 50 bots, not map send. `run_comparison.py` sets
  `TFS_UPGRADE_SHA1_ON_LOGIN=0` so the rust cell matches TVP (SHA1 verify
  only). Production keeps the upgrade. Override the env to `1` only when
  measuring the upgrade itself.

### 3.4 Accounts, map, spawns

- `python3 scripts/seed_bench_accounts.py --count 600 --apply --target both`
  inserts `accounts.id` 1..N and characters `Test`, `Test1`, … with identical
  `(posx,posy,posz)` on Rust `TFS` and TVP `test_tvp`. One character per
  account (`onePlayerOnlinePerAccount` on TVP).
- Separate databases. Rust `TFS`, TVP `test_tvp`
  (`./scripts/setup_tvp_db.sh` creates it under the MariaDB PUBLIC `test_%`
  grant and imports `gameserver/schema.sql`). Do not point TVP at `TFS`.
- Layouts (`--layout`): `scatter` (default, Chebyshev gap 16 from
  `spawns.xml` NPC stands then spawn centers — no shared spectator sets),
  `cluster` (Chebyshev disk radius ~6, gap 1–2 around a chosen spawn center —
  shared spectator sets, active monsters; default center `32776,32240,7`
  Cyclops, also on TVP `tvpspawn`), `--stack-temple` (single Thais
  temple tile 32369,32241,7). Cluster packing stacks extra characters on the
  same tiles; 772 `SetOnMap` then `Create`s overflow onto the hometown temple
  (TVP `internalAddThing`). TFS-style temple `queryAdd` + disconnect is
  wrong here — rust follows the corpus Create.
- Shared OTBM: `./scripts/sync_tvp_world.sh` hardlinks Rust
  `data/world/forgotten.otbm` onto TVP `gameserver/data/world/map.otbm`.
  `run_tvp.sh` / `run_comparison.py` call it before TVP starts. TVP
  `enableMapDataFiles` stays `false` so `gamedata/map.tvpm` cannot overlay a
  different tile set.
- Spawns: TVP `tvpspawn` vs Rust nested XML, same 9950 zones, some radii
  differ. `items.otb` and Lua trees differ. Spawn density must be checked
  equal or monster count silently drives the result. The equivalence gate
  (§7) is what catches this.

### 3.5 Modified TVP (disclose)

`Ban::acceptConnection` is disabled in the local `reference/tvp-772` tree
(always `true`). Stock TVP FORCE_CLOSEs >5 accepts / 5 s with ≤500 ms gaps;
login + game is two accepts per bot, so a ramp looks like a flood. State this
against an unmodified TVP binary.

## 4. Workload

### 4.1 Two headline scenarios

The original single headline (`mixed_300`) measures walking and spectator
recalculation. With scattered bots and random cardinal walks, Melee / Rune
roles almost never have a target (`session.rs` binds `Attack` to
`InboundState.last_other_creature_id`, usually empty) and few monsters are
activated. That is a walk simulator, not a game server. Real server cost is
active monster AI, combat, and dense fan-out at depots and hunts.

Publication therefore carries **two** headline lines:

| Scenario | File | Layout | What it measures |
|---|---|---|---|
| City churn | `bench/scenarios/mixed_300.ron` (**frozen**) | `scatter` | wire, walk-ack, spectator recalc, login/chat noise |
| Hunt | `bench/scenarios/clustered_hunt.ron` (not frozen) | `cluster` | monster AI, combat pipeline, AoE × spectator fan-out |

`mixed_300.ron` is frozen: 300 bots, 120 s + 30 s warmup, seed 42,
`walk_period_ms 200`, `say_period_ms 2500`, roles Walker 0.40 / Melee 0.20 /
Caster 0.15 / Rune 0.10 / AoeRune 0.05 / Noise 0.10, `exori vis`, rune sprite
3155 in slot 10. Do not edit it between publication runs; new mixes are new
files.

`clustered_hunt.ron`: melee-heavy mix, `bots: 50` default (orchestrator
`--bots` overrides), `waypoint_file` set to a hunt loop, seeded on a
`cluster` layout around a `spawns.xml` center that has creatures on **both**
servers. Freeze it once the equivalence gate passes on it.

Isolation roles (`walker`, `melee`, `caster`, `rune`, `aoe_rune`, `noise`)
each have a 5-bot `.ron` for attribution and for the equivalence gate.
Those files use `walk_period_ms: 500` and `cyclops_hunt_loop.csv`. Seed
`--layout cluster` before `--mode equivalence` so login tiles match the
loop. Scatter cannot test combat (`unique_creatures` stays 0).

### 4.2 Server gates the bots respect

- Login ramp ≤ 8/s (Rust `MAX_CONCURRENT_LOGIN_LOADS = 8`, `login.rs`).
  Report login throughput separately; do not let it distort steady state.
- `say_period_ms ≥ 2500` (`RecordTalk` window, `chat_talk.rs`).
- `walk_period_ms` default 200 is **below** the seeded character's LinearGo
  step: vocation `base_speed` 70, level 50 → Go 119, GetSpeed 318, grass 150
  waypoints, ceil-to-`beat_ms` 50 → **500 ms**. Frozen `mixed_300.ron` stays
  at 200 (too-early walks become `walk.rejections` via §6.2). New scenarios
  (`walker_loop`, `clustered_hunt`, isolation roles) use 500. A replacement headline freeze is
  a later methodology change, not an edit of `mixed_300.ron`.
- Seeded characters are sorcerer (`vocation` 1), premium, have learned
  `Energy Strike`, hold SD (server 2268) in ammo slot 10 and GFB (2304) in
  left-hand slot 6. Loadgen resolves client look ids from `TFS_ITEMS_OTB`
  (per-server in `run_comparison.py`). `mixed_300.ron` still lists sprite
  3155 as a fallback only.

Cooldowns shape the **schedule**; they never drop a due timestamp.

### 4.3 Waypoint loops (replaces random cardinals)

`bench/waypoints/<name>.csv` — UTF-8, `#` comments, header `x,y,z`, tiles on
the shared 772 OTBM, walkable surface only, one floor, implicit loop back to
the first tile. Authored by hand from `data/world/` and walked once in-game.
Never import waypoints from other worlds or GPL trees.

Expander in `tools/loadgen/src/waypoints.rs` (not in `roles.rs`): polyline +
`walk_period_ms` + bot RNG → precomputed list of 772 cardinal move opcodes,
axis-aligned steps between tiles (diagonals become two steps), repeated to
fill `duration_s + warmup_s`. Bad CSV = data bug, not a runtime pathfinder.
Inbound `pos` is never used to replan. CSV tiles are authored from the 772
OTBM; in-game walk-through of the starter loops is still pending.

Desync so bots do not form a conga line — all folded into the precomputed
list, no extra packets:

- **Phase:** bot `i` starts at waypoint index `i % n`.
- **Lane:** optional ±1 tile offset perpendicular to the segment, clamp
  documented in the CSV comment.
- **Jitter bit:** swap horizontal-then-vertical vs vertical-then-horizontal.

Scenario field `waypoint_file: Option<String>` (repo-relative, default
`None`). Absent → today's random cardinals so existing RON files stay valid.

Starter loops: `thais_depot_loop.csv` (temple ↔ depot street),
`thais_temple_ring.csv` (tight ring around 32369,32241,7), one hunt loop per
clustered scenario.

### 4.4 Out of scope

Closed-loop AI (wait for ack, hibernate when unobserved, server-side A*),
in-process `Player` bots, cast/spectator logins, market/houses/PK content,
density caps. Loadgen must not pathfind and must not know the map at runtime.
No GPL code, Lua, CSVs, or coordinates from other bot trees — ideas only.

## 5. Loadgen — how the client works

`tools/loadgen` (bin `tfs-loadgen`) reuses `tfs-rust-net` (`rsa::encrypt`,
`xtea_tfs`, `protocol_game::encrypt_xtea_game_frame`,
`game_frame::read_sized_payload`) and `tfs-rust-common` opcode tables. It does
**not** depend on `tfs-rust-core`. Loadgen is the client; Python is the runner.
Do not grow loadgen into an orchestrator.

`--progress` prints one stderr line per second:
`connected / in_world / actions_sent / bytes_in / bytes_discarded /
disconnects / reconnects`. `run_comparison.py` always passes it. A stuck
login burst shows `connected=N in_world=0`; a kick under load increments
`disconnects` without changing offered load — loadgen does **not**
auto-reconnect during the window (`reconnects` stays 0).

Session: connect 7171 → RSA login packet → char list (`0x64`, no Adler on 772)
→ connect 7172 → RSA game packet → XTEA opcode stream.

Inbound parse is deliberately minimal (`inbound.rs`): self id (`0x0A`), own
position (`0x64` header / `0x6D` / NotifyGo `0xBF` z-down), `0x6C` remove,
`0x83` magic effect, `0x84` animated text (damage), `0xB5` cancel walk,
counters for everything else. Map bodies and login trailers are length-skipped
so a later `0x83` in the same payload still counts. Chat `SAY` is **not**
correlated as a spell; walk `0xB4` cylinder texts are histogram-only. A full
client decoder is out of scope. Do not quote walk/spell p99 unless
`skip_failures==0` and `bytes_discarded==0`.

Validation before any number is trusted:

- **Client ceiling** — `--walk-ns` null-echo mode; find where loadgen itself
  saturates. If near the intended bot count, we are measuring the client.
- **Byte stream is real** — diff loadgen output against a genuine client
  session recorded through `tools/packet-proxy`
  (`scripts/bench/proxy_log_to_frames.py`).

## 6. Measurement

### 6.1 Open loop (mandatory)

Every action is scheduled at a fixed intended `Instant` from a per-bot seeded
schedule. Latency = ack time − **intended** send time. If a prior action is
still outstanding when the next is due, that wait counts toward the next
action's latency. `hdrhistogram`, 3 significant figures.

A closed-loop bot stops generating load exactly when the server stalls and
records a few slow samples instead of the backlog real players would have
suffered (coordinated omission). No p99 from a closed-loop bot is publishable.

Correlation, per action type:

| SLO | Sent | Ack |
|---|---|---|
| Walk | cardinal move opcode | self `0x6D` |
| Spell / rune | spell-word `SAY` / `USE_ITEM_EX` rune | `0x83` at the target tile **with the expected effect id** (AoE runes; spell `SAY` stays pos-only). Chat `SAY` is **not** correlated. |

Rune `USE_ITEM_EX` is scheduled at `walk_tick + walk_period/2`, not on the walk
tick itself: the use still clears the in-flight step via ToDoClear, but a
rejected use no longer wipes the walk just sent. Server cancel text on `0xB4`
(`SendResult` / `SendMessage`) retires the oldest outstanding spell as
`spell_rune.rejections`, not a sample — matched on known `ReturnValue` texts
only, so MOTD / broadcasts never count. Report `spell_rune.rejections` next to
`samples`; the equivalence gate compares rejection rate as well.

### 6.2 Walk-ack correlation defect (blocker)

`latency.rs` `on_walk_ack` pops the **oldest** outstanding walk FIFO. A walk
the server rejects (wall, too-early) never produces a self `0x6D`. Until
`0xB5` retires that head, the next accepted walk's ack pops the rejected
timestamp. Effects (when cancel-walk is ignored):

- Every accepted walk after a rejection is charged ≥ 1 `walk_period` extra.
- The queue only grows (`outstanding_at_end`), so measured p99 drifts up over
  the run regardless of server speed.
- Rejection rate depends on walkability (`items.otb` differs) and walk-timing
  enforcement, which differ between the two servers — the bias is
  **asymmetric**.

Under random cardinals rejections are frequent, so walk p99 without this
fix measures "how often did this bot hit a wall". Two fixes, both required:

1. Retire the head outstanding walk on `0xB5` cancel-walk and record it as
   `walk.rejections`, not a latency sample. **Landed** (`CancelWalk` event,
   `on_walk_cancel`). Login-trailer `0xB5` with an empty queue is a no-op.
2. Waypoint loops (§4.3) so rejections are rare in the first place.

Report `walk.rejections` next to `walk.samples`; the equivalence gate
compares rejection rate as well.

### 6.3 Sampling

`scripts/bench/sample_proc.py` at 1 Hz from `/proc/<pid>`, identical on both
servers:

- `utime`/`stime` → CPU-seconds and CPU%
- RSS (`stat`) and PSS (`smaps_rollup`)
- thread count; per-thread `comm` + CPU (`--threads-out`)
- voluntary / nonvoluntary context switches
- `/proc/<pid>/io`
- bytes/packets from `/proc/<pid>/net/dev` (netns-wide, includes `lo`)

Game-thread CPU vs process CPU (diagnosis only): Rust names the game OS
thread `game` (`std::thread::Builder::name` in `run_server.rs`) so
`threads.csv` `comm` is stable. TVP `comm` names (dispatcher vs asio workers)
must be read from a live `threads.csv` on the pinned host — do not guess.
Optional overlay in `plot_results.py` on the time-series CPU panel, never on
the publication CPU chart or `cpu_per_action.png`. Do not add
`CLOCK_THREAD_CPUTIME_ID` sampling inside the game loop for A/B.
`GameObs` (`RUST_LOG` must include `tfs_obs=info`; the runner appends it)
splits game-thread work the beat wall misses: `command_dispatch_us`,
`flush_outgoing_us`, walk/use/talk/other packet µs, `lua_callback_us`. After
a clean window, `perf record -F 99 -g -p <tfs-rust-pid>` names functions.

### 6.4 Headline metrics

TVP is asio-multithreaded; Rust is one game thread plus Tokio I/O. Raw CPU%
rewards or punishes that difference. Report both:

- **(a)** max concurrent bots sustained with walk-ack p99 < 100 ms, per
  headline scenario (`mixed_300`, `clustered_hunt`).
- **(b)** CPU-seconds per delivered action (`walk.samples +
  spell_rune.samples`) and per outbound byte.

Plus load curve (25/50/100/200/300/400/600 bots: CPU%, RSS, p99), steady-state
1 Hz time series at 300, overload past each knee (degrade vs collapse), and a
3600 s soak at 200 bots for RSS growth.

## 7. Content-equivalence gate (blocker before any chart)

```
python3 scripts/bench/run_comparison.py --mode equivalence --reps 1
```

Runs 5 bots of each isolation role against rust then tvp. The engines
differ; this gate only checks that **the same actions were sent** and the
sessions stayed up. Combat outcomes are logged, not gated.

| Observable | Threshold | Why |
|---|---|---|
| `sends` | 5% | same open-loop schedule |
| `skip_failures` and `bytes_discarded` | both 0 | loadgen decoder health; walk/spell p99 is not a latency if either is non-zero |
| `spell_rune.rejections` / `spell_rune.samples` | 25% (skip if both 0) | scheduled use actually ran (empty-slot / sprite miss) |
| `disconnects` / `bots` | 25% (skip if both 0) | bots stayed in world |
| `reconnects` | 25% (skip if both 0; both should be 0) | same |

Not gated (server-side, expected to differ): `damage_sum`, `magic_effects`,
`unique_creatures`, `bytes_in` per send, `walk.rejections`.

The 600-bot chart is the same RON + bot count on each server. Isolation
roles exist to catch “one side never used the rune” before that chart.

Seeded bench characters carry non-zero `players.lastlogin` (both schemas) so
`firstlogin.lua` starter gear never runs on them; the script itself also
skips occupied slots. Cluster cells seed `--health 5000` (both servers
identically) so Cyclops focus does not kill a 1000-HP bot mid-cell — a death
drops inventory and respawns at temple, poisoning the combat rows.

If the two servers do not send the same actions at 5 bots, the 600-bot
chart is not the same workload. **Publication layout is `--layout cluster`**
(Cyclops `32776,32240,7`) plus the isolation RONs so rune/walk packets
actually fire. Scatter isolation never sees monsters; that is fine for a
city-churn headline, not for hunt. Freeze `clustered_hunt.ron` after this
action-sameness gate passes. Server-side RNG and combat fan-out are not
locked across processes.

```
python3 scripts/seed_bench_accounts.py --count 10 --apply --target both --layout cluster
python3 scripts/bench/run_comparison.py --mode equivalence --reps 1
```

`--mode equivalence` re-applies that cluster seed before **each** isolation
cell. Logout after death persists temple (or walked-off) `pos*`; without a
reset, later roles are not on Cyclops.

### 7.1 Live cluster gate — **PASS** (actions, `results/20260913T083946Z`)

Isolation RONs, `walk_period_ms: 500`, `cyclops_hunt_loop.csv`, per-cell
`--layout cluster` re-seed (`--health 5000`), 5 bots × 30 s. Includes item
wipe (`player_items` under `FOREIGN_KEY_CHECKS=0`), `lastlogin`, chase 100 ms,
rune offset + effect-id ack. Sends matched. `disconnects`/`reconnects` 0.
`bytes_discarded`/`skip_failures`/`unknown_opcodes` 0.

GFB now fires on rust (`aoe_rune` 60/60 spell acks, 0 outstanding). Gate
still fails every combat row:

| Role | rust dmg / ME / cre / shoots | tvp dmg / ME / cre / shoots |
|---|---|---|
| walker | 4868 / 172 / 6 / 0 | 13550 / 296 / 3 / 0 |
| melee | 5738 / 219 / 2 / 0 | 16080 / 462 / 4 / 0 |
| caster | **379592** / 343 / 7 / 0 | 13842 / 685 / 8 / 0 |
| rune | 6380 / 369 / 6 / 0 | 9139 / 448 / 9 / 0 |
| aoe_rune | **1537469** / 6822 / 11 / **170** | 27860 / 11119 / 15 / **325** |
| noise | 3257 / 154 / 3 / 0 | 11515 / 389 / 5 / 0 |

Rust `aoe_rune` ME is now thousands (was 302). Combat fan-out still differs
(TVP denser spawn / more effects); that is **not** a gate. Action-sameness
on this run: `sends` matched every role, `disconnects`/`reconnects` 0,
`spell_rune.rejections` 0, `bytes_discarded` 0.

Prior FAIL `20260913T081030Z` (pre item-wipe): aoe rust 3869/302/3/**0**.
`20260913T070214Z` and `20260913T062814Z` as before.

### 7.2 Root cause of the rust `aoe_rune` collapse (inventory wipe landed)

A 1-bot probe against the release binary showed rust `distance_shoots` 0 and
a ToDo trace with zero `enqueue_player_use` events: the seeded GFB never
reached slot 6. `firstlogin.lua` (`getLastLoginSaved() == 0`, true for every
freshly seeded char) did `addItem(2382 → CONST_SLOT_LEFT)`, and the explicit-
slot fallback placed a coat in the left hand; every `USE_ITEM_EX` then failed
sprite validation (`NotPossible`) before queueing. Side effects of the same
failure: each rune packet still cleared the in-flight walk (shared tick), and
the 26 "acks" were blood splashes on the monster tile matching the pos-only
ack. Landed: seeder `lastlogin = UNIX_TIMESTAMP()` (both schemas),
`firstlogin.lua` occupied-slot guards, rune schedule offset `+walk_period/2`,
`0xB4` cancel text → `spell_rune.rejections`, effect-id ack for AoE runes
(`aoe_rune_effect`, default FIREAREA 7). Re-run `20260913T081030Z` still has
rust `aoe_rune` `distance_shoots = 0` and ME 302. Cause: seeder
`SET FOREIGN_KEY_CHECKS=0` then `DELETE FROM players` skips CASCADE, so
`player_items` kept firstlogin coats/torches and stacked a GFB row per cell.
Hydrate last-write on pid 6 often was not 2304; sprite check against client
3191 cancelled before `enqueue_player_use`. Seeder now deletes `player_items`
and `player_spells` by id first. Post-wipe seed is clean (one 2304 on pid 6,
one 2268 on pid 10). 1-bot rust probe after wipe: `distance_shoots = 6`,
`magic_effects = 241` in 12 s (was 0 shoots). Gate not re-run.

### 7.3 Monster-engagement gap (`walker`/`melee`/`noise`, corpus-adjudicated)

Melee cadence is a match (2000 ms post-hit both sides), so the ~2.5× TVP hit
gap is engagement, not swing rate. Corpus (`tibia-game-master`) adjudication
of the five candidate deltas:

| # | Delta | Verdict |
|---|---|---|
| 1 | Lose-target 5%/idle roll | Corpus agrees with **rust** (`crnonpl.cc:2431`, `cyclops.mon:18`); TVP `changeTargetChance` maps to corpus `LoseTarget` |
| 2 | Out-of-home while attacking | Corpus agrees with **rust** — despawn via `StartLogout` (`crnonpl.cc:2412-2413`), no walk-home |
| 3 | Path failure during chase | Corpus agrees with **rust** — NOWAY → `Target = 0` + roam tail (`crnonpl.cc:2895`, `:2920-2933`) |
| 4 | Chase re-think wait | Corpus agrees with **TVP** — active re-arm is `ToDoWait(100)` (`cract.cc:1359`) / 200 on target move (`crmain.cc:955`); 1000 ms is roam/idle only |
| 5 | Sight 10 / wake sources | Corpus agrees with **rust** (`crnonpl.cc:2423-2424`, `:2966-2982`, `operate.cc:937`) |

Fixed #4: rust's target-retained retry paths
(`monster_combat_handle_close_chase_blocked` off-band arm, both `Retry` arms,
move-stimulus `Retry` arm) waited `MONSTER_IDLE_WAIT_MS` (1000). They now wait
`MONSTER_CLOSE_CHASE_RETRY_MS` (100). Cadence is still enforced by the 2000 ms
`DelayAttack` gate, so this only re-arms sooner. Spawn placement also
contributes to initial density (TVP scatters all monsters within radius 30;
rust follows the corpus minimize-first/extend-later within radius 10), but
rust's placement is the corpus behavior — not changed. Re-run
`20260913T081030Z` still shows walker/melee/noise damage ~2.3–2.5× on TVP;
100 ms re-arm did not close the gap.

## 8. Remaining work (parent implements; sub-agents research only)

Harness items 1–7 landed in code. Isolation RONs use the Cyclops loop at
500 ms; inbound length-skips `0xA7`/`0xD3`/`0xD4` and reports
`unknown_opcodes`. Live cluster **action-sameness PASS** — see §7.1
(`results/20260913T083946Z`): same `sends`, 0 drops, 0 spell rejects. Combat
fan-out still differs; not gated. `clustered_hunt.ron` freeze is optional
before a load-curve. In-game walk of the CSV loops still pending.

1. **Walk-ack retire on `0xB5`** — **done.**
2. **Waypoint CSV + expander + `waypoint_file`** — **done** (`waypoints.rs`,
   `thais_depot_loop.csv`, `walker_loop.ron`). In-game walk of the loops still
   pending.
3. **`--layout cluster`** — **done** (default center `32776,32240,7`).
   Equivalence gate on cluster layout, then freeze `clustered_hunt.ron`.
4. **`walk_period_ms`** — **verified 500 ms** for level-50 sorcerer / grass.
   Frozen `mixed_300.ron` stays 200; new RON files use 500.
5. **Game-thread `comm`** — **done** (named `game` thread + diagnosis overlay).
6. **Orchestrator** — **done** (default load-curve runs both headlines).
7. **`--vocation-cycle`** — **done** on the seeder.
8. **Rune-ack retire on `0xB4` + effect-id ack** — **done** (`SpellRejected`
   event on known `ReturnValue` texts → `spell_rune.rejections`; `UseItemEx`
   carries `expect_effect`, `aoe_rune_effect` default FIREAREA 7; rune tick
   offset `+walk_period/2`). Gate compares `spell_rejections_per_sample`.
9. **Seeder `lastlogin` + `firstlogin.lua` guards** — **done** (non-zero
   `lastlogin` both schemas; script skips occupied slots, nil-guards the
   backpack). Cluster cells seed `--health 5000` both sides.
10. **Seeder wipe `player_items` under `FOREIGN_KEY_CHECKS=0`** — **done**
    (CASCADE does not fire; leftover slot-6 coats blocked GFB).

Crate placement: wire work in `tools/loadgen` and `bench/`; seeder and
orchestrator stay Python; nothing in `tfs-rust-core` beyond the existing
read-only obs access. No in-process bots in core.

## 9. Reproduce

```
./scripts/build_tvp.sh                 # stock (-Og)
./scripts/build_tvp.sh --o3            # sensitivity build, label tvp-o3
python3 scripts/seed_bench_accounts.py --count 600 --apply --target both
python3 scripts/bench/run_comparison.py --mode equivalence --reps 1
python3 scripts/bench/run_comparison.py --mode load-curve --reps 3 \
  --cpuset-server 0-7 --cpuset-loadgen 8-15
python3 scripts/bench/run_comparison.py --mode load-curve --reps 3 \
  --scenario bench/scenarios/clustered_hunt.ron \
  --cpuset-server 0-7 --cpuset-loadgen 8-15
python3 scripts/bench/plot_results.py results/<timestamp>
```

Engineering loop:

```
rtk cargo bench -p tfs-rust-core --bench hot_paths -- --noplot --quick
python3 scripts/bench/check_regression.py          # vs cached main baseline, +25% fails
target/release/scale_sweep --axis monsters --points 50,100,200,400,800 --beats 600
scripts/profile_sim.sh --axis monsters --n 400    # flamegraph of our own binary
```

Loadgen verification:

```
rtk cargo test -p tfs-loadgen
rtk cargo clippy -p tfs-loadgen --all-targets -- -D warnings
python3 scripts/seed_bench_accounts.py --self-test
python3 scripts/bench/check_equivalence.py --self-test
```

## 10. Publication checklist

- [ ] §8 items 1–3 landed; `clustered_hunt.ron` frozen
- [ ] Client ceiling measured (null echo) and ≫ max bot count
- [ ] Loadgen byte stream diffed against a real client capture
- [x] Equivalence gate (action-sameness) passes on **cluster** (`20260913T083946Z`, §7.1)
- [ ] `tvp` and `tvp-o3` both run; both on the chart
- [ ] Alternating cells, ≥ 3 reps, median + spread, pinned cpusets, governor
  `performance`, `meta.json` present
- [ ] Loadgen CPU recorded and shown not to saturate
- [ ] Modified-TVP (`Ban::acceptConnection`) and persistence asymmetry stated
- [ ] Raw CSVs + harness (`scripts/bench/`, `tools/loadgen`,
  `bench/scenarios/*.ron`, `bench/waypoints/*.csv`) published with the charts
