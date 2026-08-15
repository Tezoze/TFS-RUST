# E2: zero-thing target + `isHotkey` — 2026-08-15

From [other-actions-plan.md](other-actions-plan.md). TFS `Action::executeUse` `callFunction(6)`; `pushThing(nullptr)` is a table, not nil.

- [x] `runtime.rs` — no-target → `{uid,itemid,actionid,type=0}`; 6th arg `isHotkey`
- [x] `lua_scope.rs` / dispatcher — thread `is_hotkey`; TFS `(0xFFFF,0,0)` from client pos
- [x] `pumpkinhead.lua` / `used_lamp.lua` — `type(target) ~= "userdata"` guard
- [x] Test: `target.uid==0 and target.itemid==0`; `isHotkey` boolean
- [x] `emit-lua-defs --check`; lessons
