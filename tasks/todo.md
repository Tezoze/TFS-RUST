# E5–E7: `say` / spellbook / `getHouse` — 2026-08-15

From [other-actions-plan.md](other-actions-plan.md). **Done.**

## E5 `Player:say(text[, type])`
- [x] `LuaMutation::PlayerSay` → `broadcast_creature_say_viewport` (not `player_say`)
- [x] Default `TALKTYPE_SAY` (1). 772 `Talk`; TFS `luaCreatureSay` / `luaPlayerSay`
- [x] `CreatureRef:say` userdata + `lua_defs`

## E6 `showTextDialog` + learned-spell list + `spellbook.lua`
- [x] `Player:showTextDialog(itemId, text)` → `send_text_window_simple_item` (`0x96`)
- [x] `Player:hasLearnedSpell(name)` from `persist.spells` (`SpellKnown`)
- [x] `Game.getInstantSpells()` = all **instant** defs (not TFS `canCast`)
- [x] Rewrite `spellbook.lua` to `GetSpellbook` (`magic.cc:3830-3901`)
- [x] Tests: Light Healing `exura - Light Healing: 25`; Berserk `4*Level`; no ML groups; `lua_defs`

## E7 `Tile:getHouse()`
- [x] `nil` or House userdata; **never `0`**. 772 `IsHouse(Obj1)`
- [x] Rewrite `construction_kits.lua`: house → transform + effect 3; else effect 4, no text
- [x] `lua_defs` `getHouse`
