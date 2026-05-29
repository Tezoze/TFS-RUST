# Rust Port Reference Corrections: 1.4 vs 1.8 Logic Diffs

**Context:** Your Rust port used TFS 1.4 (AuseraServer) as its reference. TFS 1.8 contains bug fixes and logic changes. This document identifies where your Rust implementation may be wrong because 1.4 was wrong, and separately where 1.8 changed behaviour you should intentionally keep from 1.4.

**Rule of thumb used throughout:**
- `[BUG]` — 1.4 had a bug; 1.8 fixed it; **fix your Rust port**
- `[KEEP 1.4]` — 1.8 changed the behaviour deliberately (usually for custom systems); **keep your Rust port as-is, it's more faithful to 8.6 gameplay**
- `[NEW 1.8]` — 1.8 added a whole new system; only relevant if you plan to add it later

---

## COMBAT (`combat.cpp` / `combat.h`)

### 1. `[BUG]` Wand/rod origin is wrong — `ORIGIN_MELEE` instead of `ORIGIN_WAND`

**1.4:** Wands fall through to `ORIGIN_MELEE` in the origin switch.  
**1.8:** Explicit `case WEAPON_WAND: damage.origin = ORIGIN_WAND;`

**Rust impact:** Any code branching on `damage.origin` for leech/crit eligibility will incorrectly apply melee rules to wand hits.

---

### 2. `[BUG]` WEAPON_FIST never advances the skill

**1.4:** The fist weapon branch is missing a `addSkillAdvance(SKILL_FIST, ...)` call.  
**1.8:** Fixed — fist fighting gains tries like every other skill.

**Rust impact:** Players using fist fighting never train the skill.

---

### 3. `[BUG]` Leech/crit special skill scale mismatch

**1.4:** Special skills (crit chance, leech chance) are stored and compared at 1–100 scale in some paths and 1–10000 in others.  
**1.8:** Normalised — chance stored as 1–10000 (`/10000.0` divisor), amount stored as 1–100 (`/100.0` divisor).

**Rust impact:** If your Rust port copied the 1.4 scale, all leech and critical hit chances are silently broken — either never trigger or always trigger.

**Correct formula:**
```rust
// chance: stored as 0–10000 (basis points)
// amount: stored as 0–100 (percent)
let triggered = rng.gen_range(1..=10000) <= skill_chance;
let bonus = base_damage as f64 * (skill_amount as f64 / 100.0);
```

---

### 4. `[BUG]` Dead creature receives condition damage after death

**1.4:** `ConditionDamage::doDamage` does not check `creature->isDead()` before applying damage ticks.  
**1.8:** Guard added at entry: if creature is dead or removed, return immediately.

**Rust impact:** A condition tick scheduled before death fires after death, causing double-kill or phantom health changes.

---

### 5. `[BUG]` `ConditionSpeed::endCondition` doesn't guard against null creature removal

**1.4:** Missing removal guard — can operate on a despawned creature reference.  
**1.8:** Guard added.

---

### 6. `[BUG]` `ConditionAttributes::unserializeProp` — missing array bounds check

**1.4:** Reads skill/stat indices directly without bounds checking — can read out of bounds on a corrupted save blob.  
**1.8:** Bounds checked before array access.

**Rust impact:** Rust will panic on out-of-bounds by default, so this is safe. But malformed save data will crash your server rather than being silently ignored — decide if you want to log-and-skip instead.

---

### 7. `[KEEP 1.4]` Fight mode defense multipliers

**1.4:** Defensive mode: defense roll × 1.8; Offensive mode: defense roll × 0.6.  
**1.8:** Removed entirely — fight mode is cosmetic only.

**Rust impact:** Keep the 1.4 multipliers. This is core 8.6 gameplay feel.

---

### 8. `[NEW 1.8]` Forge dodge, fatal hit, imbuement damage hooks

Entirely new in 1.8 — `damage.dodge`, `damage.fatal` flags, forge tier armor check. Not present in 1.4. Add only when you implement the Forge system.

---

## CREATURES (`creature.cpp`)

### 9. `[BUG]` Movement speed formula — logarithmic coefficients

**1.4 formula:**
```cpp
int32_t speed = speedA * std::log((level + speedB) * speedC);
```
**1.8 formula:**
```cpp
int32_t speed = std::floor(level * speedA / speedB) + speedC;
// linear, not logarithmic
```

**Rust impact:** Player speed at every level is calculated incorrectly if you copied the 1.4 logarithmic formula. This is a fundamental movement feel issue.

---

### 10. `[KEEP 1.4]` Diagonal movement cost — 3× for all creatures

**1.4:** All creatures pay 3× speed cost for diagonal steps.  
**1.8:** Players pay 2× for diagonals; monsters still pay 3×.

**Rust impact:** Keep 1.4 behaviour (3× all) for authentic 8.6 feel. The 1.8 change makes players feel faster diagonally, which is a post-8.6 change.

---

### 11. `[BUG]` Block count regen logic

**1.4:** Every 1000ms: `blockCount++` (accumulates).  
**1.8:** Every 2000ms: resets to `hasShield ? 2 : 1` (correct reset behaviour).

**Rust impact:** In 1.4, `blockCount` climbs unboundedly. Since shield usage is gated on `blockCount >= 1`, this only has subtle effects, but the reset logic is what Tibia 8.6 actually did. Check your `walk.rs`/creature tick.

---

### 12. `[KEEP 1.4]` Unfair fight reduction on PvP death

**1.4:** When killed by a player significantly higher level, skill loss is reduced proportionally.  
**1.8:** Removed.

**Rust impact:** Keep it. This is a core 8.6 PvP mechanic that protects low-level players from griefing.

---

## MONSTERS (`monster.cpp`)

### 13. `[BUG]` Division by zero in `canUseSpell` when `speed == 0`

**1.4:** `if (attackTicks % sb.speed >= interval)` — crashes if `sb.speed == 0`.  
**1.8:** `if (sb.speed == 0 || attackTicks % sb.speed >= interval)`.

**Rust impact:** Rust's integer modulo panics on divide-by-zero in debug mode, silently wraps in release. A monster spell with `speed="0"` in XML will crash your server in debug. Add the guard.

**Same bug exists in `onThinkDefense` and summon speed.**

---

### 14. `[BUG]` `minCombatValue > maxCombatValue` swap missing in `doAttacking`

**1.4:** If a monster XML defines min > max by accident, the range is used as-is.  
**1.8:** Added `if (minCombatValue > maxCombatValue) std::swap(...)` before the RNG call.

**Rust impact:** Monster XML with inverted min/max causes `uniform_random(max, min)` — undefined behaviour in C++, panic in Rust. Add the swap.

---

### 15. `[BUG]` Stale `followCreature` freezes monster targeting

**1.4:** If `followCreature` is in a PZ, `searchTarget()` is never called — monster freezes.  
**1.8:** Stale follow target in PZ is cleared before `searchTarget()`.

**Rust impact:** Monsters that were targeting a player who entered a PZ will freeze indefinitely. Add the stale-follow cleanup in your monster think tick.

---

### 16. `[BUG]` Summon per-type cap not enforced mid-loop

**1.4:** Iterates summons list counting by name each iteration but doesn't update the map mid-loop — in one tick, a monster can spawn multiple summons past its per-type `max`.  
**1.8:** Uses `unordered_map<string, uint32_t> summonCounts` updated after each successful spawn.

**Rust impact:** Without this fix, a monster with `max="1"` for a summon type can spawn 2+ in a single defense tick if the scheduler fires twice fast.

---

### 17. `[KEEP 1.4]` Monster sight range — 9×9 across all floors

**1.4:** `Map::getSpectators` with `9×9` range and `multifloor=true` for monster sight.  
**1.8:** Changed to `9×7` and same-z-only enforcement.

**Rust impact:** Keep 1.4 (9×9, multifloor). This is 8.6 monster behaviour.

---

### 18. `[KEEP 1.4]` Target search modes — `NEAREST` and `RANDOM` only

**1.8** adds `HEALTH` (target lowest HP%) and `DAMAGE` (target highest damage dealer) search modes. These don't exist in 8.6 monster XMLs. Keep your Rust port to `NEAREST`/`RANDOM` for now; the new modes are an opt-in extension.

---

### 19. `[NEW 1.8]` Monster faction system

Monsters check `mType->info.faction` before treating another creature as an opponent. Not in 1.4. Only add when you want faction-aware monsters.

---

## PLAYERS (`player.cpp`)

### 20. `[BUG]` Healing dispatch — type check vs value sign check

**1.4:** Detects a heal by `if (damage.primary.value > 0)` — positive value = heal.  
**1.8:** Detects by `if (damage.primary.type == COMBAT_HEALING)` — type-first.

**Rust impact:** If a typed heal arrives with `value == 0` (e.g., a heal that was fully absorbed), 1.4 routes it as damage. 1.8 always routes by type. Your `combatChangeHealth` in Rust should check type first.

---

### 21. `[BUG]` `combatChangeHealth` — no dead/removed guard at entry

**1.4:** Processes health changes on creatures that died during a callback chain.  
**1.8:** Entry guard: `if (target->isDead() || target->isRemoved()) return false;`

**Rust impact:** In Rust with your SlotMap arena, accessing a removed creature via a stale ID would fail at the SlotMap lookup. Verify you're checking for this before dispatching health changes.

---

### 22. `[KEEP 1.4]` Death skill/exp loss — `unfairFightReduction`

Already covered under creatures #12. Keep it.

---

## SPELLS (`spells.cpp`)

### 23. `[BUG]` Dead target check missing before spell execution

**1.4:** Spell `executeCastSpell` does not recheck if the target is still alive between cast start and execution.  
**1.8:** Added `if (!target || target->isDead())` guard at the top of execution.

**Rust impact:** A scheduled spell cast can fire at a corpse and apply effects (damage, conditions) to a dead creature, potentially causing health to go more negative or conditions to apply to despawned entities.

---

### 24. `[BUG]` `canThrowSpell` — missing null pointer guard

**1.4:** Calls methods on `toTile` without checking it's non-null (can happen for out-of-map positions).  
**1.8:** Null check added before tile access.

**Rust impact:** In Rust this would be an `Option` unwrap panic. Ensure your `can_throw_spell` equivalent returns `Err`/`None` when the destination tile doesn't exist.

---

### 25. `[BUG]` `vocSpellMap` — vocation 0 (no vocation) excluded from learned spells

**1.4:** Spell learning only checks vocation IDs > 0, so vocation 0 (no-voc) players can't learn any spells even if the XML says they should.  
**1.8:** Includes vocation 0 in the spell map check.

---

## TILE / MAP (`tile.cpp` / `map.cpp`)

### 26. `[BUG]` LOS (`isSightClear`) is asymmetric

**1.4:** Checks LOS in one direction only — `isSightClear(A, B)` can return true while `isSightClear(B, A)` returns false for the same two tiles.  
**1.8:** Checks both directions and returns true only if both pass.

**Rust impact:** Your `map/los.rs` — check whether your LOS implementation is bidirectional. Asymmetric LOS means a creature can throw at you when you can't throw back.

---

### 27. `[BUG]` Spectator Z-range at surface floors (z=6/7) is unbounded in 1.4

**1.4:** Any event at z=6 or z=7 (surface) calls `getSpectators` across **all floors 0–9** including underground, causing massive unnecessary event dispatch.  
**1.8:** Surface floor spectators are capped to ±2 floors from the event position.

**Rust impact:** Your `map/mod.rs` `getSpectators` equivalent — if you ported 1.4 logic, every combat event on the surface triggers lookups on underground floors too. High CPU cost on busy maps.

**Correct 1.8 logic:**
```rust
// At surface (z <= 7):
let min_z = z.saturating_sub(2);
let max_z = (z + 2).min(7); // cap at surface
// Underground z > 7: only same floor
```

---

### 28. `[BUG]` `decayTo` comparison is `!= 0` — should be `> 0`

**1.4:** `if (it.decayTo != 0)` — a `decayTo` value of -1 (used as "no decay target") passes this check and attempts to transform to item ID 0xFFFF.  
**1.8:** `if (it.decayTo > 0)`.

**Rust impact:** In Rust, your `ItemType` likely uses `Option<u16>` for `decay_to`, making this a non-issue. But if you used a sentinel value (0 or u16::MAX), verify the comparison is correct.

---

### 29. `[BUG]` `internalMoveItem` — `flags` not cleared after first destination hop

**1.4:** `flags` from the original move is passed unchanged to the second hop in a multi-container move, causing incorrect pickup/movement validation on the second step.  
**1.8:** `flags = 0` after the first hop.

---

### 30. `[BUG]` Player pathfinding does not avoid magic fields

**1.4:** Auto-pathfinding (`getPathTo`) ignores magic field tiles — player walks through fire/poison fields when auto-walking.  
**1.8:** Magic field tiles cost extra in the pathfinding heuristic, causing players to route around them.

**Rust impact:** Your `pathfinding.rs` — verify magic field tiles add penalty to the A* cost.

---

### 31. `[BUG]` `ignoreBlocking` and `allowPickupable` are collapsed into one flag

**1.4:** `allowPickupable` flag is dual-purposed — used both to allow pickup AND to ignore blocking.  
**1.8:** Split into two separate flags with distinct semantics.

**Rust impact:** In your `queryAdd` / tile move validation, using one flag for two purposes causes items that should block movement to be passable (or vice versa) in certain edge cases.

---

### 32. `[NEW 1.8]` Decay duration is a random range, not a fixed value

**1.8 `ItemType`:** Has both `decayTimeMin` and `decayTimeMax`. When an item starts decaying, duration is `uniform_random(min, max)`.  
**1.4:** Single `decayTime` value only.

**Rust impact:** Your `items.rs` XML parser in `tfs-rust-content` and your `ItemType` struct — if you only have one decay duration field, items with XML-defined random decay durations will always use a fixed value. Add both fields and roll on decay start.

---

## IOLOGINDATA / PERSISTENCE (`iologindata.cpp`)

### 33. `[BUG]` `levelPercent` precision — integer division truncates

**1.4:** `levelPercent = (experience - expForLevel) / (expForNextLevel - expForLevel) * 100` — integer division happens before the multiply, losing precision.  
**1.8:** Multiply first: `((experience - expForLevel) * 100) / (expForNextLevel - expForLevel)`.

**Rust impact:** Your `player.rs` DB layer — check your XP-to-percent calculation. In Rust with integer types, the order of operations matters identically.

---

### 34. `[BUG]` Guild war query doesn't filter by `status = 1` (active wars only)

**1.4:** `SELECT guild1, guild2 FROM guild_wars WHERE (guild1=X OR guild2=X) AND ended=0` — loads wars with any status including pending/rejected.  
**1.8:** Adds `AND status = 1` to only load active (accepted) wars.

**Rust impact:** Your `tfs-rust-db` guild war query — players in pending war invitations would incorrectly have red skulls against each other.

---

## VOCATIONS (`vocation.cpp`)

### 35. `[KEEP 1.4]` No per-vocation PvP damage multipliers

**1.8** adds `pvpDamageDealt` and `pvpDamageReceived` per vocation. Not in 1.4. Do not add unless you want this.

---

### 36. `[NEW 1.8]` Mitigation system

**1.8** adds a `mitigation` factor per vocation reducing incoming damage. Not in 1.4, not 8.6 behaviour. Skip.

---

## Priority Order for Rust Fixes

| Priority | Item | File in Rust Port |
|---|---|---|
| 🔴 Critical | LOS asymmetry (#26) | `crates/tfs-rust-core/src/map/los.rs` |
| 🔴 Critical | Spectator Z-range bug (#27) | `crates/tfs-rust-core/src/map/mod.rs` |
| 🔴 Critical | Healing dispatch by type not sign (#20) | `crates/tfs-rust-core/src/combat/mod.rs` |
| 🔴 Critical | Leech/crit scale mismatch (#3) | `crates/tfs-rust-core/src/combat/mod.rs` |
| 🔴 Critical | Division by zero in monster spell speed (#13) | `crates/tfs-rust-core/src/creature/monster.rs` |
| 🔴 Critical | Speed formula logarithmic vs linear (#9) | `crates/tfs-rust-core/src/creature/base.rs` |
| 🔴 Critical | Dead/removed guard in combatChangeHealth (#21) | `crates/tfs-rust-core/src/combat/mod.rs` |
| 🟠 High | Dead creature receives condition damage (#4) | `crates/tfs-rust-core/src/condition.rs` |
| 🟠 High | Dead target check in spell execution (#23) | `crates/tfs-rust-core/src/spell.rs` |
| 🟠 High | Guild war active-only query (#34) | `crates/tfs-rust-db/src/player.rs` |
| 🟠 High | levelPercent precision (#33) | `crates/tfs-rust-core/src/creature/player.rs` |
| 🟠 High | Stale followCreature freezes monster (#15) | `crates/tfs-rust-core/src/creature/monster.rs` |
| 🟠 High | Summon per-type cap mid-loop (#16) | `crates/tfs-rust-core/src/creature/monster.rs` |
| 🟠 High | minCombatValue > maxCombatValue swap (#14) | `crates/tfs-rust-core/src/creature/monster.rs` |
| 🟠 High | Block count regen reset vs increment (#11) | `crates/tfs-rust-core/src/creature/base.rs` |
| 🟡 Medium | Wand origin ORIGIN_WAND not ORIGIN_MELEE (#1) | `crates/tfs-rust-core/src/weapon.rs` |
| 🟡 Medium | Fist skill never trains (#2) | `crates/tfs-rust-core/src/combat/mod.rs` |
| 🟡 Medium | decayTo > 0 comparison (#28) | `crates/tfs-rust-core/src/decay.rs` |
| 🟡 Medium | Decay random min/max range (#32) | `crates/tfs-rust-content/src/items.rs` |
| 🟡 Medium | internalMoveItem flags not cleared (#29) | `crates/tfs-rust-core/src/cylinder.rs` |
| 🟡 Medium | Pathfinding avoids magic fields (#30) | `crates/tfs-rust-core/src/pathfinding.rs` |
| 🟡 Medium | canThrowSpell null tile guard (#24) | `crates/tfs-rust-core/src/spell.rs` |
| 🟢 Low | Vocation 0 excluded from spells (#25) | `crates/tfs-rust-core/src/spell.rs` |
| 🟢 Low | ignoreBlocking/allowPickupable split (#31) | `crates/tfs-rust-core/src/tile.rs` |
| 🟢 Low | ConditionAttributes bounds check (#6) | `crates/tfs-rust-core/src/condition.rs` |

---

*Generated from 4-agent parallel diff of `/opt/AuseraServer-1.4` (TFS 1.4) vs `/tmp/tfs-downgrade-86` (TFS 1.8) — focused on logic correctness for the Australis Rust Port.*
