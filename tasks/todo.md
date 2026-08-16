# GM town teleport talkactions — 2026-08-16

**Status:** done.

## Goal
Make `/town`, `/t` `/home`, and `omani` work: TFS `Town` / `Player:getTown` domain, OTBM temple positions, existing `teleportTo`.

## Work
- `Town(id|name)` + `getId` / `getName` / `getTemplePosition` (`luaTownCreate`).
- `Player:getTown()` (`luaPlayerGetTown`).
- `TalkAction(words...)` joins with `;` so `/t` and `/home` both register.
- Load `data/scripts/talkactions/**` (not only `god/`).
