# PC-3a Spell System — Remaining Gaps

Updated: July 2026 (corrected against tree)
Scope: `data/scripts/spells/**/*.lua` (70 spell scripts, excluding `areas.lua`)

## Current State

All 70 spells **load** and **register** correctly. All 70 spells **cast** —
`player_say_spell` dispatches vocation/level/mana/soul gates, exhaustion, PZ
check, mana deduction, and fires the `onCastSpell` Lua callback.

The gap is **inside `combat:execute()`** and **missing Lua APIs** used by
spell bodies (damage callbacks, conditions, conjure helpers, utility).

**What already works:**
- `Combat()` constructor, `:setParameter`, `:setArea`, `:setFormula`, `:setCallback`
- `combat:execute()` resolves area offsets, computes formula-based damage, dispatches to core
- `combat:addCondition(condition)` — stores conditions on `CombatDef` (not yet applied)
- `Condition(type[, id])` constructor, `:setTicks`, `:setParameter` (all params wired)
- `CreatureRef:addCondition` / `:removeCondition` / `:addItem` / `:getLevel` / `:getGuid` /
  `:getPosition` / `:sendCancelMessage` (and related inventory helpers)
- All `COMBAT_PARAM_*`, `CONDITION_PARAM_*`, `ITEM_*`, `DIRECTION_*`, `CONST_PROP_*`,
  `TILESTATE_*`, `MESSAGE_*` constants registered
- All `SpellBuilder` methods (`runeMagicLevel`, `runeId`, `allowFarUse`, `blockWalls`,
  `checkFloor`, `isPzLock`, `cooldownSpellTime`, `needDirection`, `isBlocking`, etc.)

**Not yet wired (frequently needed by spell scripts):**
- `getMagicLevel`, `getSkillLevel`, `getAttackFactor`
- `combat:getTargets` / `combat:getPositions`
- Value / event callback invocation at execute time
- Loading `data/scripts/functions.lua` (defines `Player:conjureItem`,
  `Player:computeDamage` / `computeHealing` / `computeSkillDamage`)
- Bridging Lua `function Player:…` methods onto `CreatureRef` userdata
  (`Player` is currently an empty class table for event scripts)

**Note:** No spell in this pack uses `:setFormula()` — all damage/healing ranges
come from value callbacks (or hard-coded condition cycles).

---

## Gap 1: `setCallback` Lua function invocation (CRITICAL)

**Affects:** 24 spells that register Lua callbacks via `combat:setCallback`

### Value callbacks (return `(min, max)` damage) — 22 spells

These register `CALLBACK_PARAM_LEVELMAGICVALUE` or `CALLBACK_PARAM_SKILLVALUE`.
The callback function is a Lua global (e.g. `function onGetFormulaValues(...)`)
that computes and returns `(min, max)` damage at cast time. Our `Combat:execute()`
reads `combat.formula` (set via `:setFormula()`), which is `None` for all of these —
so damage defaults to `(0, 0)`.

### Event callbacks (fire-and-forget / return `bool`) — 2 spells

These register `CALLBACK_PARAM_TARGETCREATURE`. The callback is called per-target
during combat resolution and performs side effects (dispel, challenge). It does
**not** return damage values.

### C++ reference
- `Combat::doCombat` → `getCombatDamage` — `combat.cpp:100`
- `LuaScriptInterface::luaCombatSetCallback` — `luascript.cpp:13092`
- `ValueCallback::getMinMaxValues` — `combat.cpp:1111-1170` (value callbacks)
- `TargetCallback::onTargetCombat` — `combat.cpp:1223` (event callbacks)
- `TileCallback::onTileCombat` — `combat.cpp:1193` (event callbacks)

### Callback param constants (from `enums.h:127-132`, sequential 0..=3)

| Constant | Value | Type | Lua signature |
|----------|-------|------|---------------|
| `CALLBACK_PARAM_LEVELMAGICVALUE` | 0 | Value | `fn(player, level, maglevel) → (min, max)` |
| `CALLBACK_PARAM_SKILLVALUE` | 1 | Value | `fn(player, attackSkill, attackValue, attackFactor) → (min, max)` |
| `CALLBACK_PARAM_TARGETTILE` | 2 | Event | `fn(creature, position)` |
| `CALLBACK_PARAM_TARGETCREATURE` | 3 | Event | `fn(creature, target) → bool` |

**Source:** TVP `gameserver/src/enums.h:127-132` — `CallBackParam_t` enum.
Signatures from `combat.cpp:1134-1163` (value) and `combat.cpp:1193-1218`/`1223` (event).

### What needs to happen

1. **Value callbacks (LEVELMAGIC / SKILL):**
   - At `Combat:execute()` time, look up the callback function name in Lua globals.
   - Call it with the appropriate arguments based on the callback param type:
     - `LEVELMAGIC` → `fn(player, level, magic_level)` → `(min, max)`
     - `SKILL` → `fn(player, skill, attack, factor)` → `(min, max)`
   - Use the returned `(min, max)` as the damage range in `CombatExecuteRequest`.
   - Callback bodies call `player:computeDamage(...)` / `player:computeHealing(...)`
     / `player:computeSkillDamage(...)` — defined in `data/scripts/functions.lua`
     as `Player:` methods. **`functions.lua` must be loaded**, and those methods
     must resolve on the `CreatureRef` userdata passed as `player`.

2. **Event callbacks (TARGETCREATURE / TARGETTILE):**
   - At combat resolution time, for each affected creature/tile, call the callback.
   - `TARGETCREATURE` → `fn(creature, target) → bool` — if false, skip damage for that target.
   - `TARGETTILE` → `fn(creature, position)` — fire-and-forget side effect.
   - `cancel_invisibility` also needs `isPlayer`, `Game.getWorldType`, ring-item
     remove; `challenge` needs `doChallengeCreature` (see Gap 6).

### Affected spells (24)

**LEVELMAGIC value callback (21):**
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

**SKILL value callback (1):**
- `attack/berserk.lua` — `ex,ori` — Rust must supply `(skill, attack, factor)` when
  invoking the callback (`combat.cpp:1161` `player->getAttackFactor()`; weapon
  attack + skill from equipped weapon). Lua then calls
  `player:computeSkillDamage(80, 20, skill, false, true)`.

**TARGETCREATURE event callback (2):**
- `support/cancel_invisibility.lua` — `ex,ana, ina` (`target:removeCondition(CONDITION_INVISIBLE)`;
  `:removeCondition` already exists on `CreatureRef`)
- `support/challenge.lua` — `ex,eta, res` (`doChallengeCreature(creature, target)`)

### Implementation notes
- Callback name is a **Lua global**, stored in `combat.callbacks: HashMap<i32, CombatCallback>`.
- `computeDamage` / `computeHealing` / `computeSkillDamage` are in
  `data/scripts/functions.lua` (~463–512). They call `self:getMagicLevel()` and
  `self:getLevel()`. **`getMagicLevel` is not on `CreatureRef` yet**; `getLevel` is.
- `functions.lua` is currently not loaded — load it before spell scripts (after
  `areas.lua`), and ensure `Player:` Lua methods are visible on `CreatureRef`.

---

## Gap 2: Condition application in combat execution (HIGH)

**Affects:** 18 spells that apply or dispel conditions (some overlap with Gap 1 / 3)

These spells create a `Condition` via `Condition(type)` and either:
- Attach it to a combat via `combat:addCondition(condition)` (8 spells)
- Apply it directly via `creature:` / `target:addCondition(condition)` in
  `onCastSpell` (5 spells — **not** via `setCallback`)
- Dispel via `combat:setParameter(COMBAT_PARAM_DISPEL, …)` (9 spells)

`combat:addCondition(condition)` stores on `CombatDef`, but execute does not
**apply** those conditions. Direct `addCondition` / `removeCondition` methods
already exist on `CreatureRef`; remaining work is combat-path application,
condition-field mapping (outfit / cycle DoT), and helper APIs those scripts call.

### C++ reference
- `Combat::doCombat` → `Combat::postCombatEffects` — `combat.cpp:643`
- `Condition::addCondition` — `condition.cpp`
- `CombatParams::conditionList` — `combat.h:44`
- `CombatParams::dispelType` — `combat.h:52`

### What needs to happen

1. **`combat:addCondition` conditions (8 spells):**
   - After damage resolution, apply each `CombatDef.conditions` entry to each
     target via `tfs-rust-core` condition system.
   - Map `ConditionBuilder` fields → core `Condition` (ticks, speed, light,
     outfit, etc.).

2. **Direct `addCondition` in `onCastSpell` (5 spells):**
   - `CreatureRef:addCondition` already exists — verify `ConditionBuilder` →
     core mapping covers outfit + poison/fire cycle params
     (`CONDITION_PARAM_CYCLE`, `COUNT`, `MAX_COUNT`, `OWNERGUID`).
   - Extra APIs these scripts need:
     - `poison_storm`: `combat:getTargets`, `creature:computeDamage` (from
       `functions.lua`), `creature:setInFight`
     - `envenom_rune` / `soulfire_rune`: `Creature(variant.number)`,
       `creature:computeDamage`
     - `creature_illusion` / `chameleon_rune`: `MonsterType`, `setOutfit`,
       `hasFlag(PlayerFlag_CanIllusionAll)`

3. **`COMBAT_PARAM_DISPEL` (9 spells):**
   - Wire `CombatDef.dispel_type` (currently ignored in `set_parameter`).
   - At resolution, remove conditions of that type from each target.

4. **`target:removeCondition(type)` (cancel_invisibility):**
   - Method already on `CreatureRef`; depends on Gap 1 event-callback invocation.

### Affected spells

**`combat:addCondition` (8):**
- `support/light.lua` — light (6 min 10 sec)
- `support/great_light.lua` — light (11 min 35 sec)
- `support/ultimate_light.lua` — light (33 min 10 sec)
- `support/haste.lua` — haste (30 sec, speed +30)
- `support/strong_haste.lua` — strong haste (30 sec, speed +60)
- `support/magic_shield.lua` — magic shield (3 min 20 sec)
- `support/invisibility.lua` — invisibility (3 min 20 sec)
- `runes/paralyze_rune.lua` — paralyze (10 sec, speed -101)

**Direct `addCondition` in `onCastSpell` (5) — not callbacks:**
- `attack/poison_storm.lua` — poison on each `combat:getTargets(...)` hit
- `runes/envenom_rune.lua` — poison on `Creature(variant.number)`
- `runes/soulfire_rune.lua` — fire on target + `COMBAT_PARAM_NODAMAGE`
- `support/creature_illusion.lua` — outfit on caster
- `runes/chameleon_rune.lua` — outfit on caster

**`COMBAT_PARAM_DISPEL` (9):**
- `support/antidote.lua` — `CONDITION_POISON`
- `runes/antidote_rune.lua` — `CONDITION_POISON`
- `healing/light_healing.lua` — `CONDITION_PARALYZE`
- `healing/intense_healing.lua` — `CONDITION_PARALYZE`
- `healing/ultimate_healing.lua` — `CONDITION_PARALYZE`
- `healing/mass_healing.lua` — `CONDITION_PARALYZE`
- `healing/heal_friend.lua` — `CONDITION_PARALYZE`
- `runes/intense_healing_rune.lua` — `CONDITION_PARALYZE`
- `runes/ultimate_healing_rune.lua` — `CONDITION_PARALYZE`

**`target:removeCondition` (1, depends on Gap 1):**
- `support/cancel_invisibility.lua` — remove `CONDITION_INVISIBLE`

---

## Gap 3: `conjureItem` via `functions.lua` (MEDIUM)

**Affects:** 32 scripts that call `creature:conjureItem(...)` (+ `food.lua` uses
`addItem`, which is already on `CreatureRef`)

`conjureItem` is **not** a missing native Rust method. It is defined in
`data/scripts/functions.lua` as:

```lua
function Player:conjureItem(conjureMana, reagentId, conjureId, conjureCount, effect)
```

### Call shapes (same Lua function)

**Rune conjure (26 scripts)** — reagent = blank rune `2260`:
```lua
creature:conjureItem(mana_cost, 2260, rune_item_id, charges)
```

**Conjuring spells (6 scripts)** — first arg is the `spell` userdata (unused when
`reagentId == 0`; mana already taken by the spell system):
```lua
creature:conjureItem(spell, 0, item_id, count)       -- arrows/bolts
creature:conjureItem(spell, 2401, 2433, 1)           -- enchant staff (reagent 2401)
```

**Food (1 script)** — already uses Rust `CreatureRef:addItem`:
```lua
creature:addItem(foods[math.random(#foods)])  -- food.lua
```

### C++ reference
- Datapack helper in `functions.lua` (not in C++ trees).
- Closest C++ analog for item insert: `luaCreatureAddItem` — `luascript.cpp:7800`.

### What needs to happen
1. Load `data/scripts/functions.lua` before spell scripts (after `areas.lua`).
2. Make `function Player:…` methods resolve on `CreatureRef` userdata (metatable
   bridge / `__index` — today `Player` is an empty class table).
3. Ensure helpers used inside `conjureItem` work: `getSlotItem`, `addItem`,
   `ItemType`, `item:remove` / `:transform` / `:hasAttribute` / `:decay`,
   `getMana` / `addMana` / `addManaSpent`, `getGroup():hasFlag`,
   `sendCancelMessage`, `getPosition():sendMagicEffect`.
4. Do **not** reimplement `conjureItem` as a special-cased Rust method unless the
   Lua bridge proves insufficient.

### Affected scripts (32 conjureItem + food)
- All 26 rune scripts (instant half conjures the rune)
- `conjuring/conjure_arrow.lua` — `ex,evo, con`
- `conjuring/conjure_bolt.lua` — `ex,evo, con, mort`
- `conjuring/conjure_power_bolt.lua` — `ex,evo, con, vis`
- `conjuring/enchant_staff.lua` — `ex,eta, vis`
- `conjuring/explosive_arrow.lua` — `ex,evo, con, flam`
- `conjuring/poisoned_arrow.lua` — `ex,evo, con, pox`
- `conjuring/food.lua` — `ex,evo, pan` (`addItem` only; mostly wired)

---

## Gap 4: House spell APIs (LOW)

**Affects:** 4 house management spells

### Affected spells (4)
- `houses/invite_guests.lua` — `aleta sio`
- `houses/invite_subowners.lua` — `aleta som`
- `houses/kick_guest.lua` — `alana sio`
- `houses/edit_door.lua` — `aleta grav`

### What needs to happen
- House userdata + house access resolution from creature position.
- Full house system is a separate milestone (not PC-3a scope).

---

## Gap 5: `Game.createMonster` / summon APIs (LOW)

**Affects:** 3 spells that summon creatures

### Affected spells (3)
- `support/summon_creature.lua` — `ut,evo, res`
- `support/undead_legion.lua` — `ex,ana, mas, mort`
- `runes/animate_dead_rune.lua` — `ad,ana, mort`

### What needs to happen
- `Game.createMonster(name, position[, …])` Lua API.
- Summon tracking on player (master/summon relationship).
- Corpse→monster conversion for animate_dead.

---

## Gap 6: Utility spell APIs (LOW)

**Affects:** 7 spells with custom non-combat / non-CREATEITEM logic

Field, wall, bomb, magic-wall, and wild-growth item placement go through
`COMBAT_PARAM_CREATEITEM` — see Gap 8 (not listed here).

### Affected spells (7)
- `support/find_person.lua` — `ex,iva` (player search by name)
- `support/levitate.lua` — `ex,ani, hur` (floor change)
- `support/magic_rope.lua` — `ex,ani, tera` (rope action)
- `runes/desintegrate_rune.lua` — `ad,ito, tera` (destroy item)
- `runes/destroy_field_rune.lua` — `ad,ito, grav` (destroy field)
- `runes/convince_creature_rune.lua` — `ad,eta, sio` (tame monster)
- `support/wild_growth.lua` — `ex,evo, grav, vita` (also Gap 8 CREATEITEM;
  needs `Tile`, `getNextPosition`, PZ cancel)

### Shared helpers also needed by Gap 1 event spells
- `doChallengeCreature(creature, target)` — C++ `luaDoChallengeCreature`,
  `luascript.cpp:1033` (`challenge.lua`)
- `Game.getWorldType()` / `isPlayer` / ring remove (`cancel_invisibility.lua`)

### What needs to happen
- `Game.getPlayerByName(name)` (find_person)
- Floor change / rope action APIs (levitate, magic_rope)
- Item / field destruction APIs (desintegrate, destroy_field)
- Monster taming / convince system
- `doChallengeCreature` global
- Tile / direction helpers for wild_growth front-tile checks

---

## Gap 7: `setArea` / `createCombatArea` area shape resolution (PARTIAL)

**Affects:** 20 spells with area shapes

`createCombatArea(AREA_…)` is wired and produces an `AreaCombat` with offsets.
Matrix tables in `areas.lua` are extracted as non-zero `(dx, dy)` offsets via
`affected_offsets()` — correct for these matrices.

**Status:** Partially working. Offsets bypass the `disc_offsets` ring model in
`circles.rs`; both produce the same tile lists for areas these spells use.

**Note:** Wall spells pass a diagonal overlay:
`createCombatArea(AREA_WALLFIELD, AREADIAGONAL_WALLFIELD)`. `extArea` is accepted
but not processed — `combat.rs` notes "PC-2b scope: matrix only".

### Spells with area (20)
- `attack/berserk.lua` — `AREA_SQUARE1X1`
- `attack/energy_beam.lua` — `AREA_BEAM5`
- `attack/energy_wave.lua` — `AREA_SQUAREWAVE5`
- `attack/fire_wave.lua` — `AREA_WAVE4`
- `attack/great_energy_beam.lua` — `AREA_BEAM8`
- `attack/poison_storm.lua` — `AREA_CIRCLE5X5`
- `attack/ultimate_explosion.lua` — `AREA_CIRCLE5X5`
- `healing/mass_healing.lua` — `AREA_CIRCLE3X3`
- `runes/energybomb_rune.lua` — `AREA_SQUARE1X1`
- `runes/energy_wall_rune.lua` — `AREA_WALLFIELD` + diagonal overlay
- `runes/explosion_rune.lua` — `AREA_CROSS1X1`
- `runes/fireball_rune.lua` — `AREA_CIRCLE2X2`
- `runes/firebomb_rune.lua` — `AREA_SQUARE1X1`
- `runes/fire_wall_rune.lua` — `AREA_WALLFIELD` + diagonal overlay
- `runes/great_fireball_rune.lua` — `AREA_CIRCLE3X3`
- `runes/poisonbomb_rune.lua` — `AREA_SQUARE1X1`
- `runes/poison_wall_rune.lua` — `AREA_WALLFIELD` + diagonal overlay
- `support/cancel_invisibility.lua` — `AREA_CIRCLE3X3`
- `support/challenge.lua` — `AREA_SQUARE1X1`
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
| `COMBAT_PARAM_DISTANCEEFFECT` | ⚠️ Stored | Not sent (distance effect packet deferred) |
| `COMBAT_PARAM_CREATEITEM` | ⚠️ Stored | On `CombatDef.create_item`; not created on hit tiles |
| `COMBAT_PARAM_NODAMAGE` | ⚠️ Stored | On `CombatDef.no_damage`; not applied |
| `COMBAT_PARAM_DISPEL` | ❌ Ignored | See Gap 2 (9 spells) |
| `COMBAT_PARAM_USECHARGES` | ❌ Ignored | Weapon charge consumption |
| `COMBAT_PARAM_TARGETCASTERORTOPMOST` | ❌ Ignored | Self-target resolution |
| `COMBAT_PARAM_FORCEONTARGETEVENT` | ❌ Ignored | Used by `poison_storm.lua` |

**`COMBAT_PARAM_CREATEITEM` scripts (11):**
- Field: `fire_field_rune`, `energy_field_rune`, `poison_field_rune`
- Wall: `fire_wall_rune`, `energy_wall_rune`, `poison_wall_rune` (+ Gap 7 diagonal)
- Bomb: `firebomb_rune`, `energybomb_rune`, `poisonbomb_rune`
- Other: `magic_wall_rune`, `support/wild_growth.lua` (item id `1499`)

**Note:** `COMBAT_PARAM_CREATECONDITION` does **not** exist in the 772 TVP tree
or any script. Conditions use `combat:addCondition(condition)`.

---

## Implementation Plan

### Phase 1: Load `functions.lua` + value callback invocation (Gap 1 — value)

**Goal:** 22 combat spells deal correct damage / heal correct amounts; unlock
`conjureItem` / `computeDamage` Lua helpers.

**Files:**
- `crates/tfs-rust-lua/src/combat_scripts.rs` — load `data/scripts/functions.lua`
  after `areas.lua`, before spell scripts.
- `crates/tfs-rust-lua/src/runtime.rs` / `userdata/player.rs` — bridge `Player:`
  Lua methods onto `CreatureRef`; add `getMagicLevel()` (and skill/attack factor
  reads for SKILL callback args).
- `crates/tfs-rust-lua/src/userdata/combat.rs` — when `formula` is `None` but a
  value callback exists, invoke it and set `damage_min` / `damage_max`.

**Steps:**
1. Load `functions.lua` in `load_spell_scripts`.
2. Ensure `CreatureRef` exposes `getLevel`, `getMagicLevel`; bridge Lua `Player:` methods.
3. In `CombatRef::execute()`, resolve `CALLBACK_PARAM_LEVELMAGICVALUE` (0) /
   `CALLBACK_PARAM_SKILLVALUE` (1), call the global, parse `(min, max)`.
4. For SKILL: pass `(skill, weapon_attack, attack_factor)` from Rust (C++ parity).
5. Test: `energy_strike` → non-zero damage; `conjure_arrow` → arrows if helpers OK.

### Phase 2: Condition application (Gap 2 — combat:addCondition)

**Goal:** 8 buff/debuff spells apply conditions to targets.

**Files:**
- `crates/tfs-rust-lua/src/lua_mutation.rs` — `conditions` on `CombatExecuteRequest`
- `crates/tfs-rust-core/src/combat/` — apply after damage
- `crates/tfs-rust-lua/src/userdata/combat.rs` — pass `combat.conditions`

**Steps:**
1. Add `conditions: Vec<ConditionBuilder>` to the request.
2. Populate from `combat.conditions` in `execute()`.
3. Map builder → core `Condition`; apply per target.
4. Test: cast `light`, verify light condition on caster.

### Phase 3: Direct-condition spell helpers (Gap 2 — onCastSpell path)

**Goal:** 5 spells that call `addCondition` outside combat:addCondition work end-to-end.

**Files:**
- `userdata/combat.rs` — `getTargets` / `getPositions` if missing
- Condition mapping for cycle DoT + outfit
- `Creature(id)` constructor, `setInFight`, `MonsterType` as needed

**Steps:**
1. Confirm `addCondition` mapping covers poison/fire cycles + outfit.
2. Add `combat:getTargets` for `poison_storm`.
3. Test: `creature_illusion`, `envenom_rune`, `poison_storm`.

### Phase 4: `COMBAT_PARAM_DISPEL` (Gap 2 — dispel)

**Goal:** 9 dispel scripts remove the named condition.

**Files:**
- `userdata/combat.rs` — `dispel_type` on `CombatDef` + `set_parameter`
- `lua_mutation.rs` / core combat — remove on hit

**Steps:**
1. Store `COMBAT_PARAM_DISPEL` (8) on `CombatDef`.
2. Remove matching conditions per target after (or instead of) damage.
3. Test: `antidote` clears poison; healing clears paralyze.

### Phase 5: Conjure path verification (Gap 3)

**Goal:** 32 `conjureItem` scripts work via `functions.lua` (not a new Rust API).

**Depends on:** Phase 1 load + Player method bridge + inventory helpers.

**Steps:**
1. After Phase 1, cast `conjure arrow` / fireball rune conjure.
2. Fill any missing helpers (`transform`, mana, `ItemType`, decay).
3. `food.lua` should already work if `Position:sendMagicEffect` is wired.

### Phase 6: Event callbacks (Gap 1 — event)

**Goal:** `cancel_invisibility` + `challenge` TARGETCREATURE callbacks run.

**Depends on:** Gap 6 for `doChallengeCreature`; `removeCondition` already exists.

**Steps:**
1. Per target, call `CALLBACK_PARAM_TARGETCREATURE` (3) before damage.
2. Skip damage when callback returns false.
3. Test: challenge + cancel invisibility.

### Phase 7: Diagonal area overlay (Gap 7)

**Goal:** Wall runes cover diagonal orientations.

**Steps:**
1. Process `extArea` in `createCombatArea`.
2. Select orientation from caster direction at execute.
3. Test: fire wall on diagonal facing.

### Phase 8: Remaining `COMBAT_PARAM_*` (Gap 8)

**Goal:** `CREATEITEM`, `NODAMAGE`, `DISTANCEEFFECT` fully wired (11 CREATEITEM scripts).

**Steps:**
1. Pass `create_item`, `no_damage`, `distance_effect` into `CombatExecuteRequest`.
2. Create item on affected tiles; skip damage when `no_damage`; send distance FX.
3. Test: fire field rune, soulfire (`NODAMAGE`), wild growth.

---

## Priority Order

1. **Phase 1** — value callbacks + `functions.lua` + `getMagicLevel` / Player bridge
2. **Phase 2** — `combat:addCondition` application
3. **Phase 3** — direct-condition helpers (`getTargets`, cycle/outfit mapping)
4. **Phase 4** — `COMBAT_PARAM_DISPEL` (9 scripts)
5. **Phase 5** — verify conjure via Lua (not new Rust `conjureItem`)
6. **Phase 6** — event callbacks
7. **Phase 7** — diagonal wall areas
8. **Phase 8** — CREATEITEM / NODAMAGE / DISTANCEEFFECT

---

## Summary Table

| Status | Count | Category |
|--------|-------|----------|
| ✅ Loads + casts (damage = 0; value callback not invoked) | 22 | Phase 1 |
| ✅ Loads + casts (`combat:addCondition` not applied) | 8 | Phase 2 |
| ✅ Loads + casts (direct `addCondition` / helpers incomplete) | 5 | Phase 3 |
| ✅ Loads + casts (dispel not applied) | 9 | Phase 4 |
| ✅ Loads + casts (`conjureItem` needs `functions.lua` bridge) | 32 | Phase 5 |
| ✅ Loads + casts (`addItem` food; mostly wired) | 1 | Phase 5 footnote |
| ✅ Loads + casts (event callback not invoked) | 2 | Phase 6 |
| ✅ Loads + casts (diagonal area partial) | 3 | Phase 7 |
| ✅ Loads + casts (`CREATEITEM` not created) | 11 | Phase 8 |
| ✅ Loads + casts (house API missing) | 4 | Gap 4 |
| ✅ Loads + casts (summon API missing) | 3 | Gap 5 |
| ✅ Loads + casts (utility API missing) | 7 | Gap 6 |
| **Unique scripts** | **70** | Categories overlap |

**Overlap examples:**
- `soulfire_rune` — Phase 5 (conjure) + Phase 3 (`addCondition`) + Phase 8 (`NODAMAGE`)
- `wild_growth` — Gap 6 (Tile/PZ) + Phase 8 (`CREATEITEM`)
- Healing instants/runes — Phase 1 (value callback) + Phase 4 (`DISPEL`)

---

## Correction log (July 2026)

- Counted all 70 scripts; documented missing `wild_growth.lua`.
- Fixed Gap 7 area constant names to match scripts.
- Reclassified poison_storm / envenom / soulfire as direct `onCastSpell` condition
  use (not TARGETCREATURE callbacks).
- Noted `addCondition` / `removeCondition` / `addItem` already on `CreatureRef`.
- DISPEL = 9 scripts (was undercounted; healing runes included).
- CREATEITEM = 11 scripts (was “4 field/magic wall”).
- `conjureItem` lives in `functions.lua` — Phase 5 is load/bridge, not a new Rust API.
- Removed contradictory “~10 formula-based” summary row (`setFormula` unused).
