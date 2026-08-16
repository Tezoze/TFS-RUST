# Movements Lua fire path — M2 AddItem/RemoveItem — 2026-08-16

**Status:** M2 done. Full audit: [movements-plan.md](movements-plan.md).

## M2 shipped
`executeAddRemItem(moveitem, tileitem, pos)` TFS signature. `:tileItem(true)` remaps to ITEMTILE. Sibling tile iteration (skip moved item). Actor not required. Fire after tile add/remove under mutation + ScriptContext (same nest as StepIn). Lua `false` does not undo.

Then **M3** (`setTown`, `getMaster`), **M4** (772 trap/field script pass).
