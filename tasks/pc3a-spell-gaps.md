# PC-3a Spell System — Remaining Gaps

Updated: 2026-07-18 (Phases 6–8 + MonsterType landed)
Scope: `data/scripts/spells/**/*.lua` (70 spell scripts, excluding `areas.lua`)

## Current State

All 70 spells **load** and **register** correctly. All 70 spells **cast** —
`player_say_spell` dispatches vocation/level/mana/soul gates, exhaustion, PZ
check, mana deduction, and fires the `onCastSpell` Lua callback.

**Phases 1–8 + 4b are done** (except houses / summons / heavy utilities).

Remaining gaps: **Gap 4 houses**, **Gap 5 summons**, **heavy Gap 6 utilities**
(find person, levitate/rope, destroy/desintegrate, convince, wild_growth
movement helpers beyond CREATEITEM).

**What already works:**
- `Combat()` constructor, `:setParameter`, `:setArea`, `:setFormula`, `:setCallback`
- `combat:execute()` — area offsets, LoS, damage/heal roll, magic effects, damage text
- **Value callbacks** — `CALLBACK_PARAM_LEVELMAGICVALUE` / `SKILLVALUE` invoked at
  execute; `(min, max)` used as damage range
- **Event callbacks** — `TARGETCREATURE` / `TARGETTILE` invoked after execute
- `data/scripts/functions.lua` loaded before spell scripts (after `areas.lua`)
- `Player:` Lua method bridge via `CreatureRef` `__index` fallback
- `getMagicLevel`, `getLevel`; SKILL path uses `get_player_weapon_combat_params`
- **`combat:addCondition`** — stored + applied per target in `aoe.rs` (Phase 2)
- **`COMBAT_PARAM_DISPEL`** — stored + removes matching conditions after damage (Phase 4)
- **`COMBAT_PARAM_CREATEITEM` / `NODAMAGE` / `DISTANCEEFFECT`** — Phase 8
- **`ConditionBuilder` → `ActiveCondition`** — Light/Speed/Damage/Outfit/Generic/
  Regeneration + cycle→timer_rounds (Phase 3 mapper)
- `CreatureRef:addCondition` (full spec) / `:removeCondition` / `:isPlayer` /
  `:setInFight` / `:addItem` / inventory helpers
- `getMana` / `addMana` / `addManaSpent`; `item:transform` / `hasAttribute`;
  `ItemType:getCharges`; `Group:hasFlag`
- `combat:getTargets(creature, variant)` — area creature list
- `Creature(id)` constructor; `MonsterType(name):getOutfit()` / `isIllusionable()`
- `Tile(pos):hasProperty(CONST_PROP_BLOCKSOLID)`; `Game.getWorldType()`;
  `doChallengeCreature`; diagonal `extArea`
- Constants + `SpellBuilder` methods registered

**Note:** No spell in this pack uses `:setFormula()` — all damage/healing ranges
come from value callbacks (or hard-coded condition cycles).

---

## Gap 1: `setCallback` Lua function invocation

### Value callbacks — ✅ DONE (Phase 1)
### Event callbacks — ✅ DONE (Phase 6)

| Spell | Status |
|-------|--------|
| `support/cancel_invisibility.lua` | ✅ TARGETCREATURE + `Game.getWorldType` |
| `support/challenge.lua` | ✅ TARGETCREATURE + `doChallengeCreature` |

| Constant | Value | Type | Status |
|----------|-------|------|--------|
| `CALLBACK_PARAM_LEVELMAGICVALUE` | 0 | Value | ✅ Invoked |
| `CALLBACK_PARAM_SKILLVALUE` | 1 | Value | ✅ Invoked |
| `CALLBACK_PARAM_TARGETTILE` | 2 | Event | ✅ Invoked |
| `CALLBACK_PARAM_TARGETCREATURE` | 3 | Event | ✅ Invoked |

---

## Gap 2: Condition application — ✅ DONE (Phases 2–4 + 4b)

**Remaining for illusion/chameleon:** ✅ `MonsterType:getOutfit()` landed.

---

## Gap 3: `conjureItem` via `functions.lua` — ✅ DONE (Phase 5)

`item:decay` remains a logged no-op.

---

## Gap 4: House spell APIs (LOW) — out of PC-3a scope

**Affects:** 4 house management spells — separate milestone.

---

## Gap 5: `Game.createMonster` / summon APIs (LOW) — OPEN

**Affects:** 3 spells — summon creature, undead legion, animate dead.

---

## Gap 6: Utility spell APIs (LOW) — PARTIAL

| Piece | Status |
|-------|--------|
| `Game.getWorldType` / `doChallengeCreature` | ✅ (Phase 6) |
| `MonsterType` / illusion | ✅ |
| `Tile:hasProperty(BLOCKSOLID)` | ✅ (minimal, Phase 8) |
| find person / levitate / rope / destroy / convince / wild_growth helpers | ❌ OPEN |

---

## Gap 7: `setArea` / `createCombatArea` — ✅ DONE (Phase 7)

`extArea` diagonal overlay stored + oriented at execute/`getTargets`
(NW=raw, NE=mirror, SW=flip, SE=transpose).

---

## Gap 8: `COMBAT_PARAM_*` — ✅ CREATEITEM / NODAMAGE / DISTANCEEFFECT

| Parameter | Status | Notes |
|-----------|--------|-------|
| `COMBAT_PARAM_TYPE` | ✅ | |
| `COMBAT_PARAM_EFFECT` | ✅ | |
| `COMBAT_PARAM_AGGRESSIVE` | ✅ | |
| `COMBAT_PARAM_BLOCKARMOR` | ✅ | |
| `COMBAT_PARAM_BLOCKSHIELD` | ⚠️ Stored | Not applied |
| `COMBAT_PARAM_DISTANCEEFFECT` | ✅ | `broadcast_distance_shoot` |
| `COMBAT_PARAM_CREATEITEM` | ✅ | `combatTileEffects` remap + place |
| `COMBAT_PARAM_NODAMAGE` | ✅ | Skip damage arm |
| `COMBAT_PARAM_DISPEL` | ✅ | |
| `COMBAT_PARAM_USECHARGES` | ❌ Ignored | |
| `COMBAT_PARAM_TARGETCASTERORTOPMOST` | ❌ Ignored | |
| `COMBAT_PARAM_FORCEONTARGETEVENT` | ❌ Ignored | |

---

## Implementation Plan

### Phases 1–5 + 4b — ✅ DONE
### Phase 6: Event callbacks — ✅ DONE
### Phase 7: Diagonal area overlay — ✅ DONE
### Phase 8: CREATEITEM / NODAMAGE / DISTANCEEFFECT — ✅ DONE

---

## Priority Order (remaining)

1. **Gaps 5–6** — summons + remaining utilities
2. **Gap 4** — houses (separate milestone)

---

## Summary Table (2026-07-18)

| Status | Count | Category |
|--------|-------|----------|
| ✅ Value callbacks + damage/heal range | 22 | Phase 1 **done** |
| ✅ `combat:addCondition` applied | 8 | Phase 2 **done** |
| ✅ Direct `addCondition` + MonsterType | 5 | Phase 3 **done** |
| ✅ Dispel applied | 9 | Phase 4 **done** |
| ✅ Condition client notifies | buffs | Phase 4b **done** |
| ✅ `conjureItem` helpers | 33 | Phase 5 **done** |
| ✅ Event callbacks | 2 | Phase 6 **done** |
| ✅ Diagonal area | 3 | Phase 7 **done** |
| ✅ `CREATEITEM` / NODAMAGE / DISTANCE | 11+ | Phase 8 **done** |
| ❌ House API missing | 4 | Gap 4 |
| ❌ Summon API missing | 3 | Gap 5 |
| ⚠️ Utility API partial | rest | Gap 6 |
| **Unique scripts** | **70** | Categories overlap |

---

## Correction log

### 2026-07-18 Phases 6–8 + MonsterType
- Phase 8: `create_item` / `no_damage` / `distance_effect` on `CombatExecuteRequest`
  → `aoe.rs` (skip damage, distance shoot, place items + PvP remap).
- Minimal `Tile(pos):hasProperty(BLOCKSOLID)` for field rune gates.
- Phase 7: `extArea` on `AreaCombat` + diagonal orientation at execute.
- Phase 6: TARGETCREATURE/TILE invoke; `Game.getWorldType`; `doChallengeCreature`
  + `monster_challenge_creature`.
- `MonsterType(name):getOutfit` / `isIllusionable`; parse `illusionable` /
  `challengeable` flags.

### 2026-07-18 Phase 5 — conjure helpers
- `getMana` / `addMana` / `addManaSpent`; transform / charges / Group:hasFlag.
- `item:decay` still logged no-op.

### 2026-07-18 Phase 4b — condition client updates
- icons / speed / light / invis / outfit on start+end.

### 2026-07-18 Phases 2–4 landed
- Shared `ConditionApplySpec` + mapper; DISPEL after damage.
