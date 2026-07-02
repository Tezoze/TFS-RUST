# Player Combat System — Implementation Plan (772 mechanics, TVP data shape)

**Goal.** Wire the player weapon-combat *strike* into the unified ToDo engine so that
**formulas and flow match the CipSoft 7.72 decompile** (`tibia-game-master/src/`), while all
**tunable data lives in TVP-shaped files** (`data/XML/vocations.xml` combat block; weapon
attack/defense/armor from `items.otb`/`items.xml`). No CipSoft source is transcribed — we replicate
observable outcomes in idiomatic Rust (`tfs-core.md` porting model). Era knobs stay in
`MechanicsProfile` / `data/formulas/772.lua`; per-vocation balance stays in `vocations.xml`.

**Active target:** `clientVersion = 772` (`config.lua:17`). 1098 shares the same code paths through
`MechanicsProfile`; this plan does not add version branches to core.

---

## 1. Current state (audit)

### 1.1 Already implemented and correct
| Piece | Location | Notes |
|-------|----------|-------|
| Combat math (probe, defense, armor, spell, exp, skill tries, condition ticks) | `combat/math.rs` | Pure fns over `MechanicsProfile` + `FormulaHooks`; unit-tested; matches `crskill.cc`/`crcombat.cc` outcomes. |
| Damage application (HP/mana/conditions/dispel, damage_map) | `combat/mod.rs`, `idle_stimulus.rs::combat_execute_with_stimulus` | Death + `DamageStimulus` already wired for the monster path. |
| Attack targeting / chase routing (`SetAttackDest`/`CanToDoAttack`/`StopAttack`/cancel) | `player_combat.rs` | Routes attack/follow/cancel packets; **strike deferred** (see below). |
| Monster melee strike (attack roll, defense gate, armor, poison-on-hit) | `creature/monster_combat.rs` | The shape the player strike should mirror. |
| Player weapon accessors (`getWeapon`/`getWeaponType`/`getWeaponSkill`) | `player_inventory_util.rs` | Slot resolution + ammo pairing done. |
| Condition ticks + fed regen tick loop | `process_skills.rs` | Regen cadence **hardcoded**, not from `vocations.xml` (gap 3.4). |
| Item weapon attributes (`weapon_type`, `attack`, `defense`, `extra_defense`, `armor`, `ammo_type`, `attack_speed`, `shoot_range`, `hit_chance`) | `tfs-rust-content/src/otb.rs` `ItemType` | Data source for `GetAttackValue`/`GetDefendValue`/`GetArmorStrength`. |
| Fight-mode enum + modifiers | `combat/math.rs` `FightMode`, `formulas.rs` `FightModes` | 772 `+20/−40` atk, `−40/+80` def already tuned. |

### 1.2 The gap — player strike is a stub
`player_combat.rs::player_execute_attack` → `PlayerChaseOutcome::Adjacent` currently only
`DelayAttack(200)` + re-arms. The doc comment states plainly: *"The melee strike … is deferred — no
player weapon-combat system exists yet."* This plan fills that hole.

### 1.3 The gap — vocation combat data is dropped on load
`tfs-rust-content/src/vocations.rs` parses only `id`/`clientid`/`name`/`description`/`fromvoc`. The
active `data/XML/vocations.xml` already carries the full TVP block (`gaincap`, `gainhp`, `gainmana`,
`gainhpticks`, `gainhpamount`, `gainmanaticks`, `gainmanaamount`, `manamultiplier`, `attackspeed`,
`basespeed`, `soulmax`, `gainsoulticks`, `<formula meleeDamage/distDamage/defense/armor>`,
`<skill id multiplier>`). All of it is discarded today.

`creature/vocation.rs` is stubbed as a result: `vocation_base_speed()` returns a hardcoded `220`,
`per_level_gains()` uses "example" numbers, `recalculate_vitals()` uses `150 + hp_gain*(l-1)`
constants. These must read the loaded `VocationDatabase`.

---

## 2. Reference spec (CipSoft 7.72 — cite in code headers, do not copy)

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

### 2.3 ProbeValue — `crskill.cc:535` `TSkillProbe::ProbeValue`
```
if Increase: this.Increase(1)               # +1 skill exp (may level the skill)
RandomFactor = ((rand()%100) + (rand()%100)) / 2      # triangular 0..99
MaxValue    = Max * (skillValue*5 + 50)
Result      = (RandomFactor * MaxValue) / 10000
```
Already in `combat/math.rs::probe_value` (`skill_mult=5`, `skill_base=50`, `random_max=99`) with
glibc-rand parity hook. **Reuse verbatim.**

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
Gate + roll already in `monster_combat.rs::roll_target_defense`. Extend the player `GetDefendValue`
side (shield/weapon defend value, shielding skill, wearout).

### 2.5 Armor strength — `crcombat.cc:286` `GetArmorStrength`
```
Armor = sum(ARMORVALUE of equipped CLOTHES+ARMOR at correct BODYPOSITION) + RaceData.Armor
if Armor >= 2: Armor = (Armor/2) + rand()%(Armor/2)
```
Randomized reduction already in `combat/math.rs::armor_reduction` (772 profile). Add the player
inventory summation (`GetArmorStrength`). Note: applied **inside** `TCreature::Damage(PHYSICAL)`,
not in `CloseAttack`.

### 2.6 Strike dispatch — `crcombat.cc:531` `Attack` / `:648` `CloseAttack` / `:739` `DistanceAttack` / `:704` `WandAttack`
- Validation (target present, invisible-vs-player, secure/PVP `IsAttackJustified`, `NO_ATTACK`/
  `ATTACK_EVERYWHERE`, distance > 8, PZ) → most already in `validate_player_attack_target`; add
  secure-mode/PVP + rights.
- `BlockLogout(60)` on attacker + target; `RecordAttack` for PVP.
- `DelayAttack(200)` → strike by `GetDistance()` range (1 close / 2 throw / 3 missile-wand) →
  `DelayAttack(2000)`.
- `CloseAttack`: `Damage = max(0, GetAttackDamage − target.GetDefendDamage)`;
  `target.Damage(PHYSICAL)` (armor inside); `if DamageDone>0: ActivateLearning()`; race poison
  (monster-only); weapon wearout.
- `DistanceAttack`: ammo present/range; `HitChance` bow 90 / throw 75;
  `Difficulty = Distance>=2 ? Distance : 5`; `Probe(Difficulty*15, HitChance, learning)`; on hit
  `target.Damage(GetAttackDamage, PHYSICAL)` (the shield-defense call is a documented CipSoft bug —
  replicate outcome: no defense subtraction on ranged); on miss drop ammo at scattered tile;
  special effects (1 = poison arrow periodic, 2 = burst arrow area). Ammo consumption + fragility.
- `WandAttack`: `WANDRANGE`; `CheckMana(WANDMANACONSUMPTION)`; `Damage = AttackStrength +
  random(-Variation, +Variation)`; `WANDDAMAGETYPE`; missile.

### 2.7 Learning / skill advance — `crskill.cc:549` `Probe`, `:387` `Increase`, `crcombat.cc:526` `ActivateLearning`
`ActivateLearning()` sets `LearningPoints = 30`. Each `ProbeValue`/`Probe` with `LearningPoints>0`
calls `Increase(1)` then decrements. `Increase` adds to skill exp and levels when
`Exp >= NextLevel`; `NextLevel = GetExpForLevel(Act+1)` geometric with `FactorPercent`/`Delta`.
Maps to `combat/math.rs::req_skill_tries` — **fed by `vocations.xml` `<skill multiplier>`**
(→ `FactorPercent`, i.e. `1000 * multiplier` shape) rather than TFS `skillBase`.

### 2.8 Fight/chase/secure mode packet — `receiving.cc` `0xA0`, `crcombat.cc:333/354` `SetAttackMode`/`SetChaseMode`
`SetAttackMode` mode change → `DelayAttack(2000)`. `SetChaseMode` only NONE/CLOSE for players.
SecureMode stored on `TCombat`. Currently unwired (`player_combat.rs::CombatResult::SecureMode`
reserved). Add the `0xA0` parse → `GamePacket::FightModes` → core setter.

### 2.9 Level / vitals — `vocations.xml` gains + `crskill.cc:352` `GetExpForLevel`
Level exp `(((L-6)*L+17)*L-12)/6 * Delta` already in `combat/math.rs::experience_for_level`
(and `creature/vocation.rs::total_experience_for_level`, which uses an **equivalent expanded
polynomial** — consolidate onto one). Vitals per level from `vocations.xml` `gainhp`/`gainmana`/
`gaincap`, base speed from `basespeed` + level rule (772: `+level`; 1098: `+2*(level-1)`).

---

## 3. Work breakdown

### Phase PC-0 — Vocation combat data (data-driven, TVP shape)
**Files:** `tfs-rust-content/src/vocations.rs`, `crates/tfs-rust-core/src/creature/vocation.rs`,
`config.rs`/wiring where `VocationDatabase` is threaded.

1. Extend `content::Vocation` with the full TVP block: `gain_cap`, `gain_hp`, `gain_mana`,
   `gain_hp_ticks`, `gain_hp_amount`, `gain_mana_ticks`, `gain_mana_amount`, `mana_multiplier`,
   `attack_speed_ms`, `base_speed`, `soul_max`, `gain_soul_ticks`, `allow_pvp`,
   `formula: { melee_damage, dist_damage, defense, armor }`, `skill_multipliers: [f64; 7]`
   (indices = `SKILL_FIST..SKILL_FISHING`; note client skill ids 0..6 in XML).
2. Parse `<formula>` and `<skill>` child elements (loader currently ignores children — switch from
   `Event::Empty`-only vocation handling to tracking the open `<vocation>` scope).
3. Replace stubs in `creature/vocation.rs`:
   - `vocation_base_speed` → `VocationDatabase.get(id).base_speed`.
   - `per_level_gains` / `recalculate_vitals` → `gain_hp/gain_mana/gain_cap` with 772 base
     (`150` hp / `0` mana / `400` cap anchors confirmed against `crmain.cc` starting skills — verify).
   - Consolidate `total_experience_for_level` onto `combat::experience_for_level` (single source).
4. Thread `&VocationDatabase` (or a `Copy` per-vocation snapshot cached on `Player` at login) into
   `GameWorld` so combat/regen/level-up read it without a content dependency in hot paths.

**Test:** golden parse of `data/XML/vocations.xml` (knight skill[4]=1.4, sorcerer gainmana=30,
attackspeed=2000, basespeed=70); base-speed + vitals per level vs known 772 values.

### Phase PC-1 — Player attack/defend/armor value resolution
**File:** new `crates/tfs-rust-core/src/player_combat_values.rs` (core).

1. `player_get_attack_value(cid) -> (max_value, SkillNr)` — `GetAttackValue` from equipped
   weapon/ammo/throw/wand/fist via existing `player_get_weapon*` + `ItemType.attack`.
2. `player_get_defend_value(cid) -> (max_value, SkillNr)` — `GetDefendValue` (shield/weapon defend).
3. `player_get_armor_strength(cid) -> i32` — `GetArmorStrength` sum over `equipment_slots` armor
   pieces (uses `ItemType.armor` + body-position check via `slot_type_for_item_type`).
4. Fight-mode source: `Player.attack_mode` (new field; default `Balanced`).
   All cite `crcombat.cc` GetAttackValue/GetDefendValue/GetArmorStrength.

### Phase PC-2 — The strike (`CloseAttack`) — melee first
**Files:** `player_combat.rs` (replace the `Adjacent` stub), reuse
`combat::math::{weapon_damage, defense_value, armor_reduction}` +
`combat_execute_with_stimulus`.

1. On `PlayerChaseOutcome::Adjacent` with melee range (`GetDistance()==1`):
   - `attack = weapon_damage(profile, hooks, rng, skill, atk_value, mode, level)` × vocation
     `formula.melee_damage` (floor).
   - `defense = roll_target_defense(target, …)` (existing gate/roll; extend snapshot so a **player
     target** contributes shield/weapon defend value + shielding skill).
   - `damage = max(0, attack − defense)`; apply via `combat_execute_with_stimulus(Some(cid),
     target, PHYSICAL, -damage)` — armor mitigation handled in the shared physical path
     (`armor_reduction` with target `GetArmorStrength`).
   - `if damage_done>0`: `ActivateLearning()` on the player (set `LearningPoints=30`).
   - Weapon wearout (`REMAININGUSES`) if the item type has it.
   - Cadence: `DelayAttack(200)` before, `DelayAttack(2000)` after (vocation `attackspeed`), then
     re-arm `TDAttack`; `if target dead: StopAttack`.
2. Learning wiring: thread `LearningPoints` (new `Player`/`CreatureBase` field
   `learning_points: i32`) into the `Increase` flag of `probe_value` so skills advance only while
   learning is active — matches `GetAttackDamage`/`ProbeValue`.

### Phase PC-3 — Distance + wand strikes
**File:** `player_combat.rs` / `player_combat_values.rs`.

1. `DistanceAttack`: ammo resolution (already in `player_get_weapon`), range vs `shoot_range`,
   `HitChance` (bow 90 / throw 75), `Probe(Difficulty*15, HitChance, learning)` — add
   `combat::math::probe_hit(skill, diff, prob, rng)` mirroring `TSkillProbe::Probe`. On hit apply
   `GetAttackDamage` × `formula.dist_damage`; on miss scatter-drop ammo. Ammo consume + fragility;
   special effects (poison arrow → periodic poison condition; burst arrow → area physical via the
   existing shape helpers).
2. `WandAttack`: mana check against `Player.mana` (`WANDMANACONSUMPTION`), fixed
   `AttackStrength ± Variation`, wand `WANDDAMAGETYPE`, missile effect. Wand attack/damage-type
   attributes need `ItemType` fields (`wand_*`) — add to content parser if absent (verify against
   `items.xml` wand entries + `objects.srv`).

### Phase PC-4 — Fight/chase/secure mode + PVP gating
**Files:** `tfs-rust-net` `game_parse.rs` (`0xA0` → `GamePacket::FightModes`), `player_combat.rs`.

1. Parse `0xA0` → `{ attack_mode, chase_mode, secure_mode }` semantic variant (no raw bytes in core).
2. Core setter: `SetAttackMode` (change → `DelayAttack(2000)`), `SetChaseMode` (NONE/CLOSE),
   `SecureMode` stored on player. Enforce the reserved `CombatResult::SecureMode` +
   `AttackNotAllowed` (rights) branches in `validate_player_attack_target`.
3. PVP: `can_player_attack_player` / `is_protected` already in `combat/pvp.rs` — wire
   `IsAttackJustified`, `RecordAttack`, `BlockLogout(60)`, skull/frag outcomes (scope-check against
   772 `cract.cc`/`crcombat.cc`; skulls may be a follow-up sub-phase).

### Phase PC-5 — Skill/exp gain + regen from vocation data
**Files:** `process_skills.rs`, kill/death path (`death.rs`/`idle_stimulus.rs`).

1. Replace `process_skills.rs::fed_regen_cadence` hardcoded table with `vocations.xml`
   `gainhpticks/gainhpamount/gainmanaticks/gainmanaamount`.
2. On kill: `distribute_experience` + `pvp_exp_cap` (already present) → `add_experience`; skill
   `Increase` already handled inline via learning during strikes.
3. Skill tries curve: feed `vocations.xml` `<skill multiplier>` into `req_skill_tries`
   (`FactorPercent = 1000 * multiplier` shape per `crskill.cc:497`), not TFS `skillBase`.

---

## 4. Architecture / placement rules (steering compliance)

- **Formulas & flow** live in `tfs-rust-core` combat modules; **no** `NetworkMessage`/opcode bytes
  in core (`0xA0` parse stays in `tfs-rust-net`). (`tfs-packets.md`, `tfs-wire-codec.md`.)
- **Era knobs** (fight-mode %, armor mode, defense gate, probe tuning, condition ticks) stay in
  `MechanicsProfile` / `data/formulas/772.lua`. **Per-vocation balance** stays in `vocations.xml`.
  No new balance literals in Rust (`tfs-mechanics-profile.md` R11).
- **No `if version == 772`** in core — melee/dist/wand paths are shared; only profile fields differ.
- **Reuse** `probe_value`, `armor_reduction`, `defense_value`, `roll_target_defense`,
  `combat_execute_with_stimulus` — do **not** fork a parallel player combat math module
  (`tfs-code-hygiene.md`).
- Every new `.rs` gets the C++ reference header (`crcombat.cc`/`crskill.cc` + TFS structure cite).
- SlotMap IDs, `?` errors, enums + match; no `unsafe` (re-entrant Lua stays confined to
  `lua_scope.rs`).

---

## 5. New/changed data model

| Field | Where | Source |
|-------|-------|--------|
| `learning_points: i32` | `CreatureBase` (or `Player`) | `ActivateLearning`/`ProbeValue` |
| `attack_mode: FightMode`, `chase_mode` (exists), `secure_mode: bool` | `Player`/`CreatureBase` | `0xA0` packet |
| Full vocation combat block | `content::Vocation` | `vocations.xml` |
| Cached per-vocation snapshot | `Player` at login (or `GameWorld` map) | `VocationDatabase` |
| Wand attributes (`wand_damage_type`, `wand_attack_strength`, `wand_variation`, `wand_range`, `wand_mana`) | `content::ItemType` | `items.xml`/`objects.srv` (verify) |

`earliest_attack_ms`/`earliest_defend_ms`/`last_defend_ms` already exist on `CreatureBase`.

---

## 6. Test plan
- **Formula goldens** (extend `combat/math.rs` tests): melee `max(0,atk−def)` then randomized armor;
  distance hit-probe bounds; wand fixed±variation; skill-tries curve from vocation multipliers.
- **Vocation parse golden**: `data/XML/vocations.xml` full block.
- **Integration** (`sim_harness`/beat-driven world): player vs `human.mon`/`rat` — verify damage
  ranges, defense gate 2000 ms, learning advances skill, ammo consumed on distance, wand mana cost,
  death → exp/skill gain, PZ/secure-mode denial text.
- **glibc-rand parity** where `sim_glibc_rng_enabled()` (deterministic vs decompile expectation).

---

## 7. Verification
```
cargo check -p tfs-rust-core -p tfs-rust-content
cargo clippy -p tfs-rust-core -p tfs-rust-content --all-targets
cargo test  -p tfs-rust-core -p tfs-rust-content
```

---

## 8. Open questions / confirm against C++ before coding
1. **772 starting vitals anchors** — confirm base HP/mana/cap and per-level application against
   `crmain.cc` skill init + `crskill.cc` `Jump`/`Advance` (vitals are `TSkill` advances, not a flat
   `150 + gain*(l-1)` — may need the `TSkillLevel::Jump` advance path instead of the current
   `recalculate_vitals` shortcut).
2. **Skill-tries mapping** — `vocations.xml` `multiplier` (e.g. 1.1) vs `crskill.cc` `FactorPercent`
   (1100) and `Delta`: confirm `FactorPercent = 1000*multiplier` and the `Delta` source per skill.
3. **Wand data source** — whether wand damage/type/mana come from `items.xml`, `objects.srv`, or a
   hardcoded table in the decompile (`operate.cc`/`magic.cc`); cite before adding `ItemType` fields.
4. **Ranged defense "bug"** (`crcombat.cc:766`) — replicate the outcome (no defense on ranged, but
   still rolls target defense/wearout if attacker holds a shield). Confirm intended behavior.
5. **Skulls / PVP frags** — scope: include in PC-4 or defer to a dedicated PVP phase.
