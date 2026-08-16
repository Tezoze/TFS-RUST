# R3–R5 remaining actions APIs — 2026-08-16

**Status:** done.

## Goal
Unblock `onUseQuest` (`addItemEx` + infinite-capacity flag) and Mintwallin `Game.createTile`.

## Work
- **R3** `Player`/`Container`/`Tile:addItemEx` → `RETURNVALUE_*`; detached item only.
- **R4** Inject `getPlayerFlagValue`; `Player()` accepts userdata/id/name.
- **R5** `Game.createTile` get-or-create.
- `Item:getParent()` tile arm pushes `Tile` userdata.
