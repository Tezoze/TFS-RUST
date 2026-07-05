# Player Combat System — Implementation Plan (772 mechanics, TVP data shape)

**Goal.** Wire the player weapon-combat *strike* into the unified ToDo engine so that
**formulas and flow match the CipSoft 7.72 decompile** (`tibia-game-master/src/`), while all
**tunable data lives in TVP-shaped files** (vocation combat block — migrated to `data/vocations.lua`
per §0.10/PC-0, superseding `data/XML/vocations.xml`; weapon attack/defense/armor from
`items.otb`/`items.xml`). No CipSoft source is transcribed — we replicate observable outcomes in
idiomatic Rust (`tfs-core.md` porting model). Era knobs stay in `MechanicsProfile` /
`data/formulas/772.lua`; per-vocation balance stays in `data/vocations.lua`.

**Active target:** `clientVersion = 772` (`config.lua:17`). 1098 shares the same code paths through
`MechanicsProfile`; this plan does not add version branches to core. Architecture reference for the
Tier-1/Tier-2 profile split: `tfs-mechanics-profile.md` (steering) + `formulas.rs` module doc
comment — **not** `docs/PROTOCOL_VERSIONING.md`, which does not exist in this repo (see §0.1).

> **Audit history.** Pass 2 re-verified every claim in this file against the current tree
> (`crates/tfs-rust-core`, `tfs-rust-content`, `tfs-rust-net`, `tfs-rust-lua`); corrections/gaps live
> in §0.1–§0.9. Pass 3 (this revision) cross-referenced `docs/DATA_FORMAT_MIGRATION.md` and aligned
> Phase PC-0 (vocation data) onto that doc's Lua-as-data pilot instead of extending the outgoing XML
> parser — see §0.10. Nothing in §1–§8 was re-derived from stale assumptions beyond what §0 calls out.

---

## 0. Audit pass 2 — corrections and additions

### 0.1 Broken citation — `docs/PROTOCOL_VERSIONING.md` does not exist
The plan cites `docs/PROTOCOL_VERSIONING.md` five times (intro, §2.1, §3.6, §12/§12.13 refs). **This
file is not in the repo** (`find` confirms no match anywhere). The actual docs directory has no
protocol-versioning doc at all — closest relatives are `docs/GAME_LOOP_ARCHITECTURE.md` (loop-mode
selection from `MechanicsProfile`) and `docs/REFACTOR_AUDIT.md` (era-selection-by-profile rule). The
Tier-1/Tier-2 split described in `tfs-mechanics-profile.md` (steering) is the real source for that
model — **cite the steering file and `formulas.rs` module doc comment, not a missing doc.** Fix all
five references before this plan is used as an implementation checklist (a `todo` grep of the doc
for a nonexistent path is a footgun for whoever implements PC-3a).

### 0.2 `FightModes` (`0xA0`... actually `0xA7`) parse is *closer to done* than "unwired"
§2.8 says `SetAttackMode`/`SecureMode` storage is "currently unwired" and describes adding a new
`0xA0` parse. Checked `game_parse.rs` + `game_loop.rs`:
- The packet is already parsed (`C::FIGHT_MODES`, `game_parse.rs`) into
  `GamePacket::FightModes { raw_fight_mode, raw_chase_mode, raw_secure_mode, raw_pvp_mode }` — no
  new parse needed, PC-4 step 1 is **already done**.
- `game_loop.rs` already handles it and sets `chase_mode` (772 `SetChaseMode`, NONE/CLOSE clamp,
  `Following` override respected). What's actually missing is **only**: (a) storing
  `raw_fight_mode` → an `attack_mode: FightMode` field (currently discarded via `..`), and (b)
  storing `raw_secure_mode` → `secure_mode: bool`. Update PC-4 step 2 to say "extend the existing
  `FightModes` arm" rather than implying net-side work is needed.
- Note the wire opcode is `0xA7` client→server (`FIGHT_MODES`) — `0xA0` is the *server→client*
  `AddPlayerStats` opcode (confusingly reused byte value in different directions in the TFS opcode
  space; the packet-proxy tool table shows `0xA0 → PlayerStats` and `0xA7 → FightModes`
  separately). §2.8 and PC-4's heading say "`0xA0` → `GamePacket::FightModes`" — this is the wrong
  opcode number; fix to `0xA7` so nobody greps the wrong byte later.

### 0.3 `player_combat.rs::CombatResult::SecureMode` — confirmed reserved-but-dead, matches plan
Verified: the variant exists with `#[allow(dead_code)]` and a comment pointing at this exact plan.
No correction needed here — just confirming PC-4 step 3 has a real anchor to wire into.

### 0.4 Vocation gains anchors — plan's "150/0/400" constants are already wrong vs the shipped XML
§3 Phase PC-0 step 3 says to replace `recalculate_vitals` with `vocations.xml` gains and floats
"150 hp / 0 mana / 400 cap anchors ... verify". The shipped `data/XML/vocations.xml` (read this
pass) has explicit **per-vocation** `gaincap`/`gainhp`/`gainmana` attributes (e.g. vocation 0 "None"
already carries `gaincap="10" gainhp="5" gainmana="5"`), so there's no need to "confirm" a base
constant — the anchor values (base HP/mana/cap at level 1) still need a source since the XML only
gives *gain per level*, not the level-1 floor. Open question 1 (§8) already flags this but should
also note: **`creature/vocation.rs::recalculate_vitals` currently hardcodes `150`/`0`/`400`** as the
level-1 floor for *every* vocation — this needs to become data too (or at minimum a documented
772-wide constant with a citation), since nothing in `vocations.xml` supplies it.

### 0.5 Skill-tries counters have nowhere to live at runtime (new gap, not in original plan)
`crskill.cc` `Increase`/`ProbeValue` need per-skill **tries** counters (`LearningPoints`-adjacent but
distinct — tries accumulate exp within the current skill level). The DB layer already has full
round-trip support: `PlayerRecord` (`tfs-rust-db/src/player.rs`) has `skill_fist_tries` /
`skill_club_tries` / ... / `skill_fishing_tries` (loaded and saved). **But `login.rs` never copies
these into the runtime `Player`, and `PlayerSkills` (`creature/player.rs`) has no `_tries` fields at
all** — only `fist`/`club`/`sword`/`axe`/`dist`/`shielding`/`fishing`/`maglevel` levels, no exp/tries
counters. `combat::math::req_skill_tries` exists and is unit-tested but is never called from
anywhere in `crates/tfs-rust-core/src` outside its own test module. **Add to Phase PC-2 step 2**
(currently only mentions `learning_points`): `PlayerSkills` needs a `_tries: u64` (or `exp: u64`)
counter per skill, wired at login/save like the level fields, and `player_execute_attack`'s strike
path must call `req_skill_tries` + `Increase`-equivalent leveling, not just gate on
`learning_points`. Without this, "skill advances only while learning is active" (PC-2 step 2) has
no counter to advance.

### 0.6 `Player.attack_mode` field doesn't exist yet — PC-1 step 4 correctly calls this out, confirmed
Checked `creature/player.rs` fully: no `attack_mode`, `secure_mode`, or `learning_points` field
exists on `Player` or `CreatureBase` today. §5's "New/changed data model" table already lists these
as new — no correction, just confirming the gap is real and none of the three exist under a
different name (`chase_mode` does exist on `CreatureBase` and is unrelated to `attack_mode`).

### 0.7 Wand item attributes — confirmed fully absent, and 1098 `items.xml`/OTB has none either
Grepped `otb.rs` `ItemType` and `data/XML/items.xml` — no `wand_*` fields/attributes anywhere in the
Rust content layer or the shipped XML. The CipSoft `objects.cc` attribute table (`WANDRANGE`,
`WANDMANACONSUMPTION`, `WANDATTACKSTRENGTH`, `WANDATTACKVARIATION`, `WANDDAMAGETYPE`, `WANDMISSILE`)
confirms these are real 772 object attributes, but `reference/cipsoft-772/runtime/dat/objects.srv`
has no plaintext `WAND` strings to grep (binary/obfuscated format — needs the `.srv` parser, not a
raw grep). **Open question 3 (§8) undersells the size of this gap** — it's not just "which source";
today there is *zero* wand data path from any file into `ItemType`. Phase PC-3 wand work is
blocked on either (a) extending the `.otb`/`.xml` parser with new attributes sourced from
TVP-equivalent wand items.xml entries (if the TVP `items.xml` already carries attack/mana-cost for
wands under existing generic fields — worth checking before adding new ones), or (b) a small
`772_wands.lua` era-data table keyed by item id, mirroring the `772_spell_areas.lua` pattern already
proposed in §3.6.2. Recommend (a) first since `ItemType.attack`/`attack_speed` may already cover
"attack strength"/mana cost isn't modeled anywhere yet either way.

### 0.8 §3.6/§3.6.2 AoE plan is accurate but should note the Lua spell-scripting layer doesn't exist yet
Checked `tfs-rust-lua`: there is **no `Combat`, `Spell`, `createCombatArea`, or `Weapon` Lua
userdata/global registered anywhere** (`runtime.rs::register_event_script_bootstrap` only stubs
`Player`/`Creature`/`Monster`/`Npc`/`Game`/`Tile`/`Item`/`Container` as empty tables +
`Condition`/a few globals for `events/scripts/player.lua`). The scripts referenced throughout §3.6.1
(`ultimate_explosion.lua`) and §3.6.2 (`createCombatArea`) are **data files with no corresponding
Lua API implementation** — `spell.rs` (core) only has cooldown/gating logic
(`can_cast_instant`/`SpellDefinition`), not an XML/Lua spell loader, and nothing calls
`can_cast_instant` from `game_loop.rs` yet (no `Say`-triggered instant-spell dispatch exists — the
`0xA?` talk packet is parsed but spellword matching/`onCastSpell` invocation isn't wired). This
doesn't change §3.6's design (it's still the right target shape), but **PC-3/PC-3a should list
"stand up the `Combat`/`Spell`/`createCombatArea` Lua bindings + spellword dispatch" as a
prerequisite**, not something already in place that the new `area_offsets` seam merely hooks into.
Right now there is no seam to hook into because the caller doesn't exist. Player weapon-combat
(PC-2/PC-3, ammo/wand) does **not** depend on this — it's pure core/native combat — but the worked
example in §3.6.1/§3.6.2 (a Lua instant spell) implicitly assumes scripting plumbing that is still
Track-2/未 (deferred) work per `tfs-lua-boundaries.md`'s own port-plan ordering ("creaturescripts →
move/talk/global/actions → combat last").

### 0.9 Minor: `player_get_weapon_skill` already matches `GetAttackValue`'s skill-selection shape
Confirmed §2.1/PC-1 step 1 has a ready helper (`player_inventory_util.rs::player_get_weapon_skill`)
that already maps `WEAPON_SWORD/CLUB/AXE/DISTANCE` → the right `PlayerSkills` field and falls back to
fist. PC-1 can reuse this directly instead of re-deriving `SkillNr` selection — worth an explicit
note in PC-1 step 1 so the implementer doesn't duplicate it.

### 0.10 Vocations are the pilot migration in `docs/DATA_FORMAT_MIGRATION.md` — PC-0 should target that path, not extend the XML parser
`docs/DATA_FORMAT_MIGRATION.md` (read this pass, real doc, exists) proposes moving all static game
data off XML onto Lua-as-data (`data/*.lua` tables → `serde::Deserialize` into immutable Rust
structs, materialized once at startup, zero runtime Lua cost). **`vocations.xml` → `data/vocations.lua`
is explicitly called out as "Phase 1 — vocations (pilot)"** in that doc, and it names the *exact*
same gap this combat plan is fixing: `TSkillFed` regen (`process_skills.rs::fed_regen_cadence`) and
`recalculate_vitals`/`per_level_gains` (`creature/vocation.rs`) reading hardcoded stub tables instead
of the vocation definition (their "Finding 3"). **Phase PC-0 as originally written extends the
existing `quick-xml` parser in `tfs-rust-content/src/vocations.rs`** to pick up the `<formula>`/
`<skill>` children. That would work, but it directly contradicts the migration doc's own guidance —
we'd be teaching the XML loader more schema the week before that loader is slated for deletion, and
`vocations.xml` is the one file the migration doc says to move *first*. Rewritten PC-0 below targets
`data/vocations.lua` + a `VocationDef`/`VocationRegistry` instead of deepening the XML path. Two
prerequisites this pulls forward from `DATA_FORMAT_MIGRATION.md` §"Phase 0 — infrastructure" (not yet
done anywhere in the tree — confirmed no `sandboxed_data_lua`/`require_schema`/`VocationDef` symbols
exist today):
- Add mlua's `"serde"` feature to the workspace (`Cargo.toml:47` currently
  `features = ["luajit", "vendored"]`, no `"serde"`) so `LuaSerdeExt`/`lua.from_value` works.
  `serde` itself is already a dependency (currently only pulled into `tfs-rust-lua` for
  `quick-xml`'s `serialize` feature) — add it to `tfs-rust-content` too.
- A minimal `sandboxed_data_lua()` (fresh `Lua::new()` with `io`/`os`/`package`/`require`/`dofile`/
  `loadfile` stripped) + `require_schema(&table, expected)` helper, living in `tfs-rust-content`
  (the migration doc's suggested location) since that's the natural home for pure-data loaders,
  reusing the pattern already proven in `formulas.rs::load_mechanics` (which loads
  `data/formulas/772.lua`/`1098.lua` with a plain `Lua::new()` today — same shape, just needs the
  sandboxing + schema gate + `serde` deserialize added).
- This is genuinely new infra work not accounted for anywhere in the original PC-0 estimate — call
  it out as a sub-step rather than assuming `vocations.rs` only needs a bigger XML parser.

The other TVP-flavored era-data tables this combat plan proposes — 772 per-spell radius overrides
(§3.6.2, `data/formulas/772_spell_areas.lua`) and wand attributes (§0.7/§8 Q3, if resolved as a small
lookup table rather than new `ItemType` fields) — are small flat/keyed tables and fit the same
Lua-as-data pattern the migration doc describes ("Lua where computation/derivation/era-conditionals
help"). Recommend building the sandboxed loader once in Phase PC-0 and reusing it for those two
later phases instead of three bespoke loaders.

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
| Condition ticks + fed regen tick loop | `process_skills.rs` | Regen cadence **hardcoded**, not from vocation data — becomes `data/vocations.lua`-sourced in PC-0/PC-5 (§0.10; `DATA_FORMAT_MIGRATION.md` "Finding 3"). |
| Item weapon attributes (`weapon_type`, `attack`, `defense`, `extra_defense`, `armor`, `ammo_type`, `attack_speed`, `shoot_range`, `hit_chance`) | `tfs-rust-content/src/otb.rs` `ItemType` | Data source for `GetAttackValue`/`GetDefendValue`/`GetArmorStrength`. |
| Fight-mode enum + modifiers | `combat/math.rs` `FightMode`, `formulas.rs` `FightModes` | 772 `+20/−40` atk, `−40/+80` def already tuned. |

### 1.2 The gap — player strike is a stub
`player_combat.rs::player_execute_attack` (moves to `player/combat/mod.rs` in PC-2 — §4.0) →
`PlayerChaseOutcome::Adjacent` currently only
`DelayAttack(200)` + re-arms. The doc comment states plainly: *"The melee strike … is deferred — no
player weapon-combat system exists yet."* This plan fills that hole.

### 1.3 The gap — vocation combat data is dropped on load
`tfs-rust-content/src/vocations.rs` parses only `id`/`clientid`/`name`/`description`/`fromvoc`. The
active `data/XML/vocations.xml` already carries the full TVP block (`gaincap`, `gainhp`, `gainmana`,
`gainhpticks`, `gainhpamount`, `gainmanaticks`, `gainmanaamount`, `manamultiplier`, `attackspeed`,
`basespeed`, `soulmax`, `gainsoulticks`, `<formula meleeDamage/distDamage/defense/armor>`,
`<skill id multiplier>`). All of it is discarded today. **Migrating this file to
`data/vocations.lua` is Phase PC-0 below, per `DATA_FORMAT_MIGRATION.md` (§0.10) — the XML file is
the interim source only until PC-0 lands, not the long-term target.**

`creature/vocation.rs` is stubbed as a result: `vocation_base_speed()` returns a hardcoded `220`,
`per_level_gains()` uses "example" numbers, `recalculate_vitals()` uses `150 + hp_gain*(l-1)`
constants. These must read the loaded vocation registry (`VocationRegistry` post-PC-0).

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
Maps to `combat/math.rs::req_skill_tries` — **fed by `data/vocations.lua` `skill_multipliers`**
(post-PC-0; today only in `vocations.xml` `<skill multiplier>`, discarded) (→ `FactorPercent`, i.e.
`1000 * multiplier` shape) rather than TFS `skillBase`.

### 2.8 Fight/chase/secure mode packet — `receiving.cc` `0xA7`, `crcombat.cc:333/354` `SetAttackMode`/`SetChaseMode`
`SetAttackMode` mode change → `DelayAttack(2000)`. `SetChaseMode` only NONE/CLOSE for players.
SecureMode stored on `TCombat`. **Parse + chase-mode storage already exist** (`game_parse.rs`
`C::FIGHT_MODES` → `GamePacket::FightModes`; `game_loop.rs` sets `chase_mode`) — what's missing is
storing `raw_fight_mode`/`raw_secure_mode` into new `Player` fields (`CombatResult::SecureMode` is
reserved in `player_combat.rs` for this). See §0.2.

### 2.9 Level / vitals — `data/vocations.lua` gains + `crskill.cc:352` `GetExpForLevel`
Level exp `(((L-6)*L+17)*L-12)/6 * Delta` already in `combat/math.rs::experience_for_level`
(and `creature/vocation.rs::total_experience_for_level`, which uses an **equivalent expanded
polynomial** — consolidate onto one). Vitals per level from `data/vocations.lua` `gain_hp`/
`gain_mana`/`gain_cap` (post-PC-0), base speed from `base_speed` + level rule (772: `+level`; 1098:
`+2*(level-1)`).

---

## 3. Work breakdown

### Phase PM — Player module consolidation (pure moves, very low risk — do first)
**Goal:** stand up the `player/` directory (§4.0) so the combat phases add files into a real module
instead of scattering more `player_*.rs` at the crate root. This is the same behavior-preserving,
cut/paste move as `REFACTOR_AUDIT.md` Phase 4 (`monster_ai.rs` → `monster_ai/`) and discharges the
audit's §6 `player/inventory/` recommendation.

**Files moved (no logic edits):**
- `player_inventory_query_add.rs` → `player/inventory/query_add.rs`
- `player_inventory_load.rs` → `player/inventory/load.rs`
- `player_inventory_notifications.rs` → `player/inventory/notifications.rs`
- `player_inventory_util.rs` → `player/inventory/util.rs`
- `game_world_player.rs` → `player/stats.rs`
- `player_flags.rs` → `player/flags.rs`
- `player_depot.rs` → `player/depot.rs`
- `player_ping.rs` → `player/ping.rs`
- `player_combat.rs` → `player/combat/mod.rs` (may defer to PC-2 step 0 if PC-2 lands first)

**Process (one file per commit, compile between each):**
1. Create `player/mod.rs` with `pub mod` declarations for each sub-item + `inventory/mod.rs` and
   `combat/mod.rs` surfaces. Replace the flat `mod player_*;` / `mod game_world_player;` lines in
   `lib.rs` with a single `mod player;`.
2. Move one file at a time; keep visibility unchanged. Add `pub use` compatibility re-exports in
   `player/mod.rs` (e.g. `pub use flags::*;`) so existing `crate::player_flags::…`,
   `crate::player_inventory_util::…`, `crate::game_world_player::…` call sites resolve unchanged —
   repoint them to `crate::player::…` opportunistically, not in this phase.
3. Preserve each file's `//!` C++ reference header verbatim.
4. Do **not** split `player_inventory_query_add.rs` (1184 LOC) here — that decomposition is Phase 5
   / audit §5 work; PM only relocates it into `player/inventory/`.

**Leave in place** (audited — see §4.0 table): `player_lua_context.rs`,
`game_world_player_rotate.rs`, `game_world_player_throw.rs`, `walk_action.rs`, `floor_change_use.rs`,
`creature/player.rs`, `creature/vocation.rs`, `login*.rs`, `process_skills.rs`.

**Exit criteria:** `crate::player::…` paths compile; `lib.rs` has one `mod player;` for the moved
set; `rtk cargo check && rtk cargo clippy --all-targets && rtk cargo test` — identical test count
and no new clippy warning vs. the pre-PM baseline (pure move, per `REFACTOR_AUDIT.md` verify gate).

### Phase PC-0 — Vocation combat data (Lua-as-data, per `DATA_FORMAT_MIGRATION.md` Phase 0 + Phase 1)
**Files:** `tfs-rust-content/src/vocations.rs` (rewritten, not extended), new
`data/vocations.lua` (replaces `data/XML/vocations.xml` as the source of truth),
`crates/tfs-rust-core/src/creature/vocation.rs`, `config.rs`/wiring where `VocationDatabase` is
threaded. See §0.10 — this phase is also `DATA_FORMAT_MIGRATION.md`'s named "Phase 1 — vocations
(pilot)"; do it once, here, rather than deepening the XML loader first and migrating later.

0. **Infra (shared with §3.6.2 / wand data, do once):** add mlua `"serde"` feature to the workspace
   `Cargo.toml`; add `serde` (with `derive`) to `tfs-rust-content`; add a small
   `sandboxed_data_lua()` (fresh `Lua::new()`, `io`/`os`/`package`/`require`/`dofile`/`loadfile`
   stripped) + `require_schema(&table, expected_version)` helper in `tfs-rust-content` (new
   `data_lua.rs` or similar). Reuses the `Lua::new()` + table-load pattern already proven in
   `formulas.rs::load_mechanics`, just sandboxed and schema-gated.
1. Author `data/vocations.lua` — one file, `schema = 1`, a `vocations` array — carrying the full
   TVP block already present in `data/XML/vocations.xml` today: `gain_cap`, `gain_hp`, `gain_mana`,
   `gain_hp_ticks`, `gain_hp_amount`, `gain_mana_ticks`, `gain_mana_amount`, `mana_multiplier`,
   `attack_speed_ms`, `base_speed`, `soul_max`, `gain_soul_ticks`, `allow_pvp`,
   `formula: { melee_damage, dist_damage, defense, armor }`, `skill_multipliers: [f64; 7]`
   (indices = `SKILL_FIST..SKILL_FISHING`; client skill ids 0..6 in the XML map 1:1). Also add the
   **level-1 vitals floor per vocation** here (base HP/mana/cap) — §0.4 confirmed this value exists
   nowhere today (`recalculate_vitals` hardcodes `150`/`0`/`400` for every vocation); putting it in
   `vocations.lua` alongside the per-level gains closes that gap instead of leaving a second
   hardcoded constant behind.
2. Define `VocationDef` (`#[derive(serde::Deserialize)]`) in `tfs-rust-content` mirroring the Lua
   shape 1:1, and a `VocationRegistry` (`HashMap<u16, VocationDef>` or `Arc<[VocationDef]>`) with a
   `load(path) -> Result<Self>` that: loads via `sandboxed_data_lua()`, checks `schema`,
   `lua.from_value` into `Vec<VocationDef>`, runs a validation pass (unique ids, non-zero tick
   divisors, `skill_multipliers` len 7), and indexes by id.
3. Retire the `quick-xml`-based `Vocation`/`VocationDatabase::load` in `vocations.rs` — replace with
   the above (keep `client_id_u8` and any other consumer-facing methods on the new
   `VocationRegistry`, same call sites).
4. Replace stubs in `creature/vocation.rs`:
   - `vocation_base_speed` → `VocationRegistry.get(id).base_speed`.
   - `per_level_gains` / `recalculate_vitals` → `gain_hp/gain_mana/gain_cap` off the registry, using
     the new per-vocation level-1 floor from step 1 (no more shared `150 + gain*(l-1)` constant).
   - Consolidate `total_experience_for_level` onto `combat::experience_for_level` (single source).
5. Thread `&VocationRegistry` (or a `Copy` per-vocation snapshot cached on `Player` at login) into
   `GameWorld` so combat/regen/level-up read it without a content dependency in hot paths.
6. **Migration-doc bookkeeping:** ship a tiny `xml → lua` one-shot converter for this file (per
   `DATA_FORMAT_MIGRATION.md` "Keep a converter") since `vocations.xml` is small and this is the
   pilot; a temporary dual-load golden test (old XML loader vs new Lua loader, assert identical
   `VocationDef`s) satisfies that doc's "golden equivalence" verification step before the XML loader
   is deleted.

**Test:** golden parse of `data/vocations.lua` (knight skill[4]=1.4, sorcerer gainmana=30,
attackspeed=2000, basespeed=70) — plus the temporary dual-load-vs-XML equivalence test from step 6;
base-speed + vitals per level vs known 772 values.

### Phase PC-1 — Player attack/defend/armor value resolution
**File:** new `crates/tfs-rust-core/src/player/combat/values.rs` (core; see §4.0 for the `player/`
module-directory layout — this replaces the crate-root `player_combat_values.rs` originally planned).

1. `player_get_attack_value(cid) -> (max_value, SkillNr)` — `GetAttackValue` from equipped
   weapon/ammo/throw/wand/fist via existing `player_get_weapon*` + `ItemType.attack`.
2. `player_get_defend_value(cid) -> (max_value, SkillNr)` — `GetDefendValue` (shield/weapon defend).
3. `player_get_armor_strength(cid) -> i32` — `GetArmorStrength` sum over `equipment_slots` armor
   pieces (uses `ItemType.armor` + body-position check via `slot_type_for_item_type`).
4. Fight-mode source: `Player.attack_mode` (new field; default `Balanced`).
   All cite `crcombat.cc` GetAttackValue/GetDefendValue/GetArmorStrength.

### Phase PC-2 — The strike (`CloseAttack`) — melee first
**Files:** `player/combat/mod.rs` (the moved `player_combat.rs` — replace the `Adjacent` stub; strike
body extracted into `player/combat/strike.rs` per §4.0), reuse
`combat::math::{weapon_damage, defense_value, armor_reduction}` +
`combat_execute_with_stimulus`.

**Step 0 (pre-req):** ensure the `player_combat.rs → player/combat/mod.rs` move (Phase PM / §4.0) has
landed and is green. If PC-2 runs before Phase PM completes, do just that one file move here as an
isolated pure-move commit first, then land the strike logic — keep the move separate from the logic
change.

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
**File:** `player/combat/ranged.rs` (new), reusing `player/combat/values.rs` (§4.0).

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
**Files:** `player/combat/fight_mode.rs` (new — setters + PVP gating; §4.0), `game_loop.rs`
(`FightModes` arm — parse already exists, see §0.2).

1. ~~Parse `0xA7` → `{ attack_mode, chase_mode, secure_mode }`~~ — **already done**
   (`GamePacket::FightModes { raw_fight_mode, raw_chase_mode, raw_secure_mode, raw_pvp_mode }`).
   No net-side work needed.
2. Core setter: extend the existing `FightModes` arm in `game_loop.rs` — currently only sets
   `chase_mode` and discards `raw_fight_mode`/`raw_secure_mode` via `..`. Add `SetAttackMode`
   (change → `DelayAttack(2000)`) writing `Player.attack_mode: FightMode` (new field, via
   `FightMode::from_wire`, already implemented in `combat/math.rs`), and `SecureMode` writing
   `Player.secure_mode: bool` (new field). Enforce the reserved `CombatResult::SecureMode` +
   `AttackNotAllowed` (rights) branches in `validate_player_attack_target`.
3. PVP: `can_player_attack_player` / `is_protected` already in `combat/pvp.rs` — wire
   `IsAttackJustified`, `RecordAttack`, `BlockLogout(60)`, skull/frag outcomes (scope-check against
   772 `cract.cc`/`crcombat.cc`; skulls may be a follow-up sub-phase).

### Phase PC-5 — Skill/exp gain + regen from vocation data
**Files:** `process_skills.rs`, kill/death path (`death.rs`/`idle_stimulus.rs`).

1. Replace `process_skills.rs::fed_regen_cadence` hardcoded table with `data/vocations.lua`
   (post-PC-0) `gain_hp_ticks/gain_hp_amount/gain_mana_ticks/gain_mana_amount` — this is the exact
   bug `DATA_FORMAT_MIGRATION.md` calls "Finding 3" and names as the driver for the vocation
   migration; PC-0 and this step close it together.
2. On kill: `distribute_experience` + `pvp_exp_cap` (already present) → `add_experience`; skill
   `Increase` already handled inline via learning during strikes.
3. Skill tries curve: feed `data/vocations.lua` `skill_multipliers` into `req_skill_tries`
   (`FactorPercent = 1000 * multiplier` shape per `crskill.cc:497`), not TFS `skillBase`. Per §0.5,
   `req_skill_tries` exists and is tested but has no caller and `PlayerSkills` has no tries counters
   yet — this step also needs the `_tries` fields added, not just the data source swapped.

---

## 3.6 AoE shape model — baked `circles.rs` (772) vs `MatrixArea` (1098)

**Decision: 772 area-of-effect uses the CipSoft circle-ring model, matching the decompile — baked
into a Rust const table (`circles.rs`), not loaded from `circles.dat` at runtime.** The ring table
is static (never changes at runtime), so a startup file parse buys nothing; we replicate what
`InitCircles` *produces* (outcome parity per `tfs-core.md`), not the file format. Required by PC-3
(burst arrow) and shared by all radius spells.

**Reference** (`reference/cipsoft-772/tibia-game-master/src/magic.cc`):
- `InitCircles` (`:4344`) reads `circles.dat`: header `W H Center` (`21 21 10`), then `W*H` ring
  indices. Each tile `(X−Center, Y−Center)` with ring value `R < 10` is appended to `Circle[R]`
  (offset list, ≤32 pts), scanned **row-major (Y outer, X inner)**. `99` = excluded.
- `ExecuteCircleSpell` (`:459`, inlined at `:939/:1161/:2053/:2151/:2385/:2641`): `for R in
  0..=Radius { for each (dx,dy) in Circle[R] { apply impact at center+(dx,dy) } }`. A "radius N"
  spell = filled disc of rings `0..=N`, applied ring-0-outward in the file's scan order.
- Burst arrow: `crcombat.cc:840` `CircleShapeSpell(..., Radius=2, EFFECT_FIRE_BURST)`.

**Current Rust state (mismatch to fix):**
- Player spells (`spell.rs`) use TFS `MatrixArea` (`matrix_area.rs`) — the **1098** per-spell tile
  matrix, not the 772 disc.
- Monster spells (`monster_combat.rs`) parse `radius` + `SpellShape::Origin/Angle` but never expand
  to tiles (`parse_area_effect_name` → `None`; no `ExecuteCircleSpell` equivalent).
- No circles model exists in Rust yet (only `reference/.../runtime/dat/circles.dat`).

**Plan (new sub-phase PC-3a — AoE shape provider):**
1. Add `crates/tfs-rust-core/src/combat/circles.rs` — a **baked const** ring table
   (`CIRCLE_RINGS: &[&[(i8,i8)]]`, index = ring 0..=7) plus
   `fn disc_offsets(radius) -> impl Iterator<Item=(i32,i32)>` returning rings `0..=radius` in
   CipSoft scan order. **No runtime file load.**
   - **Generate, don't hand-type:** a one-shot generator (`build.rs` or an ignored bin) reads
     `reference/.../circles.dat`, applies the same `X−Center`/`Y−Center` + row-major scan as
     `InitCircles`, and emits the const. Ship a `#[test]` that re-derives from the reference file
     (when present) and asserts equality with `CIRCLE_RINGS` — guarantees parity + preserves ring
     insertion order, without shipping the file at runtime. Header cites `magic.cc` `InitCircles`
     (`:4344`) / `ExecuteCircleSpell` (`:459`).
2. Add an `AreaShapeModel` field to `MechanicsProfile` (`Circles772` | `Matrix1098`) selected by era
   at startup — **no `if version` at call sites**. `data/formulas/772.lua` defaults to circles.
3. Introduce a neutral `fn area_offsets(center, radius) -> impl Iterator<Item=(dx,dy)>` seam that
   dispatches on the profile: 772 → `circles::disc_offsets(radius)`; 1098 → existing `MatrixArea`
   offsets (`spell::matrix_tile_offsets`).
4. Route the PC-3 burst-arrow effect and monster `Origin`/`Angle` area impacts through this seam
   (reuse `combat_execute_with_stimulus` per tile; `ThrowPossible`/LoS + PZ filtering as in
   `ExecuteCircleSpell`).
5. Preserve application **order** (ring 0 outward) — some effects/animations depend on it; the
   baked `CIRCLE_RINGS` const already encodes the `Circle[R]` insertion order from the file scan.

**Cite** `magic.cc` `InitCircles`/`ExecuteCircleSpell` + `crcombat.cc:840` in the new module header.
Open question 6 (§8): confirm whether 1098 area spells should also migrate to circles, or stay on
`MatrixArea` (default: keep `MatrixArea` for 1098 — TFS-native shapes differ from the CipSoft disc).

### 3.6.1 Worked example — `ultimate_explosion.lua`

The Lua script (`data/scripts/spells/attack/ultimate_explosion.lua`) is the **shared, TFS-shaped
spell definition** for both eras:
```
COMBAT_PARAM_TYPE   = COMBAT_PHYSICALDAMAGE      COMBAT_PARAM_BLOCKARMOR  = true
COMBAT_PARAM_EFFECT = CONST_ME_EXPLOSIONAREA     COMBAT_PARAM_BLOCKSHIELD = false
combat:setArea(createCombatArea(AREA_CIRCLE5X5))
onGetFormulaValues -> player:computeDamage(250, 50)
mana 1200, level 60, vocation Sorcerer/Master Sorcerer
```

**Decompile equivalent — `magic.cc:3484` case 24 (`ex evo gran mas vis`):**
```
Damage = ComputeDamage(Actor, 24, 250, 50);
MassCombat(Actor, Actor->CrObject, ManaPoints, SoulPoints, Damage,
           EFFECT_EXPLOSION, /*Radius=*/6, DAMAGE_PHYSICAL, ANIMATION_FIRE);
```
Every scalar in the Lua matches CipSoft exactly: base `250`, variation `50`, physical, explosion
effect, block-armor (physical mitigates through `GetArmorStrength`), no shield block. `MassCombat`
→ `CircleShapeSpell` → `ExecuteCircleSpell` expands **radius 6 via `circles.dat` rings 0..=6**
(`magic.cc:809`, `:512`, `:459`).

**Damage:** `computeDamage(250,50)` = `ComputeDamage` (`magic.cc:776`) — already
`combat/math.rs::spell_damage`: `dmg = 250 + random(-50,50)`; then for players
`dmg = dmg * (2*level + 3*magicLevel) / 100` with the `&4`/`&8` flag clamps.

**Area — the one real divergence:** `AREA_CIRCLE5X5` is a TFS **radius-5** matrix (11×11 disc,
`areas.lua:163`); CipSoft UE is **radius 6** and its disc differs tile-for-tile from the matrix
(e.g. circles.dat excludes the `(0,±5)` spike the matrix includes). So on 772 we do **not** apply
the matrix — the `area_offsets` seam (§3.6) resolves the spell's era-correct radius through
`circles.dat`. The intended radius is 772 **era data**, not derived from the matrix (naive extent
derivation gives 5, not the CipSoft 6). Options, in preference order:
1. Per-spell 772 radius override (spell keyed → radius) loaded as era data — exact parity (radius 6).
2. 772 `areas.lua` variant mapping `AREA_CIRCLE5X5` → circles radius (shared across all UE-shape spells).
3. Derive radius from matrix extent (radius 5) — close but **not** CipSoft-exact; last resort.

**End-to-end flow on 772:**
1. `spell.onCastSpell` → `combat:execute(creature, variant)` → core spell-cast entry.
2. Cast gate: mana `1200` / level `60` / vocation check (Sorcerer/Master Sorcerer) — TFS spell
   metadata, shared.
3. `onGetFormulaValues` → `spell_damage(level, magicLevel, base=250, var=50)` (one roll, per
   `ComputeDamage`; the same value is applied to every tile, matching `TDamageImpact`).
4. Center = caster tile (self-cast UE, `Actor->CrObject`); `ThrowPossible` LoS check.
5. `area_offsets(center, radius=6)` → circles rings `0..=6` (ring 0 outward order).
6. Per tile: PZ/LoS filter → `combat_execute_with_stimulus(Some(caster), victim, PHYSICAL,
   -damage)` with block-armor on (armor mitigates), block-shield off (no defense roll); explosion
   effect emitted per tile.
7. Post-cast attack/spell delay per `CheckMana` (`2000`ms, `1000` under PVP-enforced).

On 1098 the identical script runs with `area_offsets` returning the `AREA_CIRCLE5X5` `MatrixArea`
offsets and TFS damage tuning — same call sites, era selected by `MechanicsProfile.area_shape`.

### 3.6.2 Keeping TFS Lua scripts unchanged while sourcing area from `circles.rs`

**Yes — the TFS-style scripts stay byte-for-byte unchanged; only the `createCombatArea` builtin
becomes era-aware.** The scripts never see the circle table; they keep calling
`createCombatArea(AREA_CIRCLE5X5)`.

**Mechanism:**
- `createCombatArea(matrix)` returns an **opaque `CombatArea` handle** (userdata), not a raw
  `MatrixArea`. Internally:
  - **1098** → wraps a `MatrixArea` built from the Lua matrix (current behavior).
  - **772** → wraps `Circles { radius }`, expanded via the baked `circles::disc_offsets(radius)`.
- `combat:setArea(handle)` stores the handle on the `Combat` object. At cast time, tile expansion
  goes through the §3.6 `area_offsets(center, radius)` seam, which reads the handle's era variant.
- No `data/scripts/**` edits. `areas.lua` (`AREA_CIRCLE5X5`, …) stays as-is; on 772 the matrix is
  only used to **derive a default radius** (max ring extent), never to place tiles.

**The catch — shared matrix, per-spell CipSoft radius (must resolve, cannot auto-derive):**

| TFS script | TFS area | CipSoft (`magic.cc`) | CipSoft radius |
|------------|----------|----------------------|----------------|
| `ultimate_explosion.lua` | `AREA_CIRCLE5X5` | case 24 `MassCombat(…,6,…)` | **6** |
| `poison_storm.lua` | `AREA_CIRCLE5X5` | case 56 `MassCombat(…,8,…)` | **8** |

Both scripts pass the *same* matrix, but CipSoft uses radius 6 vs 8. So a matrix→radius derivation
alone (would give ~5 for both) **cannot** reproduce CipSoft-exact discs. The radius is per-spell 772
data. Resolution order for the 772 `CombatArea.radius`:
1. **Per-spell 772 radius override** — small era-data table keyed by spell `name`/`words`
   (`ultimate explosion → 6`, `poison storm → 8`). Loaded at startup; **scripts untouched**.
2. Else **matrix-derived radius** (max ring extent from the passed matrix) — a consistent
   circles-shaped disc, playable, but not CipSoft-exact for spells whose CipSoft radius ≠ matrix
   extent.

**Recommendation:** default to (2) so every TFS area script immediately renders as a `circles.dat`
disc with zero config; layer (1) as an optional `data/formulas/772_spell_areas.lua` (or a field in
the ported spell metadata) for the handful of spells where exact CipSoft radius matters. This keeps
100% of TFS Lua scripts unchanged while the actual tile set comes from `circles.dat`.

## 4. Architecture / placement rules (steering compliance)

### 4.0 Module placement — the `player/` directory (aligns with `REFACTOR_AUDIT.md` Phase 4 + §6)

This plan lands its new/rewritten combat code inside a **`player/` module directory**, mirroring the
Phase 4 move that carves `idle_stimulus.rs` → `monster_idle/` and `monster_ai.rs` → `monster_ai/`
(`REFACTOR_AUDIT.md` §1/§4). It also seeds the §6 "module fragmentation" recommendation, which names
`player/inventory/` as the target for the flat `player_inventory_*` / `player_*` cluster. Rather than
adding two more crate-root `player_*.rs` files, this plan owns the full consolidation (the §6
"module fragmentation" recommendation), pulling the whole player subsystem into one directory:

```
crates/tfs-rust-core/src/player/
  mod.rs                # module surface doc + re-exports (keeps call-site paths stable)
  combat/
    mod.rs              # player_execute_attack strike dispatch (was crate-root player_combat.rs)
    values.rs           # player_get_attack_value / _defend_value / _armor_strength (PC-1)
    strike.rs           # CloseAttack melee strike body (PC-2) — extracted so mod.rs stays a dispatch surface
    ranged.rs           # DistanceAttack + WandAttack (PC-3)
    fight_mode.rs       # attack/chase/secure-mode setters + PVP gating (PC-4)
  inventory/
    mod.rs              # inventory surface (§6 explicitly names `player/inventory/`)
    query_add.rs        # was player_inventory_query_add.rs (1184 LOC — audit §1 second-tier)
    load.rs             # was player_inventory_load.rs
    notifications.rs    # was player_inventory_notifications.rs
    util.rs             # was player_inventory_util.rs  (player_get_weapon* — reused by PC-1, §0.9)
  stats.rs              # was game_world_player.rs (sendStats / capacity / group-flag helpers)
  flags.rs              # was player_flags.rs (PlayerFlag bits)
  depot.rs              # was player_depot.rs (depot chest / inbox / locker)
  ping.rs               # was player_ping.rs (keepalive)
```

Rules for the moves (behavior-preserving, same as Phase 4's split process):
- Each move is a **pure file relocation** — no logic edits. Keep every item's visibility
  (`pub(crate)`) unchanged. Convert the flat `mod player_*;` lines in `lib.rs` to a single
  `mod player;`, and have `player/mod.rs` `pub use` the sub-items so existing
  `crate::player_flags::…` / `crate::player_inventory_util::…` call sites keep resolving via a
  compatibility re-export until they're repointed to `crate::player::…`. (`send_player_stats` and
  friends already cross-reference `crate::player_flags::` — moving both keeps them together.)
- **New PC-1/PC-3/PC-4 code goes straight into `player/combat/`** — do **not** create crate-root
  `player_combat_values.rs`; it becomes `player/combat/values.rs`.
- Do the consolidation as **Phase PM** (§3, before the combat phases add `player/combat/`) so the
  directory exists first; PC-2 step 0 then only moves `player_combat.rs` into it.
- Each new file carries its own C++ reference header (`crcombat.cc`/`crskill.cc` + TFS structure
  cite) per `TFS-cpp-references`.

**Audited — files that look player-ish but stay put (with rationale):**

| File | Verdict | Why |
|------|---------|-----|
| `player_lua_context.rs` | **Leave** (misnamed) | Generic Lua/item/container **read** helpers (`script_item_parent`, `script_container_*`, `resolve_item_u64`) — not player state. Belongs with script-context infra, not `player/`. Rename/rehome is a separate `script/` task. |
| `game_world_player_rotate.rs` | **Leave** | Despite the name it's the `TDTurn` executor that rotates an **item** (`cract.cc TCreature::Turn(Object)`), an action executor — not player state. |
| `game_world_player_throw.rs` | **Leave** | `playerMoveThing`/`playerMoveItem` item-move path — belongs with `game_world_item_move.rs`, not the player module. |
| `walk_action.rs` | **Leave** | Deferred walk-action pairs with the `walk/` subsystem (`Player::walkTask` + `on_walk`); moving it would split the walk state machine. Optional future `walk/` merge, not here. |
| `floor_change_use.rs` | **Leave** | Item-use action executor (up/down-floor teleport items), not player state. |
| `creature/player.rs`, `creature/vocation.rs` | **Leave** | Entity **data model** — the entity-storage rule keeps creature types in `creature/`. This plan edits their contents (PC-0/PC-2/PC-5 fields) but not their location. |
| `login.rs`, `login_out.rs` | **Leave** | Distinct login/logout subsystem (812 LOC in `login_out`); its own `login/` dir is separate backlog. |
| `process_skills.rs` | **Leave** | Timer-skill tick for **all** creatures (`ProcessSkills`); PC-5 edits its contents, not its home. |

- **Formulas & flow** live in `tfs-rust-core` combat modules; **no** `NetworkMessage`/opcode bytes
  in core (`0xA0` parse stays in `tfs-rust-net`). (`tfs-packets.md`, `tfs-wire-codec.md`.)
- **Era knobs** (fight-mode %, armor mode, defense gate, probe tuning, condition ticks) stay in
  `MechanicsProfile` / `data/formulas/772.lua`. **Per-vocation balance** stays in
  `data/vocations.lua` (post-PC-0; `DATA_FORMAT_MIGRATION.md`-aligned, supersedes `vocations.xml`).
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
| `skill_*_tries: u64` per skill (§0.5 — new, not in original plan) | `PlayerSkills` | DB `skill_*_tries` columns (already round-tripped, never loaded into runtime `Player`) |
| `attack_mode: FightMode`, `chase_mode` (exists), `secure_mode: bool` | `Player`/`CreatureBase` | `0xA7` packet (fixed from `0xA0`, see §0.2) |
| `VocationDef` (full combat block, incl. level-1 vitals floor) | `tfs-rust-content` (new type) | `data/vocations.lua` (Phase PC-0, §0.10 — supersedes `vocations.xml`) |
| Cached per-vocation snapshot | `Player` at login (or `GameWorld` map) | `VocationRegistry` |
| Wand attributes (`wand_damage_type`, `wand_attack_strength`, `wand_variation`, `wand_range`, `wand_mana`) | `content::ItemType` fields **or** a `data/772_wands.lua` era-data table (§0.7/§0.10 — same Lua-as-data pattern as vocations; no source currently wired either way) | `items.xml`/`objects.srv` (verify — see §0.7, gap is bigger than "which source") |
| `CIRCLE_RINGS` baked const + `disc_offsets` | `combat/circles.rs` | generated from `circles.dat` (`InitCircles`) |
| `area_shape: AreaShapeModel` (`Circles772`\|`Matrix1098`) | `MechanicsProfile` | era / `772.lua` |
| 772 per-spell radius override (opt) | `data/formulas/772_spell_areas.lua` (or spell metadata) | CipSoft `magic.cc` cases |

`earliest_attack_ms`/`earliest_defend_ms`/`last_defend_ms` already exist on `CreatureBase`.

---

## 6. Test plan
- **Formula goldens** (extend `combat/math.rs` tests): melee `max(0,atk−def)` then randomized armor;
  distance hit-probe bounds; wand fixed±variation; skill-tries curve from vocation multipliers.
- **Vocation parse golden**: `data/vocations.lua` full block (plus the temporary dual-load
  equivalence test against the outgoing `vocations.xml` loader, per PC-0 step 6).
- **Circles parity** (`combat/circles.rs`): re-derive rings from `reference/.../circles.dat`
  (`InitCircles` scan) and assert equality with `CIRCLE_RINGS`; spot-check `disc_offsets(6)` (UE)
  and `disc_offsets(8)` (poison storm) tile sets + ring-0-outward order.
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
   `recalculate_vitals` shortcut). **Confirmed gap (§0.4):** today `recalculate_vitals` hardcodes
   `150`/`0`/`400` as the level-1 floor for every vocation regardless of `vocations.xml`'s per-voc
   `gaincap`/`gainhp`/`gainmana` — those attributes are gain-per-level only, so the level-1 floor
   still needs its own source/citation even after PC-0 wires the per-level gains.
2. **Skill-tries mapping** — `vocations.xml` `multiplier` (e.g. 1.1) vs `crskill.cc` `FactorPercent`
   (1100) and `Delta`: confirm `FactorPercent = 1000*multiplier` and the `Delta` source per skill.
   **Confirmed gap (§0.5):** beyond the formula mapping, there is currently no runtime storage for
   per-skill tries at all — `PlayerSkills` has level fields only, `login.rs` never loads the DB's
   `skill_*_tries` columns, and `combat::math::req_skill_tries` (despite being implemented + tested)
   is never called outside its own test module. This needs its own field + wiring, not just a
   formula confirmation.
3. **Wand data source** — whether wand damage/type/mana come from `items.xml`, `objects.srv`, or a
   hardcoded table in the decompile (`operate.cc`/`magic.cc`); cite before adding `ItemType` fields.
   **Confirmed gap (§0.7):** verified there is currently *no* wand attribute anywhere in `ItemType`
   or the shipped `items.xml` — this isn't a "which source is authoritative" question so much as
   "no source is wired at all yet." `objects.srv` is a binary/obfuscated format, not greppable
   plaintext, so confirming CipSoft's wand values requires the `.srv` parser output, not a raw grep.
4. **Ranged defense "bug"** (`crcombat.cc:766`) — replicate the outcome (no defense on ranged, but
   still rolls target defense/wearout if attacker holds a shield). Confirm intended behavior.
5. **Skulls / PVP frags** — scope: include in PC-4 or defer to a dedicated PVP phase. Note PC-4's
   packet-parsing prerequisite is already satisfied (§0.2) — this phase is now pure core-side work
   (new `Player` fields + `game_loop.rs` `FightModes` arm + `combat/pvp.rs` wiring).
6. **AoE model for 1098** — confirm 1098 keeps TFS `MatrixArea` (default) vs migrating to circles;
   772 is settled on the `circles.dat` disc (§3.6).
7. **Lua spell-scripting plumbing** (new, §0.8) — `Combat`/`Spell`/`createCombatArea`/`Weapon`
   userdata and spellword dispatch (`Say` → `onCastSpell`) don't exist in `tfs-rust-lua` yet. §3.6's
   worked example (`ultimate_explosion.lua`) assumes this plumbing when illustrating the
   `area_offsets` seam. Player weapon-combat (PC-0..PC-5) doesn't depend on it, but PC-3a's Lua-facing
   design (§3.6.2) needs this listed as a prerequisite, not assumed-present.
