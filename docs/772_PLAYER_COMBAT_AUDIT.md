# 772 Player Combat — Parity Audit + Implementation Plan

**Audited:** 2026-08-03
**Reference (772 mechanics):** `reference/cipsoft-772/tibia-game-master/src/` — `crcombat.cc`,
`crskill.cc`, `crmain.cc`, `cract.cc`.
**Reference (772 wire):** `reference/tvp-772/gameserver/src/` — not used here (no packet changes;
`SendMarkCreature` in F10 is an existing codec call).

**Rust files audited:**

| Rust file | 772 counterpart |
|---|---|
| `player/combat/mod.rs` | `crcombat.cc:357` `SetAttackDest`, `:442` `CanToDoAttack`, `:513` `StopAttack`, `:531` `Attack` |
| `player/combat/values.rs` | `crcombat.cc:164` `GetAttackValue`, `:191` `GetDefendValue`, `:286` `GetArmorStrength`, `:309` `GetDistance` |
| `player/combat/strike.rs` | `crcombat.cc:648` `CloseAttack` |
| `player/combat/ranged.rs` | `crcombat.cc:697` `WandAttack`, `:739` `DistanceAttack` |
| `player/combat/fight_mode.rs` | `crcombat.cc:325` `SetAttackMode`, `:339` `SetChaseMode`, `:348` `SetSecureMode` |
| `player/combat/skills.rs`, `process_skills.rs` | `crskill.cc` `TSkill*` (`ProbeValue`, `Probe`, `Increase`, `Jump`, `Process`) |
| `player/inventory/util.rs` | `crcombat.cc:36` `GetWeapon`, `:104` `GetAmmo` |
| `player/inventory/notifications.rs` | `crcombat.cc:128` `CheckCombatValues` |
| `combat/math.rs`, `creature/monster_combat.rs` | `crskill.cc:535` `ProbeValue`, `:549` `Probe`; `crcombat.cc:220`/`:237` damage+defend |
| `death.rs`, `idle_stimulus.rs`, `combat/mod.rs` | `crmain.cc:487` `TCreature::Damage`, `crcombat.cc:862`–`961` combat list + exp |
| `creature_todo.rs` | `cract.cc:1353` `ToDoAttack`, `:843` `Execute` `TDAttack` |

**Layer split (per `TFS-Core`):** the domain stays TFS-shaped (skill accessors, cylinder-based
inventory, `ItemType` attributes, Lua combat hooks) so `data/` keeps loading. The **outcomes** —
weapon-category resolution, probe RNG order, periodic-damage ticks, combat-list eviction — come
from the decompile and belong in `MechanicsProfile` / `data/formulas/772.lua` where they are
era-tunable. When a finding says "772 reads flag X", the fix is "check the OTB/`ItemType`
equivalent at the same mechanics point", not "add 772 flag X to the data layer".

**Wands / rods stay Lua-shaped:** `data/scripts/weapons/wands.lua` and `rods.lua` (`Weapon(WEAPON_WAND)`,
`weapon:level` / `:mana` / `:damage` / `:vocation` / `:register()`) load into `WandDef` via
`WeaponRegistry`. A `CombatWeapons` snapshot (B1) only answers *which* hand item is the wand;
`player_wand_attack` still reads mana/damage/element/gates from `world.weapons.get_wand(...)`.
Do not move wand level/vocation onto raw item `MINIMUMLEVEL` / `PROFESSIONS` — era content already
expresses those in Lua (`tasks/lessons.md` §240).

---

## 0. Verdict summary

The orchestration layer is genuinely faithful and well-cited. `SetAttackDest` / `CanToDoAttack` /
`StopAttack` / `DelayAttack` monotonicity, the fight-mode integer-tenths scaling, `ProbeValue`'s
two-rand average, `GetArmorStrength`'s `(A/2)+rand%(A/2)`, the `EarliestDefendTime =
LastDefendTime + 2000` ordering, `defend_fight_mode_for_target`, `ActivateLearning = 30`, and the
whole `DistanceAttack` drop-scatter/fragility path are all correct.

The defects cluster in three areas: **weapon resolution lacks the 772 `CombatWeapons` snapshot**
(TFS single-weapon accessor; latent under normal one-weapon+shield equip), **`probe_hit`
desynchronizes the glibc RNG stream**, and **the three periodic damage types do not exist**, which
cascades into poison override, DoT attribution, and PvP DoT magnitude.

**Findings: 8 bugs (5 critical, 3 high), 12 parity gaps, 2 non-772 hooks.**

| # | Finding | Severity | Outcome differs? |
|---|---------|----------|------------------|
| B1 | `GetWeapon` is a TFS port — missing category snapshot (`CombatWeapons`) | **High** | Latent / structural (see §1) |
| B2 | `GetDistance` checks ranged before melee and returns raw `shoot_range` | **Critical** | Yes |
| B3 | `RESTRICTLEVEL` / `RESTRICTPROFESSION` hand-slot gates missing; no `Fist` field | **Critical** | Yes |
| B4 | `probe_hit` draws the second `rand()` unconditionally — RNG stream desync | **Critical** | Yes |
| B5 | `Increase(1)` applied after the roll, and scaled by `rateSkill` | **Critical** | Yes |
| B6 | `TSkill::Get()` omits the `Min` floor and merges `MDAct`+`DAct` | **Critical** | Yes |
| B7 | Periodic damage types absent → poison override, DoT origin, PvP DoT halving | **High** | Yes |
| B8 | `CombatList` is a `HashMap` — no ring-buffer eviction, no `TimeStamp` window | **High** | Yes |
| G1 | Soul regeneration on exp gain missing | Medium | Yes |
| G2 | `SendMarkCreature` (black square on attacker) never sent | Medium | Yes |
| G3 | Amulet of loss: necklace-slot-only, hardcoded id, no `CLOTHES`/`BODYPOSITION` check | Medium | Yes |
| G4 | `MANADRAIN` message + `EFFECT_MAGIC_RED` + blue text missing; mana-shield msg lacks attacker | Medium | Yes |
| G5 | Drop-tile validation missing `BANK` / `UNLAY` legs | Medium | Yes |
| G6 | Burst arrow (`SpecialEffect == 2`) has no native path | Medium | Yes |
| G7 | `AMMO*`/`THROW*` special-effect attributes unread; stringly-typed `poisondamagecycles` | Medium | Yes |
| G8 | Invisibility re-checked only at target selection, not per beat | Medium | Yes |
| G9 | Ranged `ToDoWait(100)` missing from `ToDoAttack` | Medium | Yes |
| G10 | Level-up does not re-run `CheckCombatValues` | Low | Edge |
| G11 | `CheckCombatValues` approximated by slot-change detection | Low | Edge |
| G12 | `GetExpForLevel` guards absent (linear `< 1050` branch, `Level > 500`); `Decrease` abort quirk | Low | Edge |
| N1 | `vocation.formula.melee_damage` multiplier has no 772 counterpart (inert at 1.0) | Low | No (latent) |
| N2 | Wand attacks grant magic-level tries — unverified against `CheckMana` | Low | Unknown |

### Observed symptoms → root cause

| Symptom | Cause |
|---|---|
| Bow equipped, no matching ammo → fist attack values / fist skill if damage path is reached | **B1** — `player_get_weapon` returns `None` and falls through to fist; 772 keeps the `Missile` branch (`WEAPON_AMMO` / `SKILL_DISTANCE`). Usually masked by `OUTOFAMMO` before damage |
| Out-of-range / chase distance uses item `shoot_range` instead of category 1/2/3 | **B2** — `player_weapon_range` conflates `GetDistance` with per-weapon range; wrong `TARGETOUTOFRANGE` threshold when `items.xml` `range` ≠ 3 |
| Simulated fights diverge from the reference after the first missed arrow | **B4** — every failed distance probe advances `parity` one extra step |
| Underleveled / wrong-vocation **item-flag** weapons are fully usable | **B3** — note: wand/rod level+vocation are already gated via Lua `WandDef` |
| Weaker poison overwrites a stronger active poison | **B7** — no `Damage > TimerValue()` comparison |
| PvP DoT ticks land at half strength | **B7** — the periodic exclusion from the PvP halving is unreachable |
| A 21st attacker never displaces an old one; stale attackers keep skull/exp claim | **B8** |

> **Not a real loadout:** “sword + bow in opposite hands → fires arrows” is **not** reachable under
> normal inventory. Bows/crossbows are `SLOTP_TWO_HAND`; 772 `CheckInventoryDestination`
> (`operate.cc:695+`) throws `HANDSNOTFREE` / `HANDBLOCKED`, and `ONEWEAPONONLY` rejects a second
> attack weapon. Everyday dual-hand is **weapon + shield**. Category precedence still matters for
> the snapshot API and for synthetic / bypassed-equip tests — not for that dual-wield story.

---

## 1. B1 — `GetWeapon` is a TFS port (High)

### 772 reference (`crcombat.cc:36-102`)

`GetWeapon` iterates `INVENTORY_HAND_FIRST..INVENTORY_HAND_LAST` and fills **five independent
fields** from the flags on each item, plus a `Fist` boolean:

```cpp
if(ObjType.getFlag(SHIELD)){ this->Shield = Obj; }
if(ObjType.getFlag(WEAPON)){ this->Close = Obj; this->Fist = false; }
if(ObjType.getFlag(BOW)){ this->Missile = Obj; this->Fist = false; }
if(ObjType.getFlag(THROW)){ this->Throw = Obj; this->Fist = false; }
if(ObjType.getFlag(WAND)){ this->Wand = Obj; this->Fist = false; }
```

Nothing `break`s or `continue`s after a match, so a multi-flag item populates several fields and a
later hand slot overwrites an earlier one. `GetAttackValue` (`:167-185`) then selects by
**category**: `Close > Missile > Throw > Wand > RaceData[Race].Attack`.

### Equip reality (do not over-claim)

Normal inventory cannot put two attack weapons in the hands at once:

- Bows/crossbows are `SLOTP_TWO_HAND` (`data/items/#items.lua`); 772 `operate.cc:695+` requires both
  hands free when placing a two-handed item (`HANDSNOTFREE`) and blocks the free hand when the other
  already holds two-handed (`HANDBLOCKED`).
- A second attack weapon against an occupied weapon hand throws `ONEWEAPONONLY`.

So **sword + bow** (or melee + throw, etc.) is not a player-reachable dual-wield. Everyday dual-hand
is **one attack weapon + optional shield**. Earlier drafts used sword+bow as the poster symptom;
that was a synthetic dual-equip illustration, not a live loadout.

### Rust (`player/inventory/util.rs:346-379`)

```rust
/// TFS `Player::getWeapon(slots_t)` — `player.cpp` ~195–217.
pub fn player_get_weapon_in_slot(&self, cid: CreatureId, slot: u8, ignore_ammo: bool) -> Option<ItemId> { … }

pub fn player_get_weapon(&self, cid: CreatureId, ignore_ammo: bool) -> Option<ItemId> {
    self.player_get_weapon_in_slot(cid, InventorySlot::Left as u8, ignore_ammo)
        .or_else(|| self.player_get_weapon_in_slot(cid, InventorySlot::Right as u8, ignore_ammo))
}
```

This is upstream TFS's accessor: it returns the **first** hand-slot weapon and has no notion of the
five categories. `player_get_attack_value` (`values.rs:139-164`) then reads whatever came back.

### Impact

**Structural:** `GetAttackValue`, `GetDistance`, `GetAmmo`, and `CheckCombatValues` all assume the
five-field + `Fist` snapshot. A single Left→Right `Option<ItemId>` cannot represent that API — even
when everyday equip only fills one attack category plus optional `Shield`. Severity is **High**
(foundation for B2/B3/G11), not a Critical dual-wield combat bug.

**Observable / latent under normal equip:**

- Bow equipped, no matching ammo: `player_get_weapon` returns `None` → Rust fist attack value +
  fist skill; 772 keeps `Missile` (`AttackValue` from `NONE` ammo, `WeaponType = WEAPON_AMMO` →
  `SKILL_DISTANCE`). Masked today because `DistanceAttack` throws `OUTOFAMMO` before
  `GetAttackDamage`.
- Shield-only: needs `Fist == true` so `GetDistance` stays 1 (SHIELD never clears `Fist`) — tied to
  B3 / G11 once the snapshot exists.
- `player_get_shield` (`util.rs:384`): first-match Left→Right vs 772 last hand slot wins (edge;
  two shields are also blocked by equip rules).

### Domain fit — TFS Lua wands / rods

`CombatWeapons.wand: Option<ItemId>` is only the **equipped wand item**. Content and strike data stay
on the TFS Lua path already used by this port:

| Concern | Owner |
|---|---|
| Which hand item is the wand | `CombatWeapons.wand` (772 `GetWeapon` resolution) |
| Mana, damage min/max, element, level, vocation | `WandDef` from `wands.lua` / `rods.lua` via `WeaponRegistry` |
| Strike | `player_wand_attack` → `weapons.get_wand(type)` (unchanged contract) |
| Scripted ammo behavior | Lua `onUseWeapon` (burst/poison arrows) — untouched |

Do **not** replace Lua wand gates with item-flag `RESTRICTLEVEL` / `RESTRICTPROFESSION` for
wands/rods. Those flags in 772 era content map to the Lua `weapon:level` / `:vocation` surface;
`player_meets_wand_requirements` already implements that (`ranged.rs`, `tasks/lessons.md` §240).

### Fix

Introduce a resolved-weapon snapshot that mirrors the C++ field set, and make it the single source
for every combat consumer:

```rust
/// 772 `TCombat` weapon fields — `crcombat.cc:19-25`, filled by `GetWeapon` (`:36-102`).
pub(crate) struct CombatWeapons {
    pub shield: Option<ItemId>,
    pub close: Option<ItemId>,
    pub missile: Option<ItemId>,
    pub throw_: Option<ItemId>,
    pub wand: Option<ItemId>,
    pub ammo: Option<ItemId>,
    pub fist: bool,
}
```

`player_get_combat_weapons(cid) -> CombatWeapons` walks both hand slots in ascending order applying
item-flag level/profession gates where present (B3) and the non-exclusive flag assignment, then a
separate `player_get_ammo(&mut CombatWeapons)` ports `GetAmmo` (`:104-126`) including the branch
where `Ammo` is **not** cleared when `Missile == NONE`. Rewrite `player_get_attack_value`,
`player_get_defend_value`, `player_get_shield`, and `player_weapon_range` on top of it. Keep
`player_get_weapon` for the non-combat TFS callers so unrelated call sites don't churn.

For wands: after resolution, keep calling `player_meets_wand_requirements` / `get_wand` so
underleveled or wrong-vocation wands behave like today's Lua skip — do not double-gate via
item XML for the same content.

---

## 2. B2 — `GetDistance` inverted, returns raw `shoot_range` (Critical)

### 772 reference (`crcombat.cc:309-319`)

```cpp
int Distance = 0;
if(this->Close != NONE || this->Fist){ Distance = 1; }
else if(this->Throw != NONE){ Distance = 2; }
else if(this->Missile != NONE || this->Wand != NONE){ Distance = 3; }
return Distance;
```

Melee wins first, and the return is a **category** (1/2/3), never the item's own range. The item's
range is checked separately and later, inside `DistanceAttack` (`:762` `BOWRANGE`, `:775`
`THROWRANGE`) and `WandAttack` (`:706` `WANDRANGE`).

### Rust (`player/combat/values.rs:265-290`)

```rust
for slot in [InventorySlot::Left as u8, InventorySlot::Right as u8] {
    …
    match it.weapon_type {
        WEAPON_DISTANCE => return it.shoot_range.max(1),
        w if w == crate::inventory::WEAPON_WAND => { … return 3; }
        _ => {}
    }
}
1
```

### Impact

Two distinct defects. First, ranged is tested before melee in the Left→Right slot scan — under
normal equip only one attack weapon is present, so category order rarely flips a live fight; the
bug still matters for classification helpers that must match `GetDistance`'s melee-first order once
`CombatWeapons` exists, and for any synthetic dual-equip tests. Second, `player_execute_attack`
uses the return value as the out-of-range boundary (`mod.rs:405`, `cheb > weapon_range`),
conflating `GetDistance` with weapon range — so any bow whose `items.xml` `range` is not 3 gets the
wrong `TARGETOUTOFRANGE` threshold, and the check fires at arm time instead of inside the strike
where 772 raises it (after `DelayAttack(200)` and before the LoS probe). That second defect is the
Critical, player-visible half of B2.

### Fix

Reimplement as a categorical `player_weapon_distance(cid) -> i32` over `CombatWeapons` (B1), and
introduce a separate `player_weapon_max_range(cid)` for the in-strike range gate. Move the range
comparison out of `player_can_to_do_attack_chase` into `player_ranged_attack_strike` /
`player_wand_attack` so the ordering matches `crcombat.cc:611-639` → `:762`/`:775`/`:706`. Retain
the `abs(dx) > 7 || abs(dy) > 5` viewport gate at the `Attack()` level (`:624-627`) — that one *is*
correctly placed today.

---

## 3. B3 — Hand-slot restriction gates + `Fist` missing (Critical)

### 772 reference (`crcombat.cc:62-76`)

```cpp
if(ObjType.getFlag(RESTRICTLEVEL)){
    int CurrentLevel = Master->Skills[SKILL_LEVEL]->Get();
    int MinimumLevel = (int)ObjType.getAttribute(MINIMUMLEVEL);
    if(CurrentLevel < MinimumLevel){ continue; }
}
if(ObjType.getFlag(RESTRICTPROFESSION) && Master->Type == PLAYER){
    uint32 ProfessionMask = ObjType.getAttribute(PROFESSIONS);
    uint8 Profession = ((TPlayer*)Master)->GetEffectiveProfession();
    if((ProfessionMask & (1 << Profession)) == 0){ continue; }
}
```

`continue` skips the item for **weapon resolution only** — it stays equipped and still contributes
armor via `GetArmorStrength`, which iterates independently (`:289-300`).

### Rust

`player_get_weapon_in_slot` (`util.rs:352-371`) filters on `weapon_type` alone. Neither gate
exists. `Fist` is unmodelled anywhere — `CreatureBase` has no such field.

### Impact

A level-8 character wields a level-30 **item-flag-gated** weapon at full attack value. A sorcerer
swings a knight-only axe when those restrictions live on the item. Note `GetEffectiveProfession`
(not `GetRealProfession`) is the correct source: promotion and rook status participate, and 772
deliberately uses the *real* profession only for the PvP gate at `:397`/`:581`.

**Wands/rods are already gated** via Lua `WandDef.level` + vocation names
(`player_meets_wand_requirements`). B3 is about OTB/`ItemType` `RESTRICTLEVEL` /
`RESTRICTPROFESSION` on hand items that carry those flags — not a mandate to re-implement wand
gates in XML.

`Fist` absence blocks two behaviors: the shield-only-still-punches state (`GetDistance` → 1 via
`Fist`, since SHIELD does not clear it), and `CheckCombatValues`' `OldFist != this->Fist` leg
(G11).

### Fix

Fold both **item-flag** gates into `player_get_combat_weapons` (B1) as the per-slot `continue`.
Requires typed `ItemType` access to `MINIMUMLEVEL` and `PROFESSIONS`; check whether `minlevel` /
`vocation` are already registered in `items_xml_keys.rs` before adding fields — `vocation` is listed
as a registered-but-unused key in `772_THROW_MOVE_AUDIT.md` §A.3, so the registry entry likely
exists and only the typed field plus the mask decode are needed. Set `fist = true` initially and
clear it on WEAPON/BOW/THROW/WAND (never on SHIELD).

Leave wand/rod level+vocation on `WandDef` / `player_meets_wand_requirements` — do not duplicate
those into item XML for the same `wands.lua` / `rods.lua` content.

---

## 4. B4 — `probe_hit` desynchronizes the RNG stream (Critical)

### 772 reference (`crskill.cc:549-568`)

```cpp
bool Result = true;
if(Diff != 0){
    if(this->Act >= (rand() % Diff)){
        Result = (rand() % 100) <= Prob;
    }else{
        Result = false;
    }
}
return Result;
```

The second `rand()` is drawn **only** when the skill gate passes. Note also the gate reads
`this->Act` — the raw stored value, *not* `Get()` — so `Min`/`MDAct`/`DAct` deliberately do not
apply here (contrast `ProbeValue` at `:544`, which does use `Get()`).

### Rust (`combat/math.rs:387-419`)

```rust
let (diff_roll, chance_roll) = { … (parity.rand_mod(diff.max(1) as u32) as i32,
                                    parity.rand_mod(100) as i32) };
if skill < diff_roll { return false; }
chance_roll <= prob
```

Both draws happen unconditionally, and `skill` is the caller-supplied `Get()`-style value rather
than `Act`.

### Impact

Every missed distance shot advances the glibc stream one step beyond the reference. In a codebase
with a dedicated emulator (`sim_glibc_rand.rs`) and a sim battery
(`scripts/run_sim_battery.py`), this poisons every subsequent roll for the rest of the run — it
silently invalidates any long-horizon parity comparison. Highest value-per-line fix in this audit.

### Fix

Short-circuit so the second draw is lazy:

```rust
pub fn probe_hit(skill: i32, diff: i32, prob: i32, parity: &GlibcRngState) -> bool {
    if diff == 0 { return true; }
    if skill < rand_mod_diff(parity, diff) { return false; }
    rand_mod_100(parity) <= prob
}
```

Keep the existing `sim_glibc_rand` override branches, but hoist them into the two small helpers so
each is drawn at its own call site. Separately, feed `Act` rather than `Get()` from
`ranged.rs` — this depends on B6 landing first, since today the two are indistinguishable.

---

## 5. B5 — `Increase(1)` ordering and rate scaling (Critical)

### 772 reference (`crskill.cc:535-546`)

```cpp
int TSkillProbe::ProbeValue(int Max, bool Increase){
    if(Increase){ this->Increase(1); }
    int RandomFactor = ((rand() % 100) + (rand() % 100)) / 2;
    int MaxValue = Max * (this->Get() * 5 + 50);
    return (RandomFactor * MaxValue) / 10000;
}
```

`Increase(1)` runs **before** `this->Get()` is read, so a probe that levels the skill rolls with
the new value. The increment is a flat `1` — one exp per probe, never scaled, never per-damage.

### Rust (`player/combat/strike.rs:47-90`)

```rust
let (skill, level, mode, melee_mult, attack_speed_ms, learning_active) = … p.skill_level(atk_skill_nr) …;   // line 51
let attack_roll = weapon_damage(&profile, hooks, skill, atk_value, mode, level, &self.parity_rng); // line 72
let skill_tries = ConfigManager::scale_tries(1, self.config.rate_skill().unwrap_or(1.0));          // line 79
… p.skill_increase(atk_skill_nr, skill_tries, &profile, hooks);                                     // line 85
```

`skill` is snapshotted at line 51 and the increase applied at line 85 — inverted. And
`scale_tries(1, rateSkill)` makes the gain `1 × rateSkill`.

### Impact

On the exact tick a weapon skill levels, the damage roll uses the stale value — an off-by-one-level
divergence at every skill advance, on both the attack probe (`strike.rs`) and the shielding probe
(`strike.rs:284`). The `rateSkill` scaling is a TFS server-rate knob with no 772 counterpart; at
`rateSkill = 1.0` it is inert, but any shard raising it silently desynchronizes skill progression
from the reference.

### Fix

Reorder: apply the learning-gated `skill_increase` **before** reading the skill for the roll, then
decrement `learning_points`. Route the flat-vs-scaled decision through `MechanicsProfile` (e.g.
`skill_try_scaling: TryScaling::Flat` for 772, `Rated` for 1098) rather than reading
`config.rate_skill()` directly in the 772 strike path — same pattern the profile already uses for
`damage_probe` and `armor_random`.

---

## 6. B6 — `TSkill::Get()` omits `Min` and merges the two modifiers (Critical)

### 772 reference (`crskill.cc:19-25`)

```cpp
int TSkill::Get(void){
    int Value = this->Act;
    if(Value < this->Min) Value = this->Min;
    Value += this->MDAct + this->DAct;
    return Value;
}
```

`MDAct` is the timed/magic modifier; `DAct` is the equipment modifier. They are distinct because
`TSkillProbe::Event` (`:644-662`) zeroes **only** `MDAct` when a timer expires, and
`TSkill::SetMDAct` (`:79-85`) writes only that term.

### Rust (`creature/player.rs:512-516`)

```rust
pub fn skill_level(&self, skill: crate::player::combat::SkillNr) -> i32 {
    let base = skill.level(&self.skills);
    let var = self.var_skills[skill.try_index()];
    (base + var).max(0)
}
```

No `Min` floor, and one TFS-style `var_skills` term where 772 has two.

### Impact

Feeds every combat probe — attack (`strike.rs:51`), defense (`monster_combat.rs:415`), and distance
(`ranged.rs`). Two consequences: a skill debuffed below `Min` reads too low, and a timed skill buff
expiring cannot be cleared without also discarding the equipment contribution (or vice versa),
because both live in one accumulator.

### Fix

Split `var_skills` into `mdact_skills` and `dact_skills` on `Player`, add the `Min` clamp, and give
`skill_level` the exact `max(act, min) + mdact + dact` shape. Add `skill_act(skill)` for the
`Probe` gate (B4) which needs raw `Act`. Persistence: check `condition_blob.rs` /
`player_save` for how `var_skills` is serialized — the split needs a migration or a
back-compat read that folds a legacy single value into `dact`.

---

## 7. B7 — Periodic damage types absent (High)

### 772 reference (`crmain.cc:582-613`)

Three branches sit **before** the race-immunity table and armor, each returning early:

```cpp
if(DamageType == DAMAGE_POISON_PERIODIC){
    if(RaceData[this->Race].NoPoison){ return 0; }
    if(Damage > this->Skills[SKILL_POISON]->TimerValue()){
        this->PoisonDamageOrigin = AttackerID;
        this->SetTimer(SKILL_POISON, Damage, 3, 3, -1);
    }
    this->DamageStimulus(AttackerID, Damage, DamageType);
    return Damage;
}else if(DamageType == DAMAGE_FIRE_PERIODIC){
    if(RaceData[this->Race].NoBurning){ return 0; }
    this->FireDamageOrigin = AttackerID;
    this->SetTimer(SKILL_BURNING, Damage / 10, 8, 8, -1);
    …
}else if(DamageType == DAMAGE_ENERGY_PERIODIC){
    …  this->SetTimer(SKILL_ENERGY, Damage / 20, 10, 10, -1);  …
}
```

Poison is **strength-gated** — a weaker application stimulates but does not re-arm the timer. Fire
and energy re-arm unconditionally. The PvP halving at `:497-502` explicitly excludes all three.

### Rust

`CombatType` (`tfs-rust-common/src/enums.rs:19-33`) has `Poison` / `Fire` / `Energy` but no
periodic variants. DoT is driven from `process_skills.rs:214-240` with `attacker: None`. The PvP
halving in `idle_stimulus.rs:322-345` therefore has no periodic type to exclude and applies
unconditionally.

### Impact

Four observable divergences:

1. **Poison override** — a rat's weak poison replaces a giant spider's strong one.
2. **DoT attribution** — `PoisonDamageOrigin` / `FireDamageOrigin` / `EnergyDamageOrigin` are never
   stored, so a DoT kill cannot credit the killer (exp, skull, "killed by" log).
3. **PvP DoT magnitude** — periodic ticks between players land at half strength.
4. **Tick counts** — 772's `(Damage/10, 8, 8)` fire and `(Damage/20, 10, 10)` energy shapes are
   profile-driven in Rust (`ConditionTicks`) rather than derived from the applied damage, so the
   *number* of ticks no longer scales with the hit.

### Fix

Add `PoisonPeriodic` / `FirePeriodic` / `EnergyPeriodic` to `CombatType` and branch on them at the
top of the damage pipeline, before immunity and armor, each returning early. Add the three
`*DamageOrigin` fields to `CreatureBase`. Gate the poison re-arm on
`damage > current_poison_timer_value()`. Keep the tick divisors reading from `MechanicsProfile`
(`ConditionTicks`) so 1098 stays tunable, but let the *cycle count* derive from the damage as 772
does. This is the largest single item here and should land alone.

---

## 8. B8 — `CombatList` has no ring buffer or timestamps (High)

### 772 reference (`crcombat.cc:862-906`)

```cpp
void TCombat::AddDamageToCombatList(uint32 Attacker, uint32 Damage){
    this->CombatDamage += Damage;
    for(int i = 0; i < NARRAY(this->CombatList); i += 1){
        if(this->CombatList[i].ID == Attacker){
            this->CombatList[i].Damage += Damage;
            this->CombatList[i].TimeStamp = RoundNr;
            return;
        }
    }
    int NextEntryIndex = this->ActCombatEntry;
    this->CombatList[NextEntryIndex] = { Attacker, Damage, RoundNr };
    if(++NextEntryIndex >= NARRAY(this->CombatList)){ NextEntryIndex = 0; }
    this->ActCombatEntry = NextEntryIndex;
}
```

Fixed-size array with wrapping eviction. `GetMostDangerousAttacker` (`:895-906`) filters on
`(RoundNr - TimeStamp) < 60` and uses a strictly-greater comparison, so ties keep the lowest index.
Also note `crmain.cc:690-693` clamps `Damage` to remaining HP **before** the list is appended, so
the recorded value is the clamped one — and the summon split at `:698-703` credits attacker and
responsible `Damage / 2` each (integer division loses 1 on odd damage).

### Rust

`base.damage_map: DamageMap` (`creature/base.rs:124`) is a `HashMap<CreatureId, u64>` with no
timestamp. `skulls.rs:474-478` picks the max with a bare `max_by_key`.

### Impact

Attackers are never forgotten — a player who hit once an hour ago still competes for
most-dangerous-attacker (skull assignment, monster retargeting) and for exp share. Conversely the
eviction that lets a 21st attacker *displace* the oldest is absent, so 772's bounded-memory
behavior in large fights is not reproduced.

### Fix

Replace `DamageMap` with a fixed-capacity structure carrying `{ id, damage, timestamp_round }` and
an `act_entry` cursor. Size and window belong in `MechanicsProfile` (`combat_list_slots`,
`combat_list_window_rounds`) so 1098 can differ. Apply the `< 60` filter in
`GetMostDangerousAttacker` and preserve tie-to-lowest-index. Verify the clamp-before-append and
summon-halving order while touching this.

---

## 9. Gaps (not implemented)

| # | 772 reference | Gap |
|---|---|---|
| G1 | `crcombat.cc:938-955` | Soul regen on exp gain: when `Amount >= AttackerLevel`, `SetTimer(SKILL_SOUL, 240/Interval, Count, Interval, -1)`, `Interval = 120` (15 if promoted), `Count = TimerValue() % Interval` or `Interval` if 0. `death.rs:214-232` has none — soul is spent, never regained from kills |
| G2 | `crmain.cc:494` | `SendMarkCreature(Connection, Attacker->ID, COLOR_BLACK)` on every hit taken by a player — the black square on the attacker. No call site in Rust |
| G3 | `crmain.cc:792-817` | Amulet of loss: loop **all** inventory slots requiring `CLOTHES && BODYPOSITION == Position`, compare `GetNewObjectType(77,12)`, set `LoseInventory = NONE`, delete. `game_world_lifecycle.rs:645-655` checks only the necklace slot against a hardcoded `2173`. Only fires when `Damage == HitPoints` (exactly lethal) |
| G4 | `crmain.cc:649-657`, `:672-680` | `MANADRAIN` needs `"You lose %d mana."` + `EFFECT_MAGIC_RED` + blue text — `apply_mana_change` (`combat/mod.rs:90`) emits none. Mana-shield full absorb uses `"You lose %d mana blocking an attack by %s."`; `idle_stimulus.rs:514` omits the attacker name |
| G5 | `crcombat.cc:822-824` | Drop-tile validation is `!CoordinateFlag(BANK) \|\| CoordinateFlag(UNLAY) \|\| !ThrowPossible(…)`; `ranged.rs:624` checks only `throw_possible`. `UNLAY` now has a typed source (`772_THROW_MOVE_AUDIT` P0-2), so both legs are newly implementable |
| G6 | `crcombat.cc:837-842` | Burst arrow `SpecialEffect == 2`: `ComputeDamage(Master,0,EffectStrength,EffectStrength)` + `CircleShapeSpell(…, radius 2, EFFECT_FIRE_BURST)`, fired **regardless of hit**. `AmmoSpecialEffect` (`ranged.rs:41`) is `None \| Poison` only; burst is Lua-only |
| G7 | `crcombat.cc:770-771`, `:783-784` | `AMMOSPECIALEFFECT` / `AMMOEFFECTSTRENGTH` / `THROWSPECIALEFFECT` / `THROWEFFECTSTRENGTH` unread; `ranged.rs:388` infers from an unregistered stringly-typed `poisondamagecycles` (already filed as **A-2** in `772_THROW_MOVE_AUDIT` §A.5) |
| G8 | `crcombat.cc:460-465`, `:556-561` | Invisibility is re-checked by both `CanToDoAttack` and `Attack()` every beat. `mod.rs:717` covers only `SetAttackDest`, so a monster turning invisible mid-fight keeps taking hits instead of `TARGETLOST` |
| G9 | `cract.cc:1358-1360` | `if(this->Combat.GetDistance() != 1){ this->ToDoWait(100); }` before appending `TDAttack`. `enqueue_creature_attack` (`creature_todo.rs:286-301`) pushes unconditionally — ranged attacks lack the 100 ms arm floor |
| G10 | `crskill.cc:367` | `TSkillLevel::Jump` calls `Combat.CheckCombatValues()` after advancing HP/mana/go/carry, so `RESTRICTLEVEL` gear crossing its threshold triggers `DelayAttack(2000)`. `Player::add_experience` (`creature/player.rs:391-422`) updates speed only. Depends on B3 to be meaningful |
| G11 | `crcombat.cc:129-147` | `CheckCombatValues` diffs all seven resolved fields (incl. `OldFist`). `player_maybe_delay_attack_on_weapon_slot_change` (`player/inventory/notifications.rs:29-45`) delays on **any** Left/Right/Ammo mutation, so swapping two identical swords — or equipping a non-weapon into a hand — wrongly costs 2 s. Depends on B1 for the field snapshot |
| G12 | `crskill.cc:483-496`, `:342-353`, `:300-303` | `TSkillProbe::GetExpForLevel` falls back to **linear** `(Level - Min) * Delta` when `FactorPercent < 1050`; `req_skill_tries` (`combat/math.rs:587`) is always geometric. `TSkillLevel::GetExpForLevel`'s `Level > 500 → -1` overflow guard is absent from `experience_for_level_poly` (`math.rs:535`). `TSkillLevel::Decrease`'s `if(Amount > Exp && Exp > 100000) return;` abort quirk is unreproduced |

---

## 10. Non-772 behavior (currently inert, but latent)

| # | Site | Note |
|---|---|---|
| N1 | `strike.rs:74` | `attack_roll × vocation_profile.formula.melee_damage`. `GetAttackDamage` has no vocation multiplier. Defaults to `1.0` and `data/XML/vocations.xml` carries no `<formula>` block, so it is inert today — but it is a live hook that will silently break parity if a data pack sets it. Same shape for `dist_damage` in `ranged.rs`. Either gate on the 1098 profile or delete |
| N2 | `ranged.rs:214-247` | Wand attacks drain mana **and** grant magic-level tries via `magic_increase`. `WandAttack` (`crcombat.cc:722`) only calls `CheckMana(Master, ManaConsumption, 0, 0)`. Needs a read of `CheckMana` in `magic.cc` to decide whether it drains and whether it advances magic level — do not "fix" either way until confirmed |

---

## 11. What is already correct

Verified faithful; do not re-audit these without a reason:

- **Fight modes** — integer tenths `Max ± (Max*k)/10` (`math.rs:100`) reproduces `:222-227` /
  `:250-256` exactly, including the truncation (defensive atk 7 → 5, not 4).
- **`ProbeValue`** — two `rand()%100` draws averaged, `Max * (Get()*5 + 50)`, `/10000` integer
  order (`math.rs:133-157`).
- **`GetArmorStrength`** — `(A/2) + rand()%(A/2)` gated on `A >= 2`, with the `A ∈ {2,3}` →
  `rand()%1` degenerate case preserved (`math.rs:338-368`). The randomization is *correct* here;
  the earlier assumption that 772 subtracts flat armor is wrong.
- **Defend gate** — `EarliestDefendTime = LastDefendTime + 2000` using the **old**
  `LastDefendTime`, then `LastDefendTime = now` (`monster_combat.rs:448-453`).
- **`defend_fight_mode_for_target`** — forces Defensive when `Following || AttackDest == 0`
  (`monster_combat.rs:335-346`), matching `:245-248`.
- **`DelayAttack`** — monotonic `max`, never shortens (`base.rs:191-193`).
- **`ActivateLearning`** — flat reset to 30 (`base.rs:244`); decrement of exactly one point per
  probe, consumed even on a miss.
- **`GetDefendValue` precedence** — Shield > Close > **Throw** > Missile(0) > race, with Throw
  correctly *before* Missile (the opposite of `GetAttackValue`) — `values.rs:205-221`.
- **`SetAttackMode`** — `DelayAttack(2000)` before the assignment, only on an actual change.
- **`DistanceAttack` body** — difficulty `(Distance >= 2) ? Distance : 5`; drop scatter only when
  `DistanceX > 1 || DistanceY > 1`; `DropZ` deliberately not reset; missile drawn to the **drop**
  tile not the target; fragility consumption on hit *and* miss; `EFFECT_POFF` on miss; the
  attacker-shield `GetDefendDamage()` discard quirk.
- **Blood/splash table** — `BT_*` → effect/color, pool only for blood and slime
  (`monster_inventory.rs:64-69`).
- **`attackspeed`** — data-driven at 2000 across every vocation in `vocations.xml`, so the
  `DelayAttack(attackspeed)` after a strike matches 772's flat `DelayAttack(2000)` in practice.
- **`TSkillProbe::Decrease`** — the `Act > Min` loop guard is present (`skills.rs:238`).

---

## 12. Implementation plan

Each phase is independently shippable and ends green on `cargo check` + `cargo clippy` +
`cargo test`. Ordering is deliberate: **P0-1 first** because every downstream parity measurement is
untrustworthy until the RNG stream matches, and **P0-2 collapses four findings into one rewrite**.

### P0 — Parity blockers

| Task | Finding | Files | Effort | Status |
|---|---|---|---|---|
| **P0-1** Lazy second draw in `probe_hit` | B4 | `combat/math.rs` | S | Done |
| **P0-2** Real `GetWeapon` → `CombatWeapons` + level/profession gates + `Fist` | B1, B2, B3, G11 | `player/inventory/util.rs`, `player/combat/values.rs`, `player/inventory/notifications.rs`, `otb.rs`, `items_xml_keys.rs` | L | Done |
| **P0-3** Split `MDAct`/`DAct`, add the `Min` floor, add `skill_act` | B6 | `creature/player.rs`, `condition_blob.rs`, `process_skills.rs` | M | Done |
| **P0-4** `Increase(1)` before the roll; profile-gate the try scaling | B5 | `player/combat/strike.rs`, `player/combat/ranged.rs`, `mechanics profile` | M | Done |

**P0-1** is a handful of lines and unblocks the sim battery, so land it first and re-baseline
`scripts/run_sim_battery.py` before anything else moves. Expect existing RNG-sensitive test
fixtures to shift — that is the fix working, but re-record them in the same commit so the diff
stays reviewable.

**P0-2** is the centerpiece. Sequence inside the task: (a) typed `ItemType` access for
`MINIMUMLEVEL` / `PROFESSIONS` *where item flags exist*, checking `items_xml_keys.rs` first since
`vocation` is already a registered key (`772_THROW_MOVE_AUDIT` §A.3); (b) `player_get_combat_weapons`
with the non-exclusive flag assignment, ascending-slot overwrite, and the two `continue` gates;
(c) `player_get_ammo` including the not-cleared branch; (d) re-point `player_get_attack_value`,
`player_get_defend_value`, `player_get_shield`, and the new categorical
`player_weapon_distance` / `player_weapon_max_range`; (e) move the range gate out of
`player_can_to_do_attack_chase` into the strike bodies; (f) rewrite
`player_maybe_delay_attack_on_weapon_slot_change` as a true seven-field diff (G11 falls out for
free once the snapshot exists). Leave `player_get_weapon` in place for non-combat TFS callers.

Preserve the TFS Lua wand path: `CombatWeapons.wand` selects the item; mana/damage/element/level/
vocation stay on `WandDef` from `wands.lua` / `rods.lua`. Keep
`player_meets_wand_requirements` — do not replace it with item-flag gates for wand content.

**P0-3** needs a persistence decision before coding — inspect how `var_skills` round-trips through
the player save and either migrate or fold a legacy value into `dact` on read. `skill_act` is
required by P0-1's follow-up (feeding `Act` rather than `Get()` to the `Probe` gate), so land P0-3
before closing that half of B4.

**P0-4** depends on P0-3 only for the `Get()` semantics being correct; the reorder itself is
independent.

### P1 — Missing mechanics

| Task | Finding | Files | Effort | Status |
|---|---|---|---|---|
| **P1-1** Periodic damage types + origins + poison strength gate | B7 | `tfs-rust-common/src/enums.rs`, `creature/base.rs`, `idle_stimulus.rs`, `process_skills.rs`, `condition.rs` | L | Done |
| **P1-2** Ring-buffer `CombatList` with `TimeStamp` + 60-round window | B8 | `creature/base.rs`, `combat/mod.rs`, `player/combat/skulls.rs`, `death.rs` | L | Done |
| **P1-3** Soul regeneration on exp gain | G1 | `death.rs`, `process_skills.rs` | M | Done |
| **P1-4** Per-beat invisibility re-check | G8 | `player/combat/mod.rs` | S | Done |
| **P1-5** Ranged `ToDoWait(100)` in the attack builder | G9 | `creature_todo.rs` | S | Done |
| **P1-6** `CheckCombatValues` on level Jump | G10 | `creature/player.rs` / `notifications.rs` / lifecycle | S | Done |

**P1-1** should land alone — it touches the damage pipeline's control flow and the condition
system simultaneously. Add the enum variants and the three early-return branches first with the
existing profile tick values, then switch the cycle count to derive from applied damage.

**P1-2** and **P1-1** both touch `idle_stimulus.rs`; do them sequentially, not in parallel.

**P1-5** and **P1-6** are one-liners and can ride along with anything.

### P2 — Fidelity polish

| Task | Finding | Files | Effort | Status |
|---|---|---|---|---|
| **P2-1** `SendMarkCreature` on damage taken | G2 | `idle_stimulus.rs`, codec | S | Done |
| **P2-2** Mana-drain effects + mana-shield attacker name | G4 | `combat/mod.rs`, `idle_stimulus.rs` | S | Done |
| **P2-3** Amulet-of-loss full slot scan + `CLOTHES`/`BODYPOSITION` check | G3 | `game_world_lifecycle.rs` | M | Done |
| **P2-4** `BANK` / `UNLAY` legs on the ammo drop tile | G5 | `player/combat/ranged.rs` | S | Done |
| **P2-5** Native burst arrow + typed `AMMO*`/`THROW*` effect attributes | G6, G7 | `player/combat/ranged.rs`, `items_xml_keys.rs`, `otb.rs` | M | Done |
| **P2-6** `GetExpForLevel` guards + `Decrease` abort quirk | G12 | `combat/math.rs`, `player/combat/skills.rs` | S | Done |
| **P2-7** Gate or delete `formula.melee_damage` / `dist_damage` | N1 | `player/combat/strike.rs`, `player/combat/ranged.rs` | S | Done |
| **P2-8** Read `CheckMana` in `magic.cc`; reconcile wand magic tries | N2 | investigation | S | Done (verified) |

**P2-5** subsumes task **A-2** from `772_THROW_MOVE_AUDIT.md` §A.5 (register + type
`poisondamagecycles` / AMMO*/THROW* specials) — closed together.

### Verification

```bash
rtk cargo check --workspace
rtk cargo clippy --workspace --all-targets
rtk cargo test -p tfs-rust-core
rtk cargo test --workspace
python3 scripts/run_sim_battery.py     # re-baseline after P0-1
```

`cargo test -p tfs-rust-core` has pre-existing unrelated failures recorded in
`772_THROW_MOVE_AUDIT.md` §10 — confirm the count is unchanged rather than zero.

### Tests to add

| Test | Covers |
|---|---|
| `category_precedence_close_over_missile` — synthetic Close+Missile fields → melee dispatch, distance 1 (bypasses equip rules; documents API, not a live loadout) | B1, B2 |
| `later_hand_slot_overwrites_same_category` — two shields forced into hands → the higher slot index wins (synthetic) | B1 |
| `bow_without_ammo_keeps_distance_skill` — `GetAttackValue` yields `SkillNr::Distance`, not Fist | B1 |
| `wand_resolution_still_uses_wand_def` — equipped wand → `CombatWeapons.wand` set; strike still reads mana/damage from Lua `WandDef` | B1 |
| `underleveled_weapon_skipped_in_resolution` — level 8 + item-flag `minlevel=30` axe → fist attack, armor unaffected | B3 |
| `wrong_profession_weapon_skipped` — sorcerer + knight-masked axe → skipped; promoted voc passes via effective profession | B3 |
| `shield_only_still_fists` — shield equipped, no weapon → `Fist` true, distance 1 | B3 |
| `probe_hit_draws_one_rand_when_skill_gate_fails` — assert the `GlibcRngState` cursor advances by exactly 1 | B4 |
| `probe_value_rolls_with_post_increase_skill` — a probe that levels the skill uses the new value | B5 |
| `skill_get_applies_min_floor_and_both_modifiers` — `max(act,min) + mdact + dact` | B6 |
| `mdact_expiry_preserves_dact` — timer expiry zeroes only the magic term | B6 |
| `weaker_poison_does_not_override_stronger` — strong poison then weak → timer unchanged, stimulus still fired | B7 |
| `fire_periodic_rearms_unconditionally` — weak fire after strong fire re-arms | B7 |
| `pvp_periodic_damage_not_halved` — player→player periodic tick lands full | B7 |
| `dot_kill_credits_origin` — poison kill attributes exp/skull to `PoisonDamageOrigin` | B7 |
| `combat_list_evicts_oldest_when_full` — capacity+1 attackers → the first is displaced | B8 |
| `most_dangerous_attacker_ignores_stale_entries` — an attacker 60+ rounds old is excluded | B8 |
| `combat_list_records_clamped_damage` — overkill records remaining HP, not raw damage | B8 |
| `summon_damage_splits_between_attacker_and_master` — odd damage halves per 772 integer division | B8 |
| `soul_regen_armed_when_exp_at_least_level` — promoted vs unpromoted interval 15 vs 120 | G1 |
| `invisible_target_mid_fight_drops_attack` — monster turns invisible → `TARGETLOST` | G8 |
| `ranged_attack_builder_prepends_wait_100` — distance weapon → `[Wait{100}, Attack]` | G9 |
| `identical_weapon_swap_does_not_delay_attack` — swapping two same-type swords → no 2 s penalty | G11 |
| `ammo_drop_rejects_unlay_tile` — miss scattering onto a tree/wall reverts to the target tile | G5 |
| `burst_arrow_fires_on_miss` — `SpecialEffect == 2` triggers the radius-2 burst even when the shot misses | G6 |
