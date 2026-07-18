# PC-3a Spell System — Remaining Gaps

Updated: 2026-07-18 (Phases 2–5 + 4b landed)
Scope: `data/scripts/spells/**/*.lua` (70 spell scripts, excluding `areas.lua`)

## Current State

All 70 spells **load** and **register** correctly. All 70 spells **cast** —
`player_say_spell` dispatches vocation/level/mana/soul gates, exhaustion, PZ
check, mana deduction, and fires the `onCastSpell` Lua callback.

**Phases 1–5 + 4b are done.** Value callbacks, combat `addCondition` application,
full `ConditionApplySpec` → `ActiveCondition` mapping, `COMBAT_PARAM_DISPEL`,
`combat:getTargets`, `Creature(id)`, `isPlayer`, `setInFight`, condition
**client notifies**, and **conjure helpers** (`getMana` / transform / charges /
`Group:hasFlag`) are wired.

Remaining gaps are **CREATEITEM / NODAMAGE / distance FX** (Phase 8),
**event callbacks** (Phase 6), diagonal areas (Phase 7), and **non-combat APIs**
(houses / summons / utilities).

**What already works:**
- `Combat()` constructor, `:setParameter`, `:setArea`, `:setFormula`, `:setCallback`
- `combat:execute()` — area offsets, LoS, damage/heal roll, magic effects, damage text
- **Value callbacks** — `CALLBACK_PARAM_LEVELMAGICVALUE` / `SKILLVALUE` invoked at
  execute; `(min, max)` used as damage range
- `data/scripts/functions.lua` loaded before spell scripts (after `areas.lua`)
- `Player:` Lua method bridge via `CreatureRef` `__index` fallback
- `getMagicLevel`, `getLevel`; SKILL path uses `get_player_weapon_combat_params`
- **`combat:addCondition`** — stored + applied per target in `aoe.rs` (Phase 2)
- **`COMBAT_PARAM_DISPEL`** — stored + removes matching conditions after damage (Phase 4)
- **`ConditionBuilder` → `ActiveCondition`** — Light/Speed/Damage/Outfit/Generic/
  Regeneration + cycle→timer_rounds (Phase 3 mapper)
- `CreatureRef:addCondition` (full spec) / `:removeCondition` / `:isPlayer` /
  `:setInFight` / `:addItem` / inventory helpers
- `getMana` / `addMana` / `addManaSpent`; `item:transform` / `hasAttribute`;
  `ItemType:getCharges`; `Group:hasFlag`
- `combat:getTargets(creature, variant)` — area creature list
- `Creature(id)` constructor
- Constants + `SpellBuilder` methods registered

**Not yet wired (frequently needed):**
- Event callback invocation (`TARGETCREATURE` / `TARGETTILE`)
- `create_item` / `no_damage` / `distance_effect` on execute
- `MonsterType` (outfit from monster), `Game.createMonster`, house / Tile utilities
- Diagonal `extArea` overlay

**Note:** No spell in this pack uses `:setFormula()` — all damage/healing ranges
come from value callbacks (or hard-coded condition cycles).

---

## Gap 1: `setCallback` Lua function invocation

**Affects:** 24 spells that register Lua callbacks via `combat:setCallback`

### Value callbacks — ✅ DONE (Phase 1)

22 spells with `CALLBACK_PARAM_LEVELMAGICVALUE` or `CALLBACK_PARAM_SKILLVALUE`.
`Combat:execute()` invokes the Lua global and uses returned `(min, max)`.

**Landed in:**
- `userdata/combat.rs` — `invoke_value_callback`
- `combat_scripts.rs` — load `functions.lua`
- `userdata/player.rs` — `getMagicLevel` + `Player:` `__index` bridge
- `ScriptContext::get_player_weapon_combat_params` — SKILL args for berserk

**Affected (21 LEVELMAGIC + 1 SKILL):** energy/fire beams/waves/strikes,
ultimate explosion, all healing instants/runes with value callbacks, missile /
fireball / sudden death runes, `berserk` (SKILL).

### Event callbacks — ❌ OPEN (Phase 6)

2 spells with `CALLBACK_PARAM_TARGETCREATURE` — not invoked at resolution time.

| Spell | Needs |
|-------|-------|
| `support/cancel_invisibility.lua` | Per-target callback; `Game.getWorldType`, ring remove (`isPlayer` / `removeCondition` exist) |
| `support/challenge.lua` | `doChallengeCreature(creature, target)` |

### C++ reference
- `Combat::doCombat` → `getCombatDamage` — `combat.cpp:100`
- `ValueCallback::getMinMaxValues` — `combat.cpp:1111-1170`
- `TargetCallback::onTargetCombat` — `combat.cpp:1223`
- `TileCallback::onTileCombat` — `combat.cpp:1193`

| Constant | Value | Type | Status |
|----------|-------|------|--------|
| `CALLBACK_PARAM_LEVELMAGICVALUE` | 0 | Value | ✅ Invoked |
| `CALLBACK_PARAM_SKILLVALUE` | 1 | Value | ✅ Invoked |
| `CALLBACK_PARAM_TARGETTILE` | 2 | Event | ❌ Not invoked |
| `CALLBACK_PARAM_TARGETCREATURE` | 3 | Event | ❌ Not invoked |

---

## Gap 2: Condition application in combat execution — ✅ DONE (Phases 2–4)

**Affects:** 18 spells that apply or dispel conditions (overlap with Gaps 1 / 3)

### Status (2026-07-18 — Phases 2–4 + 4b landed)

| Path | Status |
|------|--------|
| `combat:addCondition` store on `CombatDef` | ✅ |
| Apply conditions in `combat_execute_from_lua` | ✅ Via `ConditionApplySpec` on request |
| Direct `CreatureRef:addCondition` | ✅ Full builder → `active_condition_from_apply_spec` |
| `condition:setOutfit` | ✅ Stores `lookType` → `ConditionData::Outfit` |
| `COMBAT_PARAM_DISPEL` | ✅ Stored + applied after damage (heal+dispel both run) |
| `combat:getTargets` / `Creature(id)` / `isPlayer` / `setInFight` | ✅ |
| **Client notifies (Phase 4b)** | ✅ Icons / speed / light / invis / outfit on start+end |

**Phase 4b:** `on_condition_started` / `on_condition_ended` — `send_player_icons`,
`announce_creature_speed`, `Player::internal_light` + `change_creature_light`,
`announce_player_change_visible`, outfit packet. Wired from AoE apply/dispel,
Lua add/remove/`setInFight`, and `ProcessSkills` expiry.

**Remaining for illusion/chameleon end-to-end:** `MonsterType:getOutfit()` still
needed so scripts can feed lookType (outfit condition + client path ready).

### C++ reference
- `Combat::doCombat` → `Combat::postCombatEffects` — `combat.cpp:643`
- `Condition::addCondition` — `condition.cpp`
- `CombatParams::conditionList` / `dispelType` — `combat.h:44,52`

### Affected spells (now unblocked for conditions/dispel)

**`combat:addCondition` (8):** light / great_light / ultimate_light / haste /
strong_haste / magic_shield / invisibility / paralyze_rune

**Direct `addCondition` (5):** poison_storm (`getTargets`), envenom/soulfire
(`Creature(id)`), creature_illusion / chameleon (need `MonsterType` for outfit source)

**`COMBAT_PARAM_DISPEL` (9):** antidote + healing clears paralyze

---

## Gap 3: `conjureItem` via `functions.lua` (MEDIUM)

**Affects:** 33 scripts that call `creature:conjureItem(...)` (+ `food.lua` uses
`addItem`, which is already on `CreatureRef`)

### Status (2026-07-18)

| Piece | Status |
|-------|--------|
| Load `functions.lua` | ✅ |
| `Player:` bridge on `CreatureRef` | ✅ |
| `getSlotItem` / `addItem` / `remove` / `getGroup():hasFlag` / `sendCancelMessage` / `Position:sendMagicEffect` | ✅ |
| `getMana` / `addMana` / `addManaSpent` | ✅ |
| `item:transform` / `item:hasAttribute` | ✅ |
| `ItemType:getCharges` | ✅ |
| `item:decay` | ⚠️ Method exists; core is logged no-op |

`conjureItem` is Lua in `functions.lua` — do **not** reimplement as a special
Rust method. Helpers above landed in Phase 5; cast-test conjure arrow / fireball
rune conjure to confirm end-to-end.

### Affected scripts (33 conjureItem + food)
- All 26 rune scripts (instant half conjures the rune) + animate_dead conjure half
- `conjuring/conjure_arrow.lua`, `conjure_bolt.lua`, `conjure_power_bolt.lua`
- `conjuring/enchant_staff.lua`, `explosive_arrow.lua`, `poisoned_arrow.lua`
- `conjuring/food.lua` — `addItem` only; mostly wired

---

## Gap 4: House spell APIs (LOW) — out of PC-3a scope

**Affects:** 4 house management spells

- `houses/invite_guests.lua` — `aleta sio`
- `houses/invite_subowners.lua` — `aleta som`
- `houses/kick_guest.lua` — `alana sio`
- `houses/edit_door.lua` — `aleta grav`

Needs house userdata + access from creature tile. Separate milestone.

---

## Gap 5: `Game.createMonster` / summon APIs (LOW)

**Affects:** 3 spells

- `support/summon_creature.lua` — `ut,evo, res`
- `support/undead_legion.lua` — `ex,ana, mas, mort`
- `runes/animate_dead_rune.lua` — `ad,ana, mort`

Needs `Game.createMonster`, summon tracking, corpse→monster for animate dead.

---

## Gap 6: Utility spell APIs (LOW)

**Affects:** 7+ spells with custom non-combat logic

Field/wall/bomb placement is Gap 8 (`CREATEITEM`), not listed here.

- `support/find_person.lua` — `Game.getPlayerByName`
- `support/levitate.lua` / `magic_rope.lua` — floor / rope APIs
- `runes/desintegrate_rune.lua` / `destroy_field_rune.lua` — Tile item destroy
- `runes/convince_creature_rune.lua` — tame / convince
- `support/wild_growth.lua` — `Tile`, `getNextPosition`, PZ cancel (+ Gap 8)

**Shared with Phase 6 event spells:**
- `doChallengeCreature` — `challenge.lua`
- `Game.getWorldType` / `isPlayer` / ring remove — `cancel_invisibility.lua`

---

## Gap 7: `setArea` / `createCombatArea` (PARTIAL)

**Affects:** 20 spells with area shapes

Matrix offsets work. Directional rotation from caster→center works.

**Still open:** `extArea` diagonal overlay accepted but unused
(`createCombatArea(AREA_WALLFIELD, AREADIAGONAL_WALLFIELD)` — 3 wall runes).

### Spells with area (20)
berserk, energy_beam/wave, fire_wave, great_energy_beam, poison_storm,
ultimate_explosion, mass_healing, energybomb/firebomb/poisonbomb,
energy/fire/poison wall (+ diagonal), explosion/fireball/great_fireball,
cancel_invisibility, challenge, undead_legion.

---

## Gap 8: `COMBAT_PARAM_*` parameter handling (PARTIAL)

| Parameter | Status | Notes |
|-----------|--------|-------|
| `COMBAT_PARAM_TYPE` | ✅ | |
| `COMBAT_PARAM_EFFECT` | ✅ | Per-tile broadcast in `aoe.rs` |
| `COMBAT_PARAM_AGGRESSIVE` | ✅ | PZ + self-skip |
| `COMBAT_PARAM_BLOCKARMOR` | ✅ | |
| `COMBAT_PARAM_BLOCKSHIELD` | ⚠️ Stored | Not applied |
| `COMBAT_PARAM_DISTANCEEFFECT` | ⚠️ Stored | Not sent |
| `COMBAT_PARAM_CREATEITEM` | ⚠️ Stored | Not created on hit tiles |
| `COMBAT_PARAM_NODAMAGE` | ⚠️ Stored | Not applied (soulfire) |
| `COMBAT_PARAM_DISPEL` | ❌ Ignored | Gap 2 / Phase 4 |
| `COMBAT_PARAM_USECHARGES` | ❌ Ignored | |
| `COMBAT_PARAM_TARGETCASTERORTOPMOST` | ❌ Ignored | |
| `COMBAT_PARAM_FORCEONTARGETEVENT` | ❌ Ignored | `poison_storm.lua` |

**`CREATEITEM` scripts (11):** fire/energy/poison field, wall, bomb; magic_wall;
`support/wild_growth.lua` (item `1499`).

---

## Implementation Plan

### Phase 1: Value callbacks + `functions.lua` — ✅ DONE

### Phase 2: Condition application (`combat:addCondition`) — ✅ DONE

`ConditionApplySpec` on `CombatExecuteRequest`; apply per target in `aoe.rs`
after damage. Shared mapper `active_condition_from_apply_spec`.

### Phase 3: Direct-condition helpers — ✅ DONE (MonsterType residual)

Full builder mapping; `combat:getTargets`; `Creature(id)`; `isPlayer`;
`setInFight`; `setOutfit` stores lookType. **Still need `MonsterType:getOutfit`
for illusion/chameleon scripts to supply lookType.**

### Phase 4: `COMBAT_PARAM_DISPEL` — ✅ DONE

Stored on `CombatDef`; applied after damage so heal+dispel both run.

### Phase 4b: Condition client updates — ✅ DONE

TFS `Condition*::start/endCondition` + `Player::onAdd/onEndCondition`:
icons (`0xA2`), speed announce, internal light vs items max, invis empty outfit,
outfit condition broadcast. Progressive light dim during duration still out of scope.

### Phase 5: Conjure path verification — ✅ DONE

**Goal:** 33 `conjureItem` scripts work via Lua.

Landed: `getMana` / `addMana` / `addManaSpent`, `transform`, `hasAttribute`,
`ItemType:getCharges`, `Group:hasFlag`, `PlayerFlag_HasInfiniteMana`,
`ITEM_ATTRIBUTE_*`. `item:decay` remains a logged no-op.

### Phase 6: Event callbacks — OPEN

**Goal:** `cancel_invisibility` + `challenge` TARGETCREATURE callbacks run.

Depends on Gap 6 globals (`doChallengeCreature`, world type, etc.).

### Phase 7: Diagonal area overlay — OPEN

Process `extArea`; pick orientation from caster direction at execute.

### Phase 8: Remaining `COMBAT_PARAM_*` — OPEN

Wire `CREATEITEM`, `NODAMAGE`, `DISTANCEEFFECT` through request → `aoe.rs`.

---

## Priority Order

1. **Phase 8** — CREATEITEM / NODAMAGE / DISTANCEEFFECT
2. **Phase 6** — event callbacks
3. **Phase 7** — diagonal wall areas
4. **Gaps 5–6** — summons + utilities (+ `MonsterType` for illusion)
5. **Gap 4** — houses (separate milestone)

---

## Summary Table (2026-07-18)

| Status | Count | Category |
|--------|-------|----------|
| ✅ Value callbacks + damage/heal range | 22 | Phase 1 **done** |
| ✅ `combat:addCondition` applied | 8 | Phase 2 **done** |
| ✅ Direct `addCondition` / helpers (MonsterType residual) | 5 | Phase 3 **done** |
| ✅ Dispel applied | 9 | Phase 4 **done** |
| ✅ Condition client notifies (icons/light/invis) | buffs | Phase 4b **done** |
| ✅ `conjureItem` helpers (`getMana`, transform, charges, …) | 33 | Phase 5 **done** |
| ✅ `addItem` food; mostly wired | 1 | Phase 5 footnote |
| ❌ Event callback not invoked | 2 | Phase 6 |
| ⚠️ Diagonal area partial | 3 | Phase 7 |
| ❌ `CREATEITEM` not created | 11 | Phase 8 |
| ❌ House API missing | 4 | Gap 4 |
| ❌ Summon API missing | 3 | Gap 5 |
| ❌ Utility API missing | 7+ | Gap 6 |
| **Unique scripts** | **70** | Categories overlap |

**Overlap examples:**
- Healing instants/runes — Phase 1 ✅ damage + Phase 4 ✅ DISPEL
- `soulfire_rune` — Phase 5 conjure + Phase 3 ✅ condition + Phase 8 NODAMAGE
- `wild_growth` — Gap 6 Tile/PZ + Phase 8 CREATEITEM
- Wall runes — Phase 5 conjure + Phase 7 diagonal + Phase 8 CREATEITEM

**Rough end-to-end readiness:**
- Damage/heal + buffs + dispel + conjure helpers: landed
- Full pack parity: still Phases 6–8 + Gaps 4–6

---

## Correction log

### 2026-07-18 Phase 5 — conjure helpers
- `getMana` / `addMana` / `addManaSpent` on `CreatureRef` (ScriptContext + LuaMutation).
- `ItemType:getCharges`, `item:hasAttribute`, `item:transform` (in-place + cylinder notify).
- Unblockers: `Group:hasFlag`, `PlayerFlag_HasInfiniteMana`, `ITEM_ATTRIBUTE_*`.
- `item:decay` still logged no-op.

### 2026-07-18 Phase 4b — condition client updates
- `Player::internal_light`; `player_creature_light` = max(internal, items).
- `on_condition_started` / `on_condition_ended` (icons, speed, light, invis, outfit).
- Wired from AoE apply/dispel, Lua add/remove/setInFight, ProcessSkills expiry.

### 2026-07-18 Phases 2–4 landed
- Shared `ConditionApplySpec` + `active_condition_from_apply_spec` mapper.
- `CombatExecuteRequest` carries `conditions` + `dispel_type`; `aoe.rs` applies
  after damage (heal+dispel both run — not exclusive early-return).
- Direct `addCondition` passes full spec; `getTargets`, `Creature(id)`,
  `isPlayer`, `setInFight`; `setOutfit` stores lookType.
- ProcessSkills ticks Light/Invisible/ManaShield/Infight via timer_rounds.

### 2026-07-18 re-audit
- Marked **Phase 1 done** (value callbacks, `functions.lua`, `Player:` bridge,
  `getMagicLevel`, SKILL weapon params).
- Clarified Phase 5: load/bridge done; `getMana` / transform / charges still missing.

### Earlier (July 2026)
- Counted all 70 scripts; documented missing `wild_growth.lua`.
- Fixed Gap 7 area constant names; reclassified poison_storm / envenom / soulfire
  as direct `onCastSpell` conditions (not TARGETCREATURE).
- DISPEL = 9; CREATEITEM = 11; `conjureItem` lives in `functions.lua`.
- Removed contradictory “~10 formula-based” row (`setFormula` unused).
