# PC-3a Spell System — Remaining Gaps

Generated: July 2026
Scope: `data/scripts/spells/**/*.lua` (67 spell scripts)

## Current State

All 67 spells **load** and **register** correctly. All 67 spells **cast** —
`player_say_spell` dispatches vocation/level/mana/soul gates, exhaustion, PZ
check, mana deduction, and fires the `onCastSpell` Lua callback.

The gap is **inside `combat:execute()`** — the damage/effect resolution layer.

---

## Gap 1: `setCallback` Lua function invocation (CRITICAL)

**Affects:** 21 combat spells (all attack + healing spells with damage/heal values)

All 21 combat spells use `combat:setCallback(CALLBACK_PARAM_*, "onGetFormulaValues")`
to register a Lua function that computes `(min, max)` damage at cast time. Our
`Combat:execute()` reads `combat.formula` (set via `:setFormula()`), which is
`None` for all of these — so damage defaults to `(0, 0)`.

### C++ reference
- `Combat::doCombat` → `getCombatDamage` — `combat.cpp:100`
- `LuaScriptInterface::luaCombatSetCallback` — `luascript.cpp:13092`
- Callback dispatch: `CallBack::getCallBack` → `LuaScriptInterface::callLuaFunction`

### What needs to happen
1. At `Combat:execute()` time, look up the callback function name (e.g.
   `"onGetFormulaValues"`) in Lua globals via `lua.globals().get::<Function>(name)`.
2. Call it with the appropriate arguments based on the callback param type:
   - `CALLBACK_PARAM_LEVELMAGICVALUE` → `fn(creature, level, magic_level, factor)` → `(min, max)`
   - `CALLBACK_PARAM_SKILLVALUE` → `fn(player, skill, attack, factor)` → `(min, max)`
   - `CALLBACK_PARAM_TARGETCREATURE` → `fn(creature, target)` → `(min, max)`
   - `CALLBACK_PARAM_TARGETTILE` → `fn(creature, position)` → `(min, max)`
3. Use the returned `(min, max)` as the damage range in `CombatExecuteRequest`.

### Callback param constants (from `enums.h`)
| Constant | Value | Signature |
|----------|-------|-----------|
| `CALLBACK_PARAM_LEVELMAGICVALUE` | 1 | `(creature, level, maglevel, factor) → (min, max)` |
| `CALLBACK_PARAM_SKILLVALUE` | 2 | `(player, skill, attack, factor) → (min, max)` |
| `CALLBACK_PARAM_TARGETTILE` | 3 | `(creature, position) → (min, max)` |
| `CALLBACK_PARAM_TARGETCREATURE` | 4 | `(creature, target) → (min, max)` |

### Affected spells (21)
**LEVELMAGIC callback (20):**
- `attack/energy_beam.lua` — `ex,evo, vis, lux`
- `attack/energy_strike.lua` — `ex,ori, vis`
- `attack/energy_wave.lua` — `ex,evo, mort, hur`
- `attack/fire_wave.lua` — `ex,evo, flam, hur`
- `attack/flame_strike.lua` — `ex,ori, flam`
- `attack/force_strike.lua` — `ex,ori, mort`
- `attack/great_energy_beam.lua` — `ex,evo, gran, vis, lux`
- `attack/ultimate_explosion.lua` — `ex,evo, gran, mas, vis`
- `healing/heal_friend.lua` — `ex,ura, sio`
- `healing/intense_healing.lua` — `ex,ura, gran`
- `healing/light_healing.lua` — `ex,ura`
- `healing/mass_healing.lua` — `ex,ura, gran, mas, res`
- `healing/ultimate_healing.lua` — `ex,ura, vita`
- `runes/explosion_rune.lua` — `ad,evo, mas, hur`
- `runes/fireball_rune.lua` — `ad,ori, flam`
- `runes/great_fireball_rune.lua` — `ad,ori, gran, flam`
- `runes/heavy_magic_missile_rune.lua` — `ad,ori, gran`
- `runes/intense_healing_rune.lua` — `ad,ura, gran`
- `runes/light_magic_missile_rune.lua` — `ad,ori`
- `runes/sudden_death_rune.lua` — `ad,ori, vita, vis`
- `runes/ultimate_healing_rune.lua` — `ad,ura, vita`

**SKILLVALUE callback (1):**
- `attack/berserk.lua` — `ex,ori` (needs `player:getSkillLevel(SKILL_AXE)` + weapon attack)

**TARGETCREATURE callback (2):**
- `support/cancel_invisibility.lua` — `ex,ana, ina`
- `support/challenge.lua` — `ex,eta, res`

### Implementation notes
- The callback function is a **Lua global** (defined as `function onGetFormulaValues(...)`
  at file scope). It's stored by name in `combat.callbacks: HashMap<i32, CombatCallback>`.
- The `factor` parameter is typically `1.0` for instant spells (C++ `Combat::doCombat`
  passes `varSkillValue / 100.0` for skill callbacks, `1.0` for levelmagic).
- For `SKILLVALUE`, we need `player:getSkillLevel(SKILL_AXE)` and the weapon's
  `attack` value — requires `ScriptContext::get_player_skill_level` and weapon
  resolution (deferred — berserk only).

---

## Gap 2: Condition attachment in combat params (HIGH)

**Affects:** 14 spells that apply conditions (light, haste, magic_shield, poison, etc.)

These spells create a `Condition` via `createConditionObject()` and attach it to
the combat via `combat:setParameter(COMBAT_PARAM_CREATECONDITION, condition)`.
Our combat execution ignores the `apply_condition` field in `CombatParams`.

### C++ reference
- `Combat::doCombat` → `Combat::postCombatEffects` — `combat.cpp:643`
- `Condition::addCondition` — `condition.cpp`

### What needs to happen
1. Wire `createConditionObject()` to produce a `ConditionDef` stored on the combat.
2. At `combat_execute_from_lua` time, apply the condition to each target creature
   via the existing condition system (`tfs-rust-core/src/condition.rs`).
3. Map `CONDITION_PARAM_*` constants to condition fields.

### Affected spells (14)
- `support/light.lua` — light condition (6 min 10 sec)
- `support/great_light.lua` — light condition (26 min 40 sec)
- `support/ultimate_light.lua` — light condition (3 min 10 sec)
- `support/haste.lua` — haste condition (33 sec)
- `support/strong_haste.lua` — strong haste condition (22 sec)
- `support/magic_shield.lua` — magic shield condition (2 min)
- `support/invisibility.lua` — invisibility condition (2 min)
- `support/cancel_invisibility.lua` — dispel invisibility
- `attack/poison_storm.lua` — poison condition (3 cycles)
- `runes/paralyze_rune.lua` — paralyze condition
- `runes/envenom_rune.lua` — poison condition
- `runes/soulfire_rune.lua` — fire condition
- `runes/chameleon_rune.lua` — outfit condition
- `support/creature_illusion.lua` — outfit condition

---

## Gap 3: `creature:addItem` on CreatureRef (MEDIUM)

**Affects:** 8 conjuring/house spells

These spells call `creature:addItem(item_id)` or `player:addItem(item_id)` inside
their `onCastSpell` body. The `CreatureRef` userdata doesn't expose `addItem`.

### C++ reference
- `luaCreatureAddItem` — `luascript.cpp:7800`

### What needs to happen
1. Add `addItem(item_id_or_name, count)` method to `CreatureRef` userdata.
2. Route through a `LuaMutation::AddItem` (already exists for talkactions).
3. Handle item creation → inventory insertion → overflow to ground.

### Affected spells (8)
- `conjuring/conjure_arrow.lua` — `ex,evo, con`
- `conjuring/conjure_bolt.lua` — `ex,evo, con, mort`
- `conjuring/conjure_power_bolt.lua` — `ex,evo, con, vis`
- `conjuring/enchant_staff.lua` — `ex,eta, vis`
- `conjuring/explosive_arrow.lua` — `ex,evo, con, flam`
- `conjuring/food.lua` — `ex,evo, pan`
- `conjuring/poisoned_arrow.lua` — `ex,evo, con, pox`
- `houses/edit_door.lua` — `aleta grav` (house door editing)

---

## Gap 4: House spell APIs (LOW)

**Affects:** 4 house management spells

These spells call house-specific APIs (`house:inviteGuest()`, `house:kickGuest()`,
etc.) that don't exist on any userdata yet.

### Affected spells (4)
- `houses/invite_guests.lua` — `aleta sio`
- `houses/invite_subowners.lua` — `aleta som`
- `houses/kick_guest.lua` — `alana sio`
- `houses/edit_door.lua` — `aleta grav`

### What needs to happen
- House userdata + house access resolution from creature position.
- Full house system is a separate milestone (not PC-3a scope).

---

## Gap 5: `Game.create` / summon APIs (LOW)

**Affects:** 3 spells that summon creatures or create field items

### Affected spells (3)
- `support/summon_creature.lua` — `ut,evo, res` (summons a creature)
- `support/undead_legion.lua` — `ex,ana, mas, mort` (summons multiple)
- `runes/animate_dead_rune.lua` — `ad,ana, mort` (raises corpse as summon)

### What needs to happen
- `Game.createMonster(name, position)` Lua API.
- Summon tracking on player (master/summon relationship).
- Corpse→monster conversion for animate_dead.

---

## Gap 6: Utility spell APIs (LOW)

**Affects:** 4 spells with custom non-combat logic

### Affected spells (4)
- `support/find_person.lua` — `ex,iva` (player search by name)
- `support/levitate.lua` — `ex,ani, hur` (floor change)
- `support/magic_rope.lua` — `ex,ani, tera` (rope action)
- `runes/desintegrate_rune.lua` — `ad,ito, tera` (destroy item)
- `runes/destroy_field_rune.lua` — `ad,ito, grav` (destroy field)
- `runes/magic_wall_rune.lua` — `ad,evo, grav, tera` (create wall item)
- `runes/energy_field_rune.lua` — `ad,evo, grav, vis` (create field item)
- `runes/fire_field_rune.lua` — `ad,evo, grav, flam`
- `runes/poison_field_rune.lua` — `ad,evo, grav, pox`
- `runes/convince_creature_rune.lua` — `ad,eta, sio` (tame monster)
- `runes/paralyze_rune.lua` — `ad,ana, ani` (condition — covered by Gap 2)

### What needs to happen
- `Game.getPlayerByName(name)` API (find_person)
- Floor change / rope action APIs (levitate, magic_rope)
- Item destruction / field creation APIs (desintegrate, field runes)
- Monster taming / convince system (convince_creature)

---

## Gap 7: `setArea` / `createCombatArea` area shape resolution (PARTIAL)

**Affects:** 16 spells with area shapes

`createCombatArea(AREA_SQUARE1X1)` etc. is wired and produces an `AreaCombat` with
offsets. However, the area shapes defined in `areas.lua` use **matrix tables**
(e.g. `AREA_SQUARE1X1 = {{1, 1, 1}, {1, 1, 1}, {1, 1, 1}}`), not the disc-ring
model. The `affected_offsets()` method extracts non-zero entries as `(dx, dy)`
offsets, which works correctly for these matrix-defined areas.

**Status:** Partially working. The area offsets are extracted correctly, but
they bypass the `disc_offsets` ring model from `circles.rs`. Both approaches
produce the same tile lists for the areas used by these spells.

### Spells with area (16)
- `attack/berserk.lua` — `AREA_SQUARE1X1` (3×3)
- `attack/energy_beam.lua` — `AREA_BEAM5` (1×5 line)
- `attack/energy_wave.lua` — `AREA_WAVE3` (3-tile wide wave)
- `attack/fire_wave.lua` — `AREA_WAVE3`
- `attack/great_energy_beam.lua` — `AREA_BEAM7` (1×7 line)
- `attack/poison_storm.lua` — `AREA_CIRCLE5X5`
- `attack/ultimate_explosion.lua` — `AREA_CIRCLE6X6` (UE)
- `healing/mass_healing.lua` — `AREA_CIRCLE3X3`
- `runes/energybomb_rune.lua` — `AREA_SQUARE1X1`
- `runes/energy_wall_rune.lua` — `AREA_WALLFIELD`
- `runes/explosion_rune.lua` — `AREA_CIRCLE3X3`
- `runes/fireball_rune.lua` — `AREA_CIRCLE2X2`
- `runes/firebomb_rune.lua` — `AREA_SQUARE1X1`
- `runes/fire_wall_rune.lua` — `AREA_WALLFIELD`
- `runes/great_fireball_rune.lua` — `AREA_CIRCLE3X3`
- `runes/poisonbomb_rune.lua` — `AREA_SQUARE1X1`
- `runes/poison_wall_rune.lua` — `AREA_WALLFIELD`
- `support/cancel_invisibility.lua` — `AREA_CIRCLE2X2`
- `support/challenge.lua` — `AREA_CIRCLE2X2`
- `support/undead_legion.lua` — `AREA_CIRCLE3X3`

---

## Gap 8: `COMBAT_PARAM_*` parameter handling (PARTIAL)

Several `COMBAT_PARAM_*` values are set by spell scripts but not fully wired:

| Parameter | Status | Notes |
|-----------|--------|-------|
| `COMBAT_PARAM_TYPE` | ✅ Wired | Combat type (physical/fire/healing/etc.) |
| `COMBAT_PARAM_EFFECT` | ✅ Wired | Magic effect broadcast at center |
| `COMBAT_PARAM_AGGRESSIVE` | ✅ Wired | PZ check + self-damage skip |
| `COMBAT_PARAM_BLOCKARMOR` | ✅ Wired | Passed to request (applied in stimulus) |
| `COMBAT_PARAM_BLOCKSHIELD` | ⚠️ Stored | Not applied (shield defense deferred) |
| `COMBAT_PARAM_USECHARGES` | ❌ Ignored | Weapon charge consumption |
| `COMBAT_PARAM_DISPEL` | ❌ Ignored | Dispel condition on hit |
| `COMBAT_PARAM_CREATECONDITION` | ❌ Ignored | See Gap 2 |
| `COMBAT_PARAM_TARGETCASTERORTOPMOST` | ❌ Ignored | Self-target resolution |
| `COMBAT_PARAM_FORCEONTARGETEVENT` | ❌ Ignored | Require valid target |

---

## Priority Order

1. **Gap 1** (setCallback invocation) — makes 21 combat spells deal correct damage
2. **Gap 2** (condition attachment) — makes 14 buff/debuff spells work
3. **Gap 7+8** (area + param polish) — correctness for area spells
4. **Gap 3** (addItem) — makes 8 conjuring spells work
5. **Gap 5** (summon APIs) — makes 3 summon spells work
6. **Gap 6** (utility APIs) — makes 10 utility spells work
7. **Gap 4** (house APIs) — makes 4 house spells work (separate milestone)

---

## Summary Table

| Status | Count | Category |
|--------|-------|----------|
| ✅ Loads + casts + deals damage | 0 | (blocked by Gap 1) |
| ✅ Loads + casts (damage = 0) | 21 | Combat spells needing callback invocation |
| ✅ Loads + casts (no condition) | 14 | Condition spells needing Gap 2 |
| ✅ Loads + casts (custom API missing) | 22 | Conjuring/house/utility/summon |
| ✅ Loads + casts (area only, no damage) | 10 | Simple combat runes without callback |
| **Total** | **67** | |
