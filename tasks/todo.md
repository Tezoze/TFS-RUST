# Map-skip other-bot speak body + leftover 0xFF skip-stream (2026-09-20)

1000-bot `20260920T013342Z`: skip=9 (`0x68`×8 + `0x65`×1), discarded=1242, unknown=14 (`255`×10). First skip peek `00000600Test22…` at z=7 is a speak-shaped body after an omitted `SendRow` (name + SAY + pos + `bots bots bots`), not eof. First unknown `0xFF` after `0xA0`, peek `ff2a11…` — drain returned on leftover `0xFF` instead of skip-tile. Drain leftover `0xFF` as skip-stream when `prev` is set; 0-floor map skip if the body is speak-shaped. Do not eat first-opcode `ff98…`. `mixed_300.ron` frozen. Quote p99 only if both decoder fields are 0. No 3-rep A/B until both-zero.

- [x] `inbound.rs`: drain `0xFF` via skip-tile; omitted-row speak body on map skip fail
- [x] Tests: `0xA0`+`0xFF`+tile+ping; `0x68`+Test22 say+ping; `0xFF`+`0x98` still unknown
- [x] `cargo test -p tfs-loadgen` — 94 passed
- [x] 50-bot `20260920T014520Z`: skip=0 discarded=0 unknown=0. Walk **p50 29.5 ms / p95 41.7 ms / p99 284 ms**. Spell **p50 59.1 ms / p95 4.58 s / p99 5.83 s**. 1 disconnect.
- [x] 1000-bot `20260920T014718Z`: 1000 in-world, 0 disconnects. **skip_failures=2** (`0x65`+`0x68`), **bytes_discarded=720**, unknown=2 (`0x20`/`0x62`). First skip peek `0400ff68…` at z=8 (real skip-stream). First unknown `0x20` after `0xB3`, peek `20626f6f70` (` boop`). No Test22, no opcode 255. Not both-zero — do not quote p99.
- [x] lessons.md + todo cell result

# 0x68 eof + leftover skip after 0xA0 (2026-09-20)

1000-bot `20260920T011720Z`: skip=7 (`0x68` peek `eof` z=8), discarded=1942, unknown=20 (first `0x04` after `0xA0`, peek `0400ff…`). Trailing skip pair `[n, 0xFF]` with `n=0x68` is parsed as SendRow (over-read to eof). Extra skip tiles after `0xA0` stay at top-level because drain stops on a known opcode. Eat leftover skip pairs when the following byte is a known opcode; drain unread skip tiles after a parsed packet (not only after map skip). Do not eat `0x6D 0xFFFF` or `ff98…`. `mixed_300.ron` frozen. Quote p99 only if both decoder fields are 0. No 3-rep A/B until both-zero.

- [x] `inbound.rs`: leftover skip-pair drain; drain after non-known byte when `prev` is set; 0-floor `0x68` if body starts with a known opcode or eof
- [x] Tests: `[0x68,0xFF]` after west row; `0xA0` then leftover tile then ping; `0x68` then ping; `0x15` still unknown
- [x] `cargo test -p tfs-loadgen` — 92 passed
- [x] 50-bot `20260920T013144Z`: skip=0 discarded=0 unknown=0. Walk **p50 28.0 ms / p95 51.8 ms / p99 149 ms**. Spell **p50 49.2 ms / p95 78.8 ms / p99 3.57 s**. Outstanding 14.
- [x] 1000-bot `20260920T013342Z`: **skip_failures=9** (`0x68`×8 + `0x65`×1), **bytes_discarded=1242**, unknown=14 (`255`×10 first). First skip peek `00000600Test22…` at z=7 (other-bot name in the strip, not eof). First unknown `0xFF` after `0xA0`, peek `ff2a11…` (skip-stream; next is not a known opcode). 1 disconnect. Not both-zero — do not quote p99.
- [x] lessons.md + todo cell result

# 0xBF 0-floor then 0x6B + leftover skip after 0x68 (2026-09-20)

1000-bot `20260920T010424Z`: no opcode 255. skip=3 (`0xBF` z=8 peek `6bfa7d…` = `0x6B` update-tile-thing, not skip-stream). discarded=150. unknown=1 (`0x2F` after `0x68`, peek `2f1171…` client id). Add `0x69`/`0x6A`/`0x6B` to `send_floors_omitted_body`. After a successful map skip, drain extra skip tiles until a known opcode. `mixed_300.ron` frozen. Quote p99 only if both decoder fields are 0. No 3-rep A/B until both-zero.

- [x] `inbound.rs`: omitted-body rewind includes tile-update opcodes; drain leftover skip tiles after map/floor skip
- [x] Tests: `0xBF` then `0x6B`+ping at z=8; `0x68` then leftover tile then magic
- [x] `cargo test -p tfs-loadgen` — 89 passed
- [x] 50-bot `20260920T011518Z`: skip=0 discarded=0 unknown=0. Walk **p50 37.9 ms / p95 45.5 ms / p99 266 ms**. Spell **p50 42.4 ms / p95 691 ms / p99 4.35 s**. Outstanding 17.
- [x] 1000-bot `20260920T011720Z`: 1000 in-world, 0 disconnects. **skip_failures=7** (`0x68` peek `eof` z=8), **bytes_discarded=1942**, unknown=20 (first `0x04` after `0xA0`, peek `0400ff…`; counts `4`×7 / `255`×5 / `5`×3 / `7`×2 / `0`/`1`/`17`). **No `0xBF` skip, no `0x2F` unknown.** Not both-zero — do not quote p99.
- [x] lessons.md + todo cell result

# Leftover top-level 0xFF (2026-09-20)

1000-bot `20260920T005241Z`: skip=0, discarded=877, unknown=7, first `0xFF` at z=8 after `0x72`, peek `ff66…`. Skip high byte of `[n, 0xFF]` left at top-level; next byte is real `0x66`. Eat `0xFF` **only** when the following byte is a known inbound opcode. Do **not** eat when next is skip-stream (`ff98…`, `20260919T221503Z`). Do **not** eat inside `map_skip` (`20260919T214742Z`). `mixed_300.ron` frozen. Quote p99 only if both decoder fields are 0.

- [x] `inbound.rs`: skip leftover `0xFF` iff `is_known_inbound_opcode(next)`
- [x] Tests: `0x72`+`0xFF`+ping; `0xFF`+`0x98` stays unknown; map+`0xFF`+`0x83` parses effect
- [x] `cargo test -p tfs-loadgen` — 87 passed
- [x] 50-bot `20260920T010200Z`: skip=0 discarded=0 unknown=0 (no 214742Z regression)
- [x] 1000-bot `20260920T010424Z`: **no more `0xFF` unknowns**. skip=3 (`0xBF` z=8), discarded=150, unknown=1 (`0x2F` after `0x68`). Not both-zero — do not quote p99.

# 1000-bot rust-only ms_swarm (2026-09-20)

Decoder + FIFO accounting are done (50-bot both-zero; 1-bot walk/spell p99 ~46 ms). First scale cell: 1-rep rust-only `ms_swarm`, `--duration-s 120`, `--disable-saves`, `env -u CARGO_TARGET_DIR`. Quote walk/spell p99 only if `skip_failures==0` and `bytes_discarded==0`. `mixed_300.ron` frozen. No 3-rep A/B in this cell.

- [x] 1000-bot 120s rust-only `ms_swarm` — `results/20260920T005241Z`: 1000 in-world, 0 disconnects, **skip_failures=0**, **bytes_discarded=877**, unknown=7 (255×3, 15×2, 116×2). First unknown `0xFF` at z=8 after `0x72`, peek `ff66…`. **Not both-zero — do not quote walk/spell p99.**
- [x] lessons.md + todo cell result

# Spell FIFO cap-1 (2026-09-20)

Walk pairing is done (1-bot p99 46.4 ms). Spell p99 ~25 s at 50 bots (`20260919T234241Z`) is the same open-loop FIFO: `say_period_ms=2500` piles `outstanding_spell` until a later `0x83` matches the oldest same-tile head. Cap outstanding spells at 1 (drop unacked head on send). Suppress the following `0xB4` spell reject (mana/PZ for the superseded cast), not the next magic effect. `mixed_300.ron` frozen. Quote p99 only if both decoder fields are 0.

- [x] `latency.rs`: cap-1 `outstanding_spell`; `suppress_paired_spell_reject`
- [x] Tests: second send drops head; lone `0xB4` still rejects
- [x] `cargo test -p tfs-loadgen` — 85 passed
- [x] 50-bot 60s rust-only `ms_swarm` — `results/20260920T003509Z`: skip=0 discarded=0. Spell **p50 79 ms / p95 181 ms / p99 4.08 s** (was ~25 s). Walk p99 131 ms. Outstanding 20.

# 1-bot ms_swarm walk p99 split (2026-09-20)

50-bot both-zero walk p99 is **150 ms** (`20260919T234241Z`), above 1-bot isolation ~35–45 ms. Next: 1-bot 60s rust-only `ms_swarm` with the current loadgen. If 1-bot is ~35–45 ms, the 50-bot tail is N-bot load. If 1-bot is still ~150 ms, pairing is still wrong. Spell FIFO parked. `mixed_300.ron` frozen. Quote p99 only if both decoder fields are 0. No 1000-bot A/B.

- [x] 1-bot 60s rust-only `ms_swarm` — `results/20260920T000007Z`: skip=0 discarded=0. Walk **p50 46.0 ms / p95 46.3 ms / p99 46.4 ms**, outstanding 0. Isolation band holds. 50-bot p99 150 ms is N-bot load, not pairing.
- [x] lessons.md + todo cell result

# WalkRejected same-payload 0xB5 gate (2026-09-20)

`0xB4` Sorry retire without a peek gate stole unpaired `NotPossible` (`20260919T225517Z`). Bump path is `0xB4` then `0xB5` in **one** XTEA frame (`on_walk_step_rejected` + `encode_one_coalesced_frame`). Emit `WalkRejected` only when a walk-bump string is followed by `0xB5` in the same payload; unpaired Sorry stays histogram-only. Cap-1 stays. Do not clear `suppress_paired_snapback` on `on_send` when the queue was empty. Throw/room strings stay off the walk FIFO. `mixed_300.ron` frozen. Quote walk p99 only if both decoder fields are 0.

- [x] `inbound.rs`: `WalkRejected` iff walk-bump text + peek `0xB5`; unpaired Sorry emits nothing
- [x] `latency.rs`: `on_walk_text_reject` pops + suppress; `on_send` does not clear suppress
- [x] `session.rs`: wire `WalkRejected`
- [x] Tests: unpaired Sorry; Sorry+`0xB5`; text-reject then cancel = 1 rejection
- [x] `cargo test -p tfs-loadgen` — 82 passed
- [x] 50-bot 60s rust-only `ms_swarm` — `results/20260919T234241Z`: skip=0 discarded=0. Walk **p50 28.5 ms / p95 51.7 ms / p99 150 ms** (was 414 ms). Outstanding 53. Spell p99 still ~25 s.

# Walk p99 FIFO (2026-09-20)

Decoder both-zero (`20260919T224609Z`) walk **p99 18.0 s**. `0xB4` Sorry retire **regressed** (`20260919T225517Z` p99 27.5 s). **Not** send-interleave: `on_walk_step_rejected` enqueues `0xB4` then `0xB5` in one tick; `encode_one_coalesced_frame` puts them in **one** XTEA payload, so `feed()` sees both before the next `on_send`. **Not** inventory: `ms_swarm` is 100% Caster, `use_cmds=0`, cell texts were Sorry/PZ/mana only. Extra pops are unpaired `SendResult(NotPossible)` (`0xB4` without a popping `0xB5`) stealing walk FIFO heads. Cap-1 stays; do **not** restore `WalkRejected` until Sorry is walk-bump-only (require trailing `0xB5` in the same payload, or ignore Sorry). `mixed_300.ron` frozen. Quote walk p99 only if both decoder fields are 0.

- [x] Revert `0xB4` walk-text retire (`20260919T225517Z`)
- [x] `latency.rs`: cap outstanding walks at 1; suppress paired `0xB5`
- [x] Tests: second send drops head; lone `0xB5` still rejects
- [x] `cargo test -p tfs-loadgen` — 78 passed
- [x] Rerun 50-bot 60s rust-only `ms_swarm` — `results/20260919T225848Z`: skip=0 discarded=0. Walk **p50 36.8 ms / p95 44.4 ms / p99 414 ms** (was 18 s). Outstanding 87. Spell p99 still 25 s (uncapped FIFO).



# 0-floor SendFloors then SendRow (2026-09-20)

Unconditional `0xBF` bump at z=8 was wrong (`20260919T223936Z`: skip 10→17, discarded 1→8533). Body-start peek `6799013e0800ff…` is `0x67` SendRow after encoder wrote **no** floor bytes. If skip-floors fails and the body starts with `0x65`–`0x68` (or `0x83`/`0xBE`/`0xBF`/`0x6D`/`0x6C`/`0x64`), rewind and treat as 0 floors. Do **not** eat `0xFF`. Do **not** bump `0xBF` at z=8. `mixed_300.ron` frozen.

- [x] `inbound.rs`: `send_floors_omitted_body` rewind on `0xBE`/`0xBF`
- [x] Tests: `0xBF` then magic at z=8; `0xBE` then encoder `0x67` row
- [x] `cargo test -p tfs-loadgen inbound` — 38 passed
- [x] Rerun 50-bot 60s rust-only `ms_swarm` — `results/20260919T224609Z`: **skip_failures=0**, **bytes_discarded=0**, unknown=0, 50 in-world, 0 disconnects. Walk p50 26.4 ms / p95 1.03 s / **p99 18.0 s** (outstanding 279). Decoder gate is both-zero — p99 is quoted; it is still not a 1000-bot CPU story.


# 0xBF leftover at z=8 (2026-09-20)

Persist-flag cell `20260919T223328Z`: skip 33→10, discarded 6→1, unknown=0. Residual `0xBE`×2 + `0xBF`×8 at z=8, peek `eof` (skip consumed then wanted more floors). Unconditional `0xBF` bump at z=8 **regressed** (`20260919T223936Z`). Reverted; next is 0-floor SendFloors.

- [x] Diagnosed: peek `67…` is SendRow, not leftover 8→9 1-floor
- [x] Revert `0xBF` bump at z=8

# 0xBE skip at z=8 (2026-09-20)

Payload-local `self_move_this_payload` left `skip_failures=33` (`20260919T222725Z`) when `0x6D` dest and `0xBE` are in different frames. Persist `awaiting_move_up_one_floor` on self `0x6D` dest z>7 && dest<old (9→8). On `0xBE` at z=8 bump only if that flag is clear. `mixed_300.ron` frozen. Do not quote walk p99 unless both decoder fields are 0.

- [x] `inbound.rs`: session flag across `feed()` calls; skip_failed peek `eof` when cursor at end
- [x] Tests: 9→8 `0x6D` then `0xBE` in a second `feed` does not bump; 8→7 without `0x6D` still bumps
- [x] `cargo test -p tfs-loadgen inbound` — 35 passed
- [x] Rerun 50-bot 60s rust-only `ms_swarm` — `results/20260919T223328Z`: **unknown_opcodes=0**, `bytes_discarded=1`, **`skip_failures=10`** (`0xBE`×2 + `0xBF`×8, peek `eof`, z=8). Not both-zero — do not quote walk p99

# 0xBE skip at z=8 payload-local (2026-09-20)

Encoder `MoveUpCreature` / `SendFloors` uses **new** z: 6 floors at z=7, 1 floor at z=8. Inbound `0xBE` at z=8 skipped 1 floor when dest was not applied (8→7 leftover skip-stream, unknown 255/`0xBC`). Bump z 8→7 on `0xBE` only when this payload did **not** already apply a self `0x6D` (9→8 dest z=8 must still skip 1). `mixed_300.ron` frozen. Do not quote walk p99 unless both decoder fields are 0.

- [x] `inbound.rs`: `self_move_this_payload`; on `0xBE` if `player_z==8 && !self_move` bump z-1 then skip
- [x] Tests: `send_notify_go` 8→7 + magic; 9→8 no extra bump; `0xBE` without preceding self `0x6D` at z=8 skips 6 floors
- [x] `cargo test -p tfs-loadgen inbound`
- [x] Rerun 50-bot 60s rust-only `ms_swarm` — `results/20260919T222725Z`: **unknown_opcodes=0**, `bytes_discarded=6`, **`skip_failures=33`** at z=8 (`0x65`–`0x68`/`0xBE`/`0xBF`, peek empty). Not both-zero — do not quote walk p99

# Lone 0xFF after map skip (2026-09-20)

Unconditional leftover-`0xFF` eat was wrong (`20260919T214742Z` skip 0→27). Targeted last-cell 10-thing eat was also wrong (`20260919T221503Z`): unknown 152/188 after `0xBE` at z=8, peek `980100ff…` is unread skip-stream, skip_failures=5×`0xBF`. Do **not** eat `0xFF`. Instrument unknown (peek/prev/z). Next: `0xBE` at z=8 skips too few floors (inbound z not 7 before SendFloors up). `mixed_300.ron` frozen. Do not quote walk p99 unless both decoder fields are 0.

- [x] Drop blanket and last-cell leftover-`0xFF` eats in `skip_skip_stream` (10-thing `Some(0)` stays)
- [x] Tests: last-cell 10-thing leaves `0xFF`; empty map does not nibble following `0xFF`/`0x65`
- [x] `inbound.rs` + `loadgen.json`: `unknown_opcode_peek` / `unknown_opcode_prev` / `unknown_opcode_player_z`
- [x] `cargo test -p tfs-rust-net --lib map_skip`; `cargo test -p tfs-loadgen inbound`
- [x] Cell `20260919T221503Z` (with last-cell eat): prev=`0xBE` z=8, peek skip-stream, skip 5×`0xBF` — eat reverted
- [x] Rerun after revert `20260919T222025Z`: `skip_failures=121` (`0x65`–`0x68`/`0xBE`/`0xBF` at z=8, peek `07`), unknown 188+255, prev=`0xBE` z=8, peek `bc1300ff…` then `0x65`. Not both-zero — do not quote walk p99
- [ ] Next: `0xBE` skip at z=8 must use 6 floors when climbing to surface (inbound z still 8; encoder used new z==7)

# 50-bot rust ms_swarm decoder gate (2026-09-20)

Prove the 0xBF 10-thing skip on a live cell before quoting walk p99. `mixed_300.ron` frozen. Rebuild loadgen (uncommitted `map_skip.rs` / `inbound.rs`).

- [x] Seed 50 `--preset ms-swarm` cluster accounts
- [x] `run_comparison.py --mode steady --servers rust --bots 50 --reps 1 --duration-s 60 --scenario bench/scenarios/ms_swarm.ron --disable-saves` — `results/20260919T213326Z`
- [x] Gate: `skip_failures=0` (0xBF 10-thing skip holds). **`bytes_discarded=230`** — one unknown opcode `255` (`0xFF`), then rest of that frame. 50 in-world, 0 disconnects. Not both-zero — do not quote walk p99. Next: lone `0xFF` as top-level opcode, not a 1000-bot cell.

# Loadgen 0xBF skip (2026-09-17)

Residual `skip_failures` at `player_z=8` peek `11241200ff` (`0x2411` is client id luxurious couch, in OTB). Root cause is not floor count (3 at z=8 matches `SendFloors`). `skip_tile_description` returns `None` after 10 things (`MAX_OBJECTS_PER_POINT`); decompile `SendMapPoint` emits 10 objects with no `0xFF00` terminator (`sending.cc:271-282`). OTClient stops at 10 and treats the next bytes as the next tile (`skip=0`). Secondary: `0xBF` always `apply_notify_go_z_down`; after `0x6D` dest is already applied — bump only on `0x6C` surface→underground (`z==7`). `mixed_300.ron` frozen.

- [x] `map_skip.rs`: after 10 things, `return Some(0)` not `None`
- [x] `inbound.rs`: bump z on `0xBF` only when `player_z()==7`
- [x] Tests: 10-thing tile then `0x2411` without terminator; encoder 10-thing still skips; `0x6D` 8→9 does not double-bump
- [x] `cargo test -p tfs-rust-net --lib map_skip`; `cargo test -p tfs-loadgen`

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
