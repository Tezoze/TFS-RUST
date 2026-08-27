# 772 beat loop audit — implement canvas fixes

**Status:** complete.
**Source:** [772 beat loop audit](/home/jessec/.cursor/projects/mnt-storage2-TFS-RUST/canvases/beat-loop-772-audit.canvas.tsx)

## P1

- [x] **B1+G5** Spawn due clock = `round_nr`; `RoundNr++` before homes. Lag skip still ticks homes.
- [x] **B2** One `GameCommand::Game` per connection per wakeup; 64 is a global cap. Deferred extras stay in `pending`.
- [x] **G1** RoundNr `AttackWaveQueue` drained from Other after homes; TFS XML + `Game.startRaid`.

## P2

- [x] **B3** `tick_player_pings` removed from Other; 30/60 ProcessConnections remains.
- [x] **B4** `send_player_icons` sweep at end of `process_creatures`.
- [x] **B5** Idle-monster skill skip kept (empty conditions ≡ no timer-skills). Raid lifetime is Other, not ProcessSkills.
- [x] **G2** Minute jobs: `NextMinute` + `process_houses_online` (no reboot/CloseGame sequence).
- [x] **G3** `NetLoadCheck` every 10 rounds: EmergencyPing + TimeStamp rewind 100 when `lag`.
- [x] **G4** `CONNECTION_LOGIN` pending conns disconnect when `game_state != Normal`.

## Leave

- **N1** Lua GC only on Other when `Delay < 1000`.
- **N2** `addEvent` stays TFS wallclock.
- Coalesced `AdvanceGame(N * Beat)` unchanged.
