# Data Format Migration — XML → Lua-as-Data

**Goal:** move all static game data off XML while keeping (and improving) the flexibility XML
gave us. Target format is **Lua tables loaded once at startup and materialized into strongly-typed,
immutable Rust structs**. This unifies our data story with the mechanics tuning we already load
from `data/formulas/*.lua`, reuses the LuaJIT runtime we already ship, and costs nothing at
runtime.

Status: **proposal / plan**. No behavior changes yet. See the Finding 3 remediation in
`GAME_LOOP_772_AUDIT.md` — the vocation regen bug is the first concrete driver.

---

## Guiding principle: format ≠ representation

Separate two concerns that XML currently conflates:

- **On-disk format** — how data is authored and stored.
- **In-memory representation** — a canonical, `serde`-derived Rust struct the engine reads.

Make the Rust struct the source of truth. The file format becomes an implementation detail behind
`serde::Deserialize`. Everything downstream (game loop, `TSkillFed` regen, level-up vitals, item
lookups, monster spawns) reads `&VocationDef` / `&ItemType` / `&MonsterDef` — never the file. Once
this boundary exists, the format is cheap to swap and easy to validate.

---

## Why Lua-as-data (for this project specifically)

- **Already a hard dependency.** mlua + LuaJIT is mandatory (`Cargo.toml`
  `features = ["luajit", "vendored"]`) and already runs mechanics tuning from
  `data/formulas/772.lua` / `1098.lua`. Vocations/items/monsters as Lua is consistent with an
  existing pattern rather than a third mechanism.
- **Flexibility XML can't match.** Shared constants, computed/derived values, loops to generate
  variants, and per-era conditionals live in one place. (Example: derive all promoted-vocation
  regen from base vocations instead of copy-pasting rows.)
- **Zero runtime cost with a hard boundary.** Parse the Lua table **once at startup**, deserialize
  into immutable Rust structs, then never touch Lua for reads. This respects the "Tier-1 scalars
  loaded at startup, zero per-tick Lua" rule (`tfs-mechanics-profile.md`) and the game-thread
  ownership model (`tfs-threading.md`).

### The enabler: mlua `serde` feature

Workspace currently has `mlua = { … features = ["luajit", "vendored"] }`. Add **`"serde"`** to get
`LuaSerdeExt`, so a Lua table deserializes straight into a `serde` struct:

```rust
let value = lua.load(&src).eval()?;                 // Lua table
let defs: Vec<VocationDef> = lua.from_value(value)?; // needs mlua "serde"
```

---

## Reference pattern

### Data file (pure data — returns a table, no side effects)

```lua
-- data/defs/vocations.lua
local HP, MANA = 1, 2  -- shared constants; loops / era conditionals allowed here
return {
  schema = 1,
  vocations = {
    { id = 0, name = "None",    gain_hp_ticks = 6, gain_hp = HP, gain_mana_ticks = 6, gain_mana = 1,
      gain_cap = 10, base_speed = 70, mana_multiplier = 4.0, attack_speed = 2000 },
    { id = 3, name = "Paladin", gain_hp_ticks = 8, gain_hp = HP, gain_mana_ticks = 4, gain_mana = MANA,
      gain_cap = 20, base_speed = 70, mana_multiplier = 1.4, attack_speed = 2000 },
    -- ...
  },
}
```

### Canonical Rust type (source of truth)

```rust
#[derive(Debug, Clone, serde::Deserialize)]
pub struct VocationDef {
    pub id: u16,
    pub name: String,
    pub gain_hp_ticks: u16,
    pub gain_hp: i32,
    pub gain_mana_ticks: u16,
    pub gain_mana: i32,
    pub gain_cap: i32,
    pub base_speed: i32,
    pub mana_multiplier: f32,
    pub attack_speed: u32,
}
```

### Loader (I/O side, `tfs-rust-content` / `tfs-rust-lua`; hands plain data to core)

```rust
pub fn load_vocations(path: &Path) -> anyhow::Result<Vec<VocationDef>> {
    let lua = sandboxed_data_lua()?;            // io/os/package/require stripped
    let root: mlua::Table = lua.load(std::fs::read_to_string(path)?).eval()?;
    require_schema(&root, 1)?;                   // version gate
    let defs: Vec<VocationDef> = lua.from_value(root.get("vocations")?)?;
    validate_vocations(&defs)?;                  // unique ids, ticks > 0, etc.
    Ok(defs)
}
```

Core stores the result as an immutable registry (`Arc<[VocationDef]>` or `HashMap<u16, _>`), and
the game thread reads structs only.

---

## Hard guardrails (make Lua-as-data safe)

1. **Sandbox the loader.** These files are data, not scripts. Load in a fresh restricted `Lua`
   with `io`, `os`, `package`, `require`, `dofile`, `loadfile` removed (or a whitelisted global
   env). Prevents a "config" file from doing I/O or escaping the sandbox.
2. **Strict deserialize + validation pass.** `serde` catches type/typo errors; add a semantic
   check (unique ids, non-zero tick divisors, required fields, cross-refs resolve). Fail fast at
   startup with a clear message — never a mid-game panic.
3. **Schema/version field.** `schema = N` at the top of each file so the format can evolve safely.
4. **Materialization boundary is absolute.** Load on the I/O side, hand owned `Vec<Def>` to core,
   store immutable. The game thread never calls Lua for a data lookup. Distinct from live scripts
   (`data/npc`, spells, actions), which remain executable Lua on the game thread per
   `tfs-lua-boundaries.md`.
5. **Keep a converter.** Ship an `xml → lua` one-shot tool so upstream TFS/TVP packs can be
   re-imported. This preserves our ability to pull community data without hand-editing.

---

## When Lua-as-data is *not* the right call

For data that is purely static (no computation, no cross-referencing), a typed config format
(**TOML** or **RON**) with `serde` is arguably safer: no code-execution surface, trivial
validation, clean diffs, same struct. Because both sit behind `serde` into the identical
`Def` types, we are not locked in — we can even support both per file type.

Rule of thumb:
- **Lua** where computation/derivation/era-conditionals help (vocations, formulas, spell tables,
  loot tables, monster stat scaling).
- **TOML/RON** for flat, locked-down tables if we ever want a non-programmable subset (e.g.
  `stages`, `groups`).

Binary formats stay as-is (they are not XML and are tied to the client/map): `items.otb`,
`*.otbm`, `objects.srv`, sprites. Only the XML *sidecars* around them migrate.

---

## XML inventory & migration plan

Current XML surface (loaders in `crates/tfs-rust-content/src/`):

| XML file(s) | Loader | Target | Notes |
|---|---|---|---|
| `data/XML/vocations.xml` | `vocations.rs` | `data/defs/vocations.lua` | **First mover** (drives Finding 3). Done in Phase PC-0. |
| `data/XML/outfits.xml` | `outfits.rs` | `data/defs/outfits.lua` | Flat table; could be TOML. |
| `data/XML/mounts.xml` | `mounts.rs` | `data/defs/mounts.lua` | Flat table. |
| `data/XML/groups.xml` | `groups.rs` | `data/defs/groups.lua` | Flat; TOML candidate. |
| `data/XML/quests.xml` | (quest system) | `data/defs/quests.lua` | Verify loader exists before migrating. |
| `data/XML/stages.xml` | (exp stages) | `data/defs/stages.lua` | Flat; TOML candidate. |
| `data/items/items.xml` | `items.rs`, `item_abilities.rs`, `items_xml_keys.rs` | `data/items.lua` | **Largest / highest-risk.** Pairs with binary `items.otb` (keep). Do last. |
| `data/monster/monsters.xml` + `data/monster/*.xml` | `monsters.rs` | `data/monster/*.lua` + index | Many files; spells parsed as nested nodes — nontrivial schema. |
| `data/world/*-spawn.xml` (+ house files) | `spawns.rs`, `otbm.rs` | `data/world/*-spawn.lua` | Referenced by OTBM `EXT_SPAWN_FILE`/`EXT_HOUSE_FILE`; keep OTBM binary, migrate the sidecar. |

**`data/defs/`** holds the static sidecar definitions (vocations, outfits, mounts,
groups, quests, stages) — loaded once into Rust structs via the sandboxed data-Lua
loader, never executed on the game thread. Distinct from `data/formulas/` (era
Tier-2 override *functions*) and the executable-script dirs (`actions/`, `spells/`,
`npc/`, …). `data/items.lua` and `data/monster/*.lua` stay co-located with their
binary companions (`items.otb`, monster assets) rather than under `defs/`.

Out of scope (not XML): `items.otb`, `*.otbm`, `objects.srv`, sprite assets. Executable Lua under
`data/{actions,spells,npc,creaturescripts,globalevents,movements,talkactions,weapons,lib}` already
*is* Lua and stays as scripts.

### Suggested phasing

1. **Phase 0 — infrastructure.**
   - Add `"serde"` to the mlua workspace feature set.
   - Add `sandboxed_data_lua()`, a `require_schema` helper, and a shared `DataLoadError`.
   - Establish the loader location (`tfs-rust-content` for pure data; keep it off the game thread).
2. **Phase 1 — vocations (pilot).** Migrate `vocations.xml` → `data/defs/vocations.lua`, define
   `VocationDef`, build a `VocationRegistry`, and switch `TSkillFed` regen + `recalculate_vitals` +
   base speed to read it. This also closes Finding 3 (regen values come from the vocation
   definition, not a hardcoded table) and the neighboring `vocation.rs` stubs. **Done in Phase PC-0.**
3. **Phase 2 — small flat tables.** outfits, mounts, groups, stages, quests. Low risk, builds the
   pattern and the `xml → lua` converter.
4. **Phase 3 — monsters.** Per-file defs + index; design the nested spell/loot schema carefully.
   Validate against existing `monsters.rs` behavior with golden tests.
5. **Phase 4 — items.** Highest risk (thousands of entries, many attribute keys, OTB pairing).
   Migrate `items.xml` last; keep `items.otb` binary. Diff-test the resulting `ItemType` set
   against the current XML loader before deleting the XML path.
6. **Phase 5 — cleanup.** Remove `quick-xml` / `roxmltree` deps and the XML loaders once every
   consumer is on the Lua path and golden tests pass.

### Verification per phase

- **Golden equivalence.** For each migrated file, load both XML (old) and Lua (new) and assert the
  materialized Rust structs are identical (a temporary dual-load test). Delete the XML loader only
  after the golden test is green.
- **Startup validation.** Every registry runs its validation pass; a malformed data file aborts
  startup with a precise error and file/line where possible.
- **No game-thread Lua for data.** Assert loaders run on the I/O/startup side and hand owned data
  to core.

---

## Open decisions (need a call)

1. **One big file vs many?** e.g. single `data/vocations.lua` vs `data/vocations/*.lua`. Monsters
   almost certainly stay per-file; vocations/outfits fit one file each.
2. **Era handling.** Inject `CLIENT_VERSION` as a sandboxed global so one file can branch, vs
   separate `data/772/…` and `data/1098/…` trees (mirrors `data/formulas/<version>.lua`).
   Recommendation: mirror the formulas layout for consistency.
3. **Lua vs TOML for flat tables** (groups/stages/outfits). Recommendation: Lua for uniformity now;
   revisit if we want a locked-down non-programmable subset later — the `serde` struct is shared
   either way.
4. **Converter scope.** One-shot migration only, or a maintained `xml → lua` importer for pulling
   future upstream packs? Recommendation: keep it maintained; it's cheap insurance.

---

## Why this is better than "just fix the XML"

- Kills duplicated constant tables in Rust (the Finding 3 `fed_regen_cadence` / `per_level_gains`
  stubs) by making the data file authoritative.
- One data mechanism (Lua) instead of XML + hardcoded stubs + formulas Lua.
- Programmability where it helps, `serde` type-safety everywhere, and zero runtime cost via the
  startup materialization boundary.
- Drops two parser dependencies (`quick-xml`, `roxmltree`) once complete.
