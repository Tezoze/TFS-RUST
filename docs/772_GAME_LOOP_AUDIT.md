# 772 Game Loop Audit — `AdvanceGame` vs Rust (2026-09-06)

Scope: everything driven from the decompile's main loop — `LaunchGame` / `AdvanceGame` (`main.cc:318-501`) and the arms it calls each beat/second/minute. Rust side: `game_loop.rs`, `game_world_tick.rs`, `subsystem_counters.rs`, `connections.rs`, `creature_think.rs`, `process_skills.rs`, `walk/mod.rs` (ToDo drain), `spawn*.rs`, `raid_waves.rs`, `sector_refresh.rs`, `mail.rs`, `server_save.rs`.

Corpus paths are relative to `reference/cipsoft-772/tibia-game-master/src/`; Rust paths to `crates/tfs-rust-core/src/` unless stated. Every finding below was cross-checked in the corpus by the integrating agent; sub-agent claims that could not be verified were dropped.

Companion docs: `docs/772_PARITY_GAP_AUDIT.md` (feature gaps), `docs/772_SECTOR_REFRESH_DECAY_PLAN.md` (Step 11 — sector refresh detail, not repeated here), `docs/GAME_LOOP_OBS_BASELINES.md` (timing baselines).

---

## 0. Corpus shape (anchor)

```
LaunchGame:                      (main.cc:483-497)
  wait SIGUSR1 | SIGALRM
  if SigUsr1 > 0   → ReceiveData()          (no SendAll)
  if NumBeats > 0  → AdvanceGame(NumBeats*Beat)     Beat = 50 (runtime/.tibia:22)

AdvanceGame(Delay):              (main.cc:318-456)
  all four counters += Delay
  Creature >= 1750 → -=1000 → ProcessCreatures()      item regen, CheckState, PK clear, HP<=0→Death, LoggingOut→delete
  Cron     >= 1500 → -=1000 → ProcessCronSystem()     decay heap
  Skill    >= 1250 → -=1000 → ProcessSkills()         TimerList: Fed, poison, burning, energy, haste, light, …
  Other    >= 1000 → -=1000 → RoundNr++ ; ProcessConnections ; ProcessMonsterhomes ; ProcessMonsterRaids ;
                              ProcessCommunicationControl ; ReaderReplies(RefreshSector, SendMails) ; WriterReplies ;
                              ProcessCommand ; Ambiente(brightness) ; RoundNr%10 → NetLoadCheck ;
                              RoundNr >= NextMinute → RefreshCylinders, %5 CreatePlayerList, %15 SavePlayerDataOrder,
                                                      ==0 NetLoadSummary, ==55 WriteKillStatistics, reboot schedule
  if Delay > Beat  → Log("lag")
  if Delay < 1000  → MoveCreatures(Delay)  (ServerMilliseconds += Delay; drain ToDo heap)   else Lag=true (error once)
  SendAll()
```

---

## 1. Verified matches (no action)

| Area | Corpus | Rust |
|---|---|---|
| Counter thresholds `1750/1500/1250/1000`, `-= 1000`, one fire per call, residual catch-up | `main.cc:327-348` | `subsystem_counters.rs:36-62` |
| Arm order Creatures → Cron → Skills → Other → MoveCreatures → SendAll | `main.cc:332-455` | `game_world_tick.rs:91-143`, flush at `game_loop.rs:1598` |
| `RoundNr++` first in Other; spawn poll sees the incremented round | `main.cc:350-353` | `game_world_tick.rs:19-20`, test `spawn_poll_sees_round_nr_after_increment` |
| `GetRoundForNextMinute` = `RoundNr + (60 - sec) + 30`, initial 30 | `time.cc:106-110`, `main.cc:324` | `game_world_tick.rs:39-45`, `game_world.rs:488` |
| Lag guard: `Delay < 1000` → `server_ms += Delay` + ToDo drain; Other/Cron keep running; decay clock is `RoundNr` | `main.cc:445-453`, `crmain.cc:1143` | `game_world_tick.rs:129-134`, tests `lag_guard_*` |
| `Beat = 50`; `NumBeats*Beat` coalescing; first tick after one full beat | `runtime/.tibia:22`, `main.cc:161-164, 493-497` | `772.lua:9`, `drain_burst_beats`, `new_beat_timer` |
| `SendAll` only from `AdvanceGame`; receive-only wakes do not flush | `main.cc:455, 488-491` | single `flush_pending_outgoing` site in `obs_advance_beats` |
| One `Game` packet per connection per wake (`WaitingForACK`) | `receiving.cc:1796-1812` | `game_loop.rs:1718-1767` `served_game_conns` |
| Ambiente: brightness-only compare, `-1` sentinel, `GetAmbiente` table | `main.cc:361-373`, `time.cc:43-93` | `connections.rs:125-137`, `world_light.rs` |
| Ping at `LastCommand == 30 || 60`; dead-conn timeout `>= 90` → `Logout(0,false)`; idle 900 warn / 960 kick → `Logout(0,true)`; `ResetTimer` exemptions | `connections.cc:22-63` | `connections.rs:17-21, 84-105, 168-176` |
| Login takeover without `StartLogout` on the body | `connections.cc:244-249` | `game_loop.rs:600-613` |
| ToDo drain: pop while `key <= now`; skip if creature gone / `NextWakeup > now` / dead; zero-delay chain in one `Execute`; drained list → `IdleStimulus`; `Delay<1 → 1` | `crmain.cc:1144-1156`, `cract.cc:783-898, 1016-1018` | `walk/mod.rs:492-575, 655-660`, `idle_stimulus.rs:4317-4424` |
| Item regen gate `RoundNr % N == 0 && !IsDead && !PZ`, `+1 HP / +4 mana`; PK-mark clear; HP<=0 safety net; deferred player logout + `LogoutAllowed` sticky; `CheckState` dedupe | `crmain.cc:1087-1125, 417-431` | `creature_think.rs:49-143`, `game_world_lifecycle.rs:156-178, 354-390` |
| Poison `(Damage,3,3)`, `Range = Cycle*FP/1000` floor ±1, FP clamp; fire `(Damage/10,8,8)` dmg 10; energy `(Damage/20,10,10)` dmg 25; field re-extension `Cycle += 1` | `crmain.cc:588-611`, `crskill.cc:966-1102` | `process_skills.rs:14-23, 130-213, 265-371`, `formulas.rs:595-596` |
| Fed: `Cycle -= 1` then `TimerValue() % SecsPerHP/Mana`; vocation tables 6/6, 8/4, 12/3, 4/6, 6/3, 12/2; +1 HP / +2 mana | `crskill.cc:186-191, 828-882` | `process_skills.rs:530-545`, `vocations.xml` ids 1-8 |
| Soul timer count-then-event; beer drunk `(DrunkLevel+1,120,120)` cap 5 | `crskill.cc:793-805`, `moveuse.cc:1776-1782` | `process_skills.rs:439-466`, `condition.rs:219-252` |
| Cron heap + expiry transform (container empty, corpse delete, target 0 → remove) | `map.cc:209-268`, `operate.cc:2763-2794, 1755-1787` | `decay.rs:56-119`, `decay_apply.rs:261-304` |
| `RefreshCylinders` (not `RefreshMap`) on the minute arm; `SectorRefreshable` ±31 + `CanSeeFloor` | `main.cc:383`, `operate.cc:2796-2821, 2964-2988` | `game_world_tick.rs:60`, `sector_refresh.rs:132-203` |
| Monsterhome death-path delay: player scaling + `random(Max/2, Max)`; radius clamp 10 / `ActMonsters==0 → 1` / negative extended search; no shrink at startup | `crnonpl.cc:1296-1323, 1372-1398, 1427-1471` | `spawn_lifecycle.rs:1531-1553`, `spawn_placement.rs:26-39, 436` |
| Raid queue pop while `execution_round <= RoundNr`; `LifeEndRound` set at spawn, checked in `IdleStimulus`; announce before spawn | `crmain.cc:2022-2064`, `crnonpl.cc:2352` | `raid_waves.rs:80-95, 245-282`, `idle_stimulus.rs:1059` |
| Online mail: locker-full silent fail, 772 locker (beside chest), "New mail has arrived." only when locker UI is open, stamp on send, addressee ≥30 rejected | `moveuse.cc:767, 791-811` | `mail.rs` `place_mail_in_depot` |
| `WriteKillStatistics` at minute 55 + shutdown | `main.cc:393-395`, `crmain.cc:1177-1223` | `game_world_tick.rs:62-64`, `kill_statistics.rs` |

---

## 2. Findings

IDs keep the sub-agent slice letter (A scheduler, B per-creature arms, C world cron, D connections). Duplicates across slices are merged.

### 2.1 High — observable gameplay / persistence / disconnect differences

**H1 (B3) — Eating arms an invented permanent item regen (`food_level = 12`). DONE.**
- Corpus: `ProcessCreatures` item regen uses `RegenInterval = Skills[SKILL_FED]->Get()` = `Act(≥Min) + MDAct + DAct` (`crmain.cc:1087`, `crskill.cc:19-23`). `SetTimer(SKILL_FED, secs, 0, 0, -1)` on eating (`moveuse.cc:1846`) writes only `Cycle/Count/MaxCount` (`crskill.cc:195-204`) — **never `Act`**. `Act = 0` for humans (`human.mon:36`). `DAct` comes only from equipped `SkillNumber=14` items via `NotifyChangeInventory` (`cract.cc:1639-1660`): life ring `SkillModification=3` (`objects.srv:14082`), ring of healing `1` (`:14137`). So: no ring → no item regen; life ring → +1 HP / +4 mana every 3 rounds; ring of healing → every round.
- Rust: `game_world_inventory.rs:716-732` `lua_script_player_feed` sets `p.food_level = 12` when `<= 0`; `creature_think.rs:61-79` grants +1 HP / +4 mana every 12 rounds forever; persisted (`game_world_save.rs:191`). Rings use TFS `ConditionType::Regeneration` on the Skills arm instead (`equip_abilities.rs:321-341`, `process_skills.rs:59-110`) — see M3.
- Impact: every player who ever ate gets +5 HP / +20 mana per minute for life, on top of vocation regen. Tests in `creature_think_tests.rs:166-413` encode the wrong model.
- Fix: new `item_regen.rs` — `fed_regen_interval(player) = Σ SkillNumber-14 modifications of equipped items` (map `objects.srv` SkillNumber/SkillModification → `items.xml` `healthTicks`/`manaTicks` or add an OTB/XML attribute); `process_creatures` calls it. Remove `food_level` write from `feed`; drop the `Regeneration` condition path for life ring / ring of healing (M3). Keep `food_level` column only if needed for migration; otherwise remove. Rewrite `creature_think_tests` F2 cases around ring equip/unequip.

**H2 (B4, B13) — Fed (vocation) regen never sends stats or announces health. DONE.**
- Corpus: `Skills[SKILL_HITPOINTS]->Change(1)` → `TSkillHitpoints::Set` → `SendPlayerData` + `AnnounceChangedCreature(HEALTH_CHANGED)` (`crskill.cc:679-696`); mana → `SendPlayerData` (`:701-711`).
- Rust: `process_skills.rs:515-556` `process_player_fed_regen` mutates `health`/`mana` and returns — no `send_player_stats`, no spectator announce. (Item regen in `creature_think.rs:74-80` sends stats but no spectator announce.)
- Impact: client HP/mana bars go stale until any other stats packet; spectators never see regen.
- Fix: return `gained: bool` from fed regen; call `send_player_stats` + the existing health-changed broadcast (same helper `process_equipment_regeneration` uses at `:107-109`). Add the spectator announce to the item-regen path too.

**H3 (C1) — Monsterhome refill is serial in the corpus, parallel per slot in Rust. DONE.**
- Corpus: one `Timer` per home; on expiry **one** `CreateMonster`, then `if ActMonsters < MaxMonsters → StartMonsterhomeTimer` (`crnonpl.cc:1485-1487`). `NotifyMonsterhomeOfDeath` arms the timer only `if Timer == 0` (`:1510-1512`). A wiped home of N refills in ≈N regen cycles.
- Rust: one `respawn_at` per slot (`spawn.rs:34, 177-185`); `on_creature_removed_for_spawn` arms every slot independently (`spawn_lifecycle.rs:1496-1520`). `spawns.xml` zones with N identical entries refill all N within one cycle.
- Impact: respawn throughput after mass kills is N× the corpus.
- Fix: `spawn.rs` — per-zone (or per zone+race group, matching one `monsterhome.dat` line) `home_timer: Option<u32>`; arm on death only if unset; `poll_spawn_respawns` spawns ≤1 monster per zone per expiry, re-arms with `compute_respawn_delay_ms`. Slots keep occupancy only. Needs a decision on how a mixed-race `spawns.xml` zone maps to corpus monsterhomes (one home per race in the zone is the closest reading).

**H4 (C4) — Offline mail parked until the daily save; not applied on login; likely overwritten. DONE.**
- Corpus: offline `SendMail` → `DelayedMail` + `LoadCharacterOrder` (`moveuse.cc:825-844`); next Other arm `ProcessReaderThreadReplies` → `SendMails(Slot)` splices into `PlayerData->Depot` and marks dirty (`reader.cc:225-235`, `moveuse.cc:869-919`). Latency: a few rounds; persisted with the slot.
- Rust: `mail.rs:211-232` `deliver_mail_offline` → `houses.pending_depot_dumps`; sole consumer is `house/persist.rs:287-326` on `FlushStay`/shutdown/SIGINT. No login drain (grep `pending_depot_dumps` → `house/*`, `mail.rs` only). At `game_loop.rs:165-168` the dump is written **before** `flush_online_players_to_db`, so a recipient who logged in meanwhile has their in-memory depot saved over the dumped rows.
- Impact: recipient logging in before the save sees no mail; items lost on crash; probable silent loss on save ordering.
- Fix: `mail_delivery.rs` — on offline delivery spawn the DB depot append immediately (mirrors the reader thread), ack via `GameCommand`; on login apply any pending dump for that guid into the live depot before it is first opened; make the save-path ordering explicit (online players first, then dumps for offline guids only).

**H5 (D1) — TCP drop / lane shed logs out with `StopFight = true` (corpus `false`). DONE.**
- Corpus: `TConnection::Process` `!ConnectionIsOk || LastCommand >= 90 → Logout(0, false)` (`connections.cc:37-38`) → `StartLogout(false, false)` → `Combat.StopAttack(60)` — the body keeps attacking up to 60 rounds.
- Rust: `tfs-rust-net/src/server.rs:392` emits `PlayerDisconnect{display_effect:false}` on reader-loop end; `game_loop.rs:1420-1432` hard-codes `stop_fight = true` (comment: "CQuitGame / intentional"); `creature_start_logout_stop_fight(cid, true)` → `combat_stop_attack(cid, 0)`.
- Impact: a crashed/dropped client stops attacking instantly; kill credit / `EarliestLogoutRound` outcomes differ.
- Fix: add `stop_fight: bool` (or `reason: DisconnectReason`) to `GameCommand::PlayerDisconnect` (`tfs-rust-common/src/game_command.rs:52`); net-side drop/shed → `false`; `CL_CMD_LOGOUT` handler (`game_loop.rs:1057-1087`) → `true`.

**H6 (D2, D12) — `CONNECTION_DEAD` sessions escape `ProcessConnections` and ambiente. DONE.**
- Corpus: `InGame() = GAME || DEAD` (`connections.hh:181-184`); dead-but-connected clients still get ping 30/60, `Logout(0,false)` at `LastCommand >= 90`, idle kick, and `SendAmbiente` (`connections.cc:22`, `main.cc:368`).
- Rust: death inserts `dead_connections` and then `unregister_conn_mapping` (`game_world_lifecycle.rs:571-617`, comment claims idle timeout applies). `process_connections` (`connections.rs:69-73`) and `tick_ambient_light` (`:133`) iterate only `conn_to_creature`; `dead_connections` is never visited.
- Impact: a dead client that never presses OK holds the slot/TCP forever; no keepalive.
- Fix: `connections.rs` — second arm over `dead_connections` with a small `DeadConnState { last_command_round }` map (stamped from `handle_game_packet` for `Ping`/`Logout`); push `(conn,false)` at ≥90, ping at 30/60; include dead conns in ambiente recipients.

**H7 (A4, D3, B11) — `NetLoadCheck` is a different algorithm with a mass-kick hazard; `LagDetected()` missing. DONE.**
- Corpus: every `RoundNr % 10 == 0` (`main.cc:375`): `DeltaRecvPerPlayer` over a 360-entry `LoadHistory`; lag iff `RoundNr >= 3600 && PlayersOnline >= 50 && DeltaRecvPerPlayer < Avg/2` (`communication.cc:164-197`). Then `LagEnd = RoundNr + 30` (`LagDetected() = RoundNr <= LagEnd`, `:141-143`, consumed by `StartLogout`/`LogoutPossible` `crmain.cc:406, 419`), free-account admission delay (`:207-218`), and `EmergencyPing` per live conn: `if LastCommand < 80 → TimeStamp = RoundNr - 100; SendPing` (`connections.cc:66-79`).
- Rust: `game_world_tick.rs:70-85` gates on `self.lag` — the **beat-stall** flag (`Delay >= 1000`), unrelated to recv rate; no player floor, no warm-up, no `LagEnd`; rewinds `last_command_round -= 100` relatively for every online player (no `< 80` guard) then pings. `player_logout_possible` / `creature_begin_logout` (`game_world_lifecycle.rs:152-179, 255-263`) have no lag clause.
- Impact: because `run_other_subsystems` runs before the lag flag is updated, the beat after any ≥1 s stall that also fires Other on a `round % 10 == 0` rewinds every player to `LastCommand ≥ 100`; anyone whose ping reply is not processed before the next Other arm is logged out (`StopFight=false`). Rare combination, but it fires exactly when the server is already struggling. `LagDetected` lag-logout exemption is absent.
- Fix: `net_load.rs` — `NetLoad { total_send, total_recv, history: [i32; 360], ptr, total_load, lag_end }` fed by byte counters from `tfs-rust-net` (atomic or `GameCommand`); corpus gate; `lag_detected(round)`; `emergency_ping` with the `< 80` guard and absolute stamp; `summary()` for L4. Wire `lag_detected` into `player_logout_possible` / `creature_begin_logout`. Delete `net_load_check`'s `self.lag` dependency.

### 2.2 Medium — cadence / timing / ordering differences

**M1 (B1, B2) — Death and despawn are finalized immediately; corpus defers to the next `ProcessCreatures` pass. DONE.**
- Corpus: `Death()` only sets `IsDead + LoggingOut` (`crmain.cc:878-881`); corpse/pool/loot/`DelOnMap`/`Connection->Logout(30)` run in `~TCreature` when `ProcessCreatures` hits `LoggingOut && LogoutPossible()==0` (`:1113-1125`). Monster despawn via `StartLogout(true,true); State=SLEEPING` (`crnonpl.cc:2352-2415`) likewise. Body lingers 0-1000 ms at 0 HP; `Execute` and `Damage` skip it.
- Rust: `creature_death_defer.rs` `mark_dead` / `start_logout_despawn` / `kill_for_despawn` / `finalize_pending`; combat lethal sites call `mark_dead` only; `process_creatures` HP safety then destructor. Summons idle-despawn when the master is gone (no `remove_creature` cascade). Linger TCP/OK detaches the conn and leaves the body for `finalize_pending` (does not `remove_creature`). `Execute` skips `IsDead` only. AoL is consumed in `mark_dead`. Pack `playerdeath.lua` is a no-op (native sends the death text).
- Fix: landed Step 2.1 — combat `Death()` is flags + player death UI; corpse / monster XP wait for the ProcessCreatures destructor.

**M2 (B5) — Food is not consumed inside a protection zone. DONE.**
- Corpus: `TSkill::Process` always `Cycle -= 1` before `Event`; `TSkillFed::Event` skips only the regen in PZ (`crskill.cc:186-188, 816-818`).
- Rust: `process_skills.rs:521-531` returns on PZ before writing `food_remaining - 1`.
- Fix: decrement first, gate only the HP/mana grant.

**M3 (B6) — Life ring / ring of healing regen on the wrong arm, not PZ-gated, phase differs. DONE.**
- Corpus: item regen is `ProcessCreatures` (1750 counter), `RoundNr % N == 0`, `!IsDead && !PZ` (`crmain.cc:1087-1095`).
- Rust: `process_skills.rs:59-110` on the Skills arm (1250), ms accumulator, explicitly not PZ-gated (`:64`). Amounts match.
- Fix: folded into H1's `item_regen.rs`.

**M4 (B7) — Spell skill timers do not model `(Cycle, Count, MaxCount)`; durations and light shrink differ.**
- Corpus: event period `MaxCount+1` ticks; timer alive until `Cycle == 0`, removed the tick after. Haste `SetTimer(GO_STRENGTH, 3|2, 10, 10)` → 33 / 22 s (`magic.cc:2288, 3431, 3501`); paralyze `(1,10,10)` → 11 s (`:4250`); manashield `(1,200,200)` → icon off at 202 s (`:2308`); invisibility `(1,200,200)` → outfit back at 201 s (`:2349`); light `(Radius, Duration/Radius, Duration/Radius)` shrinks by 1 and re-announces every `Duration/Radius+1` s (`:2336`, `crskill.cc:913-921`) → utevo lux 504 s, gran lux 1008 s, vis lux 2007 s.
- Rust: `game_world_chat.rs:1624-1628` `rounds = ceil(ms/1000)`; `process_skills.rs:215-237` lives exactly `rounds` ticks; constant light radius. Pack values: haste 30000, strong haste 30000, paralyze 10000, magic shield 200000, invisibility 200000, light 370000 / 695000 / 1990000.
- Fix: `skill_timer.rs` — generic 772 timer `{cycle, count, max_count}` with `Event` hook (the fire/energy/poison mapper at `game_world_chat.rs:1667-1713` already does this shape); the Lua `addCondition` mapper sets the triple from `772.lua` literals; light `Event` decrements radius and re-announces. Strong haste 30 → 22 s and light 370 → 504 s are user-visible; document before landing.

**M5 (C2) — Stall / suppressed respawn re-arms with fixed `spawntime`, not `random(Max/2, Max)` with player scaling. DONE.**
- Corpus: `StartMonsterhomeTimer` is used on both death and failed/suppressed attempts (`crnonpl.cc:1296-1323, 1485-1487`).
- Rust: `spawn.rs:202-206` `stall_respawn` → `now + spawntime`; only the death path uses `compute_respawn_delay_ms`.
- Fix: pass `compute_respawn_delay_ms` at both `stall_respawn` call sites (`spawn_lifecycle.rs:232, 386`); merges with H3's zone timer.

**M6 (C3) — Radius shrink uses same-Z instead of `CanSeeFloor`; extra ghost/flag filters.**
- Corpus: `TFindCreatures(MaxRadius+9, MaxRadius+7, FIND_PLAYERS)`, skip only `!Player->CanSeeFloor(MH->z)`, `Radius = max(dx-9, dy-7)` (`crnonpl.cc:1432-1455`).
- Rust: `spawn_placement.rs:42-85` `pos.z != home.z → continue`; also skips ghost / `IGNORED_BY_MONSTERS` players.
- Fix: reuse `sector_refresh.rs:122-128` `can_see_floor` (hoist to a shared `visibility` helper); drop or gate the extra filters.

**M7 (C5) — Interval raids scheduled every boot; corpus gates by `SecondsToReboot`.**
- Corpus: `Duration <= SecondsToReboot && random(0, Interval-1) < SecondsToReboot`, start `RoundNr + random(0, SecondsToReboot - Duration)`; dated raids only inside the reboot window (`crmain.cc:1980-2010`).
- Rust: `raid_waves.rs:219-238` enqueues every interval raid unconditionally at `round + random(0, interval)`.
- Fix: compute `seconds_to_reboot` from the same `RebootTime` source as the save schedule; apply both gates; duration = max over waves of `delay + (lifetime or 3600)` (`crmain.cc:1921-1925`).

**M8 (C6) — Raid placement: no `SearchFreeField(1)`, no PZ skip, no 64 cap, no wave `Radius` leash.**
- Corpus: `crmain.cc:2046-2069`.
- Rust: `raid_waves.rs:264-306` random offset + `lua_script_create_monster(force=true)`.
- Fix: in `spawn_raid_wave` call `search_free_field(pos, 1)` (`spawn_placement.rs:256`), skip PZ, clamp count to 64, set `Monster::radius` from the wave.

**M9 (A5, A6) — Reboot/save schedule, `RefreshMap` on reboot, `LogoutAllPlayers`, SIGTERM.**
- Corpus: broadcasts at +5 (with `CloseGame`), +3, +1 with reboot/going-down wording; at `RebootTime`: `CloseGame; LogoutAllPlayers; SendAll; if Reboot RefreshMap; SaveMap; EndGame` (`main.cc:397-433`); SIGTERM → `RebootTime = now+6` + `CloseGame` (`:88-95`). `LogoutAllPlayers` deletes each player (`~TPlayer` → save, POFF, `DelInList`) (`crplayer.cc:1874-1892`).
- Rust: `server_save.rs:134-200` single warning at `serverSaveNotifyDuration`, "saving" wording only, `GameState::Closed` if `serverSaveClose`; shutdown = `flush_online_players_to_db` in place (`game_loop.rs:76-142`) — creatures not removed, `players_online` left stale until boot truncate; no `refresh_map()` on reboot; SIGINT only (`game_loop.rs:1800-1803`).
- Fix: `server_save.rs` — `[5,3,1]` schedule with both message variants; call `world.refresh_map()` before flush when shutting down; new `shutdown.rs` iterating `conn_to_creature` through the normal logout path then awaiting saves; SIGTERM arm in `run_server.rs` setting `next_save = now + 6 min` with close.

**M10 (A11) — House rent/eviction runs every minute; corpus runs `ProcessHouses` once at boot.**
- Corpus: `ProcessHouses` (`FinishAuctions/ProcessRent/StartAuctions/UpdateHouseOwners`) is called only from `InitHouses` (`houses.cc:1943-1960`) — i.e. once per daily reboot.
- Rust: `game_world_tick.rs:54-58` `process_houses_online` + `spawn_house_policy_scan` every minute.
- Decision needed: this is TFS `housePriceRentPeriod` pack policy layered onto the corpus cron. Either move to the save/reboot fire path (`server_save.rs` already calls `process_and_persist_houses`) or keep and document as a gated extra in `docs/DATA_PACK_LUA.md`.

**M11 (D4) — Idle warn/kick ignores `NO_LOGOUT_BLOCK`.**
- Corpus: both the 900 warning and 960 kick are skipped when `CheckRight(CharacterID, NO_LOGOUT_BLOCK)` (`connections.cc:29-36`).
- Rust: `connections.rs:89-102` no right check. `PLAYER_FLAG_NOT_GAIN_IN_FIGHT` is already documented as the 772 `NO_LOGOUT_BLOCK` analogue (`player/flags.rs:35-37`).
- Fix: skip warn+kick for that flag; keep the 90-round dead-connection branch unconditional.

**M12 (A1, D6) — Wake ordering inverted: Rust runs a due beat before the command; corpus runs `ReceiveData` first. Deliberate.**
- Corpus: `main.cc:483-497`. Rust: `game_loop.rs:1677-1697` `send_all_if_beat_pending` before `dispatch_command`; rationale at `:1608-1614` (Tokio `Interval` is Ready at the deadline; SIGALRM usually not yet pending during `ReceiveData`; fixed the 0xA3 red-square race).
- Impact: a packet arriving in the same wake as a due beat has its output flushed one beat (50 ms) later than corpus. Accepted deviation. Record in `tasks/lessons.md` if not already; optional refinement: only pre-advance when `drain_ready_beats` reports ≥1 full beat of lateness.

### 2.3 Low — log-only, ±1 tick, tooling, ordering with no outcome change

| ID | Finding | Corpus | Rust | Fix |
|---|---|---|---|---|
| L1 (A2, B14) | Lag error logged every lagging beat, not once per episode | `main.cc:449-452` `!Lag && RoundNr > 10` | `game_world_tick.rs:133-143` no `!lag` gate | `let entering = !self.lag;` gate |
| L2 (A3) | `Delay > Beat` "lag" log absent (replaced by `wall_ms >= 100` debug) | `main.cc:440-442` | `game_world_tick.rs:162-180` | `tracing::debug!(target:"lag")` when `delay_ms > beat_ms` |
| L3 (A7) | Idle kicks applied **after** MoveCreatures (corpus: inline in `ProcessConnections`, before); extra `npc_tick_conversation_timeouts` + `lua_gc_step` on Other | `main.cc:350-373`, `connections.cc:35-38` | `game_world_tick.rs:18-36`, `game_loop.rs:1587-1597` | apply `pending_idle_kick` before `drain_todo_queue` (split `advance_beat` pre/post) or accept |
| L4 (A10, D11) | `NetLoadSummary` hourly byte log missing | `communication.cc:155-162`, `main.cc:390` | not found | **DONE** `net_load.rs` (H7), minute==0 |
| L5 (A7, D10) | `ProcessCommunicationControl` (statement/listener 1800 s pruning) missing | `operate.cc:3193-3220` | only `alloc_statement_id` | `statements.rs` only if GM report context is in scope |
| L6 (A8) | `CreatePlayerList(true)` every 5 min missing (online record / `Log("load")`) | `main.cc:384-386`, `crplayer.cc:1942-1970` | `players_online` table only | `player_list.rs`: `players_record` upsert + info log |
| L7 (A9) | `SavePlayerDataOrder` every 15 min — corpus saves only **offline** dirty slots; Rust saves at logout | `writer.cc:410`, `crplayer.cc:2919-2942` | logout save | none (match by outcome) |
| L8 (A12) | Minute block reads `now()` twice; houses before `RefreshCylinders` | `main.cc:380-395` | `game_world_tick.rs:48-66` | single `Local::now()` at top |
| L9 (A13) | Pending logins dropped in `Closed` (corpus keeps them, rejects via `LoginAllowed`) | `connections.cc:42-44` | `connections.rs:62-67` | kick only when `Shutdown` |
| L10 (B8) | Drunk `Count` decremented before `<= 0` check (period `Duration` vs `Duration+1`) | `crskill.cc:176-193`, `magic.cc:285` | `condition.rs:261-271` | check-then-decrement |
| L11 (B9) | Fire/energy timer removed on the last Event tick (corpus one tick later; icon clears 1 s early) | `crskill.cc:177, 186-188` | `process_skills.rs:172-175` | drop `ticks_left <= 1` early removal |
| L12 (B10) | Skill order: DoTs before Fed (corpus TimerList insertion order, Fed first) | `crskill.cc:1192-1203` | `process_skills.rs:45-53` | **DONE** fed before DoT (1.2) |
| L13 (B12) | Vocation 0 mana regen 1 (corpus 2) | `crskill.cc:880-882` | `vocations.xml:3` `gainmanaamount="1"` | data fix or `772.lua` table |
| L14 (C7) | Sector refresh applied synchronously, no async re-check | `operate.cc:2824-2830, 2985` | `sector_refresh.rs:162-203` | already decided in Step 11 plan §0 |
| L15 (C8) | `RefreshCylinders` raster: corpus X fastest over the full grid; Rust Y fastest over snapshot XYs | `operate.cc:2968-2980` | `sector_refresh.rs:38-62` | fold into Step 11 G1/G2 |
| L16 (C9) | `CronInfo` returns ≥1 on a due entry; Rust 0 → full re-arm on `EXPIRESTOP` at expiry round | `map.cc:316-319` | `decay.rs:98-102`, `decay_apply.rs:224-229` | clamp remaining to ≥1 round |
| L17 (C10) | Cron drain batched (corpus loops `CronCheck` until empty; 0-delay reschedule same arm) | `operate.cc:2764-2794` | `decay.rs:104-119` | loop until empty, or document (0-delay cannot exist today) |
| L18 (D5) | Idle 900/960 are config-driven via `kickIdlePlayerAfterMinutes` (match at default) | `connections.cc:29, 35` | `config.rs:431-443` | document as gated knob |
| L19 (D7) | `Turn` handler peeks the next same-conn packet off `game_rx`, bypassing one-per-conn and ctrl-lane order | `receiving.cc:1796-1812` | `game_loop.rs:795-833` | route through `defer_extra_same_conn_game` or use deferred-turn flush |
| L20 (D8) | `CONNECTION_LOGOUT` delayed disconnect not modelled; per-conn flush outside `SendAll` on disconnect | `connections.cc:44-50, 274-296` | `game_loop.rs:667-723` | optional `logout_at_round` map |
| L21 (D9) | Dead-conn allow-list drops `CL_CMD_ERROR_FILE_ENTRY` (`DebugAssert`), adds `PingBack` | `receiving.cc:17-21` | `game_loop.rs:733-749` | **DONE** add `DebugAssert` (1.4) |
| L22 | Vendor name in comment `subsystem_counters.rs:31` ("CipSoft ~1000 ms period") | naming rule | — | reword to "772 ~1000 ms period" |

### 2.4 Out of scope (infra with no Rust counterpart expected)

`SetRoundNr` (SHM), `ProcessCommand` (SHM admin pipe), `CleanupDynamicStrings`, `LockGame`/`UnlockGame`, daemonisation, `.sec` `SaveMap` (Rust is OTBM + DB; only the `RefreshMap`-on-reboot piece is real — M9), `ProcessWriterThreadReplies` (Rust replies arrive as `GameCommand`s on command wakeups — sooner than once per Other tick, no worse outcome), `ProcessSectorReply` (Step 11), `ProcessMonsterhomes` missing-creature `break` (unreachable with `SlotMap`), BigRaid tie-breaker (TFS raid XML has no `type`), TFS-only Skills-arm work (message buffer, YellTicks/Muted — gated pack surface), `setWorldLight` override (gated).

---

## 3. Fix plan

Ground rules for every step:

- Focused module per concern; `GameWorld` gets a thin delegate only. No new `impl GameWorld` clusters in `game_world*.rs`.
- Corpus literals go in `data/formulas/772.lua` + `MechanicsProfile` (`formulas.rs`), not inline.
- DB work: `Handle::spawn` on the game thread → reply as a `GameCommand` (the VIP / mail-lookup pattern). Never `tokio::spawn` touching `GameWorld`.
- Each step: header `//! C++ reference:` on new files; `cargo check` → `clippy -D warnings` → targeted `cargo test`; update the finding's row in §2 to **DONE** and add a `tasks/lessons.md` entry if the fix contradicts a TFS pack assumption.
- Steps are independent unless a dependency is listed; PR granularity = one step.

### 3.1 Phase 1 — observable gameplay / persistence / disconnect bugs (H1-H7)

#### Step 1.1 — Ring regen on the Creatures arm; kill `food_level = 12` (H1, M3)

Corpus: `crmain.cc:1087-1095`, `crskill.cc:19-23, 195-204`, `cract.cc:1639-1660`, `objects.srv:14082, 14137`.

Mapping already present in the pack: `items.xml` `healthticks` (life ring 2205 = 3000, ring of healing 2216 = 1000) → `DAct = healthticks / 1000` (3, 1). `healthgain`/`managain` (1/4) equal the corpus constants and are **not** read — amounts stay `+1 HP / +4 mana` from `ProcessCreatures`.

Changes:

- **New `item_regen.rs`**
  - `pub fn fed_regen_interval(p: &Player, items: &ItemTypes) -> u32` — sum over equipped slots of `abilities.regeneration ? abilities.health_ticks / 1000 : 0` (`TSkill::Get` = `Act(0) + DAct`; `MDAct` unused for `SKILL_FED`). Returns 0 when nothing equipped.
  - `pub fn process_item_regen(world: &mut GameWorld, cid, round_nr) -> bool` — gate `interval > 0 && round_nr % interval == 0 && health > 0 && !PZ`; apply `+1/+4` clamped; return `true` when applied.
  - Cache: store `p.item_regen_interval: u32` recomputed in the equip/de-equip hook (`equip_abilities.rs`), so the per-second scan is a field read, not an inventory walk.
- **`creature_think.rs`** `process_creatures`: replace the `food_level` branch with `item_regen::process_item_regen`; on `true` call `send_player_stats` **and** the health-changed spectator announce (fixes B13 for this arm).
- **`equip_abilities.rs:321-345`**: delete the `ConditionType::Regeneration` add/remove for `abilities.regeneration`; instead recompute `p.item_regen_interval`. Keep `ConditionType::Regeneration` variant for Lua `addCondition` compatibility (pack surface) but no pack item arms it any more.
- **`process_skills.rs:59-110`** `process_equipment_regeneration`: delete (its only producer is gone). Remove `Regeneration` handling from the Skills arm entirely.
- **`game_world_inventory.rs:716-732`** `lua_script_player_feed`: delete the `food_level = 12` block and the doc comment; keep `food_remaining` accumulation.
- **`creature/player.rs:290`** `food_level`: remove field; **`game_world_save.rs:191`** + `login.rs` + DB column: stop reading/writing. Add a SQL migration that drops `players.food_level` (or leave the column unused if a migration is not wanted — decide in PR).
- **`772.lua`**: add `itemRegenHp = 1`, `itemRegenMana = 4` under a `creatures` block; `formulas.rs` fields `item_regen_hp`, `item_regen_mana`.

Tests (`creature_think_tests.rs` — replace the F2 group):
- `eating_does_not_arm_item_regen` — feed, advance 60 rounds, HP/mana unchanged beyond vocation regen.
- `life_ring_regens_every_3_rounds` — equip 2205, rounds 1..9 → +3 HP / +12 mana at rounds 3, 6, 9; stats packet enqueued each time.
- `ring_of_healing_regens_every_round`.
- `item_regen_blocked_in_pz`, `item_regen_stops_on_unequip`, `item_regen_skips_dead`.
- `process_skills` test asserting no `Regeneration` condition is created on equip.

Done when: no reference to `food_level` remains (`rg food_level crates/`), `process_equipment_regeneration` is gone, F2 tests pass.

#### Step 1.2 — Fed regen sends stats; food drains in PZ (H2, M2, L12 ordering)

Corpus: `crskill.cc:186-191, 816-818, 679-711`.

Changes in **`process_skills.rs`** only:
- `process_player_fed_regen(&mut self, cid) -> bool`: reorder to `if food_remaining == 0 return; let timer = food_remaining - 1; write food_remaining = timer;` **then** `if PZ return false;` then the `% hp_ticks / % mana_ticks` grants. Return `hp_gain > 0 || mana_gain > 0`.
- Caller (`:45-53`): on `true` → `send_player_stats(cid)` + health-changed announce (reuse the helper `process_player_soul_regen` uses at `:463-465`).
- Move the fed call **before** `process_creature_skills` (DoT pass) to match `TimerList` insertion order (Fed armed at load, `crskill.cc:108-111`).

Tests: `fed_regen_enqueues_stats_packet`, `fed_timer_drains_in_pz_without_regen`, `fed_regen_runs_before_dot_tick` (lethal fire tick at 1 HP with regen due → survives, matching corpus order).

#### Step 1.3 — `PlayerDisconnect { stop_fight }` (H5)

Corpus: `connections.cc:35-38`, `crmain.cc:404-415`, `crcombat.cc:513-522`.

Changes:
- **`tfs-rust-common/src/game_command.rs:52`**: add `stop_fight: bool` with doc `/// Logout(0, StopFight): true only for CL_CMD_LOGOUT / idle kick; socket drop = false (connections.cc:37)`.
- **`tfs-rust-net/src/server.rs:392`** (reader-loop end) and the `GameLaneFull` shed sites in `protocol_game.rs:126-131, 164-170`: `stop_fight: false`.
- **`tfs-rust-net/src/pending_login.rs:73`**: pattern update.
- **`game_loop.rs:1420-1432`**: pass `stop_fight` through to `handle_player_disconnect`. The `GamePacket::Logout` handler (`:1057-1087`) keeps `true`.
- Audit remaining `handle_player_disconnect(..., true, ...)` call sites: takeover (`:600-613`) stays `true` (corpus `Logout(0,true)` at `connections.cc:249`); output-shed (`drain_output_shed`) → `false`.

Tests (`game_loop.rs` test module): `socket_drop_keeps_attack_for_60_rounds` — set `attack_target`, dispatch `PlayerDisconnect{stop_fight:false}`, assert `combat_stop_attack` deadline = `round + 60` and target retained; `logout_packet_clears_attack_now`.

#### Step 1.4 — Dead connections stay in `ProcessConnections` and ambiente (H6, D12)

Corpus: `connections.hh:181-184`, `connections.cc:22-38`, `main.cc:364-373`.

Changes:
- **`connections.rs`**: add `pub struct DeadConnState { pub last_command_round: u32 }` and `GameWorld.dead_conn_state: HashMap<ConnId, DeadConnState>` (field declared next to `dead_connections`, `game_world.rs:146`; insert at death in `game_world_lifecycle.rs:571-573` with `round_nr`; remove in `handle_player_disconnect` `game_loop.rs:678`).
- `process_connections`: second loop over `dead_conn_state` — `last_command == 30 || 60` → `enqueue_periodic_ping(conn, None)` (ping packet needs no creature; add a conn-only variant of the ping builder in `player/ping.rs`), `>= 90` → `kick.push((conn, false))`. No idle-action arm (dead player has no actions).
- `tick_ambient_light`: recipient set = `conn_to_creature.keys() ∪ dead_conn_state.keys()`.
- **`game_loop.rs:733-749`** dead-conn packet gate: on `Ping`/`PingBack`/`Logout`/`BugReport`/`DebugAssert` update `dead_conn_state[conn].last_command_round = round_nr` (also closes L21).
- `game_state != Normal` drain (`connections.rs:63-67`): also kick `dead_conn_state` when `Shutdown`.

Tests (`connections.rs` tests): `dead_conn_pinged_at_30_and_60`, `dead_conn_kicked_at_90_without_stop_fight`, `dead_conn_ping_resets_stamp`, `ambiente_reaches_dead_conn`.

#### Step 1.5 — `net_load.rs`: corpus `NetLoadCheck` + `LagDetected` (H7, B11, L4)

Corpus: `communication.cc:141-229`, `connections.cc:66-79`, `crmain.cc:404-431`, `main.cc:375-377, 390-392`.

Changes:
- **New `net_load.rs`**
  ```rust
  pub struct NetLoad { total_send: u64, total_recv: u64, last_recv: u64,
                       history: [i32; 360], ptr: usize, total_load: i64, lag_end: u32 }
  impl NetLoad {
      pub fn add(&mut self, bytes: usize, send: bool);
      pub fn lag_detected(&self, round_nr: u32) -> bool;          // RoundNr <= LagEnd
      /// Returns true when lag was detected this call (caller runs EmergencyPing).
      pub fn check(&mut self, round_nr: u32, players_online: usize) -> bool;
      pub fn summary(&mut self) -> (u64, u64);                     // log + zero
  }
  pub const HISTORY_LEN: usize = 360; pub const MIN_PLAYERS: usize = 50; pub const LAG_WINDOW: u32 = 30;
  ```
  `check`: `DeltaRecv < 0 → return false`; ring update; gate `round_nr >= 10*HISTORY_LEN && players >= MIN_PLAYERS && delta_per_player < avg/2`; on lag `lag_end = round + 30`, `tracing::warn!(target:"game","Lag erkannt")`. Free-account admission delay (`:207-218`): implement only if a free-account queue exists; otherwise document as out-of-scope in the module header.
  - `pub fn emergency_ping(world: &mut GameWorld)` — for every in-game **and dead** conn: `if round - last_command_round < 80 { last_command_round = round - 100 }` (absolute), then ping.
- **Byte counters**: `tfs-rust-net` reader/writer already know sizes — add an `Arc<AtomicU64>` pair (`recv_bytes`, `send_bytes`) created in `run_server.rs`, passed to the net layer and to `GameWorld.net_load`; `NetLoad::check` reads them (`Ordering::Relaxed`, load-only from the game thread). No channel needed.
- **`game_world_tick.rs:70-85`**: replace `net_load_check` body with `if self.net_load.check(round, self.conn_to_creature.len()) { net_load::emergency_ping(self) }`. Delete the `self.lag` gate. Keep `self.lag` for the MoveCreatures skip only.
- **`tick_other_minute_jobs`**: `if minute == 0 { self.net_load.summary() }` → `tracing::info!(target:"netload", sent, recv)`.
- **`game_world_lifecycle.rs`**: `creature_begin_logout(cid, force, stop_fight)` → `logout_allowed = force || self.net_load.lag_detected(round)`; `player_logout_possible` → the combat clause becomes `earliest_logout_round > round && !lag_detected(round)`; add the `GameEnding()` clause (`game_state == Shutdown` → allowed).

Tests (`net_load.rs` unit + `game_world_tick.rs`): `no_lag_before_history_warm`, `no_lag_under_50_players`, `lag_when_recv_halves`, `lag_window_is_30_rounds`, `emergency_ping_rewinds_only_under_80`, `beat_stall_does_not_trigger_net_load` (replaces the current coupling), `lag_detected_grants_logout_in_combat`.

Sim: `sim_harness` scenario — 1 s stall, then 20 beats, 10 players → 0 kicks.

#### Step 1.6 — Serial monsterhome timer (H3, M5)

Corpus: `crnonpl.cc:1296-1323, 1409-1489, 1505-1512`.

Model: a corpus monsterhome = one race at one anchor with `MaxMonsters`. Map each `spawns.xml` zone to homes by grouping its monster entries **by race name** (`SpawnEntryKind::Monster { name }` — weighted-random entries stay one home keyed by the entry). NPC entries are excluded (no timer).

Changes in **`spawn.rs`**:
- `pub struct MonsterHome { zone_index, race_key: String, slot_indices: Vec<usize>, max_monsters: usize, act_monsters: usize, timer_at: Option<u32>, spawntime_ms: u64 }`; `SpawnManager.homes: Vec<MonsterHome>`, `slot_to_home: Vec<usize>`. Build in `from_zones`.
- Remove `SpawnSlot.respawn_at`; slots keep `current` for occupancy/leash only.
- `due_homes(now_round) -> Vec<usize>`: homes with `timer_at <= now`.
- `on_creature_removed(slot, now, delay_rounds)`: `home.act_monsters -= 1`; `if timer_at.is_none() { timer_at = Some(now + delay) }` (`:1510-1512`).
- `arm_home(home, now, delay_rounds)` replaces `stall_respawn`; callers pass `compute_respawn_delay_ms` (player-scaled `random(Max/2, Max)`) — **same** delay on death, failure, suppression (`:1296-1323`).
- `on_creature_spawned(home, slot, cid)`: `act_monsters += 1`; `timer_at = None`; caller re-arms when `act_monsters < max_monsters` (`:1485-1487`).

Changes in **`spawn_lifecycle.rs`** `poll_spawn_respawns` (`:204-242`): iterate `due_homes`; for each pick the first free slot of the home (position = slot position, radius per existing placement rules), attempt **one** placement; on success `on_creature_spawned` then `if act < max → arm_home`; on failure/suppression → `arm_home`. `on_creature_removed_for_spawn` (`:1496-1520`) → `spawn_manager.on_creature_removed(slot, round, compute_respawn_delay_ms(..))`.

Tests (`spawn.rs` + `spawn_lifecycle` tests): `wiped_three_slot_home_refills_one_per_cycle`, `second_death_does_not_rearm_running_timer`, `failed_placement_rearms_with_random_half_to_full`, `mixed_race_zone_splits_into_homes`, existing `spawn_poll_sees_round_nr_after_increment` stays green.

Risk: `spawns.xml` zones with many distinct races each become a 1-monster home (`max_monsters = 1`) — same as corpus `monsterhome.dat` singletons; confirm on `forgotten` spawn counts (log homes/slots at boot).

#### Step 1.7 — Offline mail delivered now and on login (H4)

Corpus: `moveuse.cc:825-844, 869-919`, `reader.cc:225-235`.

Changes:
- **New `mail_delivery.rs`**
  - `pub fn queue_offline(world, guid, town_id, item_id)`: detach item (as today), serialize the item tree via `append_save_item_tree` into `Vec<ItemRecord>` **now** (game thread), push `PendingMail { guid, town_id, records }` to `world.mail_outbox`, and `Handle::spawn` the DB append: `ItemStore::load_items(guid, Depot)` → sid/pid offset (reuse the offset logic from `house/persist.rs:287-326`, hoisted to `depot_append.rs` or a `pub(crate) fn` in `house/persist.rs`) → `save_items`. Reply `GameCommand::MailDeliveryFinished { guid, ok }` → remove from `mail_outbox`, release items.
  - `pub fn apply_on_login(world, guid, depot: &mut Vec<ItemRecord>)`: if `mail_outbox` still holds entries for `guid` (DB write not yet acked or failed), splice their records into the just-loaded depot rows before `login.rs:394-467` builds the depot; drop them from `mail_outbox`.
- **`mail.rs:211-232`** `deliver_mail_offline` → call `mail_delivery::queue_offline`; stop using `houses.pending_depot_dumps` for mail.
- **`house/persist.rs`** `flush_pending_depot_dumps`: house evictions keep using it, but skip any `guid` that is currently online (their live depot is authoritative) and log the skip; the online-first ordering at `game_loop.rs:165-168` is then harmless. Add a comment.
- **`login.rs`** hook: call `mail_delivery::apply_on_login` after `loaded.items.depot` is available and before the depot containers are built.

Tests: `offline_mail_serializes_and_spawns_db_append`, `login_before_ack_sees_mail_in_depot`, `login_after_ack_has_no_duplicate`, `house_dump_skips_online_guid`.

### 3.2 Phase 2 — cadence / timing (M1, M4-M9, M11)

#### Step 2.1 — Deferred death/despawn finalize (M1) DONE.

Corpus: `crmain.cc:878-881, 1108-1125`, `crnonpl.cc:2346, 2352-2415`, `cract.cc:785`.

- **New `creature_death_defer.rs`**: `mark_dead(world, cid, killer_ctx)` sets `base.is_dead = true`, `logging_out = true`, stores the death context (`DeathContext { last_hit, most_damage, .. }`) on the creature, announces HP 0 to spectators, sends "You are dead" to the player. `finalize_pending(world)` runs from `process_creatures` for every `logging_out && logout_possible == 0` creature → existing `apply_creature_death` / `remove_creature`.
- `idle_stimulus.rs:548-550`: `apply_creature_death` → `mark_dead`. Despawn sites (`:1141-1148`, LifeEnd, master lost, home range) → `start_logout(true, true)` + `state = Sleeping`, no immediate remove.
- Guards: `walk/mod.rs` ToDo execute (already skips dead), `combat_execute*` `Damage` returns 0 when `is_dead`, `IdleStimulus` returns early when `logging_out`.
- Remove the summons cascade in `remove_creature` (`game_world_lifecycle.rs:92-100`); summons despawn via their own idle check when master is gone (`crnonpl.cc:2363-2369`).
- Order inside `process_creatures`: regen → CheckState/PK → HP<=0 safety → finalize (matches `:1087-1125`).

Tests: `death_finalizes_on_next_process_creatures`, `dead_body_takes_no_damage`, `dead_body_runs_no_todo`, `summon_despawns_via_idle_after_master_removed`, existing death/loot tests updated to advance one Creatures tick.

Note: this shifts corpse/loot timing by up to 1 s and is the riskiest change in the plan — land after Phase 1 and after the sim-harness death scenarios are green.

#### Step 2.2 — `skill_timer.rs`: 772 `(Cycle, Count, MaxCount)` timers (M4, L10, L11, L12)

Corpus: `crskill.cc:176-193, 913-962`, `magic.cc:285, 2288, 2308, 2336, 2349, 3431, 3501, 3511, 3516, 4250`.

- **New `skill_timer.rs`**: `pub struct SkillTimer { cycle: i32, count: i32, max_count: i32 }`, `pub enum TimerStep { Idle, Event, Expired }`, `fn process(&mut self) -> TimerStep` implementing `Cycle==0 → Expired; Count<=0 → Count=MaxCount, Cycle-=1 → Event; else Count-=1 → Idle`. Light variant: `Event` decrements radius and returns the new radius for re-announce.
- `ActiveCondition` gains `skill_timer: Option<SkillTimer>` (replace `timer_rounds_left` for haste/paralyze/manashield/invisibility/light/drunk; fire/energy/poison already model the triple — migrate them to the same struct).
- `game_world_chat.rs:1624-1628` mapper: per condition type set the triple from `772.lua`:
  ```lua
  skillTimers = { haste = {cycle=3,count=10,max=10}, strongHaste = {cycle=2,count=10,max=10},
                  paralyze = {cycle=1,count=10,max=10}, manaShield = {cycle=1,count=200,max=200},
                  invisible = {cycle=1,count=200,max=200},
                  light = {[6]=500, [8]=1000, [9]=2000} }  -- radius → Duration; count = Duration/radius
  ```
  Lua `duration` from the pack scripts is ignored for these (pack surface stays, corpus numbers win — document in `docs/DATA_PACK_LUA.md`).
- `process_skills.rs`: iterate conditions in insertion order (Fed first — a `Vec`, not a map), remove on `Expired` (one tick after the last `Event`, closes L11), `CheckState` on removal. Drunk (`condition.rs:261-271`) → `SkillTimer` (closes L10).

Tests: `haste_lasts_33_ticks`, `strong_haste_lasts_22_ticks`, `paralyze_rune_11_ticks`, `manashield_icon_off_at_202`, `invisibility_outfit_back_at_201`, `utevo_lux_shrinks_radius_every_84_ticks_total_504`, `fire_icon_clears_one_tick_after_last_hit`, `drunk_event_every_duration_plus_one`.

User-visible: strong haste 30 → 22 s, light 370 → 504 s. Call out in the PR description.

#### Step 2.3 — Radius shrink via `CanSeeFloor` (M6)

- Hoist `can_see_floor(viewer_z, floor_z)` from `sector_refresh.rs:122-128` into `visibility.rs` (`cr.hh:576-582`); both callers import it.
- `spawn_placement.rs:42-85`: replace `pos.z != home.z` with `!can_see_floor(p.z, home.z)`; remove the ghost / `IGNORED_BY_MONSTERS` skips (corpus `TFindCreatures(FIND_PLAYERS)` has no such filter) — or keep behind `profile.tfs_extras` if GM ghost-mode camping is wanted.

Tests: `player_one_floor_up_suppresses_spawn`, `player_underground_does_not_suppress_surface_home`.

#### Step 2.4 — Raids: reboot-window gating and corpus placement (M7, M8)

- `raid_waves.rs:219-238`: `seconds_to_reboot = server_save.seconds_until_fire(now)` (expose from `ServerSaveController`); interval raids: `duration <= str && parity_random(0, interval-1) < str` → start `round + parity_random(0, str - duration)`; dated raids only when `now <= date <= now + str`. `duration = max(delay + lifetime.unwrap_or(3600))` over waves.
- `spawn_raid_wave` (`:264-306`): `count = min(count, 64)`; per monster `search_free_field(pos, 1)` → skip on `None` or PZ; set `Monster::radius` from the wave when present.

Tests: `interval_raid_skipped_when_duration_exceeds_reboot_window`, `raid_start_within_window`, `raid_monster_not_placed_in_pz`, `raid_count_capped_64`.

#### Step 2.5 — Reboot schedule, `LogoutAllPlayers`, `RefreshMap`, SIGTERM (M9, A6)

- `server_save.rs`: `ServerSavePoll::EnterWarning` → `Broadcast { minutes: 5|3|1, reboot: bool }` emitted at `fire - 5m` (also sets `Closed`), `- 3m`, `- 1m`; strings from `main.cc:399-422` (both variants). `reboot` = config `serverSaveShutdown == false`.
- **New `shutdown.rs`**: `logout_all_players(world)` — for each `conn_to_creature` run the normal logout path (`creature_begin_logout(cid, true, true)` → finalize → `players_online` delete → save), then `flush_online_players_to_db` for stragglers; `refresh_map_if_reboot(world)` → `sector_refresh::refresh_map` when shutting down for restart (`main.cc:427-429`).
- `game_loop.rs` `FlushShutdown` arm and SIGINT arm → `shutdown::run(world).await`.
- `run_server.rs`: add `tokio::signal::unix::signal(SIGTERM)` → `GameCommand::ScheduleClose { minutes: 6 }` → `ServerSaveController::schedule_in(6 min, close=true)`.

Tests: `warnings_at_5_3_1_with_reboot_wording`, `shutdown_removes_players_online_rows`, `sigterm_schedules_close_in_6_minutes`.

#### Step 2.6 — `NO_LOGOUT_BLOCK` idle exemption (M11)

- `connections.rs:89-102`: `let exempt = player_has_flag(p, PLAYER_FLAG_NOT_GAIN_IN_FIGHT)`; skip warning and idle kick when `exempt`; the `>= 90` branch stays unconditional.

Test: `gm_not_idle_kicked`.

#### Step 2.7 — Decision: house rent cadence (M10)

Options: (a) corpus — run `process_houses_online` + policy scan once at boot and on the save/reboot fire (`server_save.rs` already calls `process_and_persist_houses`); (b) keep per-minute as a documented TFS `housePriceRentPeriod` gate in `docs/DATA_PACK_LUA.md`. Default recommendation: (a), because rent/eviction timing is a corpus outcome, not pack policy. Awaiting user call; no code until decided.

### 3.3 Phase 3 — polish (Low table)

One PR, mechanical:

- L1 `game_world_tick.rs`: `let entering = !self.lag; self.lag = true; if entering && round_nr > 10 { error!(..) }`.
- L2 `game_world_tick.rs`: `if delay_ms > beat_ms { tracing::debug!(target: "lag", delay_ms) }`.
- L8 `tick_other_minute_jobs`: single `Local::now()`; order `refresh_cylinders` → houses → minute 55.
- L9 `connections.rs:62-67`: kick pending logins only when `game_state == Shutdown`.
- L13 `data/XML/vocations.xml` id 0 `gainmanaamount="2"` (or 772.lua table).
- L16 `decay_apply.rs:224-229`: remaining `max(1)` round under `RoundNumber` clock.
- L21 covered by 1.4; L22 `subsystem_counters.rs:31` comment → "772 ~1000 ms period".

Second PR (loop plumbing, review with M12):
- L3: split `advance_beat` into `advance_beat_pre_move` / `drain_move` so idle kicks apply before `drain_todo_queue`; drop `npc_tick_conversation_timeouts` / `lua_gc_step` from Other if they can live on their own cadence (or document as extras).
- L19: `Turn` peek → `defer_extra_same_conn_game`, rely on the deferred-turn flush.
- L20: optional `logout_at_round` map; close on the next Other arm.

Document-or-defer: L5 (`statements.rs` only with GM reports), L6 (`player_list.rs` if a website record is wanted), L14/L15 (Step 11 plan), L17 (0-delay cron cannot exist), L18 (idle knob gate in `docs/DATA_PACK_LUA.md`).

### 3.4 Sequencing and dependencies

```
1.1 item_regen ──┐
1.2 fed regen ───┼── independent, ship first (pure bugs, small diffs)
1.3 stop_fight ──┤
1.6 spawn timer ─┘
1.4 dead conns ──► 1.5 net_load (emergency_ping iterates dead conns)
1.7 mail ────────── independent (touches DB append helper only)
2.1 death defer ─── after all Phase 1; before 2.2 (skill removal on death interacts)
2.2 skill_timer ─── after 1.2 (ordered timer list)
2.3 / 2.4 / 2.6 ─── independent
2.5 shutdown ────── after 2.4 (shares seconds_to_reboot) 
2.7 decision ────── blocks nothing
Phase 3 ─────────── any time; L3/L19/L20 after 2.1
```

Estimated size: Phase 1 ≈ 7 PRs, 150-400 lines each; Phase 2 ≈ 6 PRs, 2.1 and 2.2 the largest; Phase 3 ≈ 2 PRs.

### 3.5 Verification

Per PR:

```bash
/home/jessec/.local/bin/rtk cargo check -p tfs-rust-core -p tfs-rust-net -p tfs-rust-common
/home/jessec/.local/bin/rtk cargo clippy -p tfs-rust-core -p tfs-rust-net -p tfs-rust-common -- -D warnings
/home/jessec/.local/bin/rtk cargo test -p tfs-rust-core <module under change>
```

Phase gates:

```bash
/home/jessec/.local/bin/rtk cargo test -p tfs-rust-core game_world_tick subsystem_counters connections creature_think process_skills spawn raid mail net_load skill_timer
/home/jessec/.local/bin/rtk cargo test -p tfs-rust-core --test '*'
/home/jessec/.local/bin/rtk cargo test -p tfs-rust-core sim_harness
```

Sim-harness scenarios to add (`sim_harness.rs`), one per phase-1 step: (a) 1 s stall → 20 beats → 0 kicks at 10 players; (b) ring equip/unequip regen trace; (c) 3-slot home wipe → respawn rounds ≈ N cycles; (d) offline mail → login depot before any save tick; (e) socket drop → target held 60 rounds; (f) dead client silent → kicked round 90.

Phase 1 coverage (unit tests, not extra `sim_harness.rs` scenarios): (a) `net_load.rs` beat-stall does not trigger net load; (b) `creature_think_tests.rs` life ring / unequip; (c) `spawn.rs` `wiped_three_slot_home_refills_one_per_cycle`; (d) `mail.rs` `login_before_ack_sees_mail`; (e) `game_loop_disconnect_tests.rs` `socket_drop_keeps_attack_for_60_rounds`; (f) `connections.rs` kick at round 90.

`docs/GAME_LOOP_OBS_BASELINES.md`: re-baseline beat wall-time after 1.6 and 2.1 (fewer immediate removals, serial spawns).

---

## 4. Lessons recorded (`tasks/lessons.md` 439-446)

1. `SKILL_FED` `Act` is never written by eating — item regen cadence comes from `SkillNumber=14` `DAct` (life ring 3, ring of healing 1). `player:feed` must not set a regen interval; TFS `Regeneration` conditions for rings are a second, wrong model. Soft boots 2640 are cadence-only (interval 6); grants stay profile +1/+4 (lesson 444).
2. `NetLoadCheck` is a **recv-bandwidth** heuristic with a 50-player floor and 1-hour warm-up, not a beat-stall detector; `EmergencyPing` rewinds only when `LastCommand < 80` and sets an absolute stamp.
3. Corpus `Logout(0, false)` for socket drops — a dropped client keeps fighting 60 rounds; only `CL_CMD_LOGOUT` and idle kick stop the fight.
4. A monsterhome has **one** timer; refills are serial. Per-slot timers are a TFS-shape leak.
5. Offline mail is an immediate depot append + login splice (`mail_delivery.rs`); `pending_depot_dumps` is house eviction / welcome letters only (lesson 445).
6. Fed `Cycle` always decrements, including in PZ; only the HP/mana grant is skipped (lesson 446).
