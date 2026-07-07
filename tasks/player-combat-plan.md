# Player Combat System — Implementation Plan (772 mechanics, TVP data shape)

**Goal.** Wire the player weapon-combat *strike* into the unified ToDo engine so that
**formulas and flow match the 7.72 decompile** (`tibia-game-master/src/`), while all
**tunable data lives in TVP-shaped files** (vocation combat block in `data/defs/vocations.lua`
per PC-0; weapon attack/defense/armor from `items.otb`/`items.xml`). No reference source is
transcribed — we replicate observable outcomes in idiomatic Rust. Era knobs stay in
`MechanicsProfile` / `data/formulas/772.lua`; per-vocation balance stays in
`data/defs/vocations.lua`.

**Active target:** `clientVersion = 772` (`config.lua:17`). 1098 shares the same code paths
through `MechanicsProfile`; this plan does not add version branches to core. Architecture
reference for the Tier-1/Tier-2 profile split: `tfs-mechanics-profile.md` (steering) +
`formulas.rs` module doc comment.

---

## Phase status summary

| Phase | Description | Status |
|-------|-------------|--------|
| PM | Player module consolidation (file moves) | ✅ Done |
| PC-0 | Vocation data migration (XML→Lua) | ✅ Done |
| PC-1 | Player attack/defend/armor value resolution | ✅ Done |
| PC-2 | The strike (`CloseAttack`) — melee first | ✅ Done (core logic + feedback effects) |
| PC-2a | `Damage` path completeness (melee audit) | ✅ Done (8 findings — §9.2) |
| PC-2b | Lua combat/spell/weapon plumbing | ✅ Done (Combat/Spell/Weapon userdata + createCombatArea + enums + spellword dispatch) |
| PC-3 | Distance + wand strikes | 🔲 Pending (+ mana shield, typed immunities; depends on PC-2b) |
| PC-4 | Fight/chase/secure mode + PVP gating | 🔲 Pending (+ invulnerability check) |
| PC-5 | Skill/exp gain + regen + death penalty | 🔲 Pending (+ shield skill learning, death penalty, level-up HP/mana gain fix M13, skill-tries data gaps: skill_base/min_level/magic-dispatch) |

---

## 1. Current state

### 1.1 Already implemented and correct
| Piece | Location | Notes |
|-------|----------|-------|
| Combat math (probe, defense, armor, spell, exp, skill tries, condition ticks) | `combat/math.rs` | Pure fns over `MechanicsProfile` + `FormulaHooks`; unit-tested; matches `crskill.cc`/`crcombat.cc` outcomes. |
| Damage application (HP/mana/conditions/dispel, damage_map) | `combat/mod.rs`, `idle_stimulus.rs::combat_execute_with_stimulus` | Death + `DamageStimulus` wired for both monster and player paths. |
| Attack targeting / chase routing | `player/combat/mod.rs` | Routes attack/follow/cancel packets; strike dispatched to `strike.rs` for `Adjacent`. |
| Monster melee strike (attack roll, defense gate, armor, poison-on-hit) | `creature/monster_combat.rs`, `monster_ai.rs` | Mirrors `CloseAttack`; uses world-aware `melee_defense_snapshot_for` for player targets. |
| Player melee strike (`CloseAttack`) | `player/combat/strike.rs` | `weapon_damage` → `roll_target_defense` → `armor_reduction` → `combat_execute_with_stimulus` → `ActivateLearning` → weapon wearout → `StopAttack` on death. |
| Combat feedback (damage text, health bar, poff/spark/blood) | `game_world_spectators.rs`, `monster_ai.rs`, `player/combat/strike.rs` | `notify_player_combat_damage` handles all creature types; poff (3) / spark (4) emitted at strike call sites; race-keyed blood via `apply_physical_hit_blood`. |
| Player weapon accessors | `player/inventory/util.rs` | `getWeapon`/`getWeaponType`/`getWeaponSkill` — slot resolution + ammo pairing. |
| Player attack/defend/armor value resolution | `player/combat/values.rs` | `player_get_attack_value` / `player_get_defend_value` / `player_get_armor_strength` returning raw unscaled values. |
| Condition ticks + fed regen tick loop | `process_skills.rs` | `VocationRegistry::fed_regen_params` wired (PC-0); hardcoded fallback only for empty-registry test worlds. Full regen-from-vocation-data is PC-5. |
| Item weapon attributes | `tfs-rust-content/src/otb.rs` `ItemType` | `weapon_type`, `attack`, `defense`, `extra_defense`, `armor`, `ammo_type`, `attack_speed`, `shoot_range`, `hit_chance`. |
| Fight-mode enum + modifiers | `combat/math.rs` `FightMode`, `formulas.rs` `FightModes` | 772 `+20/−40` atk, `−40/+80` def; era-tunable via `772.lua`. |
| Vocation data (full TVP combat block) | `data/defs/vocations.lua`, `tfs-rust-content/src/vocations.rs` | `VocationDef` + `VocationRegistry` + `VocationProfile` snapshot on `Player` (PC-0). |

### 1.2 What's still missing
- **Player distance/wand strikes** (PC-3) — `DistanceAttack`/`WandAttack` not yet wired. PC-2b's `Weapon(WEAPON_WAND)` Lua loading is now available; PC-3 consumes the `WeaponRegistry`/`WandDef` it produces.
- **Fight/chase/secure mode storage** (PC-4) — `raw_fight_mode`/`raw_secure_mode` parsed but discarded; `secure_mode` field doesn't exist.
- **Skill tries counters + skill advance** (PC-5) — `PlayerSkills` has level fields only; DB `skill_*_tries`/`manaspent` columns never loaded into runtime; `req_skill_tries` never called. Data gaps: `skill_base` (Delta) constants and `min_level` per skill not in any Lua file yet (need `data/formulas/772.lua` or `MechanicsProfile`); magic level needs separate dispatch (`min_level=0`, `skill_base=1600`).
- **Level-up HP/mana gain bug** (PC-5, M13) — `add_experience`/`remove_experience` clamp current HP/mana to the new max instead of adding/subtracting the per-level gain (C++ `TSkillAdd::Advance` raises both `Act` and `Max`).

---

## 2. Reference spec (7.72 decompile — cite in code headers, do not copy)

All line numbers `reference/cipsoft-772/tibia-game-master/src/`.

### 2.1 Attack value / skill selection — `crcombat.cc:165` `GetAttackValue`
Priority: `Close` (melee weapon `WEAPONATTACKVALUE`, skill from `WEAPONTYPE`) → `Missile`
(ammo `AMMOATTACKVALUE`, `SKILL_DISTANCE`) → `Throw` (`THROWATTACKVALUE`, `SKILL_DISTANCE`) →
`Wand` (attack 0) → fist (`RaceData[Race].Attack`, `SKILL_FIST`). Skill map: sword/club/axe/dist/fist.

### 2.2 Attack damage — `crcombat.cc:220` `GetAttackDamage`
```
MaxValue = attackValue
OFFENSIVE: MaxValue += (MaxValue*2)/10      # +20%
DEFENSIVE: MaxValue -= (MaxValue*4)/10      # -40%
Result = Skill[SkillNr].ProbeValue(MaxValue, LearningPoints>0)
if LearningPoints>0: LearningPoints -= 1
```
Fight-mode percentages are era-tunable via `data/formulas/772.lua` `fightModes`, loaded into
`MechanicsProfile.fight_modes` and applied by `combat::math::apply_attack_mode`/
`apply_defense_mode`.

### 2.3 ProbeValue — `crskill.cc:535` `TSkillProbe::ProbeValue`
```
if Increase: this.Increase(1)               # +1 skill exp (may level the skill)
RandomFactor = ((rand()%100) + (rand()%100)) / 2      # triangular 0..99
MaxValue    = Max * (skillValue*5 + 50)
Result      = (RandomFactor * MaxValue) / 10000
```
Already in `combat/math.rs::probe_value` (`skill_mult=5`, `skill_base=50`, `random_max=99`).

### 2.4 Defend damage — `crcombat.cc:237` `GetDefendDamage`
```
if EarliestDefendTime > now: return 0                 # 2000 ms gate
EarliestDefendTime = LastDefendTime + 2000; LastDefendTime = now
mode = (Following || AttackDest==0) ? DEFENSIVE : AttackMode
GetDefendValue: Shield(SHIELDDEFENDVALUE) > Close(WEAPONDEFENDVALUE) > Throw > Missile(0) > fist(RaceData.Defend)
OFFENSIVE: MaxValue -= (MaxValue*4)/10
DEFENSIVE: MaxValue += (MaxValue*8)/10
Increase only if (Shield!=NONE && LearningPoints>0)
Result = Skill[SkillNr].ProbeValue(MaxValue, Increase)
+ shield WEAROUT decrement
```
Gate + roll in `monster_combat.rs::roll_target_defense`. Player `GetDefendValue` in
`player/combat/values.rs`. Defense gate is `MechanicsProfile.defense_gate_ms` (2000ms).

### 2.5 Armor strength — `crcombat.cc:286` `GetArmorStrength`
```
Armor = sum(ARMORVALUE of equipped CLOTHES+ARMOR at correct BODYPOSITION) + RaceData.Armor
if Armor >= 2: Armor = (Armor/2) + rand()%(Armor/2)
```
Randomized reduction in `combat/math.rs::armor_reduction`. Player inventory summation in
`player/combat/values.rs::player_get_armor_strength`. Applied **inside** `TCreature::Damage(PHYSICAL)`,
not in `CloseAttack`.

### 2.6 Strike dispatch — `crcombat.cc:531` `Attack` / `:648` `CloseAttack` / `:739` `DistanceAttack` / `:704` `WandAttack`
- Validation (target present, invisible-vs-player, secure/PVP, distance > 8, PZ).
- `BlockLogout(60)` on attacker + target; `RecordAttack` for PVP.
- `DelayAttack(200)` → strike by `GetDistance()` range (1 close / 2 throw / 3 missile-wand) →
  `DelayAttack(2000)`.
- `CloseAttack`: `Damage = max(0, GetAttackDamage − target.GetDefendDamage)`;
  `target.Damage(PHYSICAL)` (armor inside); `if DamageDone>0: ActivateLearning()`; race poison
  (monster-only); weapon wearout.
- `DistanceAttack`: ammo present/range; `HitChance` bow 90 / throw 75;
  `Probe(Difficulty*15, HitChance, learning)`; on hit `target.Damage(GetAttackDamage, PHYSICAL)`
  (no defense subtraction on ranged — documented CipSoft behavior); on miss drop ammo; special
  effects (1 = poison arrow periodic, 2 = burst arrow area). Ammo consumption + fragility.
- `WandAttack`: `WANDRANGE`; `CheckMana(WANDMANACONSUMPTION)`; `Damage = AttackStrength +
  random(-Variation, +Variation)`; `WANDDAMAGETYPE`; missile. **Data source:** the
  `AttackStrength`/`Variation`/mana/element/level/vocation values come from
  `data/scripts/weapons/wands.lua` + `rods.lua` via the TFS Lua `Weapon(WEAPON_WAND)` API —
  `damage(min, max)` maps to `AttackStrength = min`, `Variation = (max - min) / 2` (centered),
  `mana` → `WANDMANACONSUMPTION`, `element` → `WANDDAMAGETYPE`. See §8 Q3 (resolved).

### 2.7 Learning / skill advance — `crskill.cc:549` `Probe`, `:387` `Increase`, `crcombat.cc:526` `ActivateLearning`
`ActivateLearning()` sets `LearningPoints = 30`. Each `ProbeValue`/`Probe` with `LearningPoints>0`
calls `Increase(1)` then decrements. `Increase` adds to skill exp and levels when
`Exp >= NextLevel`; `NextLevel = GetExpForLevel(Act+1)` geometric with `FactorPercent`/`Delta`.
Maps to `combat/math.rs::req_skill_tries` — fed by `data/defs/vocations.lua` `skill_multipliers`
(PC-0 landed; `VocationProfile.skill_multipliers` on `Player`). The `req_skill_tries` call site +
per-skill tries counters are still unwired (PC-5).

### 2.8 Fight/chase/secure mode packet — `receiving.cc` `0xA7`, `crcombat.cc:333/354` `SetAttackMode`/`SetChaseMode`
`SetAttackMode` mode change → `DelayAttack(2000)`. `SetChaseMode` only NONE/CLOSE for players.
SecureMode stored on `TCombat`. **Parse + chase-mode storage already exist** (`game_parse.rs`
`C::FIGHT_MODES` → `GamePacket::FightModes`; `game_loop.rs` sets `chase_mode`) — what's missing is
storing `raw_fight_mode`/`raw_secure_mode` into `Player` fields. `Player.attack_mode` was added
in PC-1; `secure_mode` is still PC-4.

### 2.9 Level / vitals — `data/defs/vocations.lua` gains + `crskill.cc:352` `GetExpForLevel`
Level exp `(((L-6)*L+17)*L-12)/6 * Delta` in `combat/math.rs::experience_for_level`
(and `creature/vocation.rs::total_experience_for_level` — consolidate onto one in PC-5 cleanup).
Vitals per level from `data/defs/vocations.lua` via `VocationProfile::recalculate_vitals` (PC-0).

---

## 3. Work breakdown

### Phase PM — Player module consolidation — ✅ DONE
All 9 files relocated via `git mv` (history preserved); `lib.rs` has a single `mod player;` +
crate-root `pub(crate) use player::… as <old_name>` aliases. Pure file relocation, no logic edits.
Verification: `cargo check` = 6 warnings (baseline), `cargo test` = 523 passed / 2 ignored.

### Phase PC-0 — Vocation data migration (XML→Lua) — ✅ DONE
`vocations.xml` → `data/defs/vocations.lua` via sandboxed mlua + `serde::Deserialize`. Full TVP
combat block (gains, regen cadence, `mana_multiplier`, `attack_speed_ms`, `base_speed`, `soul_max`,
`gain_soul_ticks`, `allow_pvp`, `formula` block, `skill_multipliers[7]`) + level-1 vitals floor
(`base_hp`/`base_mana`/`base_cap` from `runtime/mon/human.mon`). `VocationProfile` (`Copy`) snapshot
cached on `Player` at login. Commits `cd6ba50` + `a462d54`.

**Still open from PC-0:**
- XML→Lua one-shot converter tool deferred (dual-load golden test covers equivalence).
- `total_experience_for_level` consolidation onto `combat::math::experience_for_level` — PC-5.
- `fed_regen_cadence` hardcoded fallback for empty-registry test worlds — PC-5 step 1.

### Phase PC-1 — Player attack/defend/armor value resolution — ✅ DONE
New `player/combat/values.rs` with `SkillNr` enum + `player_get_attack_value` /
`player_get_defend_value` / `player_get_armor_strength` (all returning raw unscaled values).
`Player.attack_mode: FightMode` (default `Balanced`) wired; `defend_fight_mode_for_target` reads
it for players. `sim_melee_attack: i32` added alongside `sim_melee_defense` for the fist fallback.
Verification: `cargo test` = 574 passed / 2 ignored (13 new PC-1 tests).

**Era-tuning boundary:** these three functions return **raw unscaled** values — the item/skill
number before fight-mode scaling and probe rolls. Downstream consumers (`weapon_damage`,
`defense_value`, `armor_reduction`) apply era-tunable multipliers from `MechanicsProfile` /
`data/formulas/772.lua`.

### Phase PC-2 — The strike (`CloseAttack`) — melee first — ✅ DONE
**Files:** `player/combat/mod.rs` (dispatch), `player/combat/strike.rs` (strike body).

**What landed:**
1. `PlayerChaseOutcome::Adjacent` dispatches to `player_close_attack_strike` in `strike.rs`.
2. Attack roll: `weapon_damage(profile, hooks, rng, skill, atk_value, mode, level)` × vocation
   `formula.melee_damage` (floor).
3. Defense roll: `roll_target_defense` with world-aware `melee_defense_snapshot_for` (player
   targets contribute shield/weapon defend + shielding skill + armor).
4. Armor: `armor_reduction` with target's `GetArmorStrength`.
5. Damage: `melee_damage_after_defense_and_armor` → `combat_execute_with_stimulus(PHYSICAL)`.
6. Poff/spark: `if dmg <= 0` → `broadcast_magic_effect(pos, if attack <= defense { 3 } else { 4 })`.
7. `notify_player_combat_damage` — animated damage text + health bar for ALL creature types;
   private "You lose X hitpoints" + stats update for player targets only.
8. `if damage_done > 0: ActivateLearning()` on the player (`learning_points = 30`).
9. `LearningPoints` decrement on the attacker after the attack probe (`crskill.cc:549`).
10. Weapon wearout (`REMAININGUSES` decrement) — `player_strike_weapon_wearout`.
11. Cadence: `DelayAttack(200)` before, `DelayAttack(attack_speed_ms)` after (vocation
    `attackspeed`); `if target dead: StopAttack`.
12. Monster defense snapshot fix: `defense_skill: m.melee_skill` (was `0` — nerfed monster defense).
13. `sim_melee_attack: 7` / `sim_melee_defense: 5` in `login.rs` (was `0` — prevented fist damage).

**Bug fix (this session):** `notify_player_combat_damage` was returning early for non-player
targets (`let Some(CreatureKind::Player(p)) = ... else { return; }`). Rewrote to handle all
creature types: animated text + health bar broadcast for everyone; private status message +
stats only for players.

### Phase PC-2a — `Damage` path completeness (melee audit) — ✅ DONE
Findings from the decompile audit of `Attack` → `CloseAttack` → `Damage` (§9). These were gaps in
the shared `Damage` application path and the monster melee arm that PC-2 did not cover. All 8
findings landed in commit `b704827`.

**What landed:**
1. **A1 — StopAttack on kill:** `monster_do_attacking` now clears `attack_target`/`follow_target`
   when the target dies on a strike (`crcombat.cc:643-645`). Site: `monster_ai.rs` melee arm.
2. **A2 — ActivateLearning for monsters:** `activate_learning()` is called when
   `damage_done > 0` in the monster melee path (`crcombat.cc:664-666`). Site: `monster_ai.rs`.
3. **A3 — Race-keyed damage text color:** `damage_text_color(blood)` helper in
   `creature/monster_inventory.rs` maps Blood→180, Slime→30, Bones→129, Fire→198, Energy→35
   (`crmain.cc:712-755`). Consumed by `notify_player_combat_damage` in
   `game_world_spectators.rs`.
4. **M2 — Equipment protection (damage reduction items):** `player_absorb_percent` in
   `idle_stimulus.rs` sums `absorb_percent[combat_type]` across equipped `PROTECTION`+`CLOTHES`
   items and reduces incoming damage for player targets (`crmain.cc:540-574`). Applied in
   `combat_execute_with_stimulus` before the poff check.
5. **M3 — Physical immunity (NoHit):** New `immunity_physical` field on `MonsterDefenses`,
   `MonsterAiConfig`, `Monster`, and `MonsterCombatSnapshot` (parsed from
   `<immunity physical="1"/>`). `combat_execute_with_stimulus` blocks `Damage(PHYSICAL)` and
   emits `EFFECT_BLOCK_HIT` (4) when set (`crmain.cc:615-622`). Non-physical immunities
   (NoPoison/NoBurning/NoEnergy/NoLifeDrain) remain PC-3.
6. **M4 — Invisibility removal on hit:** `clear_nonplayer_invisibility` in `idle_stimulus.rs`
   removes the `Invisible` condition and broadcasts the outfit change for non-player creatures
   after they take physical damage (`crmain.cc:636-641`).
7. **M10 — "You are poisoned." status message:** `send_player_status_message` helper in
   `game_world_chat.rs` sends `TALK_STATUS_MESSAGE` "You are poisoned." to player targets after
   the poison condition lands (`crcombat.cc:674-676`). Wired in `monster_ai.rs` poison-on-hit
   path. Player strikes do not apply poison, so no player-side wire was needed.
8. **M11 — Shield wearout:** `player_shield_wearout` in `player/combat/strike.rs` +
   `player/inventory/util.rs` decrements the shield's `count` (REMAININGUSES) when
   `charges > 0` and the defense gate passed (`crcombat.cc:265-281`). Called after
   `roll_target_defense` in both the player strike path and the monster melee path (when
   attacking a player).

**Verification:** `cargo check` 0 errors / 10 warnings (baseline); `cargo test -p tfs-rust-core`
589 passed / 2 ignored; `cargo test -p tfs-rust-content` 45 passed.

### Phase PC-2b — Lua combat/spell/weapon plumbing — ✅ DONE
**Prerequisite for:** PC-3 (wand/rod data loading via `Weapon(WEAPON_WAND)` API), PC-3a
(spell-casting), and any script-driven combat (burst arrow, poison arrow, spell runes).
**Files:** `crates/tfs-rust-lua/src/userdata/combat.rs` (new), `spell.rs` (new),
`weapon.rs` (new); `crates/tfs-rust-lua/src/runtime.rs` (register metatables + enum table);
`crates/tfs-rust-lua/src/combat_scripts.rs` (load `data/scripts/weapons/*.lua`,
`data/scripts/spells/**/*.lua`); `crates/tfs-rust-content/src/weapons.rs` + `spells.rs`
(registries). All 8 implementation steps landed in commit `b704827`.

**What landed (8 steps, ordered by dependency):**

1. **Enum registry** — `crates/tfs-rust-lua/src/combat_enums.rs` registers ~100 TFS
   combat/spell/weapon/condition enums as Lua globals via `register_combat_enums(&lua)`,
   sourced from `tfs-rust-common` enums (`CombatType`, `ConditionType`, `TextEffect`,
   `ShootType`, etc.). Grouped by category (combat params, damage types, effects,
   conditions, weapon types, spell types, callbacks, formulas, skulls). C++ ref:
   `luascript.cpp:1200-2050` `registerEnum` block. Tests in `combat_enums.rs` verify a
   representative sample of values.

2. **`createCombatArea` global** — implemented in `combat_scripts.rs`; returns an `AreaRef`
   userdata (Rust side: `Vec<Vec<u8>>` matrix + optional diagonal) backed by `AreaCombat` in
   `userdata/combat.rs`. C++ ref: `luascript.cpp:3547` `luaCreateCombatArea`,
   `combat.cpp` `AreaCombat::setupArea`. The 772 circle-ring model (§3.6) remains a separate
   `circles.rs` const table for execution time — both coexist (matrix is the script-facing
   API, circle-ring is the era-specific execution model).

3. **`Combat` userdata** — `CombatRef`/`CombatDef` in `userdata/combat.rs`. Methods:
   `setParameter`/`getParameter`, `setFormula`, `setArea`, `setCallback`, `execute`.
   Backed by `CombatDef { params: CombatParams, area: Option<AreaMatrix>,
   formula: Option<FormulaDef>, conditions: Vec<ConditionDef>,
   callbacks: HashMap<CallbackParam, RegistryKey> }`. C++ ref: `luascript.cpp:2855-2871`,
   `combat.h:118` `Combat`. `Combat:execute` dispatches into the existing
   `combat_execute_with_stimulus` core path — the Lua `Combat` is a config bag; execution
   stays in `tfs-rust-core`.

4. **`Condition` userdata** — `ConditionBuilder` in `userdata/condition.rs` extended to mirror
   the full C++ `Condition` API (`setParameter`/`getParameter`, `setTicks`/`getTicks`,
   `setFormula`, `setOutfit`, `addDamage`, `clone`, `getId`/`getSubId`/`getType`/`getIcons`/
   `getEndTime`). Backed by `ConditionDef` (type, ticks, params, formula). C++ ref:
   `luascript.cpp:2874-2895`, `condition.h`.

5. **`Weapon` userdata** — `WeaponBuilder`/`PendingWeapon` in `userdata/weapon.rs`. Methods
   cover the TFS surface (`id`, `level`, `mana`, `element`, `damage`, `vocation`, `register`,
   plus the melee/distance fields stored for PC-3 consumption). `Weapon:register()` pushes
   into a `_pending_weapons` Lua table drained by the loader. C++ ref:
   `luascript.cpp:3209-3246`, `weapons.h:53-293`. **PC-3 scope:** wand-relevant fields
   (`id`, `level`, `mana`, `element`, `damage(min, max)`, `vocation`, `register`) are fully
   functional; melee/distance fields are stored but not yet applied to `ItemType` — they go
   live in PC-3.

6. **`Spell` userdata** — `SpellBuilder`/`PendingSpell` in `userdata/spell.rs`. Methods cover
   the TFS instant/rune surface (`words`, `level`, `mana`, `vocation`, `name`,
   `isAggressive`, `register`, etc.). Constructor takes `SPELL_INSTANT` or `SPELL_RUNE`;
   `Spell:register()` pushes into `_pending_spells` drained into `SpellRegistry`. C++ ref:
   `luascript.cpp:3095-3137`, `spells.h:108-380`.

7. **`WeaponRegistry` + `SpellRegistry`** — `crates/tfs-rust-content/src/weapons.rs` and
   `spells.rs`. `WeaponRegistry` holds `WandDef`/`DistanceWeaponDef`/`MeleeWeaponDef`;
   `SpellRegistry` holds `InstantSpellDef`/`RuneSpellDef`. Both use the Lua-to-Rust pending
   drain pattern: scripts push into `_pending_*` Lua tables, then
   `load_weapon_scripts`/`load_spell_scripts` drain them into the registries.

8. **Script loaders** — `load_weapon_scripts`, `load_spell_scripts`, and `load_areas_lua` in
   `combat_scripts.rs`. `load_weapon_scripts` scans `data/scripts/weapons/*.lua`;
   `load_spell_scripts` scans `data/scripts/spells/**/*.lua` (recursive);
   `load_areas_lua` runs `data/scripts/spells/areas.lua` (plain Lua `AREA_*` tables) before
   spell scripts.

9. **Spellword dispatch seam** — `try_dispatch_spellword` in
   `tfs-rust-core/src/game_world_chat.rs` looks up `words` in `SpellRegistry` (instant
   spells only), checks vocation/level/mana/soul gates, deducts costs, and dispatches the
   `onCastSpell` Lua callback. Wired into `player_say_spell` (mirrors `game.cpp:3579-3584`
   + `spells.cpp:30`). **PC-2b scope:** registry lookup + callback dispatch + cost
   deduction. Full spell mechanics (cooldowns, group cooldowns, PZ lock,
   aggressive-target validation) land in PC-3a.

**Era note:** the `Combat`/`Spell`/`Weapon`/`Condition` userdata API is era-agnostic —
scripts use the same Lua calls regardless of `clientVersion`. Era differences (772
circle-ring AoE vs 1098 `MatrixArea`, 772 `ProbeValue` vs 1098 damage formula) are handled
inside the Rust execution layer (`combat_execute_with_stimulus` + `MechanicsProfile`), not
in the Lua bindings. No `if version == 772` in the Lua plumbing.

**Verification:** `cargo check` 0 errors / 10 warnings (baseline); `cargo test -p tfs-rust-lua`
24 passed / 1 pre-existing failure (`player_events_script_loads_with_bootstrap`, unrelated to
PC-2b); `cargo test -p tfs-rust-content` 45 passed; `cargo test -p tfs-rust-common` 5 passed.

### Phase PC-3 — Distance + wand strikes — 🔲 PENDING
**File:** `player/combat/ranged.rs` (new), reusing `player/combat/values.rs`.
**Prerequisite:** PC-2b (for `Weapon(WEAPON_WAND)` Lua loading of `wands.lua`/`rods.lua`
into `WandRegistry`).

1. `DistanceAttack`: ammo resolution (already in `player_get_weapon`), range vs `shoot_range`,
   `HitChance` (bow 90 / throw 75), `Probe(Difficulty*15, HitChance, learning)` — add
   `combat::math::probe_hit(skill, diff, prob, rng)` mirroring `TSkillProbe::Probe`. On hit apply
   `GetAttackDamage` × `formula.dist_damage`; on miss scatter-drop ammo. Ammo consume + fragility;
   special effects (poison arrow → periodic poison condition; burst arrow → area physical via the
   existing shape helpers).
2. `WandAttack`: mana check against `Player.mana` (`WANDMANACONSUMPTION`), fixed
   `AttackStrength ± Variation`, wand `WANDDAMAGETYPE`, missile effect. **Wand/rod data source is
   `data/scripts/weapons/wands.lua` + `rods.lua`** via the TFS Lua `Weapon(WEAPON_WAND)` API
   (`level`/`mana`/`element`/`damage(min,max)`/`vocation`/`id`/`register`) — *not* `items.xml` or
   `objects.srv`. Rods are registered as `WEAPON_WAND` with druid vocations; the same loader
   handles both files. This requires the `Weapon` userdata + `Weapon:register()` plumbing noted in
   Q7 (currently absent from `tfs-rust-lua`) — PC-3 must either add that plumbing or load the
   wand table via a one-shot Lua harness that mirrors the TFS `Weapon` API surface. The loaded
   rows map to a `WandDef { item_id, level, mana_cost, element, damage_min, damage_max,
   vocations: Vec<VocationId> }` registry on `tfs-rust-content`, keyed by `item_id`. See §8 Q3
   (resolved) and §5.
3. **M5 — Mana shield:** C++ `Damage` checks `SKILL_MANASHIELD` timer and absorbs damage to mana
   before HP (`crmain.cc:662-689`). Needed when wand attacks introduce typed damage (fire/energy)
   and when spell-casting lands (PC-3a). Add `SKILL_MANASHIELD` / mana-shield condition check to
   `combat_execute_with_stimulus` before HP apply.
4. **M3 (non-physical) — Race immunities for typed damage:** C++ `Damage` checks `NoPoison`/
   `NoBurning`/`NoEnergy`/`NoLifeDrain` for non-physical damage types and emits `EFFECT_BLOCK_HIT`
   + returns 0 (`crmain.cc:615-622`). The physical immunity (`NoHit`) is in PC-2a; these
   non-physical immunities matter when wand damage types (fire/energy) and spell damage are
   introduced. Add `immunity_fire`/`immunity_energy`/`immunity_life_drain` flags to monster data.

### Phase PC-4 — Fight/chase/secure mode + PVP gating — 🔲 PENDING
**Files:** `player/combat/fight_mode.rs` (new), `game_loop.rs` (`FightModes` arm).

1. ~~Parse `0xA7`~~ — **already done** (`GamePacket::FightModes`). No net-side work needed.
2. Core setter: extend the existing `FightModes` arm in `game_loop.rs` — currently only sets
   `chase_mode` and discards `raw_fight_mode`/`raw_secure_mode`. Add `SetAttackMode`
   (change → `DelayAttack(2000)`) writing `Player.attack_mode` (field exists from PC-1), and
   `SecureMode` writing `Player.secure_mode: bool` (new field). Enforce the reserved
   `CombatResult::SecureMode` + `AttackNotAllowed` branches in `validate_player_attack_target`.
3. PVP: `can_player_attack_player` / `is_protected` already in `combat/pvp.rs` — wire
   `IsAttackJustified`, `RecordAttack`, `BlockLogout(60)`, skull/frag outcomes (scope-check
   against 772 `cract.cc`/`crcombat.cc`; skulls may be a follow-up sub-phase).
4. **M1 — INVULNERABLE right check:** C++ `Damage` checks `CheckRight(target, INVULNERABLE)` and
   sets damage to 0 for GMs with the invulnerability right (`crmain.cc:536-538`). Not implemented —
   GMs take damage. Add a `CheckRight`-equivalent flag check at the top of
   `combat_execute_with_stimulus` (or `apply_health_delta`) that zeroes incoming damage when the
   target has the invulnerability right. Fits PC-4 alongside the other rights/PVP gating.

### Phase PC-5 — Skill/exp gain + regen from vocation data — 🔲 PENDING
**Files:** `process_skills.rs`, `death.rs`/`idle_stimulus.rs`, `creature/player.rs`.

1. Remove hardcoded fallback table in `fed_regen_params` (only hit by empty-registry test worlds);
   ensure all test harnesses populate the registry from `data/defs/vocations.lua`.
2. On kill: `distribute_experience` + `pvp_exp_cap` → `add_experience`; skill `Increase` via
   learning during strikes.
3. Skill tries: add `_tries: u64` per skill to `PlayerSkills` (8 fields — 7 combat skills +
   `manaspent` for magic level); load from DB at login (`skill_*_tries` columns +
   `manaspent` already round-tripped). **Per-level storage model** (1098-style `tries` that reset
   to 0 on level-up) — matches DB schema + `req_skill_tries`; the 772 cumulative-`Exp` model
   produces identical leveling outcomes, so per-level is the idiomatic choice. Wire
   `Increase(1)`-equivalent leveling in the strike path: `ActivateLearning` sets
   `learning_points = 30`; each `ProbeValue`/`Probe` with `learning_points > 0` adds 1 try to the
   appropriate skill, decrements `learning_points`, checks for level-up via `req_skill_tries`.
   **Data gaps to fill (verified against `human.mon` + `crskill.cc:472-496` + TFS
   `vocation.cpp:139-154`):**
   - **`skill_base` (Delta) constants** — `{50, 50, 50, 50, 30, 100, 20}` for
     `{fist, club, sword, axe, dist, shielding, fishing}` (race data from `human.mon`, same for
     all vocations in 772, exactly matches TFS 1098 `skillBase` array). NOT in any Lua file yet —
     add to `data/formulas/772.lua` (e.g. `skillTuning = { skillBase = {50,50,50,50,30,100,20} }`)
     or `MechanicsProfile`.
   - **`min_level` per skill** — `Min` from `human.mon`: 10 for combat skills, 0 for MagicLevel.
     TFS uses `MINIMUM_SKILL_LEVEL = 10` for combat; magic level needs `min_level = 0` (special
     case, otherwise `req_skill_tries(maglevel, 1, 1600, 3.0, 10)` ≈ 0).
   - **Magic level dispatch** — TFS 1098 uses a separate `getReqMana(magLevel) = 1600 *
     manaMultiplier^(magLevel - 1)` (`vocation.cpp:149-154`); `getReqSkillTries` returns 0 for
     `SKILL_MAGLEVEL`. Our `req_skill_tries` CAN handle magic level if the caller dispatches it
     with `skill_base=1600, multiplier=mana_multiplier (per-vocation, already in
     VocationProfile), min_level=0`. The caller must route magic level separately from combat
     skills.
   - **Formula verification** — `req_skill_tries(skill, level, skill_base, multiplier, min_level)`
     returns `skill_base * multiplier^(level - (min_level + 1))` (per-level cost). 772
     `TSkillProbe::GetExpForLevel(L)` (`crskill.cc:472-496`) returns cumulative
     `Delta * (Base^(L-Min) - 1) / (Base - 1)`; the per-level cost
     `GetExpForLevel(L+1) - GetExpForLevel(L) = Delta * Base^(L - Min)` matches our formula when
     `skill_base = Delta`, `multiplier = Base = FactorPercent/1000`, `min_level = Min`. Verified
     numerically (sword: L11→50, L12→100, L13→200).
4. **M12 — Shield skill learning:** C++ `GetDefendDamage` decrements `LearningPoints` and passes
   `Increase=true` to `ProbeValue` when the defender has a shield and `LearningPoints > 0`
   (`crcombat.cc:259-263`). This is the shielding-skill exp gain path — the defense counterpart to
   PC-2's attack-skill learning. Wire into `roll_target_defense`: when the target has a shield and
   `learning_points > 0`, decrement and accumulate shielding skill tries.
5. **M7 — Player death penalty:** C++ `Damage` checks `Damage == HitPoints` and handles amulet of
   loss (prevent inventory drop), inventory drop, and blesses (`crmain.cc:790+`). PC-5 handles the
   killer's side (exp/skill gain); this is the victim's side — what happens to the player's
   inventory and blessings when they die. Add to the `apply_creature_death` path for player
   victims: check for amulet of loss, drop inventory (or not), apply bless reductions.
6. **M13 — Current HP/mana += gain on level-up (level-up gain bug):** C++ `TSkillAdd::Advance(Range)`
   (`crskill.cc:667-678`) raises **both** `Act` and `Max` by `Range * AddLevel` — i.e. the level-up
   gain is added to the player's *current* HP/mana, not just the cap. `TSkillLevel::Jump(Range)`
   (`crskill.cc:355-382`) calls `Advance(Range)` on HP/Mana/GoStrength/CarryStrength. Our Rust
   `add_experience`/`remove_experience` (`creature/player.rs:228-262`) **clamps** current HP/mana
   to the new max instead of adding the per-level gain — a player at full HP (150) leveling up
   (gain 15) stays at 150 instead of going to 165. Fix: track the level delta and add/subtract
   `gain_hp * delta` / `gain_mana * delta` to current HP/mana (clamped to the new max), mirroring
   `Advance`. **Era note:** 1098 TFS (`player.cpp:1800-1802`) refills to full
   (`health = getMaxHealth(); mana = getMaxMana();`) after the level loop — the 772 decompile does
   **not**. Current Rust matches 772 (no refill); the 1098 refill must be era-gated when 1098
   support lands (likely a `MechanicsProfile` flag or `StepSpeedModel`-style enum on the level-up
   path).

---

## 3.6 AoE shape model — baked `circles.rs` (772) vs `MatrixArea` (1098)

**Decision: 772 area-of-effect uses the circle-ring model from the decompile — baked into a Rust
const table (`circles.rs`), not loaded from `circles.dat` at runtime.** Required by PC-3 (burst
arrow) and shared with spell-casting (PC-3a). 1098 keeps TFS `MatrixArea`.

**Per-spell radius issue:** TFS scripts pass the same matrix (e.g. `AREA_CIRCLE5X5`) but the
decompile uses different radii per spell (UE=6, poison storm=8). Resolution:
1. Per-spell 772 radius override (`data/formulas/772_spell_areas.lua`) — exact parity.
2. Else matrix-derived radius (max ring extent) — playable but not exact.

**Lua scripting prerequisite:** `Combat`/`Spell`/`createCombatArea`/`Weapon` userdata and spellword
dispatch (`Say` → `onCastSpell`) are implemented in **PC-2b** (§3 PC-2b). PC-3a depends on PC-2b
being complete. Player weapon-combat (PC-2/PC-2a) does not depend on it; PC-3 (wand/rod data
loading) does.

---

## 4. Architecture / placement rules

### 4.0 Module layout
```
crates/tfs-rust-core/src/player/
  mod.rs                # module surface doc + re-exports
  combat/
    mod.rs              # player_execute_attack strike dispatch
    values.rs           # player_get_attack_value / _defend_value / _armor_strength (PC-1)
    strike.rs           # CloseAttack melee strike body (PC-2)
    ranged.rs           # DistanceAttack + WandAttack (PC-3)
    fight_mode.rs       # attack/chase/secure-mode setters + PVP gating (PC-4)
  inventory/
    mod.rs              # inventory surface
    query_add.rs        # was player_inventory_query_add.rs
    load.rs             # was player_inventory_load.rs
    notifications.rs    # was player_inventory_notifications.rs
    util.rs             # was player_inventory_util.rs
  stats.rs              # was game_world_player.rs
  flags.rs              # was player_flags.rs
  depot.rs              # was player_depot.rs
  ping.rs               # was player_ping.rs

crates/tfs-rust-lua/src/userdata/        # PC-2b — Lua combat/spell/weapon plumbing
  combat.rs             # Combat userdata (CombatDef: params, area, formula, callbacks)
  condition.rs          # Condition userdata (ConditionDef: type, ticks, params)
  weapon.rs             # Weapon userdata (WeaponDef: wand/distance/melee/ammo config)
  spell.rs              # Spell userdata (SpellDef: instant/rune config + onCastSpell)
crates/tfs-rust-lua/src/
  combat_enums.rs       # ~860 TFS combat/spell/weapon/condition enum registrations
  combat_scripts.rs     # load_weapon_scripts / load_spell_scripts / load_areas_lua
crates/tfs-rust-content/src/
  weapons.rs            # WeaponRegistry (drained from pending_weapons)
  spells.rs             # SpellRegistry (drained from pending_spells)
```

### 4.1 Steering compliance
- **Formulas & flow** in `tfs-rust-core` combat modules; **no** `NetworkMessage`/opcode bytes in
  core (`0xA7` parse stays in `tfs-rust-net`).
- **Era knobs** in `MechanicsProfile` / `data/formulas/772.lua`. **Per-vocation balance** in
  `data/defs/vocations.lua`. No new balance literals in Rust.
- **No `if version == 772`** in core — melee/dist/wand paths are shared; only profile fields differ.
- **Reuse** `probe_value`, `armor_reduction`, `defense_value`, `roll_target_defense`,
  `combat_execute_with_stimulus` — do not fork a parallel player combat math module.
- Every new `.rs` gets the C++ reference header (`crcombat.cc`/`crskill.cc` + TFS structure cite).
- SlotMap IDs, `?` errors, enums + match; no `unsafe`.

---

## 5. New/changed data model

| Field | Where | Source | Status |
|-------|-------|--------|--------|
| `learning_points: i32` | `CreatureBase` | `ActivateLearning`/`ProbeValue` | ✅ PC-2 |
| `skill_*_tries: u64` (8 fields) | `PlayerSkills` | DB `skill_*_tries` (7) + `manaspent` (1) | 🔲 PC-5 |
| `skill_base` constants `{50,50,50,50,30,100,20}` | `MechanicsProfile` / `data/formulas/772.lua` | `human.mon` race data (Delta/NextLevel) | 🔲 PC-5 |
| `min_level` per skill (10 combat, 0 magic) | `MechanicsProfile` / `data/formulas/772.lua` | `human.mon` race data (Min) | 🔲 PC-5 |
| `attack_mode: FightMode` | `Player` | `0xA7` packet | ✅ PC-1 |
| `secure_mode: bool` | `Player` | `0xA7` packet | 🔲 PC-4 |
| `VocationDef` (full combat block) | `tfs-rust-content` | `data/defs/vocations.lua` | ✅ PC-0 |
| `VocationProfile` (`Copy`) snapshot | `Player.vocation_profile` | `VocationRegistry` at login | ✅ PC-0 |
| Wand attributes (`WandDef`) | `tfs-rust-content` `WandRegistry` | `data/scripts/weapons/wands.lua` + `rods.lua` (TFS Lua `Weapon(WEAPON_WAND)` API) | ✅ PC-2b (loader) / 🔲 PC-3 (consumer) |
| `WeaponDef` (distance/melee/ammo) | `tfs-rust-content` `WeaponRegistry` | `data/scripts/weapons/*.lua` (`Weapon(WEAPON_*)` API) | ✅ PC-2b |
| `SpellDef` (instant/rune) | `tfs-rust-content` `SpellRegistry` | `data/scripts/spells/**/*.lua` (`Spell(SPELL_*)` API) | ✅ PC-2b |
| `CombatDef` (Lua-side combat config) | `tfs-rust-lua` userdata | `Combat()` + `:setParameter`/`:setArea`/`:setCallback`/`:execute` | ✅ PC-2b |
| `ConditionDef` (Lua-side condition config) | `tfs-rust-lua` userdata | `Condition(CONDITION_*)` + `:setParameter`/`:setTicks` | ✅ PC-2b |
| Combat/spell/weapon enums (~860) | `tfs-rust-lua` globals | `tfs-rust-common` enums → `register_combat_enums(&lua)` | ✅ PC-2b |
| `CIRCLE_RINGS` baked const + `disc_offsets` | `combat/circles.rs` | generated from `circles.dat` | 🔲 PC-3a |
| `area_shape: AreaShapeModel` | `MechanicsProfile` | era / `772.lua` | 🔲 PC-3a |
| 772 per-spell radius override (opt) | `data/formulas/772_spell_areas.lua` | `magic.cc` cases | 🔲 PC-3a |

`earliest_attack_ms`/`earliest_defend_ms`/`last_defend_ms` already exist on `CreatureBase`.

---

## 6. Test plan
- **Formula goldens** (extend `combat/math.rs` tests): melee `max(0,atk−def)` then randomized armor;
  distance hit-probe bounds; wand fixed±variation; skill-tries curve from vocation multipliers.
- **Skill-tries parity goldens** (PC-5, extend `combat/math.rs` tests): assert `req_skill_tries`
  per-level cost matches 772 `GetExpForLevel(L+1) - GetExpForLevel(L)` for each skill (sword
  L11→50, L12→100, L13→200; dist L11→30, L12→60; shielding L11→100, L12→150; fishing L11→20,
  L12→22; magic L1→1600, L2→4800 with `mana_multiplier=3.0`). Assert `skill_base`/`min_level`
  constants loaded from `data/formulas/772.lua` match `human.mon` Delta/Min. Assert
  `Increase(1)` × 30 (one `ActivateLearning` cycle) levels sword 10→11 when starting from 0
  tries (50 tries needed, 30 < 50 so no level-up; second `ActivateLearning` → 60 tries →
  level-up).
- **Vocation parse golden** (✅ PC-0): `data/defs/vocations.lua` full block + dual-load equivalence.
- **Lua combat enum registration** (✅ PC-2b): `combat_enums.rs` tests assert representative
  enums resolve to correct integer values (`COMBAT_PARAM_TYPE`, `COMBAT_HEALING`,
  `WEAPON_WAND`, `SPELL_INSTANT`, `CONDITION_POISON`, `CONST_ME_MAGIC_BLUE`,
  `CALLBACK_PARAM_LEVELMAGICVALUE`, `COMBAT_FORMULA_SKILL`).
- **`createCombatArea` golden** (✅ PC-2b): `AreaRef` userdata produced from
  `AREA_SQUARE1X1`-equivalent matrix with correct affected offsets and caster origin.
- **Weapon load golden** (✅ PC-2b): `WeaponRegistry` drains `PendingWeapon` entries from
  `Weapon:register()` calls. Full `wands.lua`/`rods.lua`/`distance_weapons.lua`/`burst_arrow.lua`
  load coverage lands with PC-3 golden tests against real data files.
- **Spell load golden** (✅ PC-2b): `SpellRegistry` drains `PendingSpell` entries from
  `Spell:register()` calls. Full `spells/attack/berserk.lua` + `#example.lua` load coverage
  lands with PC-3a golden tests against real data files.
- **Spellword dispatch** (✅ PC-2b seam): `try_dispatch_spellword` in `game_world_chat.rs`
  does registry lookup + vocation/level/mana/soul gates + cost deduction + `onCastSpell`
  callback dispatch. Full end-to-end spellword test (level gate 34→rejected, 35→accepted,
  mana deduction, spectator damage) lands in PC-3a with real spell scripts.
- **Wand/rod parse golden** (PC-3, extends PC-2b weapon load): assert all 10 wand/rod entries
  parse with correct `item_id`/`level`/`mana`/`element`/`damage_min`/`damage_max`/`vocations`;
  assert rods register as `WEAPON_WAND` with druid vocations.
- **Circles parity** (`combat/circles.rs`): re-derive rings from `circles.dat` and assert equality;
  spot-check `disc_offsets(6)` (UE) and `disc_offsets(8)` (poison storm).
- **Integration** (`sim_harness`/beat-driven world): player vs `human.mon`/`rat` — verify damage
  ranges, defense gate 2000 ms, learning advances skill, ammo consumed on distance, wand mana cost,
  death → exp/skill gain, PZ/secure-mode denial text.
- **glibc-rand parity** where `sim_glibc_rng_enabled()`.

---

## 7. Verification
```
cargo check -p tfs-rust-core -p tfs-rust-content
cargo clippy -p tfs-rust-core -p tfs-rust-content --all-targets
cargo test  -p tfs-rust-core -p tfs-rust-content
```

---

## 8. Open questions

1. **772 starting vitals floor** — ✅ **Resolved for 772.** The 772 decompile has **no vocation
   change mechanic** — "vocation" is a display string only (`operate.cc:1854-1915`). Per-level
   gains (`AddLevel`) come from `RaceData[Race].Skill` via `TSkillBase::SetSkills(Race)`
   (`crskill.cc:1165-1188`) — i.e. race data (`human.mon`), not a swappable vocation object.
   `AddLevel` never changes mid-life, so `base + gain*(level-1)` is **always correct for 772**.
   The "vocation change at level > 1" divergence is a **1098-only concern** (TFS has
   `setVocation`/vocation change); defer to the 1098 era work. **However**, the audit surfaced a
   real bug — see M13 (§3 PC-5 / §9.2): C++ `TSkillAdd::Advance` raises *current* HP/mana by the
   gain on level-up, but our Rust clamps to the new max. Fix queued in PC-5.
2. **Skill-tries mapping** — ✅ **Resolved.** `FactorPercent = 1000 * multiplier` confirmed
   against `human.mon` race data; `skill_base = Delta` (NextLevel) matches TFS 1098 `skillBase`
   exactly. `req_skill_tries` formula verified mathematically (per-level cost matches 772
   cumulative `GetExpForLevel` difference). **Data gaps identified for PC-5:** (a) `skill_base`
   constants `{50,50,50,50,30,100,20}` not in any Lua file — add to `data/formulas/772.lua` or
   `MechanicsProfile`; (b) `min_level` per skill (10 for combat, 0 for magic) needs per-skill
   dispatch; (c) magic level uses separate path (`getReqMana`-style, `skill_base=1600`,
   `min_level=0`, `multiplier=mana_multiplier`); (d) `PlayerSkills` needs 8 `_tries: u64` fields
   (7 combat + `manaspent`). Use per-level storage (1098-style, matches DB schema). Full details
   in §3 PC-5 step 3.
3. **Wand data source** — ✅ **Resolved.** Wand/rod attributes live in
   `data/scripts/weapons/wands.lua` and `rods.lua` via the TFS Lua `Weapon(WEAPON_WAND)` API
   (`level`/`mana`/`element`/`damage(min,max)`/`vocation`/`id`/`register`). Rods register as
   `WEAPON_WAND` with druid vocations — one loader covers both files. No `items.xml` or
   `objects.srv` parsing required. PC-3 loads these into a `WandDef` registry on
   `tfs-rust-content` keyed by `item_id`; the `Weapon` userdata plumbing (Q7) is the prerequisite.
4. **Ranged defense "bug"** (`crcombat.cc:766`) — replicate the outcome (no defense on ranged, but
   still rolls target defense/wearout if attacker holds a shield). Confirm in PC-3.
5. **Skulls / PVP frags** — scope: include in PC-4 or defer to a dedicated PVP phase.
6. **AoE model for 1098** — confirm 1098 keeps TFS `MatrixArea` (default) vs migrating to circles;
   772 is settled on the `circles.dat` disc.
7. **Lua spell-scripting plumbing** — ✅ **Addressed by PC-2b.** Audit complete: `Combat`/
   `Spell`/`Weapon`/`Condition` userdata, `createCombatArea` global, ~860 combat enums, and
   spellword dispatch (`Say` → `onCastSpell`) are all absent from `tfs-rust-lua` (existing
   userdata: `Container`/`Item`/`Creature` only). PC-2b (§3 PC-2b) implements the full plumbing
   in 8 ordered steps: enum registry → `createCombatArea` → `Combat` → `Condition` → `Weapon`
   → `Spell` → script loaders → spellword dispatch seam. PC-3 (wand data) and PC-3a
   (spell-casting) both depend on PC-2b. Full C++ reference mapping: `luascript.cpp:1115`
   (`createCombatArea`), `:2855-2871` (`Combat`), `:2874-2895` (`Condition`),
   `:3095-3137` (`Spell`), `:3209-3246` (`Weapon`), `:1200-2050` (enum block),
   `game.cpp:3579-3584` + `spells.cpp:30` (spellword dispatch).

---

## 9. Monster melee audit — decompile comparison (PC-2a)

Side-by-side audit of `monster_do_attacking` (melee arm) against C++ `Attack` → `CloseAttack` →
`Damage` (`crcombat.cc:531,648`, `crmain.cc:530`).

### 9.1 Correct matches
| C++ behavior | Rust | Ref |
|---|---|---|
| `DelayAttack(200)` before strike | `delay_attack_ms(server_ms, 200)` | `monster_ai.rs:393,496` |
| `DelayAttack(2000)` after strike | `delay_attack_ms(server_ms, 2000)` | `monster_ai.rs:473,584` |
| `DelayAttack` max-operation | `.max()` semantics | `base.rs:179` |
| Monster attack mode = BALANCED | `FightMode::Balanced` | `monster_ai.rs:404,506` |
| Defense gate 2000ms | `roll_target_defense` checks `earliest_defend_ms` | `monster_combat.rs:292` |
| `Following \|\| AttackDest==0` → DEFENSIVE | `defend_fight_mode_for_target` | `monster_combat.rs:201` |
| `ProbeValue` formula | `probe_value` matches exactly | `combat/math.rs:122-148` |
| `Damage = max(0, Attack-Defense)` pre-armor | `melee_damage_after_defense_and_armor` | `combat/math.rs:231` |
| `Damage <= 0` → `EFFECT_POFF` (3) | poff broadcast | `monster_ai.rs:531-535` |
| Armor absorbs all → `EFFECT_BLOCK_HIT` (4) | spark broadcast | `monster_ai.rs:531-535` |
| `DamageStimulus` only when post-armor > 0 | `if stimulus_damage > 0` gate | `idle_stimulus.rs:231` |
| `DamageStimulus` before HP apply | `monster_damage_stimulus` before `combat::execute` | `idle_stimulus.rs:233` |
| HP apply + damage map | `apply_health_delta` + `damage_map` | `combat/mod.rs:108-116` |
| Race-keyed `GraphicalEffect` + splash | `apply_physical_hit_blood` | `monster_inventory.rs:508-513` |
| Poison-on-hit condition | `melee_poison_on_hit` | `monster_combat.rs:319-320` |
| Poison damage `random(Poison/2, Poison)` | `uniform_random(rng, half, poison_cycles)` | `monster_combat.rs:331` |
| Armor randomized `(A/2)+rand%(A/2)` when `A>=2` | `armor_reduction` Randomized arm | `combat/math.rs:204-224` |
| Monster defense skill = `FistFighting` | `defense_skill: m.melee_skill` | `monster_combat.rs:227` (fixed this session) |

### 9.2 Actionable gaps — all assigned to phases

| # | C++ behavior | Rust gap | Phase | Impact |
|---|---|---|---|---|
| **A1** | `if (Target->IsDead) StopAttack(0)` (`crcombat.cc:643-645`) | ✅ Fixed — `monster_ai.rs` clears `attack_target`/`follow_target` on kill | ✅ PC-2a | Monster keeps attacking dead target. |
| **A2** | `if (DamageDone>0) ActivateLearning()` (`crcombat.cc:664-666`) | ✅ Fixed — `activate_learning()` called in monster melee path | ✅ PC-2a | Monsters never gain skill exp. Low impact. |
| **A3** | Race-keyed `TextualEffect` color (`crmain.cc:712-755`) | ✅ Fixed — `damage_text_color(blood)` in `monster_inventory.rs` | ✅ PC-2a | Wrong damage text color for non-blood races. |
| **M2** | Equipment `PROTECTION`+`CLOTHES` damage reduction (`crmain.cc:540-574`) | ✅ Fixed — `player_absorb_percent` in `idle_stimulus.rs` | ✅ PC-2a | Damage reduction items have no effect. |
| **M3** | Physical immunity `NoHit` → `EFFECT_BLOCK_HIT` (`crmain.cc:615-622`) | ✅ Fixed — `immunity_physical` field + check in `idle_stimulus.rs` | ✅ PC-2a | Immune monsters still take physical damage. |
| **M4** | Invisibility removal on hit (`crmain.cc:636-641`) | ✅ Fixed — `clear_nonplayer_invisibility` in `idle_stimulus.rs` | ✅ PC-2a | Invisible monsters stay invisible when hit. |
| **M10** | "You are poisoned." status message (`crcombat.cc:675`) | ✅ Fixed — `send_player_status_message` in `game_world_chat.rs` | ✅ PC-2a | Player gets no poison notification. |
| **M11** | Shield wearout `REMAININGUSES` (`crcombat.cc:265-281`) | ✅ Fixed — `player_shield_wearout` in `player/combat/strike.rs` | ✅ PC-2a | Shields never degrade. Player-only. |
| **M5** | Mana shield `SKILL_MANASHIELD` (`crmain.cc:662-689`) | Not implemented | PC-3 | Needed when typed damage (wand/spell) lands. |
| **M3′** | Non-physical immunities `NoPoison`/`NoBurning`/`NoEnergy` (`crmain.cc:615-622`) | Not implemented | PC-3 | Needed when wand/spell damage types are introduced. |
| **M1** | `INVULNERABLE` right check (`crmain.cc:536-538`) | Not implemented | PC-4 | GMs take damage. |
| **M6** | `BlockLogout(60)` (`crcombat.cc:601-602`) | Not implemented | PC-4 | No logout lock after combat. |
| **M8** | `SecureMode` PvP check (`crcombat.cc:563-568`) | Not implemented | PC-4 | No PvP safety. |
| **M9** | `RecordAttack` for PvP (`crcombat.cc:530-532,604-606`) | Not implemented | PC-4 | No PvP skull system. |
| **M7** | Player death penalty — amulet of loss, inventory drop (`crmain.cc:790+`) | Not implemented | PC-5 | No death penalty for player victims. |
| **M12** | Defense `LearningPoints` for shield skill (`crcombat.cc:259-263`) | Not implemented | PC-5 | Shield skill never gains exp. Player-only. |
| **M13** | `TSkillAdd::Advance` raises *current* HP/mana by gain on level-up (`crskill.cc:667-678`, via `TSkillLevel::Jump:355-382`) | Our `add_experience`/`remove_experience` clamps to new max instead of adding the per-level gain | PC-5 | Player at full HP leveling up stays at old max — level-up HP/mana gain is lost. 1098 refills to full (`player.cpp:1800-1802`); 772 does not — era-gate the refill. |
