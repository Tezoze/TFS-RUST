# Player Combat System — Implementation Plan (772 mechanics, TVP data shape)

**Goal.** Wire player combat (weapon strikes + spells) into the unified ToDo engine so
**formulas and flow match the 7.72 decompile** (`tibia-game-master/src/`), while tunable
data stays TVP-shaped (`data/defs/vocations.lua`, `items.otb`/`items.xml`, Lua weapon/spell
scripts). Outcomes only — no C++ transcription. Era knobs in `MechanicsProfile` /
`data/formulas/772.lua`; vocation balance in `data/defs/vocations.lua`.

**Active target:** `clientVersion = 772` (`config.lua`). 1098 shares the same paths via
`MechanicsProfile` — no `if version == 772` in core.

---

## Where we are (audit 2026-07-18)

**Weapon combat + spell execution core is landed.** **PC-5** (skill tries /
learning advance, shield skill, death penalty, level-up vitals) is **done**.

| Layer | Status |
|-------|--------|
| Melee / distance / wand strikes | ✅ Done (`strike.rs`, `ranged.rs`) |
| Shared `Damage` path (armor absorb, immunities, mana shield, INVULNERABLE) | ✅ Done (`idle_stimulus.rs`) |
| Lua Combat/Spell/Weapon + `Combat:execute` AoE | ✅ Done (PC-2b + PC-3a) |
| Fight / chase / secure mode + BlockLogout | ✅ Done (PC-4; skulls deferred) |
| Skill tries → level-up, shield learning, death loot/penalty | ✅ **PC-5 done** |
| Config rates (`rateExp`+stages / `rateSkill` / `rateMagic`) | ✅ Done (exp earlier; skill/magic post-PC-5) |
| Skulls / frags / aggressor | ⏳ Deferred (PvP phase) |

**Commits (recent landing):** `8b03bd1` PC-3 · `8fb81bc` PC-4 · `ce33c98`/`7506370` PC-3a.

---

## Next steps — after PC-5

PC-5 is landed (including `rateSkill`/`rateMagic`). Remaining player-combat work:

1. **PvP phase** — skulls / aggressor / RecordAttack / frags (deferred).
2. **Optional rates** — wire `rateLoot` / `rateSpawn` when loot/spawn need config parity.
3. **PC-3a residual polish** (not blocking):

- Burst arrow AoE still hits primary target only (Lua `Combat:execute` / circle path not wired from ammo special).
- Missed ammo: always delete (no ground `Move` drop arm); fragility/`breakChance` Lua not wired.
- `COMBAT_FORMULA_SKILL` in `Combat:execute` deferred (needs weapon resolution).
- `createCombatArea` diagonal overlay accepted but unused.
- TFS-style `spell_group_cooldown_end` exists on `Player` but say-spell uses 772
  `EarliestSpellTime` (correct for primary target); group CD is 1098/TFS surface if needed later.

---

## Phase status summary

| Phase | Description | Status |
|-------|-------------|--------|
| PM | Player module consolidation (file moves) | ✅ Done |
| PC-0 | Vocation data migration (XML→Lua) | ✅ Done |
| PC-1 | Player attack/defend/armor value resolution | ✅ Done |
| PC-2 | The strike (`CloseAttack`) — melee first | ✅ Done |
| PC-2a | `Damage` path completeness (melee audit) | ✅ Done (§9.2) |
| PC-2b | Lua combat/spell/weapon plumbing | ✅ Done |
| PC-3 | Distance + wand strikes | ✅ Done (`ranged.rs`; M5 mana shield; M3′ typed immunities; `probe_hit`) |
| PC-3a | AoE disc + spell-casting execution | ✅ Done (core) — residual polish above |
| PC-4 | Fight/chase/secure mode + PVP gating | ✅ Done (skulls deferred) |
| PC-5 | Skill/exp gain + regen + death penalty | ✅ Done |
| PvP | Skulls / aggressor / frags | ⏳ Deferred |

---

## 1. Current state

### 1.1 Landed
| Piece | Location | Notes |
|-------|----------|-------|
| Combat math (probe, defense, armor, spell, exp, skill tries formula, condition ticks) | `combat/math.rs` | Pure fns over profile + hooks; `probe_hit` for distance. |
| Damage application | `combat/mod.rs`, `idle_stimulus.rs::combat_execute_with_stimulus` | Absorb %, typed immunities, mana shield, INVULNERABLE, death, `DamageStimulus`. |
| Player melee / distance / wand | `player/combat/{strike,ranged,values,mod,skills}.rs` | Strikes + LP + `skill_increase`/`magic_increase` (× `rateSkill`/`rateMagic`). |
| Config rates | `config.rs` + call sites | `rateExp`/`experience_rate_for_level`; `rateSkill`/`rateMagic` via `scale_tries`. |
| AoE disc + Lua combat execute | `combat/circles.rs`, `combat/aoe.rs`, Lua `Combat:execute` | Unified 772 disc rings; matrix path for custom shapes; `throw_possible` LoS. |
| Spellword cast | `game_world_chat.rs::player_say_spell` | Voc/level/mana/soul + exhaustion + PZ + BlockLogout + `onCastSpell`. |
| Fight/chase/secure + BlockLogout | `player/combat/fight_mode.rs` | Skulls stubbed (`is_attack_justified` → false). |
| Monster melee + spells | `monster_ai.rs`, `monster_combat.rs` | Uses `disc_offsets` for radius spells. |
| Vocation / item weapon attrs | `vocations.lua`, OTB `ItemType`, `WandRegistry` | PC-0 / PC-2b / PC-3. |
| Fed regen from vocation | `process_skills.rs` | Wired via `VocationRegistry::fed_regen_params`. |

### 1.2 Still open
- **PvP phase** — `RecordAttack`, aggressor, skulls, murder/banishment, `protectionLevel`,
  `PVP_ENFORCED` / `NON_PVP` attack rules. Secure mode + BlockLogout + INVULNERABLE already wired.
- **PC-3a residual polish** — burst-arrow AoE, missed-ammo ground drop, `COMBAT_FORMULA_SKILL`.

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
+ profile `skillTuning`. ✅ PC-5: runtime tries + `skill_increase`/`magic_increase` on probes /
mana spend. **Rates (TFS `onGainSkillTries`):** call sites multiply by `config.rateSkill` /
`rateMagic` via `ConfigManager::scale_tries` before increase — not part of `skillTuning`.

### 2.8 Fight/chase/secure mode packet — `receiving.cc` `0xA7`, `crcombat.cc:333/354` `SetAttackMode`/`SetChaseMode`
`SetAttackMode` mode change → `DelayAttack(2000)`. `SetChaseMode` only NONE/CLOSE for players.
SecureMode stored on `TCombat`. Parse + chase-mode storage already exist (`game_parse.rs`
`C::FIGHT_MODES` → `GamePacket::FightModes`; `game_loop.rs` sets `chase_mode`). ✅ PC-4 wired
`attack_mode` (PC-1), `chase_mode`, and `secure_mode` into `Player` fields via
`player_set_fight_modes` in `player/combat/fight_mode.rs`.

### 2.9 Level / vitals — `data/defs/vocations.lua` gains + `crskill.cc:352` `GetExpForLevel`
Level exp `(((L-6)*L+17)*L-12)/6 * Delta` in `combat/math.rs::experience_for_level_poly`
(shared with `creature/vocation.rs::total_experience_for_level` — PC-5 cleanup). Kill XP uses
`ConfigManager::experience_rate_for_level` (`rateExp` + optional `experienceStages`). Vitals per
level from `data/defs/vocations.lua` via `VocationProfile::recalculate_vitals` (PC-0); M13 Advance
on level-up/down.

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

**Closed in PC-5:** `experience_for_level_poly` shared; `fed_regen_params` returns zeros when
vocation missing (no hardcoded `(12,1,6,2)` fallback).

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

### Phase PC-3 — Distance + wand strikes — ✅ DONE
**File:** `player/combat/ranged.rs`. **Commit:** `8b03bd1` (+ `d966744` typed hit effects).
**Prerequisite:** PC-2b (`Weapon(WEAPON_WAND)` → `WandRegistry`).

**What landed:**
1. `DistanceAttack` — ammo/range/`HitChance`, `probe_hit`, hit → attack roll × `dist_damage` +
   defense/armor (Q4 C++ semantics), miss → scatter tile + poff; ammo always consumed; poison
   arrow DoT; burst arrow primary-target physical (full AoE = residual).
2. `WandAttack` — mana/`WandDef` from `wands.lua`/`rods.lua`, fixed ± variation, typed damage,
   missile, LoS via `throw_possible`.
3. **M5** mana shield + **M3′** typed immunities in `combat_execute_with_stimulus`.

### Phase PC-3a — AoE shape model + spell-casting — ✅ DONE (core)
**Commits:** `ce33c98` (AoE modeling + cast mechanics + word matching), `7506370` (directional
AoE, damage broadcast, chat display). Residual polish listed under **Next steps**.

**What landed:**
1. `combat/circles.rs` — `DISC_RINGS` + `disc_offsets(radius)` (772 `circles.dat` rings 0–7;
   verified identical to 1098 `setupArea` rings 1–8). **No `AreaShapeModel`** — both eras share
   one disc; era variance was unnecessary.
2. `combat/aoe.rs` — `combat_execute_from_lua` iterates offsets, `throw_possible` LoS (caster
   origin for beams/waves, center for circle), PZ tile skip, damage via
   `combat_execute_with_stimulus`.
3. Lua `Combat:execute` — builds `CombatExecuteRequest` (matrix / formula / value callbacks) →
   mutation → core. Matrix path covers Lua `AREA_*` spells; ring path used by monster radius
   spells (`idle_stimulus` → `disc_offsets`).
4. Spellword execution in `player_say_spell` — vocation/level/mana/soul, 772 `EarliestSpellTime`
   exhaustion, aggressive-in-PZ reject, mana/soul deduct, `BlockLogout` on aggressive cast,
   `onCastSpell` → `Combat:execute`, chat broadcast of cast text.
5. Directional / need-direction spells + spectator damage feedback.

**Design notes (kept for reference):**
- Matrix vs ring offsets both feed the same damage layer; custom cones/waves stay on `MatrixArea`.
- Combat LoS = `throw_possible` only (not TFS `is_sight_clear`).
- Per-spell radius override files (`*_spell_areas.lua`) **not needed** yet — UE
  `AREA_CIRCLE5X5` == disc R6 verified. Add only if a matrix diverges from decompile radius.
- Burst-arrow full AoE + ammo ground-drop + `COMBAT_FORMULA_SKILL` remain residual (see top).

### Phase PC-4 — Fight/chase/secure mode + PVP gating — ✅ DONE
**Files:** `player/combat/fight_mode.rs` (new), `game_loop.rs` (`FightModes` arm),
`player/combat/mod.rs` (`validate_player_attack_target` + `player_execute_attack`),
`idle_stimulus.rs` (`combat_execute_with_stimulus`), `player/flags.rs`,
`creature/player.rs`, `config.rs` (`PvpConfig`), `game_world.rs`.

1. ~~Parse `0xA7`~~ — **already done** (`GamePacket::FightModes`). No net-side work needed.
2. ✅ Core setter: `player_set_fight_modes` in `player/combat/fight_mode.rs` — writes
   `attack_mode` (with `DelayAttack(2000)` on change per `crcombat.cc:334`), `chase_mode`
   (does not override `Close` forced by active follow), and `secure_mode: bool` (new field on
   `Player`). The `game_loop.rs` `FightModes` arm now calls this instead of only setting
   `chase_mode`.
3. ✅ PVP gating: `validate_player_attack_target` now enforces `CombatResult::SecureMode`
   (secure-mode PVP gate, `crcombat.cc:374-381`) and `CombatResult::AttackNotAllowed`
   (`CheckRight(NO_ATTACK)` → `PLAYER_FLAG_CANNOT_USE_COMBAT`, `crcombat.cc:391-394`).
   `player_execute_attack` re-checks both at strike time (`crcombat.cc:563-593`) and calls
   `BlockLogout(60)` on attacker + target (`crcombat.cc:601-602`). `SetAttackDest` `!Follow`
   also calls `BlockLogout(60)` on the attacker (`crcombat.cc:434`).
4. ✅ **M1 — INVULNERABLE right check:** `PLAYER_FLAG_CANNOT_BE_ATTACKED` (TFS `PlayerFlag` bit 3,
   mapped from `groups.xml` `cannotbeattacked`) checked at the top of
   `combat_execute_with_stimulus` — zeroes incoming damage + emits `EFFECT_POFF` (3) when the
   target player has the flag (`crmain.cc:536-538`). Uses the existing group-flag system (same
   model as `IGNORED_BY_MONSTERS`), not a separate 772 `CharacterRights` DB table.
5. ✅ **M6 — BlockLogout(60):** `player_block_logout` in `fight_mode.rs` sets
   `earliest_logout_round` + `earliest_protection_zone_round` (new field) per `crmain.cc:433-453`.
   Called from `player_execute_attack` (attacker + target, every strike) and `player_set_attack_dest`
   (attacker, `!Follow` only). `NON_PVP` worlds clear the PZ block (`crmain.cc:434-436`).
6. **Deferred to PvP skull phase** (Q5 resolved: defer all skulls). The following 772
   PvP subsystem pieces were **not** implemented in PC-4 and belong in a dedicated PvP phase:

   | Item | C++ reference | Status | Notes |
   |------|---------------|--------|-------|
   | `IsAttackJustified` | `crplayer.cc:1438-1460` | **Stub** (`false`) | `player_is_attack_justified` in `fight_mode.rs` returns `false` — no aggressor/party/attacked-players tracking. Secure mode blocks **all** PvP in `WorldType::Pvp` until this lands. |
   | `RecordAttack` | `crcombat.cc:530-532,604-606` | **Not implemented** | Aggressor flag + `AttackedPlayers` list + skull broadcast (`CREATURE_SKULL_CHANGED`). Would set `aggressor` on the attacker and add the victim to `attacked_players`. |
   | Aggressor flag | `crplayer.cc` `Aggressor` field | **Not implemented** | New `Player` field needed (`aggressor: bool` or `aggressor_until_round: u32`). Drives `IsAttackJustified` + skull display. |
   | `AttackedPlayers` list | `crplayer.cc` `AttackedPlayers` | **Not implemented** | New `Player` field needed (`attacked_players: Vec<CreatureId>` or similar). Drives `IsAttackJustified` party/attacker check. |
   | Skull broadcast | `crmain.cc` `CREATURE_SKULL_CHANGED` | **Not implemented** | Wire `0x90` (or era-equivalent) skull-change broadcast when `RecordAttack` sets/clears a skull. |
   | `RecordMurder` | `crplayer.cc` `RecordMurder` | **Not implemented** | On player kill: increment frag count, set skull to `Red`/`White`, start playerkiller timer. |
   | Playerkiller timer (30 days) | `crplayer.cc` `PlayerkillerEnd` | **Not implemented** | New `Player` field (`playerkiller_end: u32` or `DateTime`). Drives skull expiry + banishment threshold. |
   | Murder timestamps | `crplayer.cc` `MurderTimestamps` | **Not implemented** | New `Player` field (`murder_timestamps: Vec<u32>`). Drives banishment threshold (e.g. 5 murders in 30 days). |
   | Banishment | `crplayer.cc` banishment logic | **Not implemented** | On threshold: kick + DB ban record. Requires DB schema + auth integration. |
   | PK-mark clearing | `crmain.cc:1102-1105` `ClearPlayerkillingMarks` | **Stub exists** | `creature_think.rs` clears `earliest_logout_round` when the timer expires — but the actual skull-clear + `CREATURE_SKULL_CHANGED` broadcast is not wired (no skulls to clear yet). |
   | `protectionLevel` enforcement | `crcombat.cc` level gate | **Not enforced** | `PvpConfig.protection_level` is loaded from `config.lua` and stored on `GameWorld`, but the actual "both players must be ≥ protection_level" check is not wired (deferred with skulls — it's part of the same PvP gate). |
   | `PVP_ENFORCED` damage boost | `crcombat.cc` `WorldType == PVP_ENFORCED` | **Not implemented** | In `PvpEnforced` worlds, damage is boosted (1.5× in 772). No `MechanicsProfile` knob for this yet. |
   | `NON_PVP` attack block | `crcombat.cc` `WorldType == NON_PVP` | **Not implemented** | In `NoPvp` worlds, player-vs-player attacks are blocked entirely (separate from secure mode). Currently only the PZ-block clearing in `BlockLogout` honors `NoPvp`. |

   **What PC-4 *did* wire** (so the PvP phase can build on it):
   - `PvpConfig { world_type, protection_level }` loaded from `config.lua` onto `GameWorld.pvp_config`.
   - `secure_mode: bool` + `earliest_protection_zone_round: u32` fields on `Player`.
   - `player_secure_mode_blocks_attack` gate (fires only when `WorldType == Pvp`).
   - `player_block_logout` (honors `NoPvp` PZ-block clearing).
   - `PLAYER_FLAG_CANNOT_USE_COMBAT` + `PLAYER_FLAG_CANNOT_BE_ATTACKED` group flags.
   - `CombatResult::SecureMode` + `CombatResult::AttackNotAllowed` branches in
     `validate_player_attack_target` + `player_execute_attack`.

### Phase PC-5 — Skill/exp gain + regen from vocation data — ✅ DONE
**Files:** `process_skills.rs`, `death.rs`/`game_world_lifecycle.rs`, `creature/player.rs`,
`combat/math.rs`, `player/combat/skills.rs`, `data/formulas/772.lua`, `config.rs`, login/save path.

**What landed:**
1. `SkillTriesTuning` on `MechanicsProfile` + `skillTuning` in `772.lua`/`1098.lua`
   (tries *needed* per level — distinct from probe `damageTuning.skillBase`).
2. Runtime `PlayerSkills` tries (7 combat + `manaspent`) + `Player.blessings`; login/save wired.
3. `skill_increase` / `magic_increase` on probe (strike/ranged) and mana spend (wand/spell).
4. **M12** — shield learning in `player_shield_skill_learning` after defend gate.
5. **M13** — `add_experience`/`remove_experience` Advance current HP/mana by gain.
6. **M7** — AoL (2173) consume, SOME inventory drop to corpse 3128, bless-reduced exp+skill loss.
7. Cleanup — `experience_for_level_poly` shared; `fed_regen_params` no hardcoded fallback.
8. **Config rates (TFS `onGainSkillTries`)** — `rateSkill` / `rateMagic` multiply gained tries via
   `ConfigManager::scale_tries` at strike/ranged/shield/wand/spell call sites. `rateExp` +
   `expStages` / `experienceStages` already on kill XP (`experience_rate_for_level`). Curve knobs
   stay in formulas Lua; rates stay in `config.lua` (missing skill/magic → `1.0`).
   `rateLoot` / `rateSpawn` getters exist; loot/spawn consumers not wired yet.

---

## 4. Architecture / placement rules

### 4.0 Module layout
```
crates/tfs-rust-core/src/player/combat/
  mod.rs              # attack dispatch / validate
  values.rs           # attack/defend/armor resolution (PC-1)
  strike.rs           # CloseAttack (PC-2) + shield learning (M12)
  ranged.rs           # DistanceAttack + WandAttack (PC-3)
  skills.rs           # skill_increase / magic_increase (PC-5)
  fight_mode.rs       # fight/chase/secure + BlockLogout (PC-4)

crates/tfs-rust-core/src/combat/
  math.rs             # probe / armor / skill-tries formulas
  circles.rs          # DISC_RINGS + disc_offsets (PC-3a)
  aoe.rs              # combat_execute_from_lua (PC-3a)
  mod.rs / pvp.rs / rng.rs

crates/tfs-rust-lua/src/userdata/
  combat.rs / condition.rs / weapon.rs / spell.rs   # PC-2b + PC-3a execute
crates/tfs-rust-lua/src/
  combat_enums.rs / combat_scripts.rs
crates/tfs-rust-content/src/
  weapons.rs / spells.rs / vocations.rs
```

### 4.1 Steering compliance
- **Formulas & flow** in `tfs-rust-core` combat modules; **no** `NetworkMessage`/opcode bytes in
  core (`0xA7` parse stays in `tfs-rust-net`).
- **Era knobs** in `MechanicsProfile` / `data/formulas/772.lua`. **Per-vocation balance** in
  `data/defs/vocations.lua`. **Server rates** (`rateExp`/`rateSkill`/`rateMagic`/…) in
  `config.lua` via `ConfigManager` — not in formulas. No new balance literals in Rust.
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
| `skill_*_tries: u64` (8 fields) | `PlayerSkills` (runtime) | DB columns already on `PlayerRecord` / save path | ✅ PC-5 |
| Per-skill `skill_base` `{50,50,50,50,30,100,20}` | `MechanicsProfile` / `772.lua` | `human.mon` Delta — distinct from probe `skillBase=50` | ✅ PC-5 |
| `min_level` per skill (10 combat, 0 magic) | `MechanicsProfile` / `772.lua` | `human.mon` Min | ✅ PC-5 |
| `rateSkill` / `rateMagic` | `config.lua` → `ConfigManager` | TFS `onGainSkillTries` gain multipliers | ✅ Done |
| `rateExp` + `expStages` / `experienceStages` | `config.lua` → `ConfigManager` | Kill XP (`experience_rate_for_level`) | ✅ Done (pre-PC-5) |
| `rateLoot` / `rateSpawn` | `config.lua` getters only | TFS loot/spawn rates | ⏳ Getters only |
| `attack_mode: FightMode` | `Player` | `0xA7` packet | ✅ PC-1 |
| `secure_mode: bool` | `Player` | `0xA7` packet | ✅ PC-4 |
| `VocationDef` (full combat block) | `tfs-rust-content` | `data/defs/vocations.lua` | ✅ PC-0 |
| `VocationProfile` (`Copy`) snapshot | `Player.vocation_profile` | `VocationRegistry` at login | ✅ PC-0 |
| Wand attributes (`WandDef`) | `tfs-rust-content` `WandRegistry` | `data/scripts/weapons/wands.lua` + `rods.lua` (TFS Lua `Weapon(WEAPON_WAND)` API) | ✅ PC-2b (loader) / ✅ PC-3 (consumer) |
| `WeaponDef` (distance/melee/ammo) | `tfs-rust-content` `WeaponRegistry` | `data/scripts/weapons/*.lua` (`Weapon(WEAPON_*)` API) | ✅ PC-2b |
| `SpellDef` (instant/rune) | `tfs-rust-content` `SpellRegistry` | `data/scripts/spells/**/*.lua` (`Spell(SPELL_*)` API) | ✅ PC-2b |
| `CombatDef` (Lua-side combat config) | `tfs-rust-lua` userdata | `Combat()` + `:setParameter`/`:setArea`/`:setCallback`/`:execute` | ✅ PC-2b |
| `ConditionDef` (Lua-side condition config) | `tfs-rust-lua` userdata | `Condition(CONDITION_*)` + `:setParameter`/`:setTicks` | ✅ PC-2b |
| Combat/spell/weapon enums (~860) | `tfs-rust-lua` globals | `tfs-rust-common` enums → `register_combat_enums(&lua)` | ✅ PC-2b |
| `DISC_RINGS` + `disc_offsets` | `combat/circles.rs` | 772 `circles.dat` (== 1098 setupArea) | ✅ PC-3a |
| `AreaShapeModel` | — | Abandoned — single shared disc | ❌ N/A |
| Per-spell radius overrides | `data/formulas/*_spell_areas.lua` | Only if matrix ≠ decompile | ⏳ Not needed yet |

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
- **Circles parity** (`combat/circles.rs`) — ✅ unit tests for ring counts + `disc_offsets(6)`.
- **Spellword / AoE** — ✅ seam + directional AoE landed; end-to-end goldens vs live scripts can
  still grow (berserk, UE radius, PZ deny).
- **Integration** (`sim_harness`): damage ranges, defense gate 2000 ms, ammo/wand costs,
  PZ/secure deny. **PC-5 adds:** learning advances skill tries, death → penalty + exp share.
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

### 8.1 Resolved

1. **772 starting vitals floor** — ✅ **Resolved for 772.** The 772 decompile has **no vocation
   change mechanic** — "vocation" is a display string only (`operate.cc:1854-1915`). Per-level
   gains (`AddLevel`) come from `RaceData[Race].Skill` via `TSkillBase::SetSkills(Race)`
   (`crskill.cc:1165-1188`) — i.e. race data (`human.mon`), not a swappable vocation object.
   `AddLevel` never changes mid-life, so `base + gain*(level-1)` is **always correct for 772**.
   The "vocation change at level > 1" divergence is a **1098-only concern** (TFS has
   `setVocation`/vocation change); defer to the 1098 era work. **However**, the audit surfaced a
   real bug — see M13 (§3 PC-5 / §9.3): C++ `TSkillAdd::Advance` raises *current* HP/mana by the
   gain on level-up. ✅ Fixed in PC-5 (`add_experience`/`remove_experience` Advance).
2. **Skill-tries mapping** — ✅ **Resolved (PC-5).** `FactorPercent = 1000 * multiplier`;
   `skill_base = Delta`; per-level tries + `skillTuning` in formulas Lua; magic via
   `magic_skill_base=1600` / `mana_multiplier`. **Rates** are separate: `config.lua`
   `rateSkill`/`rateMagic` (and `rateExp`/stages) — not formula knobs.
3. **Wand data source** — ✅ **Resolved.** Wand/rod attributes live in
   `data/scripts/weapons/wands.lua` and `rods.lua` via the TFS Lua `Weapon(WEAPON_WAND)` API
   (`level`/`mana`/`element`/`damage(min,max)`/`vocation`/`id`/`register`). Rods register as
   `WEAPON_WAND` with druid vocations — one loader covers both files. No `items.xml` or
   `objects.srv` parsing required. PC-3 loads these into a `WandDef` registry on
   `tfs-rust-content` keyed by `item_id`; the `Weapon` userdata plumbing (Q7) is the prerequisite.
7. **Lua spell-scripting plumbing** — ✅ **Done (PC-2b + PC-3a).** Userdata + loaders in
   PC-2b; `Combat:execute` + cast gates/exhaustion/PZ/`onCastSpell` in PC-3a.
5. **Skulls / PVP frags** — ✅ **Resolved: defer all skulls to a dedicated PvP phase** (see phase
   status summary). PC-4 wired the non-skull PvP gates (secure mode, BlockLogout, INVULNERABLE);
   skulls/aggressor/RecordAttack/RecordMurder/banishment are all deferred.
6. **AoE model for 1098** — ✅ **Resolved: 1098 uses circles too.** TFS `AreaCombat::setupArea(radius)`
   (`combat.cpp:1391`) is the same disc concept as 772 `circles.dat` — a grid of tiles organized by
   ring distance from center. Both eras use the circle-ring model for radius-based AoE; custom
   non-circular shapes (cones, waves) use `MatrixArea` in both eras. Era difference is just the disc
   grid size (772: 21x21 max ring 7; 1098: 13x13 max ring 8).

### 8.2 Open

- `rateLoot` / `rateSpawn` — config getters present; apply to loot roll / spawn interval when those
  paths need TFS rate parity.
- 1098 full HP/mana refill on level-up (era-gate) if that profile should diverge from 772 Advance.

4. **Ranged defense "bug"** (`crcombat.cc:766`) — ✅ **Resolved in PC-3.** `player_distance_attack`
   rolls `roll_target_defense` + armor on hit (comments note C++ likely-bug semantics: defense
   applies when the defender has a shield). Wearout follows the shared defend path. No further
   action unless live play diverges.

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

### 9.2 Resolved gaps

| # | C++ behavior | Fix | Phase |
|---|---|---|---|
| **A1** | `if (Target->IsDead) StopAttack(0)` (`crcombat.cc:643-645`) | `monster_ai.rs` clears `attack_target`/`follow_target` on kill | ✅ PC-2a |
| **A2** | `if (DamageDone>0) ActivateLearning()` (`crcombat.cc:664-666`) | `activate_learning()` called in monster melee path | ✅ PC-2a |
| **A3** | Race-keyed `TextualEffect` color (`crmain.cc:712-755`) | `damage_text_color(blood)` in `monster_inventory.rs` | ✅ PC-2a |
| **M2** | Equipment `PROTECTION`+`CLOTHES` damage reduction (`crmain.cc:540-574`) | `player_absorb_percent` in `idle_stimulus.rs` | ✅ PC-2a |
| **M3** | Physical immunity `NoHit` → `EFFECT_BLOCK_HIT` (`crmain.cc:615-622`) | `immunity_physical` field + check in `idle_stimulus.rs` | ✅ PC-2a |
| **M4** | Invisibility removal on hit (`crmain.cc:636-641`) | `clear_nonplayer_invisibility` in `idle_stimulus.rs` | ✅ PC-2a |
| **M10** | "You are poisoned." status message (`crcombat.cc:675`) | `send_player_status_message` in `game_world_chat.rs` | ✅ PC-2a |
| **M11** | Shield wearout `REMAININGUSES` (`crcombat.cc:265-281`) | `player_shield_wearout` in `player/combat/strike.rs` | ✅ PC-2a |
| **M1** | `INVULNERABLE` right check (`crmain.cc:536-538`) | `PLAYER_FLAG_CANNOT_BE_ATTACKED` group flag check in `combat_execute_with_stimulus` | ✅ PC-4 |
| **M6** | `BlockLogout(60)` (`crcombat.cc:601-602`) | `player_block_logout` in `fight_mode.rs`; called at strike + `SetAttackDest` | ✅ PC-4 |
| **M8** | `SecureMode` PvP check (`crcombat.cc:563-568`) | `player_secure_mode_blocks_attack` gate in `validate_player_attack_target` + `player_execute_attack` | ✅ PC-4 |

### 9.2b Resolved in PC-3 / PC-3a (post-melee audit)

| # | C++ behavior | Fix | Phase |
|---|---|---|---|
| **M5** | Mana shield (`crmain.cc:662-689`) | `apply_mana_shield` in `idle_stimulus.rs` before HP apply | ✅ PC-3 |
| **M3′** | Typed immunities fire/energy/poison/life-drain (`crmain.cc:615-622`) | Flags on monster + check in `combat_execute_with_stimulus` | ✅ PC-3 |

### 9.3 Pending gaps

| # | C++ behavior | Rust gap | Phase | Impact |
|---|---|---|---|---|
| **M9** | `RecordAttack` for PvP (`crcombat.cc:530-532,604-606`) | Deferred with skulls (Q5) | ⏳ PvP | No PvP skull system. |

### 9.3b Closed in PC-5 (+ config rates)

| # | C++ / TFS behavior | Fix | Status |
|---|---|---|---|
| **M7** | Player death — AoL, SOME drop (`crmain.cc:790+`) | AoL 2173 + corpse 3128 + bless/skill loss | ✅ Done |
| **M12** | Shield skill `Increase` on defend (`crcombat.cc:259-263`) | `player_shield_skill_learning` | ✅ Done |
| **M13** | `Advance` current HP/mana on level-up (`crskill.cc:667-678`) | `add_experience`/`remove_experience` | ✅ Done |
| **Rates** | TFS `onGainSkillTries` × `RATE_SKILL`/`RATE_MAGIC`; exp × `RATE_EXPERIENCE`/stages | `scale_tries` + `experience_rate_for_level` | ✅ Done |
