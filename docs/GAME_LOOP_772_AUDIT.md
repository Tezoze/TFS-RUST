# 772 Beat-Driven Game Loop / ToDo / Scheduler — Parity Audit

**Scope:** the 772 beat loop and the subsystems it drives — `run_game_loop_772`,
`advance_beat_772`, the staggered subsystem counters, the global `ToDoQueue`, the per-creature
ToDo/`Execute` path, the delayed-event `Scheduler`, and the player-move scheduling path.

**Method:** each Rust path traced against the CipSoft 7.72 decompile
(`reference/cipsoft-772/tibia-game-master/src/`). Wire/packet concerns are out of scope here
(see `WALK_772_PARITY_AUDIT.md`, `772_MONSTER_AI_AUDIT.md`).

**Reference files:** `main.cc` (`AdvanceGame`, `LaunchGame`), `crmain.cc` (`ProcessCreatures`,
`ProcessSkills`, `MoveCreatures`), `cract.cc` (`Execute`, `ToDoStart`, `ToDoClear`, `ToDoGo`,
`CalculateDelay`, `NotifyGo`), `connections.cc/.hh` (`TConnection::Process`, `ResetTimer`),
`crskill.cc` (`TSkillFed::Event`), `receiving.cc` (`CGoDirection`, `CGoPath`).

---

## Summary

| # | Area | Severity | Status |
|---|------|----------|--------|
| 1 | Idle-timer action-exemption list wrong (`packet_counts_as_action_772`) | **High** | ✅ Fixed (F1) |
| 2 | `ProcessCreatures`: PK-mark clearing + PZ-gated item regen missing | Medium | ✅ Fixed (F2) |
| 3 | `TSkillFed` regen: hardcoded Rust table instead of `vocations.xml`; no PZ / no-food gate | Medium | ✅ Fixed (F3) |
| 4 | `Scheduler` (`addEvent`) delivers `LuaCallback` but it is never dispatched | Medium | Gap |
| 5 | Lag error logs every beat instead of once on transition | Low | Nit |
| 6 | Idle warn/kick config-driven vs fixed 900/960; no `NO_LOGOUT_BLOCK` right | Low | Nit |
| 7 | Player-move scheduling path (full trace) | — | **Correct** |
| 8 | Player non-walk actions execute reactively, not via ToDo `Execute` | Medium | Structural gap |

---

## What matches the decompile (verified correct)

- **`ToDoQueue`** (`todo_queue.rs`) — faithful verbatim port of CipSoft's 1-indexed binary heap
  (`containers.hh` `priority_queue`), including strict left-child tie bias in `deleteMin`.
  Equal-key drain order reproduces the oracle structurally without per-scenario tie maps.
- **`MoveCreatures` drain** (`drain_todo_queue`, `walk/mod.rs`) — `ServerMilliseconds += Delay`
  then drain-while-`top ≤ ServerMilliseconds`; matches `crmain.cc:1142`. `server_ms` frozen under
  the `Delay ≥ 1000` lag guard (`main.cc:445`).
- **Subsystem counters** (`subsystem_counters_772.rs`) — thresholds 1750/1500/1250/1000 with
  `-= 1000` on fire, and fire order creatures → cron → skills → other → move, matching
  `AdvanceGame`. Subsystems correctly observe the *old* `server_ms` (increment happens inside the
  move block, after the subsystem calls).
- **Beat coalescing** (`drain_burst_beats` + `advance_beat_772(beat_ms * beats)`) mirrors
  `NumBeats * Beat` from `LaunchGame` (`main.cc:498`).
- **Idle kick / connection timeout** round thresholds — ping at 30/60, idle kick at ≥960,
  command timeout ≥90 — match `connections.cc:21-51`.
- **`ToDoStart` `+1` clamp** — `Delay < 1 → 1` (`cract.cc:1016`) is honored in
  `todo_start_from_action` / `todo_start_go_delay`, guaranteeing a re-armed creature lands strictly
  in the future so `drain_todo_queue` cannot spin within a beat.

---

## Finding 1 — Idle-timer action-exemption list is wrong (High) — ✅ Fixed (F1)

**File:** `crates/tfs-rust-core/src/connections_772.rs` — `packet_counts_as_action_772`
**C++:** `TConnection::ResetTimer` (`connections.cc:53-63`), opcodes `connections.hh:14-75`

`ResetTimer` updates `TimeStampAction` (the idle-kick clock) for **every** in-game command
**except** exactly these five:

| C++ command | value | GamePacket |
|---|---|---|
| `CL_CMD_PING` | 30 | `Ping` |
| `CL_CMD_GO_STOP` | 105 | `StopAutoWalk` |
| `CL_CMD_CANCEL` | 190 | `CancelAttackAndFollow` |
| `CL_CMD_REFRESH_FIELD` | 201 | `UpdateTile` (772 "refresh field", 0xC9; shared decoder maps to `UpdateTile`) |
| `CL_CMD_REFRESH_CONTAINER` | 202 | `UpdateContainer` |

**Resolution (F1):** exemption list rewritten to match the C++ list exactly — `Ping`,
`StopAutoWalk`, `CancelAttackAndFollow`, `UpdateTile`, `UpdateContainer { .. }`. Removed
`Turn(_)` (not in C++ list — turning resets the action timer) and `PingBack` (OTClient-only
0x1D, not a 772 opcode). Test `packet_counts_as_action_matches_772_reset_timer` covers all
exempt + actionable cases.

> Note: the audit originally listed `BrowseField` for `CL_CMD_REFRESH_FIELD`, but `0xCB`
> (`BrowseField`) is in `V772_REMOVED` — 772 clients never send it. The 772 "refresh field"
> opcode is `0xC9`, which the shared decoder maps to `UpdateTile`. `BrowseField` is a
> 1098-only concept.

**Fix:** exempt `Ping`, `StopAutoWalk`, `CancelAttackAndFollow`, `BrowseField`, `UpdateContainer`
(and, if desired, keep `PingBack`); **remove `Turn`**.

---

## Finding 2 — `ProcessCreatures` responsibilities dropped (Medium) — ✅ Fixed (F2)

**File:** `crates/tfs-rust-core/src/creature_think.rs` — `process_creatures_772`
**C++:** `ProcessCreatures` (`crmain.cc:1075-1138`)

`process_creatures_772` previously kept only the death-safety net and delegated the rest. Two
pieces of the C++ function had no home:

1. **PK-mark clearing — stub implemented.** C++ clears playerkilling marks when
   `EarliestLogoutRound != 0 && EarliestLogoutRound <= RoundNr`, then zeroes `EarliestLogoutRound`
   (`crmain.cc:1104-1107`). The `EarliestLogoutRound` field is now on `Player`; the expiry check
   fires in `process_creatures_772` and zeroes the field. Full `ClearPlayerkillingMarks` (attacked-
   players list, aggressor flag, skull broadcast) is **deferred** until the PvP aggressor subsystem
   exists — the stub logs the expiry at `debug` level.
2. **PZ-gated item regen — implemented.** C++ does HP+1 / Mana+4 every `SKILL_FED` rounds, gated on
   `RegenInterval > 0 && (RoundNr % RegenInterval) == 0 && !IsDead && !IsProtectionZone`
   (`crmain.cc:1087-1096`). This is a **separate** regen from `TSkillFed::Event` (vocation regen,
   F3). Item regen keys off `food_level` (`SKILL_FED` `Act` = `RegenInterval`), while vocation
   regen keys off `food_remaining` (`SKILL_FED` `Cycle`) and vocation tick params. Both now run:
   item regen in `process_creatures_772`, vocation regen in `process_skills_772`.

**Food persistence:** `food_remaining` + `food_level` are persisted to DB (migration
`20260702000000_food_skill.sql`); loaded on login with offline food drain (`crplayer.cc:1395-1400`);
saved on logout. **Eat action:** `player:feed(amount)` Lua binding via `LuaMutation::PlayerFeed`
→ `lua_script_player_feed` (refills `food_remaining`, capped at 1200, sets `food_level = 12`).
`data/lib/core/player.lua` and `data/scripts/actions/other/food.lua` updated to use the 772
`SKILL_FED` model instead of TFS `CONDITION_REGENERATION`.

`CheckState()` (`crmain.cc:1099`) and `LoggingOut`/`LogoutPossible` removal (`crmain.cc:1114-1125`)
are approximated by `process_connections_772` + `pending_idle_kick_772`; acceptable.

---

## Finding 3 — `TSkillFed` regen: hardcoded table instead of `vocations.xml` (Medium) — ✅ Fixed (F3)

**File:** `crates/tfs-rust-core/src/process_skills.rs` — `process_player_fed_regen_772`;
`crates/tfs-rust-content/src/vocations.rs` — `Vocation` + `fed_regen_params`.
**Data source (correct):** `data/XML/vocations.xml` — `gainhpticks`, `gainhpamount`,
`gainmanaticks`, `gainmanaamount` per vocation.
**C++:** `TSkillFed::Event` (`crskill.cc:812-885`).

**Root problem:** `fed_regen_cadence` hardcodes a Rust `match vocation_id` table (and forces
`hp_amount = 1`, `mana_amount = 2` for everyone). All four regen values are already authored
per-vocation in `vocations.xml` and **must be drawn from there** — not from a table in code. The
decompile hardcodes these numbers, but the TFS/TVP data pack is the source of truth, and its values
match the decompile exactly. Loading from XML both fixes the wrong cells and removes a duplicated
constant table.

`vocations.xml` (both `data/XML` and `reference/tvp-772/.../data/XML`) vs the current hardcoded
table:

| Vocation (id) | `vocations.xml` hpticks / hpamount / manaticks / manaamount | Rust hardcoded | Status |
|---|---|---|---|
| None (0) | 6 / 1 / 6 / **1** | 12 / 1 / 6 / **2** | ✗ (ticks + mana amount) |
| Sorcerer (1) | 12 / 1 / **3** / 2 | 12 / 1 / **2** / 2 | ✗ (mana ticks) |
| Druid (2) | 12 / 1 / **3** / 2 | 12 / 1 / **2** / 2 | ✗ (mana ticks) |
| Paladin (3) | **8** / 1 / **4** / 2 | **6** / 1 / **3** / 2 | ✗ (hp + mana ticks) |
| Knight (4) | 6 / 1 / 6 / 2 | 6 / 1 / 6 / 2 | ✓ |
| Master Sorcerer (5) | 12 / 1 / 2 / 2 | 12 / 1 / 2 / 2 | ✓ |
| Elder Druid (6) | 12 / 1 / 2 / 2 | 12 / 1 / 2 / 2 | ✓ |
| Royal Paladin (7) | 6 / 1 / 3 / 2 | 6 / 1 / 3 / 2 | ✓ |
| Elite Knight (8) | 4 / 1 / 6 / 2 | 4 / 1 / 6 / 2 | ✓ |

(Note the current code also groups Sorcerer/Druid `1|2` with the promoted `5|6` at manaticks 2,
which is wrong for the base vocations — XML has 3 vs 2.)

Additional issues in the same function:

- **No protection-zone gate.** `TSkillFed::Event` returns early inside a PZ (`crskill.cc:820`);
  the Rust regens regardless of PZ.
- **Not gated on remaining food.** C++ keys on the live `SKILL_FED` timer value (regen stops when
  food runs out). The Rust uses a monotonic `skills_fed_timer` that increments every
  `ProcessSkills` unconditionally, so players regen forever with no food.

**Fix:** delete `fed_regen_cadence` and read `gainhpticks` / `gainhpamount` / `gainmanaticks` /
`gainmanaamount` from the loaded vocation definition; add the PZ gate; gate on food remaining
(`SKILL_FED > 0`). This aligns with the broader vocation stub cleanup — `vocation.rs`
(`per_level_gains`, `vocation_base_speed`, `recalculate_vitals`) is likewise hardcoded and should
source the same `vocations.xml` (`gainhp`, `gainmana`, `gaincap`, `basespeed`, `manamultiplier`,
etc.).

---

## Finding 4 — Scheduler is a no-op stub (Medium)

**File:** `crates/tfs-rust-core/src/scheduler.rs`, dispatch in `game_loop.rs`

`Scheduler::schedule_after` posts `GameCommand::LuaCallback { event_id }` after a Tokio sleep, but
`dispatch_command` only traces it:

```rust
GameCommand::LuaCallback { event_id } => {
    trace!(event_id, "lua callback — scheduler / Phase 8");
    ControlFlow::Continue(())
}
```

Scheduled (`addEvent`) callbacks never run. Not 772-specific (772 AI is driven off the
`ToDoQueue`, not `addEvent`), but the file is in scope and this is a latent gap for any Lua that
relies on `addEvent`/`stopEvent`.

---

## Finding 5 — Lag error logged every beat (Low) — RESOLVED

**File:** `crates/tfs-rust-core/src/game_world_tick.rs` — `advance_beat_772`

```rust
} else {
    self.lag_772 = true;
    if self.round_nr_772 > 10 { tracing::error!(...); }   // logs every lagging beat
}
```

C++ logs only on the transition into lag: `if(!Lag && RoundNr > 10) error(...)` then `Lag = true`
(`main.cc:447`). The Rust sets `lag_772` but doesn't use it to gate the log, so sustained lag spams
the error log. Behavior (movement skip) is correct; only the logging fidelity differs.

**Fix:** gate the `error!` on `!self.lag_772` before setting it.

**Resolved (F5):** the `error!` is now gated on `!self.lag_772 && self.round_nr_772 > 10` and
`lag_772` is set *after* the check, matching `main.cc:449-452` exactly. Sustained lag emits the
error once per `!Lag → Lag` transition.

---

## Finding 6 — Idle timing config-driven; `NO_LOGOUT_BLOCK` unhandled (Low)

**File:** `crates/tfs-rust-core/src/connections_772.rs` — `process_connections_772`

- C++ warns at 900 rounds and kicks at 960 (fixed 15/16 min, `connections.cc:29-35`). The Rust
  makes this configurable via `kickIdlePlayerAfterMinutes`. Deliberate feature, but a deviation
  from strict 772 outcome — flag if exact parity is required.
- C++ skips idle warn/kick for players with the `NO_LOGOUT_BLOCK` right (`connections.cc:29,35`).
  The Rust has no such exemption (GMs would be idle-kicked).

---

## Finding 7 — Player-move scheduling path (full trace) — CORRECT

This was the open item from the first pass: *is an incoming move executed reactively at packet
receipt, or enqueued as `TDGo` + `ToDoStart` and drained at beat time?*

**Answer: it is correctly deferred to the beat drain.** The full trace:

**C++ (`CGoDirection` / `CGoPath`, `receiving.cc:118-210`):**
1. `ToDoClear()` → `SendSnapback` if a `Go` was pending.
2. Build `TDGo` entries (one for a single move; N cumulative-absolute entries for a path).
3. Single `ToDoStart()` → `CalculateDelay(TDGo)` = `EarliestWalkTime - ServerMilliseconds` (or 0),
   clamped to ≥1, then `ToDoQueue.insert(ServerMilliseconds + Delay, ID)`.
4. Actual movement runs later in `MoveCreatures` → `Execute` (`cract.cc:783`), paced by
   `NotifyGo` setting `EarliestWalkTime` after each step.

**Rust (`player_move_request` / `player_auto_walk_path`, `walk/mod.rs`):**
1. `player_todo_clear_with_snapback` → `player_todo_clear` (clears `walk_queue`,
   `walk_destinations`, `todo.queue`, `locked`, `todo_stop`, and **`next_wakeup = None`**) and
   emits `0xB5` snapback if a `Go`/walk was pending. Matches step 1.
2. Push direction(s) into `walk_queue` (+ absolute `walk_destinations` for adjacency checks) and
   `enqueue_creature_go` (single `Go` action). Matches step 2 (the N-entry list is realized as one
   `Go` + an N-length `walk_queue` with per-step re-arm).
3. `todo_start_go_delay(cid, true)`: beat branch computes
   `calc_delay = earliest_walk_server_ms > server_ms ? earliest - server_ms : 1`, clamps to ≥1,
   and schedules via `todo_start_from_action` → `schedule_creature_wakeup(server_ms + delay)`.
   Because `player_todo_clear` set `next_wakeup = None`, the `walk_timer_idle` guard passes, so the
   schedule is **not** skipped. Matches step 3 including the `+1` clamp.
4. Execution happens at `drain_todo_queue` (inside `advance_beat_772`) →
   `process_creature_todo` → `run_monster_todo_execute` → `execute_creature_todo_action(Go)` →
   `on_walk`. `on_walk` sets `earliest_walk_server_ms = server_ms + NotifyGo_ms`
   (`walk/mod.rs:~1565`), and `finish_creature_todo_execute` re-arms the next queued step via
   `todo_start_go_delay(cid, false)` using that cooldown. Matches step 4 and the `NotifyGo` pacing
   (covered by `idle_stimulus_tests` audit #1/#5/#6 and `walk_timing.rs`).

**Latency check:** a move from standstill arms at `server_ms + 1`. The next beat does
`server_ms += beat` then drains, so `(old + 1) ≤ new` → the step lands on the very next beat —
identical one-beat latency to C++ (where `ReceiveData`/SIGUSR1 schedules and the next
`AdvanceGame`/SIGALARM drains).

**Reactive-vs-beat consistency:** commands are handled reactively in the `select!` loop, but
`player_move_request` only *schedules*; the visible movement packets are produced during the beat
drain and flushed at beat end (`FlushPolicy::BeatEndOnly`), matching `SendAll` at the end of
`AdvanceGame`. Because `server_ms` is only advanced inside the move block, reactive scheduling
always uses the last beat's `server_ms`, exactly like C++ never advancing `ServerMilliseconds`
outside `MoveCreatures`.

**Conclusion:** no reactive same-packet movement; the scheduling architecture, the `ToDoClear`
snapback preamble, the `+1` clamp, and the `NotifyGo`-paced re-arm all match. No bug found in the
move-scheduling path itself.

Minor notes in this path (not scheduling bugs):
- The `Turn` look-ahead in `handle_game_packet` peeks the next command via `cmd_rx.try_recv()` to
  coalesce Turn+Move facing — a facing-sync heuristic, not part of the 772 ToDo model.
- Player melee **strike** on `Attack` execute is a stub (`player_execute_attack`) pending a player
  weapon-combat system; the chase (`ToDoGo` toward target) is wired.

---

## Finding 8 — Player non-walk actions bypass the ToDo `Execute` engine (Medium, structural)

In 772, essentially all player actions funnel through the ToDo engine and execute during
`MoveCreatures` at beat time, gated by their earliest-action timers:
`CUseObject → ToDoUse → ToDoStart` (`EarliestMultiuseTime`, `cract.cc:766`),
`CMoveObject → ToDoMove`, `CTradeObject → ToDoTrade`, etc.

The Rust routes **walk** (and monster AI) through the ToDo/`Execute` path, but `player_use_item`,
`player_use_item_ex`, `player_look_at`, container ops, and `Say` execute **reactively** at packet
receipt (`handle_game_packet`). Mitigations in place:

- `game_packet_requires_timed_action` + `player_packet_action_ready` approximate the `nextAction`
  lockout gate, and
- walk-to-use is deferred correctly (`walk_action` → `try_run_player_walk_action_from_todo` at
  drain).

But the general consequence is that action *ordering* between a player action and other creatures'
ToDo actions within a beat differs from C++, and per-action cooldowns like `EarliestMultiuseTime`
are not enforced through a unified `CalculateDelay`. This is acceptable for the current phase but
is the largest remaining structural divergence in the loop; track it if strict intra-beat ordering
parity is required.

---

## Recommended fix order

1. **Finding 1** (idle exemption list) — small, self-contained, clear decompile values. ✅
2. **Finding 3** (regen from `vocations.xml` + PZ/food gates) — drop the hardcoded table and read
   `gainhp*`/`gainmana*` from the vocation definition; values already match the decompile. ✅
3. **Finding 5** (lag log gate) — one-line fidelity fix.
4. **Finding 2** (PK-mark clearing + PZ item regen) — ✅ Fixed (F2). PK-mark clearing is a
   stub (field zeroed, full `ClearPlayerkillingMarks` deferred). PZ-gated item regen implemented
   with `food_level` persistence + `player:feed()` Lua binding.
5. **Findings 4 / 8** — larger, phase-level (Lua scheduler dispatch; unifying player actions
   under the ToDo engine).
