# Player CTalk → ToDoTalk (2026-09-17)

Decompile `CTalk` enqueues `TDTalk` + `ToDoStart` (`receiving.cc:901-903`); `Talk()` / spells run in `Execute` (`cract.cc:848-856`). Rust ran `player_say` in the packet handler (`talk_us`). Match corpus: enqueue via `creature_todo_add` (clear only if `LockToDo`), execute `player_say` on drain. NPC/Hicks keep `check_spamming=false` viewport path. Lua `creature:say` stays immediate. `mixed_300.ron` frozen. No `0xBF`.

- [x] Extend `CreatureAction::Talk` (mode / channel / addressee / check_spamming)
- [x] `player_request_say` + `game_loop` Say packet; execute arm calls `player_say`
- [x] Tests: SAY not on wire until `move_creatures`; existing `player_say` body tests unchanged

# 200-bot rust soak (2026-09-17)

Leak slope, not boot RSS. Rust-only (TVP ServerSave at 04:30). `mixed_300.ron` stays frozen. No `0xBF` work.

- [x] `run_comparison.py --mode soak --servers rust --bots 200 --reps 1 --scenario bench/scenarios/ms_swarm.ron --disable-saves` (3600 s + 30 s warmup) — `results/20260916T221147Z`
- [x] Gate: RSS/PSS **+6.1 MiB / h** after login (1476 → 1482 MiB); explained by `decay_live_max` 1215 → 36874 (hunt_pressure corpses, 1800 s first stage). `todo_heap_max` 467 → 294 (down). `output_queued_bytes_max=0`, 0 disconnects. Not a glibc/TVP-style leak. Decoder still dirty (`skip_failures=1295`) — do not quote walk p99. Next: `0xBF` skip.

# GameObs CPU split (2026-09-17)

Diagnosis only — split the ~84% `comm=game` CPU (1000-bot cell) that `beat_wall_ms` does not cover. No per-packet INFO. `mixed_300.ron` stays frozen. No `0xBF` skip work. No `CLOCK_THREAD_CPUTIME_ID` in A/B.

- [x] `GameObs`: `command_dispatch_us`, `flush_outgoing_us`, walk/use/talk/other_cmd µs + counts, `lua_callback_us`
- [x] Time drain / `handle_game_packet` class / `LuaCallback` / `flush_pending_outgoing` in `game_loop.rs`
- [x] Force `,tfs_obs=info` in `run_comparison.py`; `parse_obs_log.py` + BENCHMARK §6.3
- [x] Tests: obs record + classifier table (`cmd_class` / `classifies_walk_use_talk_other`)
- [x] Rust-only 50-bot `ms_swarm` 60s `results/20260916T214628Z`. `tfs_obs` windows emit. After `--skip 3`: dispatch p50 4 µs / p99 512–1024; flush p99 8–16 µs; walk ~81–1009/10s at 2–4 µs p99; talk ~17–202/10s at 32–64 p50 / 256–512 p99; **use_cmds=0** (casters SAY spell words); lua 10/10s (hunt_pressure) 128–256 µs; beat `todo_us` is the sim cost. Residual skip 4×`0xBF` — do not quote walk p99.

# Fix list after ms_swarm 1000-bot A/B (2026-09-16)

Do **not** chase game-thread CPU. Cell `results/20260916T074546Z`: 1000 in-world, beat p50 16 / p99 32 ms, game thread ~25% of one core. TVP is `-Og`. Walk p99 / SAY reject / damage rows are decoder + metric + combat parity, not throughput. `mixed_300.ron` stays frozen.

Rep0: Rust `skip_failures=3075` / `bytes_discarded=168219` / `unknown_opcode_first=99`; TVP zeros. Walk p99 5022 ms vs 109 ms. Spell reject 20095/27001 vs 3044/43293. Damage per `0x84` sample ~4876 vs ~78.

- [x] Phase 0 — loadgen diagnosability: skip_failure opcode histogram; unknown-opcode counts (not first-only); `0xB4` reject-text histogram; chat SAY must not enqueue `SpellRune`; decoder-health gate is `skip_failures==0` and `bytes_discarded==0` (no p99 quote otherwise). Tests. 50-bot rust-only is enough to name the skip opcode.
- [x] Phase 1 — Rust-only inbound desync (walk p99). `0x63` is a map-object creature tag, not a server opcode. Loadgen skips `0xBF` using NotifyGo’s new z (`0x6C` has no dest). 1-bot / 5-bot isolation: skip=0, walk p99 ~35–45 ms. Residual at 50/1000: `0xBF` skip at `player_z=8`, peek `11241200ff…` (client id `0x2411` not in `items.otb`). 1000-bot rust-only `20260916T103354Z`: skip 54 (was 3075), discarded 36 kB (was 168 kB), walk **p50 42 ms / p95 199 ms / p99 still 5 s** from the desynced tail. Unique creatures 3490. No 3-rep A/B until `0xBF` skip is 0.
- [x] Phase 2 — SAY/spell rejects after the metric split. Chat SAY does not enqueue `SpellRune`. Walk `0xB4` cylinder texts are histogram-only. 1000-bot reject 1072/27040 (~4%) vs old 20095/27001 (~74%) and vs TVP ~7%. Remaining texts: PZ 794, mana 478; no exhaust/secure. SAY p99 is outstanding-at-end (3762).
- [x] Phase 3 — Combat isolation `bench/scenarios/ue_isolation.ron` (`20260916T102522Z`). Decoder clean both sides. Rust `damage_sum=0` / `unique_creatures=0` / ME 4128; TVP 172 / 11 samples (~16 each) / 3 creatures / ME 3637. hunt_pressure on both (lesson 486). Keep `damage_sum` ungated. 1000-bot per-sample still ~4839 (fleet-visible `0x84`).
- [x] Rerun: rust-only 50 (`20260916T103042Z`) then 1-rep 1000 (`20260916T103354Z`). **No 3-rep A/B** — decoder not green. TVP `-O3` / matplotlib still housekeeping.

# MS swarm + dual hunt-pressure (2026-09-16)

Same `tfs-loadgen` binary vs Rust and TVP. `mixed_300.ron` stays frozen. Map-wide deaths are pack Lua (not a Rust-only cull).

- [x] Loadgen: `chat_messages` + `spell_words_pool`; caster mixes UE/UH + chat; Noise uses chat pool; phase-offset says. New `bench/scenarios/ms_swarm.ron`.
- [x] Seeder `--preset ms-swarm`: vocation 5, level 80, HP 30000, mana 200000, maglevel 80, `Ultimate Explosion` + `Ultimate Healing`. `--spell-name` comma-separated. Both SQL dialects.
- [x] Pack: `eventcallbacks/monster/hunt_pressure.lua` (`onSpawn` id list) + `globalevents/hunt_pressure.lua` (`onStartup` + `addEvent`, not `:interval`). Kills `#Game.getPlayers() * 0.30 / 12` via `Creature(id):addHealth(-max)`.
- [x] Rust allowlist those two files (`scripts_interface.rs`). No new `config.lua` key (TVP enum).
- [x] Tests: loadgen RON + say mix; seeder self-test; `real_pack_allowlist` ONSPAWN + HuntPressure GE.
- [x] TVP pack: copy both Lua files under `gameserver/data/scripts/`; enable `events.xml` Monster `onSpawn` (was 0 — EventCallback never ran). Not `creaturescripts.xml` (empty; would need per-monster `registerEvent`).

# Phase C — boot (unstick RSS)

Drop `HashMap<Position, TileData>` staging so the Phase B −567 MiB live saving shows in RSS. No `malloc_trim`. Load-order side effects (tile flags, house tiles, refresh set) stay identical. Crate split: content walks OTBM; core converts each tile immediately.

- [x] C1a: `OtbmFile` + `visit_tiles` (`otbm.rs`). Parse tree + towns/waypoints/attrs in `open`; yield one tile at a time (reused `things` Vec). `MapData` is metadata only (no tiles HashMap).
- [x] C1b: `Map::from_otbm` in `map/otbm_load.rs` — callback → `tile_from_data` → `SparseGrid::insert_tile`; `shrink_to_fit` at end. `from_map_data` stays for synthetic tests.
- [x] C1c: `pipeline.rs` `Arc<ItemDatabase>` (clone Arc into monster `spawn_blocking`); `Content` holds `OtbmFile`. `run_server` / sim `Map::from_otbm`.
- [x] Unit: visit fixture (tile/housetile/town, 2 tests); `from_map_data` house-tile index; `from_otbm_streams_housetile_into_grid`
- [x] `cargo test -p tfs-rust-content --lib otbm` (2 pass); `cargo test -p tfs-rust-core --lib map::` (16 pass); `cargo check -p tfs-rust` bins; clippy on touched files (no new lints)
- [x] Gate: boot log `map_tiles=7848819` `map_chunks=3355` `items_slotmap=8565829`; tree 485 ms + stream 901 ms = **1.39 s** (was 5.1; TVP 2.8). Raw VmRSS **1474 MiB** (was 2386, −912). HWM=RSS (no staging high-water). `2026-09-16T06:47:40Z` release `./target/release/tfs-rust`.

# Phase A — send-path coalesce (CPU)

Decompile `SendData` already emits one XTEA frame per beat per connection (`communication.cc:373-410`, `sending.cc` `SendAll`, `main.cc:455`). The game thread already batches; the writer re-split per logical packet. Restore that wire shape (CPU win, not a mechanics change). Split threshold from the active codec (`OutData[16384]` / 1098 `MAX_PROTOCOL_BODY_LENGTH`), never TVP 24572.

- [x] A1: `frame_coalesce.rs` — concat beat payloads, XTEA once, one `write_all`; login port keeps `encrypt_xtea_game_frame`
- [x] A2: reusable per-connection `Vec` (header reserved, encrypt in place, write slice)
- [x] Unit: n packets → 1 frame; decrypt = concat; oversized split; 772 (no Adler) + 1098 (Adler); scratch does not realloc when capacity suffices
- [x] `cargo test -p tfs-rust-net` (167 passed); `cargo clippy -p tfs-rust-net --lib --profile test -- -D warnings`
- [x] Lesson: per-message writer was a silent `SendData` deviation
- [x] A3: `run_comparison.py --mode steady --bots 1000 --reps 1 --duration-s 120 --scenario bench/scenarios/clustered_hunt.ron` — first cell (`20260914T095120Z`) **invalid**: stale Sep-13 binary (`CARGO_TARGET_DIR` pointed at the sandbox cache). Valid cell `results/20260914T102600Z/1000_bots_cpu_mem.md`: **CPU gate passed** — 33.4 CPU-s vs TVP 43.0 (old writer 51.1); tokio ~12.7 s (was 36.7); `lo` 38.6 kpps (was 100.6 k); 18.0 frames/conn/s vs TVP 41.8; 450 000 sends / 0 disconnects both
- [x] Bottleneck hunt: per-thread user/sys + ctx-switch sampling → per-`send()` loopback kernel work, not wake-ups/XTEA (lesson 483); loadgen `frames_in` counter; runner resolves binaries via `CARGO_TARGET_DIR` and records `*_bin_mtime`
# Phase B — static memory (live heap)

Layout only. Stack order, flags, zone, sector-refresh timing, and spectator/sector order unchanged.

Gate is **live heap bytes** (malloc census) or **RSS after malloc_trim**, not raw `ps` RSS after `GameWorld ready`. Raw RSS measures glibc’s OTBM-staging high-water mark; HEAD recycled those 96-B chunks into `Box<Tile>`, Phase B `Vec<Tile>` does not. C1 (single-pass OTBM) is the cause fix — no `malloc_trim` in this phase.

- [x] B1: `tile_stacks.rs` — `TileBody.stacks: Option<Box<TileStacks>>`; allocate on first non-ground insert; compact when empty; accessors `down_items()` / `top_items()` / `creatures()`. Tile 48 B / TileBody 32 B; stacks boxed on ~600 k tiles
- [x] B2: snapshot REFRESH tiles on first mutation (`refresh_positions` at load; raster from positions, not snap keys). Boot `refresh_snapshot_count=0`
- [x] B3: dense chunk `Vec<Tile>` + `[u16; 4096]` slot index (`u16::MAX` = empty); `shrink_to_fit` after map load
- [x] `cargo test -p tfs-rust-core` (1412 pass / 29 pre-existing fail, same as HEAD); boot log counts `map_tiles=7848819` `map_chunks=3355` `refresh_snapshot_count=0`; live heap 1893 → 1326 MiB (−567). Raw ps RSS 2129 → 2386 is the staging lie
- [x] Walk p99 tens of seconds on rust vs ~260 ms TVP (silent-drop / `0x6D`/`0xB5` parity) — Phase D, not CPU. Reopened by `20260916T074546Z`. After loadgen histograms + `0xBF` z-tracking: 1000-bot rust-only walk **p50 42 ms / p95 199 ms**; p99 still 5 s while residual `0xBF` skip (54) remains. 1/5-bot cells are clean (~35–45 ms p99).

# 1000-bot clustered hunt cell (2026-09-13)

Same hunt as 600-bot `20260913T110154Z`. Unique tiles (`--cluster-radius 17`). `--skip-build`. TVP is A/B only.

- [x] Seed 1000 `--layout cluster --cluster-radius 17 --health 5000 --target both`
- [x] `run_comparison.py --mode steady --bots 1000 --reps 1 --duration-s 120` + cpusets `--skip-build`
- [x] Write `results/20260913T111046Z/1000_bots_cpu_mem.md`

# 600-bot clustered hunt cell (2026-09-13)

Same hunt as 300-bot `20260913T104736Z`. Unique tiles so login does not stack (`--cluster-radius 13`, 729 candidates). Temple `Create` already in rust. Do not treat TVP totals as corpus.

- [x] Seed 600 `--layout cluster --cluster-radius 13 --health 5000 --target both`
- [x] Confirm workspace `tfs-rust` has SetOnMap Create string; `--skip-build` if yes
- [x] `run_comparison.py --mode steady --bots 600 --reps 1 --duration-s 120` + cpusets
- [x] Write `results/20260913T110154Z/600_bots_cpu_mem.md`

# SetOnMap temple Create on login overflow (2026-09-13)

300-bot clustered hunt dropped 131 rust sessions: TFS temple `queryAdd` refuses a second creature, then disconnect. 772 `TCreature::SetOnMap` (`cract.cc:327-349`) assigns `startx/y/z` with **no** re-search and `Create` / `CheckMapPlace(0)` (skips `IsMapBlocked`). TVP `Map::placeCreature` `internalAddThing` at temple. Do **not** `FLAG_NOLIMIT` (void/UNPASS walls).

- [x] `place_player_on_login`: saved-pos search unchanged; temple fallback `commit_set_on_map_create` if tile has ground
- [x] Tests: occupied temple stacks; missing town / void temple still `None`
- [x] Lesson + `docs/BENCHMARK.md` §3.4 stack note
- [x] `cargo test` placement/login; clippy on touched files

# Skip SHA1→bcrypt upgrade on bench login (2026-09-13)

50-bot hunt spike was ~12 CPU-s of `tokio-rt-worker` bcrypt cost-12 rehash (`account.rs` `needs_upgrade`), not map send. TVP verifies SHA1 and stops. Seeder writes SHA1 every cell.

- [x] `PasswordHashConfig.upgrade_sha1_on_login` + `config.lua` `upgradeSha1OnLogin` (default true)
- [x] Gate `hash_bcrypt_async` in `verify_loaded_account`
- [x] `run_comparison.py` sets `TFS_UPGRADE_SHA1_ON_LOGIN=0` for rust cells
- [x] `docs/BENCHMARK.md` §3 disclose auth asymmetry
- [x] Tests: config parse + skip-upgrade gate
- [x] Rebuild rust; re-run 50-bot clustered hunt → `results/20260913T100956Z` CPU max 22% (was 788%)

# Spawn-boot CPU cuts (2026-09-13)

Same placement tiles. No parallel game-thread spawn (`GameWorld` is not `Send`). TVP `internalPlaceCreature` at boot does not run target-list / idle-yield (`spawn.cpp:397-401`, `game.cpp:498-511`).

- [x] Skip `monster_on_creature_appear_self` when `startup==true` in `spawn_monster` (respawn / `createMonster` / summons still call it)
- [x] Reuse `SearchSpawnField` phase buffer via game-thread `thread_local` (`spawn_placement.rs`)
- [x] Stop full `MonsterType::clone()` per spawn — snapshot scalars + loot; `finish_monster_spawn` takes `&[LootBlock]`
- [x] Skip second `recompute_monster_combat_from_equipment` when `onSpawn` is unregistered; skip first recompute when loot is empty
- [x] Tests: startup no ToDoYield; respawn still yields; spawn-hook / placement tests
- [x] Verify spawn/hook/placement unit tests; clippy on these files is clean (`--no-deps --lib` still has unrelated pre-existing lints)

# Cluster equivalence FAIL documented (2026-09-13)

Live 5-bot cluster gate with per-cell re-seed: walk-ack healthy, combat visible, TVP AoE/creature counts over threshold. Do not load-curve. Do not freeze `clustered_hunt.ron`.

- [x] `docs/BENCHMARK.md` §7.1 `results/20260913T070214Z` (aoe_rune ME 302 vs 11222)
- [x] §8 + publication checklist point at that FAIL
- [ ] Spawn/AoE parity (why TVP sees more creatures and GFB effects)

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
