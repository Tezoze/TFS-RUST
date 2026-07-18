# 772 Monster AI Audit — Rust vs Reference Decompile

**Date:** 2026-07-02
**Scope:** Monster brain (`IdleStimulus`, stimuli, targeting, movement, combat, push/kick,
summons, spawn/home, talk, death/loot) — 772 era only.
**Rust side:** `crates/tfs-rust-core/src/` — `monster_ai.rs`, `idle_stimulus.rs`,
`monster_targets.rs`, `monster_events.rs`, `monster_push.rs`, `monster_distance_step.rs`,
`creature/monster*.rs`, `spawn_lifecycle.rs`.
**Reference:** `reference/cipsoft-772/tibia-game-master/src/` — `crnonpl.cc`, `crcombat.cc`,
`crmain.cc`, `crskill.cc`, `cr.hh`.

Findings are graded: **GAP** (reference behavior absent), **BUG** (implemented but diverges),
**SUSPECT** (probable divergence, needs a targeted check before fixing). Items marked
`[verified]` were confirmed by direct source comparison during this audit; the rest come from
subsystem sweeps and cite both sides.

---

## Summary table

| # | Sev | Kind | Area | Finding |
|---|-----|------|------|---------|
| 1 | HIGH | GAP | Targeting | Acquire path missing `IsHouse` filter `[verified]` |
| 2 | HIGH | GAP | Targeting | Acquire path missing invisibility filter `[verified]` |
| 3 | MED | BUG | Targeting | `CanSeeFloor` awake check reduced to exact-Z equality `[verified]` |
| 4 | MED | BUG | Targeting | Acquire filter uses `is_summon()` instead of `IsPlayerControlled` `[verified]` |
| 5 | HIGH | DONE | Spells | `SpellImpact::Field` places poison/fire/energy via `CreateField` |
| 6 | HIGH | DONE | Spells | `SpellImpact::Summon` via CASTING + CreateMonster |
| 7 | MED | GAP | Spells | `IMPACT_STRENGTH` (skill % buff/debuff) impact not modeled `[verified]` |
| 8 | MED | GAP | Spells | `IMPACT_OUTFIT` (shapeshift/illusion) impact not modeled `[verified]` |
| 9 | HIGH | GAP | Summons | `ConvinceMonster` (master takeover) not implemented anywhere `[verified]` |
| 10 | MED | GAP | Lifecycle | `LifeEndRound` summon/raid lifetime despawn missing `[verified]` |
| 11 | HIGH | BUG | Combat | Rust fabricates a ranged weapon auto-attack; 772 monsters have none — ranged is `DistanceFighting` + race spells `[verified]` |
| 12 | LOW | BUG | Combat | `damage_map` unbounded vs C++ 20-slot circular `CombatList` |
| 13 | MED | SUSPECT | Stimuli | Move stimulus does not distinguish `OBJECT_DELETED` from real moves |
| 14 | MED | SUSPECT | Death | Blood/slime pool keyed off item `fluidsource` attr, not race blood enum |
| 15 | MED | SUSPECT | Death | Magic-field clearing under magic-field corpses not found |
| 16 | MED | SUSPECT | Death | `LoseInventory` flag semantics (ALL / NONE / 10 %-per-slot) unimplemented |
| 17 | MED | SUSPECT | Home | Home=0 leash fallback to global despawn radius may not match C++ (no leash) |
| 18 | LOW | SUSPECT | Spawn | Respawn player-count scaling uses `player_by_name.len()` proxy |
| 19 | LOW | SUSPECT | Summons | Summon-of-summon: Rust depth guard vs C++ reassign-master-to-grandparent |
| 20 | LOW | SUSPECT | Push | Kick escape-tile `AVOID` flag coverage (chain-push recursion path) |

---

## 1–4. Targeting / acquisition (`crnonpl.cc:2470-2545` vs `idle_stimulus.rs` `monster_idle_772_acquire_target`)

### 1. Missing `IsHouse` filter in acquire path — HIGH, GAP `[verified]`
- Rust: `idle_stimulus.rs:640-643` — only checks `ZoneType::Protection`.
- C++: `crnonpl.cc:2516` — `|| IsHouse(Target->posx, Target->posy, Target->posz)` in the
  candidate-skip condition.
- 772 monsters will acquire (and chase) targets standing on house tiles. The **lose-target**
  path already has this check (`idle_stimulus.rs:549`, added in Phase 9 / AI#25); the acquire
  path was explicitly deferred (see `tasks/lessons.md` #79) and is still open.

### 2. Missing invisibility filter in acquire path — HIGH, GAP `[verified]`
- Rust: `idle_stimulus.rs:640-650` — no invisibility check.
- C++: `crnonpl.cc:2514` — `(Target->IsInvisible() && !RaceData[this->Race].SeeInvisible)`.
- Monsters without `SeeInvisible` acquire invisible targets. Same deferred-from-Phase-9 status
  as #1; the lose-target path has the check (`idle_stimulus.rs:555`), acquire does not.
- Fix note: both checks should slot in after the Protection-zone check to preserve C++
  condition order (dist → IGNORED → invisibility → PZ → house).

### 3. `CanSeeFloor` awake check reduced to exact-Z — MED, BUG `[verified]`
- Rust: `idle_stimulus.rs:625-627` — `if target.position().z == pos.z { should_sleep = false; }`.
- C++: `crnonpl.cc:2504-2506` — `if(Target->CanSeeFloor(this->posz)) ShouldSleep = false;`
  with `CanSeeFloor` (`cr.hh:576-582`): viewer z ≤ 7 sees any z ≤ 7; viewer z ≥ 8 sees |dz| ≤ 2.
- Consequence: a surface monster with a player one floor up/down (both ≤ 7), or an underground
  monster with a player within 2 floors, incorrectly falls asleep. Note the C++ check runs on
  the *target's* `CanSeeFloor` of the *monster's* z.

### 4. Acquire monster-filter semantics — MED, BUG `[verified]`
- Rust: `idle_stimulus.rs:628-630` — skips monsters where `!m.base.is_summon()`.
- C++: `crnonpl.cc:2500-2502` — skips monsters where `!IsPlayerControlled()`.
- A summon whose master is another **monster** is a summon but not player-controlled: C++ skips
  it as a target; Rust acquires it. Filter should walk the master chain to a player (or use an
  equivalent `is_player_controlled()` helper).

Correct in this area (verified this audit): candidate search radius `TFindCreatures(12,12)`
matches `collect_spectators(…,12,12)`; strategy threshold rolling (`crnonpl.cc:2475-2483`);
tie-breaker `random(0,99)` with `>` comparison; `IGNORED_BY_MONSTERS` right; dist > 10 skip.

**False positive cleared — Strategy 2 (most damage):** a sweep flagged
`idle_stimulus.rs:654-658` as reading "damage dealt" instead of "damage received". Verified
wrong: `damage_map` lives on the **victim**, keyed by attacker (`combat/mod.rs:98-117`
`apply_health_delta` writes `victim.damage_map[attacker] += dmg`), so the monster reading its
own map entry for the candidate is exactly C++ `this->Combat.GetDamageByCreature(TargetID)`
(`crnonpl.cc:2528`). No fix needed; see finding 12 for the only real (minor) delta.

---

## 5–8. Monster spell impacts (`crnonpl.cc:2571-2736` CASTING vs `monster_combat.rs` `SpellImpact` + `idle_stimulus.rs` cast executor)

### 5. `IMPACT_FIELD` — HIGH, DONE (2026-07-18)
- Rust: `parse_spell_impact` maps `poisonfield`/`firefield`/`energyfield` →
  `SpellImpact::Field { field_type }`; `monster_create_field` implements 772
  `CreateField` (`FieldPossible`, delete MAGICFIELD, place 1490/1487/1491).
- C++: `crnonpl.cc:2596-2600` / `magic.cc:167–172` / `:984`.

### 6. `IMPACT_SUMMON` — HIGH, DONE (XML `<summons>` → CASTING)
- Rust: CASTING Origin r=0 + `SearchSummonField` / `SearchFreeField` / reparent /
  `broadcast_creature_appear`.
- C++: `crnonpl.cc:2648-2655`, `magic.cc:385–395`, `CreateMonster` `:3158`.
- C++: `TSummonImpact(this, SummonRace, MaxSummons)` spawns bound summons.
- Orc shamans / necromancers etc. never summon.

### 7. `IMPACT_STRENGTH` missing — MED, GAP `[verified]`
- Rust: `monster_combat.rs:29-58` — `SpellImpact` enum has no Strength variant; the XML→enum
  conversion cannot produce one.
- C++: `crnonpl.cc:2629-2638` — `TStrengthImpact(this, Skills, Percent, Variation, Duration)`
  (skill percentage buff/debuff, e.g. warlock/priestess skill drains).

### 8. `IMPACT_OUTFIT` missing — MED, GAP `[verified]`
- Rust: `monster_combat.rs:29-58` — no Outfit variant.
- C++: `crnonpl.cc:2639-2646` — `TOutfitImpact` (temporary shapeshift/illusion).

Correct in this area (verified): probability gate `rand() % Delay == 0` per spell per idle
round with **no** `EarliestSpellTime` exhaustion (monster CASTING at `crnonpl.cc:2577` bypasses
`CheckMana`/`CastSpell`, so the Rust probability-only gate is parity — an earlier sweep flagged
this as a gap; cleared). Flee cast damper `random(1,3) != 1` matches (`idle_stimulus.rs:809`).
Damage/Healing/Speed/Drunken impacts and shape handling present.

---

## 9. `ConvinceMonster` not implemented — HIGH, GAP `[verified]`
- Rust: `rg -i convince crates/` → zero matches.
- C++: `crnonpl.cc:~3100-3194` — `TMonster::Convince` / convince flow: reassign `Master` to the
  player, clear `Home`, adjust old/new master summon counts, clear ToDo list, `ToDoWait(100)`.
- Blocks the convince-creature rune and any player-controlled wild monster. Also feeds finding
  4 (`IsPlayerControlled`) — convinced monsters must count as player-controlled targets/wakers.

## 10. `LifeEndRound` lifetime missing — MED, GAP `[verified]`
- Rust: `rg -i "life_end|lifetime" crates/` → zero matches.
- C++: `crnonpl.cc:2352-2357` — top of `IdleStimulus`:
  `if(LifeEndRound != 0 && LifeEndRound <= RoundNr) { StartLogout(true,true); State = SLEEPING; }`.
- Needed for timed monsters (raid waves / summon lifetimes set via `crmain.cc:2062-2064`).

---

## 11. Monster ranged combat — fabricated weapon auto-attack; 772 uses `DistanceFighting` + race spells — HIGH, BUG `[verified]`

**Correction (supersedes the earlier "diverges from `TCombat::DistanceAttack`" framing).** Monsters
in 772 never run `TCombat::DistanceAttack`. Ranged behaviour is two race-data mechanisms, neither
of which is a carried/equipped weapon:

- **Positioning** — `RaceData[Race].DistanceFighting`, a race flag parsed from the
  `"distancefighting"` monster flag (`crmain.cc:1497-1498`, default `false` `:1253`). The idle
  brain (`crnonpl.cc:2797-2800`) branches on it: `!DistanceFighting || !ThrowPossible(...)` →
  close/melee positioning; otherwise the distance band (flee if `<4`, chase if `>4`, dance at
  `==4`, `crnonpl.cc:2836-2869`).
- **Damage** — race spells in the CASTING block (`crnonpl.cc:2568-2740`): typically
  `SHAPE_VICTIM`/`SHAPE_DESTINATION` + `IMPACT_DAMAGE` with a projectile `Animation`. Gated only
  by `rand() % Delay == 0`; **no** distance hit-probe. Damage is `ComputeDamage(this,0,base,var)`
  → `TCreature::Damage` (`crmain.cc:487`), which applies only the *target's PROTECTION gear*
  reduction (players) — **never** a monster shielding-defense or armor roll. The defense roll
  (`Attack − Defense`) exists only in `CloseAttack` (`crcombat.cc:647`), the melee path.

Why monsters can't reach `DistanceAttack`: `GetWeapon` (`crcombat.cc:36-102`) scans **hand slots
only** (`INVENTORY_HAND_FIRST..LAST`); monster spawn (`crnonpl.cc:2050-2101`) creates rolled
`RaceData` items — including any bow/throw — into a **loot `Bag`** (`INVENTORY_BAG`), never into
hands. So a spawned monster has `Close/Missile/Throw/Wand = NONE`, `Fist = true` →
`GetDistance() == 1` (`crcombat.cc:309-319`) → `TCombat::Attack` (`crcombat.cc:611-637`) always
takes `CloseAttack`. The entire ammo/fragility/±1-scatter/90%–75%-probe path is **player-only**.

Consequence for the doubted example: a minotaur archer shoots because it has `DistanceFighting` +
a damage spell with a bolt animation — **not** because it carries a crossbow. Any crossbow it
drops is probabilistic loot (`random(0,999) > ItemData->Probability`, `crnonpl.cc:2057`),
completely decoupled from how it attacks. There is no "spawned without the weapon → fights melee"
coupling, and no 100%-drop requirement.

### Real Rust divergence `[verified]`

Rust synthesizes a monster ranged **weapon** auto-attack that 772 does not have:

- `monster_ai.rs:483-575` (`monster_do_attacking`, ranged branch) keys off
  `has_ranged_spell = spells.any(|s| s.range > 1)` and `monster_weapon_attack_distance`
  (`creature/monster_combat.rs:178-186`, returns 3 when `melee_skill <= 0 && has_ranged_spell`).
  At Chebyshev `2..=weapon_dist` it rolls `weapon_damage(melee_skill, melee_attack, Balanced)`,
  subtracts `roll_target_defense` **and** `armor_reduction`, broadcasts a `Spear`/`Arrow` shoot
  effect, and applies physical damage.
- The same `TodoExecuteKind::DistanceAttack` then runs the **real** spell path:
  `run_monster_todo_execute` → `monster_idle_try_casting` (`idle_stimulus.rs:2472-2476`).

So a distance fighter fires **both** in one beat:

1. **Double damage / phantom projectile.** When the monster has melee stats, the fabricated shot
   lands *plus* the spell — two damage sources. When it's a pure caster (`melee_skill = melee_attack
   = 0`, common for archer-type races), `weapon_damage(0,0)` is ~0, but it still broadcasts a
   spurious shoot effect and burns the attack cadence alongside the real spell.
2. **Wrong damage shape even when it lands.** 772 spell damage has no shielding-defense roll and
   no armor roll (`TCreature::Damage`), and is not a fight-mode weapon formula. Rust's ranged
   branch applies all three.
3. **No `DistanceFighting` flag.** Rust infers ranged from `target_distance > 1`
   (`monster_effective_target_distance`, `idle_stimulus.rs:2325-2333`). Acceptable as a positioning
   generalization, but "ranged" is derived from spell range / target distance, not the race flag.

**Fix direction:** remove the ranged branch from `monster_do_attacking` entirely — monsters only
`CloseAttack` at adjacency (melee, `SKILL_FIST > 0`). Let *all* monster ranged damage come from
`monster_idle_try_casting` (the CASTING spells), with the projectile sourced from each spell's own
`shoot_effect` rather than a synthesized `Spear`/`Arrow` default. Positioning already flows from
`target_distance`; optionally load/track `DistanceFighting` for exact parity. Note the CASTING
path's "always applies" (no hit-probe) is actually correct for 772 monster spells — the earlier
"always hits = over-buff" concern was an artifact of the wrong (weapon-based) mental model.

Correct nearby (verified) — these belong to the **melee** (`CloseAttack`) path in
`monster_do_attacking` and remain valid: cross-floor block (`ObjectDistance` = INT_MAX on z
mismatch), the 7×5 viewport clamp, sight-line gate, 200 ms probe / 2000 ms attack delays,
`CloseAttack` melee (attack − defense − armor) shape, poison-on-hit `(rand()%5)==0` gate.

## 12. Damage tracking container — LOW, BUG
- Rust: `creature/base.rs:141` — unbounded `HashMap<CreatureId, u64>`.
- C++: `cr.hh:418` — `TCombatEntry CombatList[20]` circular buffer (`crcombat.cc:864-881`);
  entries beyond 20 attackers evict oldest.
- Affects Strategy 2 selection and kill-credit distribution only in >20-attacker pile-ons.
  Consider capping at 20 with FIFO eviction for strict parity.

---

## 13–20. SUSPECT — verify before fixing

### 13. `OBJECT_DELETED` move-stimulus type — MED
- Rust: `idle_stimulus.rs:334-378` `monster_sleep_wake_on_creature_move` has no move-type
  parameter.
- C++: `crnonpl.cc:2943-2982` — `CreatureMoveStimulus(CreatureID, Type)` skips sleep→idle wake
  when `Type == OBJECT_DELETED`. Check whether the Rust call sites ever fire the stimulus on
  creature removal (logout/death); if so, sleepers wake on deletions they shouldn't.

### 14. Blood pool source — MED
- Rust: `creature/monster_inventory.rs:417-420` keys pool on corpse item `fluidsource="blood"`.
- C++: `crmain.cc:210-226` keys on `RaceData[Race].Blood` (BT_BLOOD / BT_SLIME). Verify every
  race that bleeds/slimes in the reference maps onto an item with the right attribute;
  otherwise pools will be missing/wrong for some races.

### 15. Magic-field clearing under corpses — MED
- C++: `crmain.cc:233-245` deletes existing magic fields on the tile when placing a corpse
  with the MAGICFIELD flag. Not found in `drop_monster_corpse_772`. Verify and port.

### 16. `LoseInventory` flag — MED
- C++: `crmain.cc:267-282` — inventory drop honors `LoseInventory` (ALL / NONE / 10 % per
  slot). Rust drops unconditionally. Verify whether the flag applies to monsters in 772 (it
  may be player-only) before changing.

### 17. Home=0 leash fallback — MED
- Rust: `monster_ai.rs:2820-2826` `monster_roam_leash_radius` falls back to global despawn
  radius when `home_radius <= 0`.
- C++: `crnonpl.cc:2408-2415` only leashes when `Home != 0`. If C++ never leashes homeless
  monsters, the Rust fallback is a silent behavior change.

### 18. Respawn player-count scaling — LOW
- Rust: `spawn_lifecycle.rs:728-748` uses `player_by_name.len()`.
- C++: `crnonpl.cc:1314-1322` `GetNumberOfPlayers()`. Verify ghost/GM exclusion semantics.

### 19. Summon-of-summon — LOW
- C++: `crnonpl.cc:2020-2027` errors and reassigns master to the grandparent.
- Rust: `monster_push.rs:92` bounds master-chain walks at 8. Outcomes match only if
  summon-of-summon cannot be created; confirm the summon creation path reassigns.

### 20. Kick escape-tile validation — LOW
- C++: `crnonpl.cc:3062-3067` skips kicker tile + `AVOID` flag before relocating the blocker.
- Rust: `monster_push.rs:375-397` docs claim parity incl. F2 execute-mode gate; the recursive
  chain-push path (`monster_kick_creature_772_inner`) should get a regression test covering
  AVOID (magic-field) escape tiles.

---

## Verified-correct highlights (no action)

- **Idle walk branch tree** (`crnonpl.cc:2676-2869`): flee → master-follow → melee/dist arms →
  roam ordering; dist bands `< / > / ==` on per-type `target_distance` (approved
  generalization of the hardcoded 4); dist-chase budget `Distance − keepDist`; melee budget
  keyed per branch; dance `rand()%5` W/E/N/S/none order (`sim_glibc_rand.rs:237`, test
  `test_772_dance_dir_order_matches_cpp`).
- **Flee threshold** `HitPoints <= FleeThreshold` with summon/challenge guards.
- **Stimulus lifecycle**: DamageStimulus SLEEPING/IDLE → PANIC/UNDERATTACK transitions;
  wake filter (NPCs never wake, player-controlled monsters wake); summon master-despawn
  (dz > 1 / dx,dy > 30); lose-target condition set incl. Phase-9 House/Invisible additions;
  `MonsterhomeInRange` ±2 z tolerance.
- **Kick-kill attribution** (`crnonpl.cc:3076-3080`): full-HP damage recorded in the victim's
  damage map for kill credit, block-hit effect, death pipeline (`monster_push.rs:464-491`).
- **Talk**: `(rand()%50)==0` gate, 1-indexed talk pick, `#y` yell prefix incl. short-string
  edge case.
- **772 `ProcessCreatures` is regen/death-safety only** — no think sweep (lessons #85).

## Suggested fix order

1. #5, #6 — Field + Summon impacts (highest gameplay impact, self-contained executor arms).
2. #1, #2, #4 — acquire-path filters (small, mirrors existing lose-target checks; #4 wants an
   `is_player_controlled()` chain helper shared with the wake filter).
3. #3 — `CanSeeFloor` helper (also reusable by finding 13's stimulus work).
4. #11 — remove the fabricated ranged weapon auto-attack (`monster_do_attacking` ranged branch);
   route all monster ranged damage through CASTING (#5/#6 land the missing impacts).
5. #9, #10 — convince + lifetime (needed for rune support and raids).
6. #7, #8, #12 and the SUSPECT batch after targeted verification.

Verification: `rtk cargo test -p tfs-rust-core` (idle_stimulus_tests, monster_ai_world_tests,
monster_push_tests cover the touched arms); add F1-style regression tests per fix.
