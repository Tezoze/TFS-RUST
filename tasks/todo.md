# E3: `getDestroyId` / `getFluidSource` + `destroyItem` 1/3 `transform` — 2026-08-15

From [other-actions-plan.md](other-actions-plan.md). TFS `luaItemTypeGetDestroyId` / `GetFluidSource`; 772 `UseWeapon` (`moveuse.cc`) always poff, `random(1,3)==1` then Empty + `Change`.

- [x] `otb.rs` — typed `destroy_to` / `fluid_source` (defaults 0 / `FLUID_NONE`)
- [x] `items.rs` — parse `destroyto` + `fluidsource` names → 772 sequential `FLUID_*`
- [x] `script_context.rs` + `game_world_script.rs` + `userdata/item_type.rs`
- [x] `functions.lua` `destroyItem` — 1/3 + `transform` (not TFS 1/7 create+remove)
- [x] Tests: known `items.xml` rows; Lua bindings; `destroyItem` source; `emit-lua-defs --check`
- [x] `tasks/lessons.md`
