# Monsters — XML → Lua-as-data

**Scope:** replace `data/monster/monsters.xml` + `data/monster/monsters/*.xml` (157 types) with per-file Lua defs. Loader materializes the existing `MonsterType` Rust structs. Combat / AI / spawn stay unchanged.
**Date:** 2026-08-22. **Updated:** 2026-08-23 (one monster corpus; era gates on combat types only). **Companions:** [DATA_FORMAT_MIGRATION.md](../docs/DATA_FORMAT_MIGRATION.md) Phase 3, vocations pilot (`data/defs/vocations.lua` + `crates/tfs-rust-content/src/data_lua.rs`). Lessons 364–365.

**Status:** converter + 157 Lua files + full immunity round-trip **done**. Production still loads XML. Loader switch + XML delete are remaining.

Canonical file: `data/monster/dragon.lua` (generated from `monsters/dragon.xml`). Do **not** restore the old TFS revscript draft (speed 172, corpse 5973, chance/interval combat).

---

## Three layers

| Layer | Source of truth | This work |
|---|---|---|
| **Outcomes** | 772 race data (`crmain.cc` load, `crnonpl.cc` think/cast) | Keep XML numbers: GoStrength-style `speed`, `delay` on spells, melee `skill`/`attack`, strategy/lose-target, summons `delay`/`max` |
| **Domain** | TFS-style `MonsterType` + spawn name lookup so OTBM spawns / `Game.createMonster` keep working | Same `MonsterType` / `MonsterSpellNode` / `LootBlock` / `SummonBlock`. Lookup key = def `name` (lesson 19) |
| **Implementation** | Lua-as-data + serde, idiomatic Rust | **Do not** port TFS `Game.createMonsterType` / `MonsterType:register` / `MonsterSpell()` userdata. Data files `return { … }`. Game thread never evals them |

Conflict: when TFS 1.4.2 Lua (chance%, `interval` ms, `COMBAT_FIREDAMAGE`) disagrees with this pack’s XML (named `"fire"`, `delay` modulus), **this schema wins**. Do **not** keep a second 1098 attack shape. 772 and 1098 load the **same** `data/monster/*.lua`. Era differences are combat-type / condition **gates** in the engine (`MechanicsProfile` / formulas), not forked monster files.

---

## Why not copy 1098 / TFS revscript

TFS `register_monster_type.lua` + `Game.createMonsterType` is the wrong tool here:

| TFS revscript | This pack / 772 |
|---|---|
| Side-effecting scripts on the game VM, dozens of setters | Static defs; vocations already load sandboxed + serde |
| No `targetstrategy` / `losetarget` | Parsed today; 157 / 90 files |
| Summons `{chance, interval}` — no `max` / `force` | XML `delay` + `max` → `IMPACT_SUMMON` |
| Attacks rewritten to `combat` + `COMBAT_*` + `chance` | Named spells + `delay`; `monster_combat.rs` already maps that |
| `flags.runHealth` (TFS lib ignores it under `flags`) | `flags.run_health` from XML `runonhealth` |
| `voices.interval` / `chance` | 772 talk is `rand()%50` + `random(1, Talks)` — no per-race interval |
| Needs `Loot()` / `MonsterSpell()` userdata we do not have | Would be a large Lua-binding project for no outcome gain |

NPCs stay `NpcType("Name")` because they have **dialogue callbacks**. Monsters are numbers and lists. Treat them like vocations, not like NPCs.

---

## One corpus, era gates

**One** set of monster defs for every `clientVersion`. No `data/772/monster` vs `data/1098/monster`, and no TFS `chance`/`interval`/`COMBAT_*` rewrite of attacks.

| Shared (the Lua file) | Era-gated (engine / profile, not a second file) |
|---|---|
| Name, HP, speed, loot, flags, strategy, lose-target, summons `delay`/`max` | Whether a **combat type exists** this era |
| Attack/defense **shape**: `name` + `delay` + min/max + area | 1098-only types: **death, holy, earth, ice** (and their conditions) |
| Immunity bits in this pack | Mapping `poison` ↔ earth where 1098 renamed the type; 772 still poison |

If a def lists `name = "ice"` (or holy/death/earth), both eras parse the node. The 772 profile **ignores / no-ops** types that era does not have; 1098 **applies** them. Do not strip those attacks from Lua, and do not author a parallel `combat` + `COMBAT_ICEDAMAGE` table.

`monster_combat.rs` already aliases some names (`poison`/`earth`, `death`). Keep extending **name → impact** there, gated by the active profile — not by a second datapack.

---

## Current state

| Piece | Status |
|---|---|
| XML parse → `MonsterType` | **Done** — `crates/tfs-rust-content/src/monsters.rs` |
| Index `monsters.xml` name → file | **Done** — HashMap key = index name, not file `name=` (lesson 19) |
| Pipeline load before Lua VM | **Done** — `pipeline.rs` `spawn_blocking` after items; **still XML** |
| Combat from `MonsterSpellNode` | **Done** — `monster_combat.rs` `try_from_node` / `combat_from_monster_type` |
| `Game.createMonster` spawn | **Done** — looks up DB by name |
| `Game.createMonsterType` / `mType:register` | **Stubs only** — lib Lua exists, no Rust drain; not needed for defs |
| Schema + emit/parse | **Done** — `crates/tfs-rust-content/src/monster_lua.rs` |
| Converter bin | **Done** — `cargo run -p tfs-rust-content --bin export-monsters-lua` |
| `data/monster/*.lua` | **Done** — 157 files (flat slugs: `dragon.lua`, `red_butterfly.lua`, …) |
| XML kept | **Yes** — `monsters.xml` + `monsters/*.xml` still on disk |
| Production load | **Still XML** — `MonsterDatabase::load_dir` unchanged |
| XML `yell="1"` | **Done** — prefixes `#y ` on `talk_texts` (idle already stripped it) |
| Immunities (all 8) | **Done** — always emit `true`/`false` for fire, energy, poison, physical, outfit, life_drain, paralyze, invisible |
| Paralyze | **Done** — stored; 772 `NoParalyze` skips `SpellImpact::Speed` paralyze (haste still applies) |
| Outfit immunity | **Stored, not gated** — no 772 `NoOutfit`; outfit spells still apply |
| `immunity_life_drain` at spawn | **Fixed** — `from_monster_type` now copies it (was dropped) |

Corpus: **157** types. All have `targetchange` + `targetstrategy`; **90** have `losetarget`; **35** have summons; **113** have voices; **0** have `<elements>` / `script=` / `<events>`.

Re-export (does not delete XML):

```sh
rtk cargo run -p tfs-rust-content --bin export-monsters-lua
```

---

## Target format

One file per type: `data/monster/<slug>.lua`. File must `return { schema = 1, … }`. Snake_case keys (same as vocations). `#`-prefixed files are skipped (keeps `lua/#example.lua` as a non-loaded sketch).

**No** `monsters.xml` once the loader switches. Spawn / `get_by_name` key is the def’s `name` (case-insensitive). Filename is not the key.

When display name ≠ spawn name (butterflies): `name` = lookup/spawn (`"Red Butterfly"`), `title` = shown name (`"Butterfly"`). If `title` is omitted, `name` is the title.

### Canonical example — Dragon (converter output)

See `data/monster/dragon.lua`. Summary of the contract:

```lua
-- Generated from XML. Source: monsters/dragon.xml
return {
  schema = 1,
  name = "Dragon",
  description = "a dragon",
  race = "blood",
  experience = 700,
  speed = 45,          -- 772 GoStrength, not TFS display speed 172
  mana_cost = 0,
  health = 1000,
  max_health = 1000,
  outfit = {
    look_type = 34,
    look_head = 0, look_body = 0, look_legs = 0, look_feet = 0,
    corpse = 2844,     -- this pack, not 5973
  },
  change_target = { chance = 5 },
  target_strategy = { nearest = 70, weakest = 10, most_damage = 10, random = 10 },
  lose_target = { chance = 5 },
  flags = {
    hostile = true, summonable = false, illusionable = true, pushable = false,
    convinceable = false, can_push_items = true, can_push_creatures = true,
    target_distance = 1, run_health = 300,
    -- static_attack omitted → default 95
  },
  attacks = {
    { name = "melee", skill = 55, attack = 42, skill_factor = 1100,
      skill_next_level = 100, skill_add_count = 2 },
    { name = "fire", delay = 9, min = -100, max = -160, length = 8, spread = 3,
      effect = "firearea" },
    { name = "fire", delay = 7, min = -55, max = -105, range = 7, radius = 4,
      target = true, shoot = "fire", effect = "firearea" },
  },
  defenses = {
    armor = 25, defense = 38,
    spells = {
      { name = "healing", delay = 8, min = 34, max = 56, effect = "blueshimmer" },
    },
  },
  immunities = {
    fire = true, energy = false, poison = true, physical = false,
    outfit = false, life_drain = false, paralyze = true, invisible = true,
  },
  voices = {
    { text = "GROOAAARRR", yell = true },
    { text = "FCHHHHH", yell = true },
  },
  loot = {
    { id = 2187, chance = 1000 }, -- wand of inferno
    { id = 2148, chance = 50000, count_max = 60 }, -- gold coin
    -- …
  },
}
```

### Summons example — Giant Spider (`giant_spider.lua`)

```lua
summons = {
  max = 2,
  { name = "Poison Spider", delay = 10, max = 2 },
}
```

`max` on the list is XML `maxSummons` (cap 100). Per-entry `delay` / `max` / `force` map to `SummonBlock`. Do **not** emit TFS `chance`/`interval` unless we later add a 1098 profile path.

### Nested loot (container)

```lua
{ id = 1987, chance = 60000, child = {
    { id = 2152, chance = 10000, count_max = 2 },
} }
```

Same contract as XML `<item>` + `<inside>`: only filled when the id is a container in the item DB. This pack currently has no nested loot.

### Butterflies

`red_butterfly.lua` (and blue/purple/yellow): `name = "Red Butterfly"`, `title = "Butterfly"`. Spawn lookup uses `name`; `MonsterType.name` is `title`.

### Immunities (shipped 2026-08-23)

Every file has an `immunities` table with **all eight** keys, `true` and `false` (XML always listed them; omitting zeros hid Amazon-style “none”). Order matches XML: `fire`, `energy`, `poison`, `physical`, `outfit`, `life_drain`, `paralyze`, `invisible`.

| Bit | Lua | 772 / runtime |
|---|---|---|
| fire / energy / poison / physical / life_drain | stored | `NoBurning` / `NoEnergy` / `NoPoison` / `NoHit` / `NoLifeDrain` — block matching `Damage` |
| invisible | stored | `SeeInvisible` |
| paralyze | stored | `NoParalyze` — skip `SpellImpact::Speed` when the roll is a slow (haste still applies) |
| outfit | stored | **no** `NoOutfit` in `.mon` — outfit spells still apply |

`MonsterAiConfig::from_monster_type` copies all of these, including `immunity_life_drain` (that copy was previously missing).

---

## Field map (XML → Lua → Rust)

Keep `MonsterType` as the in-memory type. Lua deserializes into serde defs in `monster_lua.rs`, then fills today’s structs (including `MonsterSpellNode` attribute maps so `try_from_node` does not change).

| XML | Lua | `MonsterType` |
|---|---|---|
| index `name` / root `name` | `name` / optional `title` | lookup key / `name` |
| `nameDescription` | `description` | `name_description` |
| `race` `experience` `speed` `manacost` | same snake | existing fields |
| `<health now max>` | `health` `max_health` | `health_now` `health_max` |
| `<look type head body legs feet addons typeex mount corpse>` | `outfit.look_type` … `corpse` | `MonsterOutfit` |
| `<targetchange chance interval/speed>` | `change_target` | `change_target_chance` / `change_target_speed` |
| `<targetstrategy nearest weakest mostdamage random>` | `target_strategy` | `strategy_*` |
| `<losetarget chance>` | `lose_target.chance` (omitted when 0) | `lose_target_percent` |
| `<flag runonhealth>` | `flags.run_health` | `run_away_health` |
| `<flag staticattack>` | `flags.static_attack` (omitted when 95) | `static_attack_chance` |
| `<flag targetdistance>` | `flags.target_distance` | default 1 |
| other flags | snake bools | existing flags; `can_push_creatures` still forces `pushable = false` |
| `<attack>` / `<defense>` attrs + `<attribute key value>` | attack/defense tables | `MonsterSpellNode` (`name` → attr, `effect`/`shoot` → attribute_children) |
| `<immunities>` 0/1 attrs | `immunities` — all 8 keys always | fire/energy/poison/physical/outfit/life_drain/paralyze/invisible |
| `<voice sentence yell>` | `voices = { { text, yell } }` | `talk_texts`; `yell = true` prefixes `#y ` |
| `<summons maxSummons>` / `<summon>` | `summons.max` + numeric rows | `max_summons` / `SummonBlock` |
| `<loot><item id chance countmax …>` | `loot` | `LootBlock`; unknown ids warn-and-skip |

**Do not add** (not in this pack, not in 772 race load we use): `elements` %, `can_walk_on_*`, `attackable`, `boss`, `health_hidden`, `ignore_spawn_block`, `skull`, `light`, `events`, `script=`. Reserve nothing until a file needs them.

**Spell table rules:**

- `name` is the XML attack/defense name (`melee`, `fire`, `poisonfield`, `healing`, `speed`, `ice`, `holy`, `death`, `earth`, …) — not TFS `"combat"`. Unknown names stay on the node; the era profile decides if they fire.
- Damage fields stay `min` / `max` (signed, as XML).
- Area: `length`+`spread` or `radius`; `range`; `target` bool.
- Effects: string names as in XML (`"firearea"`, `"fire"`), not `CONST_ME_*`. The combat mapper already resolves those.
- Melee extra: `skill`, `attack`, `poison_cycles`, `skill_factor`, `skill_next_level`, `skill_add_count`.
- XML attr → Lua: `skillfactor`→`skill_factor`, `speedvariation`→`speed_variation`, `areaeffect`→`effect`, `shooteffect`→`shoot`.
- Unknown keys on a spell table become `MonsterSpellNode` attributes (forward-compatible).

---

## Loader (remaining)

Replace `MonsterDatabase::load_dir` XML path. Stay in `tfs-rust-content` (content pipeline, `spawn_blocking`, **before** `LuaRuntime`). Reuse `sandboxed_data_lua` + `load_data_table` / `load_data_table_str` + `require_schema` (`schema = 1`). `parse_monster_lua` already exists.

```
pipeline::load_all
  items (needed for loot id check)
  MonsterDatabase::load_dir(data/monster)
    scan **/*.lua, skip `#` in filename, sort
    each file: eval sandbox → MonsterDef → MonsterType
    insert by Lua name.to_lowercase(); duplicate name = hard fail
  map / spawns (names must resolve)
LuaRuntime::new   -- Game.createMonster still hits the frozen DB
```

Hard-fail on eval / schema / missing `name`. Unknown loot id: keep today’s warn-and-skip so a bad id does not drop the whole type. Duplicate spawn names abort startup.

`can_push_creatures` → force `pushable = false` remains a load-time rule.

No game-thread Lua for monster stats. No `_pending_monsters`. No `register_monster_type.lua` drain.

Until this ships, `load_dir` still reads `monsters.xml`.

---

## Converter (shipped)

`cargo run -p tfs-rust-content --bin export-monsters-lua`

- `--data` (default `data`), `--out` (default `{data}/monster`).
- Parses with the **current** XML loader so output matches today’s `MonsterType`.
- Slug: lowercase, spaces → `_` (`red_butterfly.lua`). `name` inside the file stays `"Red Butterfly"`.
- Loot line comments from `ItemDatabase` names.
- Always emits all eight `immunities` keys (including `false`).
- Flat `data/monster/*.lua`. Overwrites existing Lua (including `dragon.lua`). Does **not** delete XML.

API: `emit_monster_lua` / `parse_monster_lua` / `export_monsters_lua` / `monster_lua_slug` in `monster_lua.rs`.

---

## Implementation order

1. **Done** Schema + emit/parse (`MonsterDef` serde, `parse_monster_lua`).
2. **Done** Converter bin + 157 files.
3. **Done** Golden: `all_xml_monsters_round_trip_through_lua` (strategy, summons, spell shape, loot, voices, all 8 immunities).
4. **Done** Immunity pass (lesson 365): store paralyze/outfit; always emit zeros; copy `life_drain` at spawn; 772 `NoParalyze` on speed-down.
5. **Remaining** `load_dir` prefers `*.lua`; XML fallback optional for one release.
6. **Remaining** Drop XML fallback. Update tests that load `data/monster` (772 outcomes stay; file suffix changes).
7. **Remaining** Delete `data/monster/monsters.xml` + `monsters/*.xml`. Remove XML parser from this module if unused elsewhere. Delete or keep `lua/#example.lua` (TFS sketch — do not copy). Leave `register_monster_type.lua` as a stub.

---

## Tests

Shipped in `monster_lua.rs`:

- slug (`Red Butterfly` → `red_butterfly`)
- Dragon emit/parse (speed 45, corpse 2844, delays 9/7, strategy, yell `#y `)
- Red Butterfly `name`/`title`
- Giant Spider summons delay 10 / max 2
- Warlock spell names + `speedvariation` round-trip
- Full-dir XML→Lua round-trip (157), including all 8 immunity bits
- Amazon: all-false table still emitted
- Ancient Scarab / Dragon: `paralyze` / `outfit` / `life_drain` match XML
- `from_monster_type` copies `immunity_life_drain` and `immunity_paralyze`

Keep existing combat goldens; they must still pass after the **loader** swap:

- `parses_monster_ai_flags` / `parses_targetstrategy_and_losetarget` / `parses_monster_summons_block` — rewrite fixtures as Lua strings when XML parse goes away.
- `index_name_is_lookup_key_not_file_name_attr` → `name` vs `title` for Red Butterfly.
- `test_dragon_strategy_and_losetarget_from_xml` (rename) — strategy 70/10/10/10, lose 5.
- `test_e0_dragon_fire_spells_shape_mapping` — delay 9 wave + delay 7 ball.
- `test_giant_spider_summon_spell_from_xml` — delay 10, max 2.
- `test_dragon_merges_defense_spells_at_spawn`.

```sh
rtk cargo test -p tfs-rust-content --lib monster_lua
rtk cargo test -p tfs-rust-content --lib monsters
rtk cargo test -p tfs-rust-core --lib monster_combat
rtk cargo run -p tfs-rust-content --bin export-monsters-lua
```

---

## Decisions

| Topic | Do |
|---|---|
| Format | Lua-as-data `return { schema = 1, … }`, snake_case, sandbox + serde |
| TFS `createMonsterType` / `register` | **Do not implement** for this migration |
| Numbers | Copy XML 772 values; do not “upgrade” to TFS 8.x+ loot/speed/corpse |
| Attacks | Named + `delay` only. Same files for 772 and 1098. No `combat`/`COMBAT_*`/`chance` fork |
| Voices | List of `{ text, yell }`; no fake interval/chance |
| Index file | Delete **after** loader switch; `name` is the key |
| Butterflies | `name` = spawn key, `title` = XML file `name` attr |
| Load time | Content pipeline, before game Lua VM |
| Dual path | XML remains production until loader switch (steps 5–7). Same Lua corpus both eras |
| Extra 1098 types | death / holy / earth / ice — **profile gates**, not extra monster files |
| `elements` / walk-on-field / boss flags | Skip until a def needs them |
| Paralyze / outfit immunity | Always in Lua. Paralyze = 772 `NoParalyze` (skip speed-down). Outfit stored only (no 772 flag) |
| Live scripted custom monsters | Out of scope |

---

## Deferred

- Switch `load_dir` to Lua and delete XML (steps 5–7 above).
- 1098 death/holy/earth/ice: keep **gating** in `monster_combat` / conditions by profile; do **not** add a second attack schema or datapack tree.
- Monster `onThink` / `onAppear` Lua callbacks (none in this pack).
- `elements` percent map (none in this pack).
- Enforce `immunity_outfit` on `SpellImpact::Outfit` (no 772 twin; leave stored-only).
- Grouping files into `data/monster/bosses/` etc.
- Removing `data/scripts/lib/register_monster_type.lua` (harmless stub until something calls the missing setters).
