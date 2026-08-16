# Movements Lua fire path — M1 aid lookup — 2026-08-16

**Status:** M1 done. Full audit: [movements-plan.md](movements-plan.md).

## M1 shipped
StepIn/Out fire uses TFS `getEvent(Item*)` order (uid map skipped → **aid → itemid**). `tile_move_event_items` snapshots `action_id`. `get_by_aid` is on the live path. First registered event per key wins. No-op `MoveEvent:tileItem` so dual StepIn+AddItem files load.

Then **M2** (`tileItem` ITEMTILE remap + AddItem args + mutation scope — also 11 AddItem-only scripts), **M3** (`setTown`, `getMaster`), **M4** (772 trap/field script pass).

## 772 (do not forget)
- Fields: keep native `magic_field.rs` (`moveuse.dat` Trap Damage). No Lua double-hit.
- Traps: holes transform first; 60 physical on blades; no PZ skip on Collision; bear trap `!IsPeaceful`.
- Doors: leave `closing_doors.lua` (SeparationEvent ClearField+Change).
- Map aids: leave as Lua — they now fire on walk.
