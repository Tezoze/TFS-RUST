# PC-3a Spell System — Remaining Gaps

Updated: 2026-07-18 (Gaps 5–6 landed)
Scope: `data/scripts/spells/**/*.lua` (70 spell scripts, excluding `areas.lua`)

## Current State

All 70 spells **load** and **register** correctly. All 70 spells **cast** —
`player_say_spell` dispatches vocation/level/mana/soul gates, exhaustion, PZ
check, mana deduction, and fires the `onCastSpell` Lua callback (with
`VARIANT_STRING` when `hasParams`).

**Phases 1–8 + Gaps 5–6 are done** (except Gap 4 houses).

Remaining: **Gap 4 houses** (separate milestone).

**What already works:**
- Full combat/condition/conjure surface from Phases 1–8
- **`Game.createMonster`** + `addSummon` / `getSummons` / master link
- **`MonsterType:isSummonable` / `isConvinceable` / `getManaCost`**
- **Variant** `getString` / `getNumber` / `getPosition`; `Variant(pos)` ctor
- **Position** `.x/.y/.z`, ctor, `getNextPosition`, `moveUpstairs`
- **Tile** `hasFlag`, `getGround`, `getTopDownItem`, `getItems`, `getItemByType`
- **Utilities:** `sendTextMessage`, `getDirection`, `move`, `teleportTo`
- **Rune use-with** → `rune:{id}` `onCastSpell` + charge consume
- Globals: `ropeSpots`, `Fields`, `corpseIds`

---

## Gap 1–3, 7–8 — ✅ DONE (see correction log)

---

## Gap 4: House spell APIs (LOW) — out of PC-3a scope

**Affects:** 4 house management spells — separate milestone.

---

## Gap 5: `Game.createMonster` / summon APIs — ✅ DONE

**Affects:** summon creature, undead legion, animate dead.

---

## Gap 6: Utility spell APIs — ✅ DONE

find person, levitate, rope, destroy/desintegrate, convince, wild_growth helpers.

---

## Priority Order (remaining)

1. **Gap 4** — houses (separate milestone)

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
| ✅ Summon API | 3 | Gap 5 **done** |
| ✅ Utility API | rest | Gap 6 **done** |
| ❌ House API missing | 4 | Gap 4 |
| **Unique scripts** | **70** | Categories overlap |

---

## Correction log

### 2026-07-18 Gaps 5–6
- Cast param → `VARIANT_STRING`; `Variant` metatable methods; `Variant()` ctor.
- Position/Tile spatial surface; ScriptContext tile reads.
- `Game.createMonster` via `find_and_place_creature_tfs`; `addSummon` / summons list.
- Monster XML `summonable` / `convinceable` / `manacost`.
- Rune callbacks keyed `rune:{id}`; `player_use_item_ex` dispatches Lua cast.
- `ropeSpots` / `Fields` / `corpseIds` in `functions.lua`.

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
