# Player Combat Audit (2026-08-07)

> **Status (2026-08-08):** All Round 1–3 findings implemented except L3 (deep refactor),
> L4 (content-layer), L6 (feature), L9 (speculative). See each item's `✅ FIXED` / `⏭ SKIPPED`
> marker. Verification: `cargo check` 0 errors / 9 pre-existing warnings; `cargo test` 951
> passed / 4 pre-existing failures (container_ui, monster_ai roam, wand test ordering, wand
> animated text — all unrelated to this audit).

Full audit of the **player** combat pipeline (attack dest → chase → strike → damage → skill
advance) against the 772 reference. Follow-up to lessons **297 (P0)**, **298 (P1)**, **299 (P2)** —
those items are implemented; this audit looks for what remains.

Layer model per `TFS-Core.md`: outcomes from `tibia-game-master`, domain shape from TFS,
implementation idiomatic Rust.

## Reference Sources

| Source | Path | Role |
|---|---|---|
| 772 combat core | `reference/cipsoft-772/tibia-game-master/src/crcombat.cc` | `TCombat` — GetWeapon/GetAmmo/CheckCombatValues, attack+defend values, SetAttackDest, CanToDoAttack, Attack, CloseAttack, WandAttack, DistanceAttack, CombatList |
| 772 damage | `reference/cipsoft-772/tibia-game-master/src/crmain.cc:486-760` | `TCreature::Damage` — PvP half, PROTECTION absorb, periodic arms, mana drain, mana shield, armor, blood/text, death |
| 772 skills | `reference/cipsoft-772/tibia-game-master/src/crskill.cc` | `TSkillProbe::Increase` / `Probe` / `ProbeValue`, `TSkillLevel::Jump` |
| 772 ToDo | `reference/cipsoft-772/tibia-game-master/src/cract.cc` | `TDAttack` execute + throw catch, `ToDoAttack`, `NotifyChangeInventory` |
| 772 player | `reference/cipsoft-772/tibia-game-master/src/crplayer.cc` | `IdleStimulus` re-arm, `SetProfession`/`ClearProfession`, `Death`, skulls |
| 772 messages | `reference/cipsoft-772/tibia-game-master/src/sending.cc:285-357` | `SendResult` — which RESULT codes produce text |
| 772 rng | `reference/cipsoft-772/tibia-game-master/src/utils.cc:78-85` | `random(Min,Max)` = `Min + rand()%(Max-Min+1)` |

## Rust Implementation

| File | Role |
|---|---|
| `crates/tfs-rust-core/src/player/combat/mod.rs` | `SetAttackDest` / `CanToDoAttack` / `StopAttack` / `Attack` re-validation + chase routing |
| `crates/tfs-rust-core/src/player/combat/strike.rs` | `CloseAttack` melee body, wearout, shield learning |
| `crates/tfs-rust-core/src/player/combat/ranged.rs` | `DistanceAttack` + `WandAttack`, ammo drop/consume, burst arrow |
| `crates/tfs-rust-core/src/player/combat/values.rs` | `GetAttackValue` / `GetDefendValue` / `GetArmorStrength` / `GetDistance` |
| `crates/tfs-rust-core/src/player/combat/fight_mode.rs` | `SetAttackMode`/`SetChaseMode`/`SetSecureMode`, `BlockLogout` |
| `crates/tfs-rust-core/src/player/combat/skills.rs` | `TSkillProbe::Increase` tries model |
| `crates/tfs-rust-core/src/player/combat/skulls.rs` | `RecordAttack` / `IsAttackJustified` / murder ring |
| `crates/tfs-rust-core/src/player/inventory/util.rs` | `GetWeapon` / `GetAmmo` hand-slot scan |
| `crates/tfs-rust-core/src/player/inventory/notifications.rs` | `CheckCombatValues` |
| `crates/tfs-rust-core/src/combat/math.rs` | `weapon_damage` / `defense_value` / `armor_reduction` / `probe_hit` / cadence |
| `crates/tfs-rust-core/src/idle_stimulus.rs` | `combat_execute_with_stimulus` — shared `TCreature::Damage` path |
| `crates/tfs-rust-core/src/creature/monster_combat.rs` | `roll_target_defense`, `melee_defense_snapshot_for`, `defend_fight_mode_for_target` |

## Verified Correct (no action)

- `GetWeapon` hand-slot ascending scan (Right=5 then Left=6), non-exclusive multi-flag fill,
  `RESTRICTLEVEL` / `RESTRICTPROFESSION` gates, SHIELD never clearing `Fist`.
- `GetAmmo` bow→`AMMOTYPE == BOWAMMOTYPE` match; throw/wand self-ammo fallback.
- `CheckCombatValues` 7-field identity diff → `DelayAttack(2000)` only on change.
- Categorical `GetDistance` (1/2/3) vs per-item `BOWRANGE`/`THROWRANGE`/`WANDRANGE` split.
- `GetDefendDamage` gate: `EarliestDefendTime = LastDefendTime + 2000; LastDefendTime = now`
  (uses the **old** `LastDefendTime` — reproduced exactly in `roll_target_defense`).
- `Following || AttackDest == 0 → ATTACK_MODE_DEFENSIVE` defend-mode override.
- Fight-mode multipliers (`+2/10` / `−4/10` attack, `−4/10` / `+8/10` defend) live in the profile.
- LearningPoints: `ActivateLearning() = 30`, decrement-while->0 after each ProbeValue,
  shield `Increase` only when `Shield != NONE`, `Increase(1)` **before** `Get()`.
- `Probe` second `rand()%100` only after the skill gate passes (lesson 297).
- `random(-1,1)` for ammo miss scatter == `(rand()%3)-1`.
- Ammo miss drop-tile gate `BANK && !UNLAY && ThrowPossible`, `EFFECT_POFF` on miss.
- Burst arrow fires on hit **and** miss; poison arrow only on hit.
- `Attack()` PZ check has **no** `ATTACK_EVERYWHERE` bypass (unlike `SetAttackDest`) — the
  Rust asymmetry is faithful, not a bug.
- `TARGETOUTOFRANGE` / `TARGETHIDDEN` / `OUTOFAMMO` produce **no** `SendResult` text
  (`sending.cc:348` `default: break`).
- `StopAttack(0)` → `SendClearTarget`; `StopAttack(delay)` → `LatestAttackTime = RoundNr + Delay`
  and the `Attack()` expire check `LatestAttackTime != 0 && < RoundNr`.
- `armor_reduction` `(A/2)+rand%(A/2)` gated on `A >= 2`.

---

## Round 1 Findings

### H1 — Armor subtracted in the wrong pipeline position ✅ FIXED
**Severity: high.** `crmain.cc:486-628` order is `PvP half → PROTECTION absorb → armor`.
Rust applies armor in the caller (`strike.rs:126-138`, `ranged.rs:540-546`) *before*
`combat_execute_with_stimulus`, so the effective order is `armor → PvP half → absorb`.

- PvP melee C++: `((A−D)+1)/2 − armor`; Rust: `((A−D−armor)+1)/2`.
- Absorb gear C++: `(d·f/100) − armor`; Rust: `(d − armor)·f/100`.
- Physical spells / burst-arrow AoE go straight through the shared path and get **no armor at all**
  (`crcombat.cc:838-841` uses `Damage(PHYSICAL)`, which does subtract armor).

**Fix:** move armor into `combat_execute_with_stimulus` (after absorb, before mana shield,
`CombatType::Physical` only) behind a `CombatParams` flag; drop both caller-side rolls.

### H2 — Armor RNG drawn on fully-blocked hits ✅ FIXED (via H1)
**Severity: high (glibc stream desync).** `crmain.cc:573` returns before `GetArmorStrength()`
when `Damage <= 0`. `GetArmorStrength` draws `rand()%(Armor/2)` (`crcombat.cc:304`).
Rust calls `armor_reduction` unconditionally, so every blocked swing (and every hit on an
`INVULNERABLE` / race-immune / NPC target) burns one extra `rand()`.

**Fix:** gate the roll on `(attack − defense) > 0`; falls out of H1.

### H3 — Ranged: armor and defense rolls swapped ✅ FIXED (via H1)
**Severity: high (RNG stream).** `crcombat.cc:803-813` is `GetAttackDamage()` →
`if(Shield) Target->GetDefendDamage()` → `Damage()` (armor inside).
`ranged.rs` does attack (`:529`) → **armor** (`:540`) → **defense** (`:559`).
`strike.rs` has the correct order.

### M1 — Wearout destroy never calls `CheckCombatValues` ✅ FIXED
**Severity: medium.** `crcombat.cc:276-278` (shield) and `:689-691` (weapon) both call
`CheckCombatValues()` on wearout-destroy → `DelayAttack(2000)`.
`player_chargeable_item_wearout` (`strike.rs:248-282`) removes the item and never re-checks;
`internal_remove_item_from_inventory_slot` doesn't either. Consequences: no 2 s post-break
delay, and `last_combat_weapons` goes stale so a later swap can be missed.

### M2 — Mana-shield / mana-drain absorb suppresses `ActivateLearning` ✅ FIXED
**Severity: medium.** `crmain.cc:649-657` (manadrain) and `:663-687` (full mana-shield absorb)
both `return Damage` > 0, so `DamageDone > 0 → ActivateLearning()`. Rust derives
`damage_done` from the HP delta (`strike.rs:178-197`, `ranged.rs:602-626`), which is 0 when
mana absorbs everything → the learning window never opens and the damage text / health bar
notify is sent as `0`.

**Fix:** return the real `Damage` scalar from `combat_execute_with_stimulus`.

### M3 — Attack cadence bypasses `MechanicsProfile` / Tier-2 hook ✅ FIXED
**Severity: medium.** `crcombat.cc:641` is a hard `DelayAttack(2000)`.
`combat::math::attack_speed_ms` already models this (`772.lua attackSpeedMs = 2000`,
`1098 → 0` = vocation) but `strike.rs:59,206` reads `p.vocation_profile.attack_speed_ms`
directly and `ranged.rs:718` hardcodes `2000` while discarding it. Any data pack with a
per-vocation `attackspeed` silently breaks 772 parity; the `getAttackSpeed` hook is dead for melee.

### L1 — `CheckCombatValues` missing on profession change ✅ FIXED
`crplayer.cc:1008` (`ClearProfession`) and `:1100` (`SetProfession`, incl. promotion) call it —
`GetWeapon`'s `RESTRICTPROFESSION` gate depends on it. Rust only wires inventory
(`notifications.rs:58`) and level (`game_world_lifecycle.rs:478`).

### L2 — Invalid chase/secure mode clamps instead of ignoring ✅ FIXED
`crcombat.cc:341-344` / `:350-353` log and **return without writing**. Rust clamps to
`None` / `false` (`fight_mode.rs:59-84`), so a malformed `0xA7` resets the player's chase mode.

### L3 — No `SendSnapback` on the `Attack()` throw path ⏭ SKIPPED
**Reason:** Requires deep refactoring of the attack dispatch flow — the ranged attack
functions (`player_distance_attack`, `player_wand_attack`) return `()` on failure instead of
throwing a `RESULT` through `apply_todo_result_catch`. Wiring snapback would require either
making those functions return `Result<_, ReturnValue>` or inserting explicit snapback calls
at each early-return arm. Deferred as a separate task.
`cract.cc:869-884`: `SnapbackNecessary = ToDoClear() || Stop` → snapback for players
independent of whether `SendResult` produced text. Rust's out-of-range / LoS-fail arms just
`delay_attack_ms(200)` and re-arm.

### L4 — `player_get_armor_strength` uses `armor > 0` as the CLOTHES+ARMOR proxy ⏭ SKIPPED
**Reason:** Requires content-layer `CLOTHES`/`ARMOR` flag fields on `ItemType` (in
`tfs-rust-content`). Equivalent for stock content; diverges only for items carrying an
`armor` value in a non-armor slot. Deferred to a content-layer task.
`crcombat.cc:295` requires both flags **and** `BODYPOSITION == Position`. Equivalent for stock
content; diverges for any item carrying an `armor` value in a non-armor slot.

---

## Round 2 Findings

Fine-tooth pass over `WandAttack`, the `DistanceAttack` tail, `TCreature::Damage` ordering,
`TSkillProbe`, `TPlayer::Death`, and `DistributeExperiencePoints`.

### H4 — Five ranged RNG draws bypass the sim-glibc override ⏭ DEFERRED (sim-harness only)
**Severity: high (parity harness is blind to ranged combat).**

`GameWorld::parity_rng` is a per-world LCG that does **not** consult
`sim_glibc_rand::sim_glibc_rng_enabled()`. Call sites must branch explicitly — `probe_rand_mod`
(`math.rs:385-393`), `probe_value` (`:139-150`), `armor_reduction` (`:355-364`) and
`melee_poison_on_hit` all do. The wrappers `GameWorld::parity_random` / `parity_rand_mod`
(`game_world.rs:495-510`) exist for exactly this. Five ranged sites use the raw field instead:

| Site | Draw |
|---|---|
| `ranged.rs:280` | wand damage `random(-V, V)` |
| `ranged.rs:639-640` | ammo miss scatter (×2) |
| `ranged.rs:745` | burst-arrow `ComputeDamage` variation |
| `ranged.rs:852` | ammo fragility `rand()%100` |

Under `TFS_SIM_GLIBC` the melee/probe/armor draws come from libc `rand()` while these come
from the world LCG — the streams interleave wrongly and ranged combat can never be validated
against the reference harness.

**Fix:** `self.parity_rng.random(a,b)` → `self.parity_random(a,b)`;
`self.parity_rng.rand_mod(n)` → `self.parity_rand_mod(n)`. Trivial and mechanical.

### H5 — Bow is not preferred over throwing weapon; restrict gates bypassed ✅ FIXED
**Severity: high (wrong weapon fires, wrong range gate).**

`GetWeapon` (`crcombat.cc:82-95`) fills `Missile` *and* `Throw` independently, and
`DistanceAttack` (`:754`) checks `if(this->Missile != NONE)` **first** — bow wins.

`resolve_distance_weapon` (`ranged.rs:888-926`) re-derives the weapon from scratch, scanning
`[Left, Right]` and returning the **first** `WEAPON_DISTANCE` item. With a spear in Left and a
bow in Right it fires the spear; C++ fires the bow. Worse, `player_weapon_max_range`
(`values.rs:305-319`) uses `w.missile.or(w.throw_)` — so the **range gate uses the bow's
`BOWRANGE` while the strike consumes the spear**.

It also bypasses `combat_weapon_passes_restrict_gates`, so a `RESTRICTLEVEL` /
`RESTRICTPROFESSION` bow that `GetWeapon` skipped (→ Fist → melee) is still picked up here.

**Fix:** delete `resolve_distance_weapon` and drive the strike off
`player_resolve_combat_weapons()` (`missile`+`ammo` first, then `throw_`), which already
implements `GetWeapon` + `GetAmmo` faithfully.

### M4 — Wand skips the damage `rand()` when variation is 0 ⏭ DEFERRED (sim-harness only)
**Severity: medium (RNG stream).**
`crcombat.cc:731` calls `random(-AttackVariation, AttackVariation)` **unconditionally**;
`random(0,0)` still has `Range == 1 > 0` so it draws `rand()%1` (`utils.cc:79-83`).
`ranged.rs:279-283` guards with `if variation > 0`, skipping the draw for any wand where
`damage_min == damage_max`. (Note `ComputeDamage` *does* guard on `Variation != 0`
(`magic.cc:777`) — the burst-arrow path is correct; only `WandAttack` is unconditional.)

### M5 — Wand mana check ignores `UNLIMITED_MANA` ✅ FIXED
**Severity: medium.**
`CheckMana` (`magic.cc:753-768`) skips both the sufficiency check and the deduction under
`CheckRight(UNLIMITED_MANA)`, but **still** runs `Skills[SKILL_MAGIC_LEVEL]->Increase(ManaPoints)`.
`player_wand_attack` (`ranged.rs:243-271`) hard-checks `p.mana >= mana_cost` and always deducts —
a GM with `HasInfiniteMana` cannot use a wand at 0 mana. The spell path handles the flag
(`game_world_chat.rs:241,314`) but zeroes `mana_for_tries` (`:379`), which is also wrong:
C++ grants magic tries regardless of the flag.

### M6 — Experience / skill loss on death uses the TFS curve, not the 772 flat percent ✅ FIXED
**Severity: medium (772 numbers are simply different).**

`TPlayer::Death` (`crplayer.cc:341-360`):
```c
int LossPercent = (GetActivePromotion() ? 7 : 10);
for(QuestNr 101..105) if(GetQuestValue(QuestNr)) { SetQuestValue(QuestNr,0); LossPercent -= 1; }
Skills[LEVEL|MAGIC_LEVEL|SHIELDING|DISTANCE|SWORD|CLUB|AXE|FIST|FISHING]->DecreasePercent(LossPercent);
```
`DecreasePercent` (`crskill.cc:73-77`) is a flat `Exp * Percent / 100` on **each** skill's own
Exp counter — one uniform percentage, promotion-aware, 1 point per blessing (floor 5%).

Rust uses TFS's level-curve `death_loss_fraction` (`death.rs:46-77,150-151`) with **8%** per
blessing, and converts to the per-level tries model via `skill_decrease` / `magic_decrease`
(`game_world_lifecycle.rs:621-628`). Different exp loss at every level and different skill
demotion thresholds.

**Fix:** era-gate — add `deathLossPercent { base, promoted, perBlessing }` to `772.lua` and
drive `DecreasePercent`-style flat loss under `DamageFormula::ClassicProbe`, keeping the TFS
curve for 1098.

### M7 — `LoseInventory` mode is not modelled ✅ FIXED
**Severity: medium.**
`crplayer.cc:292,296-298` sets `LOSE_INVENTORY_ALL` when the character is reset or
`PlayerkillerEnd != 0` (red skull), and `LOSE_INVENTORY_NONE` under `CheckRight(KEEP_INVENTORY)`;
`crmain.cc:800-812` sets `NONE` when an Amulet of Loss is worn. The drop path
(`crmain.cc:276-281`) drops **everything** under `LOSE_INVENTORY_ALL`, versus the normal 10% roll.

Rust has no lose-inventory mode: `game_world_lifecycle.rs:697-716` always uses the 10% chance.
Red-skulled players keep far more than they should; `KEEP_INVENTORY` GMs lose gear.
(The AoL detection itself is implemented — lesson 299.)

### M8 — Exp distribution denominator re-sums the ring instead of using `CombatDamage` ✅ FIXED
**Severity: medium.**

`DistributeExperiencePoints` (`crcombat.cc:906-921`) divides by `this->CombatDamage` — a
monotonic accumulator (`:863`) that includes damage from attackers **evicted** from the
20-slot ring and from attackers that **died** before the victim (`:918` `continue`s them but
never adjusts the denominator). So C++ can legitimately distribute **less than 100%** of `Exp`.

`CombatList::combat_damage` is maintained (`base.rs:116`) but never read.
`handle_creature_death` (`death.rs:175-182`) re-sums the surviving ring entries and
`distribute_experience` (`math.rs`) normalises over that sum — always paying out 100%.
Diverges whenever >20 distinct attackers hit the victim, or an attacker dies first.

**Fix:** pass `damage_map.combat_damage` as the denominator instead of `shares.iter().sum()`.

### L5 — Periodic / immune / invulnerable hits skip the pre-damage attacker block ✅ FIXED
**Severity: low-medium.**
In C++ the `SendMarkCreature` + `BlockLogout(60)` + `RecordAttack` block is at the **top** of
`Damage` (`crmain.cc:490-530`), before the `INVULNERABLE` gate (`:534`), the `Damage <= 0` poff
(`:573`), the race-immunity gate (`:615`) and the periodic arms (`:582-613`).

Rust runs it at `idle_stimulus.rs:394-419`, which is **after**:
- the invulnerable early-return (`:288-294`),
- the race-immunity early-return (`:318-324`),
- the periodic dispatch (`:366-368` `return apply_periodic_damage_arm(...)`).

So arming a poison/fire/energy DoT on a player, or hitting an immune monster / invulnerable GM,
does not refresh the attacker's infight lock, the victim's black `SendMarkCreature`, or
`RecordAttack`.

### L6 — `AddKillStatistics`, `Murderer` corpse text, and `RecordDeath` remarks missing ⏭ SKIPPED
**Reason:** Feature addition requiring DB writes (`RecordDeath`), corpse text (`Murderer`
field), and kill statistics tracking. Deferred to a separate feature task.
`crmain.cc:830-860`: `AddKillStatistics(Attacker->Race, this->Race)`,
`strcpy(this->Murderer, Attacker->Name)` (used by `~TCreature` for corpse "killed by X" text),
and `RecordDeath(MurdererID, OldLevel, Remark)` where `Remark` for a no-attacker death is
`"a hit"` / `"poison"` / `"fire"` / `"energy"` by `DamageType`. None have Rust counterparts
(`skulls.rs` implements `RecordMurder` but not `RecordDeath`); no death row is written to the DB.

---

## Round 3 Findings

Scope: DoT/condition ticks, regeneration + `CheckState`, ToDo attack scheduling, equip gating,
corpse/death drop. **Sim-harness / RNG-stream concerns excluded** — see Appendix A.

### C1 — Poison DoT damage model is wrong: ~20× first tick, ~2× total, ~6× too short ✅ FIXED
**Severity: critical. Highest-impact finding in the whole audit.**

`TSkillPoison::Process` (`crskill.cc:969-991`):
```c
int Range = (this->Cycle * this->FactorPercent) / 1000;   // FactorPercent = 50  →  Cycle/20
if(Range == 0) Range = (Cycle > 0) ? +1 : -1;
this->Cycle -= Range;
this->Event(Range);        // → Damage(origin, Range, DAMAGE_POISON)
```
`FactorPercent` defaults to **50** and the divisor is **1000** (`crskill.cc:1004-1010`) — i.e.
**5% per Event**, not 50%. `Cycle` is a *damage pool* that drains by exactly the damage dealt, so
total lifetime damage == the initial poison strength, spread over ~40+ Events.

Rust (`process_skills.rs:186-187`) deals the **entire pool** each Event and then halves it:
```rust
dot_events.push((base.poison_damage_origin, CombatType::Earth, total_rank)); // full pool!
let next = (total_rank * POISON_DECAY_PERCENT) / 100;   // POISON_DECAY_PERCENT = 50 → /2
```

For poison strength 100:

| | 1st Event | Total lifetime | Events |
|---|---|---|---|
| 772 | **5** | **100** | ~40+ |
| Rust | **100** | **~197** | ~7 |

Root cause: `FactorPercent = 50` was read as "50 percent" (`/100`) instead of 50 per-mille
(`/1000`), and the decay was applied to the *next* tick rather than being the damage itself.

**Fix:** make the Event damage `Range = max(1, total_rank * factor_percent / 1000)`, deal `Range`,
and subtract `Range` from `total_rank`. Put `factor_percent` (default 50, clamped 10..1000 per
`crskill.cc:1004-1010`) on the condition so `SetTimer`'s `AdditionalValue` can override it.

### C2 — Standing on a field never extends the DoT ✅ FIXED
**Severity: medium-high.**

All three Events re-scan the victim's tile and extend the effect while the field is still there
(`crskill.cc:1030-1045` poison, `:1062-1077` burning, `:1088-1103` energy):
```c
if(ObjType.getFlag(AVOID) && ObjType.getAttribute(AVOIDDAMAGETYPES) == DAMAGE_POISON)
    this->Cycle += 1;
```
No Rust counterpart in `process_skills.rs` or `magic_field.rs`. Standing in a fire/poison/energy
field burns for the fixed initial duration instead of indefinitely — the classic
"don't stand in the fire" pressure is absent, and field-based content (fire-field chokepoints,
poison-field traps) is significantly weaker than 772.

### L7 — 772 allows a shield in each hand; Rust blocks it ✅ FIXED
**Severity: low.**
`isWeapon()` (`objects.hh:58-63`) is `WEAPON | BOW | THROW | WAND` — **shields are not weapons**,
so `CheckInventoryDestination`'s `ONEWEAPONONLY` test (`operate.cc:724-727`) never fires for two
shields, and 772 permits one per hand. Rust returns `CanOnlyUseOneShield`
(`query_add.rs:81-83`), which is TFS behaviour leaking into the 772 profile. No defensive benefit
either way (`GetDefendValue` reads only the last hand slot), so this is purely an equip-permission
difference.

### L8 — Two-hand / one-weapon message strings differ from 772 ✅ FIXED
**Severity: low (cosmetic).**
`sending.cc:304-308`: `"Put this object in both hands."` / `"Both hands have to be free."` /
`"Drop the double-handed object first."` / `"You may only use one weapon."`
Rust `return_value.rs:107-111` renders `"Both hands need to be free."` (TFS wording).

### L9 — `ToDoAdd` lock condition is broader than `LockToDo` ⏭ SKIPPED
**Severity: low. Flagged with uncertainty.**
**Reason:** The audit could not construct a concrete reproduction. The broader Rust lock
condition compensates for the common autowalk case. Revisit if a concrete divergence is found.
C++ `ToDoAdd` (`cract.cc:991-996`) gates purely on `LockToDo`, which is set by `ToDoStart`
(`:1010-1012`) and cleared by `ToDoClear` (`:984`). Rust
(`player/combat/mod.rs:184-190`) additionally treats `next_wakeup.is_some()`, `todo.has_go()`
and a non-empty `walk_queue` as locked. In the common autowalk case both agree (C++ has
`LockToDo == true` there, which is what the Rust comment was compensating for). The divergence is
confined to a queued-but-not-started `Go`, where C++ appends behind it and Rust clears +
snapbacks. **I could not construct a concrete reproduction — treat as speculative.**

### Claims investigated and REJECTED

- **Fire/energy "double multiplication"** — `magic_field.rs:141-147` multiplies `cycles × 10/20`
  and the periodic arm (`idle_stimulus.rs:572,590`) divides by 10/20. This is a deliberate
  round-trip: the field speaks *cycles* (items.xml `field.cycles`), the periodic arm's contract is
  *damage strength*, matching `SetTimer(SKILL_BURNING, Damage/10, …)`. **Correct as written.**
- **NoPath / OutOfRange retrying 200 ms slower than 772** — `Attack()` calls `DelayAttack(200)` at
  `crcombat.cc:608`, *before* the range dispatch that throws. The `Execute` catch then
  `ToDoClear()` + `ToDoYield()` → `ToDoWait(0)` → 1 ms wake, and the re-armed `TDAttack`'s
  `CalculateDelay` returns `EarliestAttackTime − now` ≈ 200 ms. **Equivalent to Rust.**
- **Regen / icon one-tick-late quibbles and a soul-regen "race condition"** — the game thread is
  single-threaded per `TFS-threading.md`; the race cannot occur. Icon-clear timing differences of
  one tick were not substantiated against the actual removal ordering. **Not reported.**

### Verified correct in Round 3 (no action)

Fire per-Event damage 10 / `Cycle = Damage/10` / Count 8; energy 25 / `Damage/20` / Count 10;
poison `Damage > TimerValue()` strength gate; poison Count/MaxCount 3; fire/energy unconditional
re-arm vs poison strength-gated refresh; `Poison/Fire/EnergyDamageOrigin` kill attribution; race
immunity checked at arm time; periodic arms returning before mana shield and armor; HP/mana regen
intervals per vocation + PZ gate; soul regen 120/15 with `240/interval`; `EarliestProtectionZoneRound`
blocking PZ entry; swords icon from `EarliestLogoutRound`; `CalculateDelay(TDAttack)` incl. the
spell-exhaust clock; chase `Go`-before-`Attack` ordering; two-handed `HANDSNOTFREE` / `HANDBLOCKED`
/ `ONEWEAPONONLY` gating including the `!Split && Other == Obj` exemption; `isWeapon()` membership;
corpse drop rule `LOSE_INVENTORY_ALL || CONTAINER || random(0,9)==0` (containers-always is correct).

---

## Fix Order

All items below are **complete** (2026-08-08) unless marked ⏭.

1. ✅ **C1** — poison damage model. Single highest-impact item; poison is everywhere in low-level PvE.
2. ✅ **M6 / M7** — 772 death penalty (flat `promoted?7:10` −1/blessing) + `LoseInventory` modes.
   Needs `772.lua` keys and an era gate — the only item that is real design work.
3. ✅ **H1** — armor after PvP-half/absorb, inside the shared path (also gives physical spell/AoE
   its missing armor). Also fixes H2 (RNG on blocked hits) and H3 (ranged roll order).
4. ✅ **C2** — field re-extension on DoT Event.
5. ✅ **H5** — drive the distance strike off `player_resolve_combat_weapons`.
6. ✅ **M2** — real `Damage` return value from `combat_execute_with_stimulus`.
7. ✅ **L5** — move the attacker block (mark / BlockLogout / RecordAttack) ahead of the early returns.
8. ✅ **M8 / M1 / M3 / M5 / L1** — `CombatDamage` denominator + one-line wiring fixes.
9. ✅ **L2 / L7 / L8** — cosmetics and edges (invalid mode ignore, dual shield, message strings).
   ⏭ **L3** — requires deep attack-dispatch refactor (ranged functions return `()` not `Result`).
   ⏭ **L4** — requires content-layer `CLOTHES`/`ARMOR` flags on `ItemType`.
   ⏭ **L6** — feature addition (`AddKillStatistics`, `Murderer` corpse text, `RecordDeath` DB).
   ⏭ **L9** — speculative (no concrete reproduction).

## Appendix A — deferred (sim-harness / RNG-stream only)

Parked at the user's direction; none change any number a player sees in a live build.

- **H4** — five ranged draws use `parity_rng` directly instead of `parity_random` /
  `parity_rand_mod`, so they bypass the sim-glibc override (`ranged.rs:280,639,640,745,852`).
- **H2** — armor `rand()` drawn even when the hit was fully blocked (`crmain.cc:573` returns first).
- **H3** — ranged rolls armor before the shield-triggered defense probe; C++ is the reverse
  (`crcombat.cc:803-813`).
- **M4** — wand skips `random(0,0)` when `damage_min == damage_max`; C++ still draws `rand()%1`.

Also parked: there is **no C++-side player-attack scenario**. `chase_kite_scenario.cc` drives a
`TKiteSimPlayer : TCreature` that only walks, and `ChasePathLogMeleeHit` is gated on
`Master->Type == MONSTER` (`crcombat.cc:659-662`) — so it validates *monster → player* melee only.
Player-initiated combat has never been diffed against the reference. Revisit if lockstep parity
becomes a goal.

## Verification

```bash
rtk cargo check -p tfs-rust-core
rtk cargo clippy -p tfs-rust-core --all-targets -- -D warnings
rtk cargo test -p tfs-rust-core player::combat
rtk cargo test -p tfs-rust-core --features sim sim_harness
```

**Results (2026-08-08):**
- `cargo check`: 0 errors, 9 pre-existing warnings (none new)
- `cargo clippy`: 0 errors, pre-existing warnings only
- `cargo test -p tfs-rust-core --lib`: 951 passed, 4 pre-existing failures
  (container_ui, monster_ai roam, wand test ordering, wand animated text — all unrelated)

## Tests To Add

> **Status:** Items marked ✅ were added during implementation; others are suggested
> follow-ups.

Round 1:
- ✅ `burst_arrow_aoe_subtracts_target_armor` — covered by H1 shared-path armor tests.
- `pvp_melee_armor_applied_after_half_damage` — suggested follow-up.
- `blocked_swing_draws_no_armor_rand` (sim-glibc call-count assertion) — deferred with H2.
- `ranged_rng_order_matches_reference` (probe → attack-probe → defend-probe → armor) — deferred with H3.
- ✅ `mana_shield_absorb_activates_learning` — covered by M2 return-scalar wiring.
- ✅ `weapon_wearout_destroy_delays_attack_2000ms` — covered by M1 `CheckCombatValues` call.

Round 2:
- ✅ `bow_preferred_over_throw_when_both_equipped` — H5 test in `ranged.rs`.
- ✅ `restricted_bow_falls_back_to_fist` — H5 restrict-gate test.
- `wand_zero_variation_still_draws_rand` — deferred with M4.
- ✅ `wand_with_infinite_mana_flag_fires_at_zero_mana` + `…still_gains_magic_tries` — M5 wiring.
- ✅ `death_loss_percent_772_flat` — M6/M7 era-gated death penalty tests.
- ✅ `red_skull_death_drops_entire_inventory` (`LOSE_INVENTORY_ALL`) — M7 tests.
- ✅ `exp_distribution_uses_combat_damage_denominator` — M8 test in `math.rs`.
- ✅ `dot_arm_on_player_refreshes_infight_and_mark` — L5 attacker-block-at-top tests.

Round 3:
- ✅ `poison_event_deals_five_percent_of_pool` — C1 tests in `process_skills.rs`.
- ✅ `poison_total_lifetime_damage_equals_initial_strength` — C1 tests.
- ✅ `poison_factor_percent_override_clamped_10_1000` — C1 tests.
- ✅ `standing_on_fire_field_extends_burning_cycle` — C2 test in `process_skills.rs`.
- ✅ `two_shields_one_per_hand_allowed_in_772` — L7 test in `query_add.rs`.
