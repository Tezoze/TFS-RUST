# Monster Spell Pipeline Audit (2026-07-16; updated 2026-07-29)

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
| `healing` | 55 | `Healing` | all 5 ✓ | non-aggressive; casts on self without target |
| `speed` | 56 | `Speed` | all 5 ✓ | non-aggressive when `percent >= 0` (self haste), aggressive when `percent < 0` (paralyze) |
| `poisoncondition` | 12 | `Condition { Poison }` | all 5 ✓ | |
| `firecondition` | 4 | `Condition { Fire }` | all 5 ✓ | |
| `energycondition` | 5 | `Condition { Energy }` | all 5 ✓ | |
| `drunk` | 9 | `Drunk` | all 5 ✓ | |
| `poison` | 25 | `Damage { Earth }` | all 5 ✓ | TFS `COMBAT_EARTHDAMAGE` / 772 `DAMAGE_POISON` |
| `manadrain` | 21 | `Damage { ManaDrain }` | all 5 ✓ | |
| `outfit` | 21 | `Outfit` | all 5 ✓ | `ConditionOutfit` with `monster=`/`item=` |
| `invisible` | 12 | `Invisible` | all 5 ✓ | `ConditionType::Invisible` |
| `firefield` | 10 | `Field { Fire }` | all 5 ✓ | places a fire field on the tile |
| `poisonfield` | 6 | `Field { Poison }` | all 5 ✓ | places a poison field on the tile |
| `energyfield` | 1 | `Field { Energy }` | all 5 ✓ | places an energy field on the tile |

**17 of 17** attack names fully functional.

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

## Working — Shoot Effects (`shooteffect`)

`parse_shoot_effect_name` maps all data-pack `shooteffect` values (fixed 2026-07-29).

| Shooteffect | Count | `ShootEffect` enum | Wire byte |
|---|---|---|---|
| `poison` / `earth` | — | `Poison = 15` | 15 |
| `poisonarrow` | — | `PoisonArrow = 6` | 6 |
| `fire` | — | `Fire = 3` | 3 |
| `energy` | — | `Energy = 4` | 4 |
| `death` | — | `Death = 8` | 8 |
| `spear` | — | `Spear = 7` | 7 |
| `bolt` | — | `Bolt = 1` | 1 |
| `arrow` | — | `Arrow = 0` | 0 |
| `burstarrow` | — | `BurstArrow = 5` | 5 |
| `throwingstar` | — | `ThrowingStar = 11` | 11 |
| `throwingknife` | 3 | `ThrowingKnife = 9` | 9 |
| `smallstone` | 2 | `SmallStone = 10` | 10 |
| `largerock` / `rock` | 3 | `LargeRock = 12` | 12 |
| `snowball` | — | `Snowball = 13` | 13 |
| `powerbolt` | — | `PowerBolt = 2` | 2 |

**14 of 14** shoot effects functional ✓

## Working — Impact Application

`monster_idle_apply_spell_impact` dispatches all `SpellImpact` variants (completed 2026-07-29).

| Impact | Status | 772 reference |
|---|---|---|
| `SpellImpact::Damage` | ✓ working | `IMPACT_DAMAGE` — `ComputeDamage` + `TDamageImpact` |
| `SpellImpact::Healing` | ✓ working | `IMPACT_HEALING` — `THealingImpact` |
| `SpellImpact::Speed` | ✓ working | `IMPACT_SPEED` — `TSpeedImpact` |
| `SpellImpact::Condition` | ✓ working | `IMPACT_DAMAGE` with periodic damage type |
| `SpellImpact::Drunk` | ✓ working | `IMPACT_DRUNKEN` — `TDrunkenImpact` |
| `SpellImpact::Field` | ✓ working | `IMPACT_FIELD` — `TFieldImpact` places field item on tile |
| `SpellImpact::Summon` | ✓ working | `IMPACT_SUMMON` — `TSummonImpact` + `SearchSummonField` / `CreateMonster` |
| `SpellImpact::Outfit` | ✓ working | `IMPACT_OUTFIT` / TFS `CONDITION_OUTFIT` — changes target outfit |
| `SpellImpact::Invisible` | ✓ working | TFS `CONDITION_INVISIBLE` — applies invisible condition |

**9 of 9** impact variants fully implemented.

## Working — Defense Spells Cast Without Target (2026-07-18; updated 2026-07-29)

### 772 behavior

772 `crnonpl.cc:2682` CASTING block:
```cpp
if(!Impact->isAggressive() || (this->Target != 0 && this->Target != this->Master)){
    switch(SpellData->Shape){ ... }  // cast the spell
}
```

Non-aggressive spells fire **regardless of target**. All spells (attack + defense) are in one `RaceData.Spells` list. The Rust `SpellImpact::is_aggressive` mirrors this gate:
- `Healing` → non-aggressive.
- `Outfit` / `Invisible` → non-aggressive (TFS `aggressive="0"`).
- `Speed` → non-aggressive when `percent >= 0` (self haste), aggressive when `percent < 0` (paralyze target).
- All other impacts → aggressive.

### Affected defense spell types

| Defense name | Count | Impact | Status |
|---|---|---|---|
| `healing` | 55 | `Healing` (non-aggressive) | ✓ casts on self without target |
| `speed` | 56 | `Speed` | ✓ self-casts when `percent >= 0`; targets enemy when `percent < 0` |
| `invisible` | 12 | `Invisible` (non-aggressive) | ✓ casts on self without target |
| `outfit` | 21 | `Outfit` (non-aggressive) | ✓ casts on self without target |

## Summary

| Category | Working | Total | % |
|---|---|---|---|
| Attack names | 17 | 17 | 100% |
| Area effects | 20 | 20 | 100% |
| Shapes | 5 | 5 | 100% |
| Shoot effects | 14 | 14 | 100% |
| SpellImpact variants | 9 | 9 | 100% |
| Defense cast without target | 4 unconditional / 1 conditional | 4* | 100% |

* `speed` self-cast depends on the `percent` sign; `healing`, `invisible`, and `outfit` are unconditionally non-aggressive.

## Remaining Work

No outstanding gaps identified by this audit. All previously listed missing attack names, shoot effects, impact variants, and defense-cast-without-target cases are implemented.

## Recent Fixes

- **2026-07-29** — `parse_spell_impact` now maps `poison`, `manadrain`, `outfit`, `invisible`/`invisibility`, and the `firefield`/`poisonfield`/`energyfield` trio; `parse_shoot_effect_name` now maps `throwingknife`, `smallstone`, `largerock`/`rock`; `SpellImpact::Outfit` and `SpellImpact::Invisible` variants were added and are dispatched by `monster_idle_apply_spell_impact`; `SpellImpact::is_aggressive` treats `Outfit`/`Invisible` as non-aggressive and `Speed` as non-aggressive when `percent >= 0`.
- **2026-07-18** — `SpellImpact::is_aggressive` added (`Healing` only); `monster_idle_try_casting` restructured to cast all non-aggressive spells without a target; defense spells merged into the cast loop.
- **2026-07-16** — Shape detection (`length`+`spread` → `Angle`; `target`+`radius` → `Destination`); `Angle` cone by caster facing; `Destination`/`Origin` circles from `circles.dat`; `parse_area_effect_name` and per-tile area-effect broadcast; damage text + health bar for `Damage`/`Healing` impacts.

See `tasks/lessons.md` lesson 176 for details.
