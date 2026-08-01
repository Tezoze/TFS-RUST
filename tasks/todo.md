# Change outfit (RequestOutfit / SetOutfit) — 2026-08-01

**Goal:** Client Ctrl+O / outfit dialog works end-to-end on 772 (primary) and 1098.

## Plan

- [x] Fix `SET_OUTFIT` parse: 772 = lookType+colors only; 1098 = +addons+mount
- [x] Encode `0xC8` outfit window: 772 classic looktype range, 772 OTClient named list, 1098 list+addons (+empty mounts)
- [x] Wire `OutfitDatabase` into `GameWorld`; `Player.outfits` from reserved storage
- [x] `player_request_outfit` / `player_change_outfit` / `can_wear` (access, premium, unlocked, owned)
- [x] Broadcast `0x8E` via codec; skip if `CONDITION_OUTFIT`; gate on `allowChangeOutfit`
- [x] Dispatch `GamePacket::RequestOutfit` / `SetOutfit` in `game_loop`
- [x] Unit tests + lesson 291

## Deferred

- Mount apply / mount list on 1098 window
- Lua `Creature:onChangeOutfit` veto
