# Relog TakeOver — 2026-07-22

**Scope:** 772 `TPlayer::TakeOver` when a character body is still on the map (`connections.cc:224-253`, `crplayer.cc:721`).

## Plan

- [x] Detect existing `player_by_guid` body at login apply
- [x] Reject if dead, or `LoggingOut && LogoutPossible==Ok` (about to despawn)
- [x] Else TakeOver: clear old conn if any, cancel `logging_out` / `logout_allowed`, attach new conn, skip fresh spawn
- [x] Re-send login/map packets for the existing creature (`enqueue_initial_login_packets`)
- [x] Tests + lessons / todo

## Still deferred

- [ ] PvP skulls / RecordAttack / protectionLevel / world-type gates
- [ ] TakeOver `RejectTrade` (trade not ported)
- [ ] Refresh rights/guild/name from sticky PlayerData on TakeOver (minimal: OS/OTC flags only)
