# Monster Spell Pipeline Audit (2026-07-16)

Full audit of the monster spell casting pipeline (`crnonpl.cc:2521-2667` CASTING block) vs the TFS data pack (`data/monster/monsters/*.xml`) and 772 reference (`tibia-game-master/src/`).

## Reference Sources

| Source | Path | Role |
|---|---|---|
| 772 outcomes | `reference/cipsoft-772/tibia-game-master/src/crnonpl.cc:2521-2667` | CASTING block — iterates all spells, delay/flee gates, shape dispatch |
| 772 shape spells | `reference/cipsoft-772/tibia-game-master/src/magic.cc:400-588` | `ActorShapeSpell`, `VictimShapeSpell`, `OriginShapeSpell`, `DestinationShapeSpell`, `AngleShapeSpell` |
| 772 impacts | `reference/cipsoft-772/tibia-game-master/src/enums.hh:631-638` | `IMPACT_DAMAGE..IMPACT_SUMMON` (8 variants) |
| 772 damage types | `reference/cipsoft-772/tibia-game-master/src/enums.hh:147-157` | `DAMAGE_PHYSICAL..DAMAGE_MANADRAIN` |
| 772 circles | `reference/cipsoft-772/runtime/dat/circles.dat` | 21×21 grid, rings 0–7, 101 tiles |
| 772 monster data | `reference/cipsoft-772/runtime/mon/*.mon` | `Spells = { Shape (params) -> Impact (params) : Delay }` |
| TFS data pack | `data/monster/monsters/*.xml` | `<attack name="..." delay="..." .../>` + `<defense>` |
| TFS name mapping | `src/monsters.cpp:191-260` | `length`+`spread` → `AreaCombat`; `target` → `needTarget` |
| TFS effect names | `src/tools.cpp:497-560` | `magicEffectNames` (areaeffect → `CONST_ME_*`) |
| TFS shoot names | `src/tools.cpp` `shootTypeNames` | shooteffect → `CONST_ANI_*` |
| 772 wire bytes | `reference/tvp-772/gameserver/src/const.h:11-35` | `CONST_ME_*` = 1..25 (772 client range) |

## Rust Implementation

| File | Role |
|---|---|
| `crates/tfs-rust-core/src/creature/monster_combat.rs` | `MonsterSpell` struct, `parse_spell_impact`, `default_shape_for_node`, `parse_shoot_effect_name`, `parse_area_effect_name` |
| `crates/tfs-rust-core/src/idle_stimulus.rs` | `monster_idle_try_casting` (CASTING block), `monster_idle_spell_tiles` (shape→tiles), `monster_idle_apply_spell_impact` (impact dispatch) |
| `crates/tfs-rust-core/src/combat/circles.rs` | `disc_offsets` — 772 `circles.dat` ring offsets (verified vs 772 + 1098) |

## Working — Attack Spells (CASTING)

These attack names are parsed, shaped, and impact-applied correctly.

| Attack name | Data-pack count | `SpellImpact` variant | Shapes | Notes |
|---|---|---|---|---|
| `fire` | 34 | `Damage { Fire }` | all 5 ✓ | dragon fire wave + fireball |
| `energy` | 20 | `Damage { Energy }` | all 5 ✓ | |
| `lifedrain` | 49 | `Damage { LifeDrain }` | all 5 ✓ | |
| `physical` | 44 | `Damage { Physical }` | all 5 ✓ | |
| `healing` | 55 | `Healing` | all 5 ✓ | **but defense-cast-without-target broken** (see below) |
| `speed` | 56 | `Speed` | all 5 ✓ | **but defense-cast-without-target broken** |
| `poisoncondition` | 12 | `Condition { Poison }` | all 5 ✓ | |
| `firecondition` | 4 | `Condition { Fire }` | all 5 ✓ | |
| `energycondition` | 5 | `Condition { Energy }` | all 5 ✓ | |
| `drunk` | 9 | `Drunk` | all 5 ✓ | |

**10 of 17** attack names fully functional.

## Broken — Attack Names Not Mapped in `parse_spell_impact`

These XML `<attack name="...">` values fall through to the `debug!("skipping unknown")` branch and are silently dropped at parse time.

| Attack name | Data-pack count | 772 equivalent | Gap |
|---|---|---|---|
| `poison` | 25 | `IMPACT_DAMAGE` `DAMAGE_POISON` (0x0002) | no match arm in `parse_spell_impact` |
| `manadrain` | 21 | `IMPACT_DAMAGE` `DAMAGE_MANADRAIN` (0x0200) | no match arm; `CombatType::ManaDrain` exists |
| `outfit` | 21 | `IMPACT_OUTFIT` | no `SpellImpact::Outfit` variant exists |
| `invisible` | 12 | TFS condition (`CONDITION_INVISIBLE`), not a 772 impact | no match arm; needs condition application |
| `firefield` | 10 | `IMPACT_FIELD` | not mapped + `SpellImpact::Field` is a stub |
| `poisonfield` | 6 | `IMPACT_FIELD` | not mapped + stub |
| `energyfield` | 1 | `IMPACT_FIELD` | not mapped + stub |

**7 of 17** attack names broken.

## Working — Area Effects (`areaeffect`)

All 20 data-pack `areaeffect` values are now mapped to 772 `CONST_ME_*` wire bytes in `parse_area_effect_name` (fixed 2026-07-16).

| areaeffect | Count | Wire byte | CONST_ME |
|---|---|---|---|
| `blueshimmer` | 74 | 13 | `MAGIC_BLUE` |
| `redshimmer` | 67 | 14 | `MAGIC_RED` |
| `poison` | 28 | 21 | `POISONAREA` |
| `firearea` | 23 | 7 | `FIREAREA` |
| `energy` | 15 | 12 | `ENERGYHIT` |
| `poff` | 13 | 3 | `POFF` |
| `teleport` | 8 | 11 | `TELEPORT` |
| `greenbubble` | 8 | 9 | `GREEN_RINGS` |
| `fire` | 8 | 16 | `HITBYFIRE` |
| `mortarea` | 7 | 18 | `MORTAREA` |
| `greenspark` | 7 | 17 | `HITBYPOISON` |
| `bluebubble` | 7 | 2 | `LOSEENERGY` |
| `rednote` | 4 | 20 | `SOUND_RED` |
| `greenshimmer` | 4 | 15 | `MAGIC_GREEN` |
| `explosionarea` | 4 | 5 | `EXPLOSIONAREA` |
| `explosion` | 3 | 6 | `EXPLOSIONHIT` |
| `redspark` | 2 | 1 | `DRAWBLOOD` |
| `blackspark` | 2 | 10 | `HITAREA` |
| `yellowspark` | 1 | 4 | `BLOCKHIT` |
| `yellowbubble` | 1 | 8 | `YELLOW_RINGS` |

**All 20** area effects functional ✓

## Working — Shapes

All 5 `SpellShape` variants are correctly handled in `monster_idle_spell_tiles` + cast dispatch (fixed 2026-07-16).

| Shape | 772 function | Tile generation | Caster facing | Area effect broadcast |
|---|---|---|---|---|
| `Actor` | `ActorShapeSpell` (`magic.cc:400`) | `[caster_pos]` | n/a | on caster tile |
| `Victim` | `VictimShapeSpell` (`magic.cc:416`) | `[target_pos]` | faces target | on target tile |
| `Origin` | `OriginShapeSpell` (`magic.cc:503`) | `disc_offsets(radius)` around caster | n/a | per tile |
| `Destination` | `DestinationShapeSpell` → `CircleShapeSpell` (`magic.cc:537,522`) | `disc_offsets(radius)` around target | faces target | per tile |
| `Angle` | `AngleShapeSpell` (`magic.cc:550`) | forward cone by **caster direction** (`length`→Range, `spread*10`→Angle) | `Rotate(Target)` first | per tile |

**All 5** shapes functional ✓

## Broken — Shoot Effects Not Mapped

`parse_shoot_effect_name` is missing 3 data-pack values (enum variants exist).

| Shooteffect | Count | `ShootEffect` enum | Wire byte |
|---|---|---|---|
| `throwingknife` | 3 | `ThrowingKnife = 9` ✓ | 9 |
| `largerock` | 3 | `LargeRock = 12` ✓ | 12 |
| `smallstone` | 2 | `SmallStone = 10` ✓ | 10 |

**11 of 14** shoot effects functional.

## Broken — Impact Application Stubs

`monster_idle_apply_spell_impact` has stubs / missing variants.

| Impact | Status | 772 reference |
|---|---|---|
| `SpellImpact::Damage` | ✓ working | `IMPACT_DAMAGE` — `ComputeDamage` + `TDamageImpact` |
| `SpellImpact::Healing` | ✓ working | `IMPACT_HEALING` — `THealingImpact` |
| `SpellImpact::Speed` | ✓ working | `IMPACT_SPEED` — `TSpeedImpact` |
| `SpellImpact::Condition` | ✓ working | `IMPACT_DAMAGE` with periodic damage type |
| `SpellImpact::Drunk` | ✓ working | `IMPACT_DRUNKEN` — `TDrunkenImpact` |
| `SpellImpact::Field` | **stub** — "not yet placed on map" | `IMPACT_FIELD` — `TFieldImpact` places field item on tile |
| `SpellImpact::Summon` | ✓ CASTING + `handleField` | `IMPACT_SUMMON` — `TSummonImpact` + `SearchSummonField` / `CreateMonster` |
| `SpellImpact::Outfit` | **does not exist** | `IMPACT_OUTFIT` — `TOutfitImpact` changes target outfit |

**5 of 8** impacts fully implemented.

## Fixed — Defense Spells Cast Without Target (2026-07-18)

### 772 behavior

772 `crnonpl.cc:2682` CASTING block:
```cpp
if(!Impact->isAggressive() || (this->Target != 0 && this->Target != this->Master)){
    switch(SpellData->Shape){ ... }  // cast the spell
}
```

Non-aggressive spells (healing) fire **regardless of target** — the `!isAggressive()` branch passes the gate with no target. All spells (attack + defense) are in one `RaceData.Spells` list. Only `THealingImpact` overrides `isAggressive` to `false` (`magic.cc:210`); all others inherit `true` (`magic.hh:16-33`).

### Fix

- Added `SpellImpact::is_aggressive()` — returns `false` only for `Healing`, `true` for all others (matches 772 `TImpact::isAggressive` / `THealingImpact::isAggressive`).
- Restructured `monster_idle_try_casting` to not early-return without target. The CASTING loop now iterates ALL spells (attack + defense), consuming delay + flee rolls for every spell. Non-aggressive spells with self-centered shapes (`Actor`/`Origin`/`Angle`) cast on self even without target; aggressive spells skip. Aggressive spells also skip when target == master (summons attacking their master).
- Defense spells are now loaded as full `MonsterSpell` structs and merged into the cast loop (was delay-moduli-only).

### Affected defense spell types

| Defense name | Count | Impact | Status |
|---|---|---|---|
| `healing` | 55 | `Healing` (non-aggressive) | ✓ casts on self without target |
| `speed` | 56 | `Speed` (aggressive in 772) | requires target (772 `TSpeedImpact` inherits `isAggressive=true`) |
| `invisible` | 12 | (needs condition) | not mapped at all |
| `outfit` | 21 | (needs `Outfit` impact) | not mapped at all |

## Summary

| Category | Working | Total | % |
|---|---|---|---|
| Attack names | 10 | 17 | 59% |
| Area effects | 20 | 20 | 100% |
| Shapes | 5 | 5 | 100% |
| Shoot effects | 11 | 14 | 79% |
| SpellImpact variants | 5 | 8 | 63% |
| Defense cast without target | 0 | 4 types | 0% |

## Fix Priority

### Quick wins (match arms only)
1. `poison` → `SpellImpact::Damage { Poison }` (25 monsters)
2. `manadrain` → `SpellImpact::Damage { ManaDrain }` (21 monsters)
3. `throwingknife`, `largerock`, `smallstone` shoot effects (8 monsters)

### Medium (new impact variants / condition application)
4. `invisible` → apply `ConditionType::Invisible` condition (12 monsters)
5. `outfit` → new `SpellImpact::Outfit` variant + application (21 monsters)
6. Defense cast-without-target path (all defense spells)

### Large (field placement / summon spawning)
7. `firefield`, `poisonfield`, `energyfield` → `SpellImpact::Field` real implementation (17 monsters)
8. `SpellImpact::Summon` — **done** (XML `<summons>` → CASTING Origin r=0; ToDo-driven IdleStimulus)

## Recent Fixes (2026-07-16)

- **Shape detection**: `length`+`spread` → `Angle`; `target`+`radius` → `Destination` (was `Actor`/`Victim`)
- **Angle cone**: 772 `AngleShapeSpell` by caster facing direction (`spread*10`→Angle, `length`→Range)
- **Destination/Origin circle**: `disc_offsets` from `circles.dat` (was Chebyshev square)
- **Area effects**: `parse_area_effect_name` implemented (was stub returning `None`)
- **Area effect broadcast**: per-tile for Origin/Angle/Destination, on target for Victim
- **Damage text + health bar**: `monster_idle_apply_spell_impact` now calls `notify_player_combat_damage` after `Damage` impacts (animated text + health bar + status message), and `notify_creature_healed` after `Healing` impacts (health bar + stats only, no animated text — matches 772 `THealingImpact::handleCreature`)

See `tasks/lessons.md` lesson 176 for details.
