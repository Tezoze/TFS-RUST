# E4: `Player:addHealth(n)` — 2026-08-15

From [other-actions-plan.md](other-actions-plan.md). TFS `luaCreatureAddHealth`; 772 `Heal` in `DrinkPotion` (`magic.cc`) via `TSkill::Change` clamp to Max.

- [x] `LuaMutation::PlayerAddHealth` + `call_lua_add_health` (same shape as `addMana`)
- [x] `lua_scope.rs` applier + `lua_script_player_add_health`
- [x] `CreatureRef:addHealth` userdata
- [x] Clamp `[0, effective_max_health]`; 772 Heal no-op when HP is already 0; health-bar notify on gain
- [x] Tests: clamp / floor-0 skip; `lua_defs` `addHealth`; `emit-lua-defs --check`
- [x] `tasks/lessons.md`
