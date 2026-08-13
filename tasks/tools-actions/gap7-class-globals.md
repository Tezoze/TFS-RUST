# Gap 7 — engine class globals

Index: [README.md](README.md) · load model this came out of: [architecture.md](architecture.md) · current failure set: [re-audit-2026-08-13.md](re-audit-2026-08-13.md)

## How this was found — the `global.lua` dofile blocker (historical, pre-7a)

*Fixed by 7a; kept because it is the clearest statement of the bug class.* The dofile chain works (CWD resolution is fine) but fails at `data/lib/core/combat.lua:1`:

```
runtime error: data/lib/core/combat.lua:1: attempt to index global 'Combat' (a function value)
```

`data/lib/core/combat.lua` does `function Combat:getPositions(...)` — adding a method to the `Combat` global. In TVP, `Combat` is a **class table** (created by `registerClass("Combat")` in `luascript.cpp`) with a `__call` metamethod, so it's both callable (`Combat()` creates a `CombatRef` userdata) AND extensible (`function Combat:method(...)` adds a method).

Our `Combat` global is a **bare function** (`userdata/combat.rs:345` — `lua.globals().set("Combat", combat_new)`), not a table. Lua can't index a function value, so `function Combat:getPositions(...)` fails.

## 7a / 7b — userdata class globals registered as bare functions / not at all ✅

**7a done 2026-08-10** (`register_class` + `register_engine_class_tables` in `crates/tfs-rust-lua/src/class_registry.rs`; all 9 core lib files load — `lib_core_files_load_with_zero_errors` test). **7b done 2026-08-10** (`class_index_lookup` shared `__index` chain helper in `class_registry.rs`; 8 userdata types wired — `CreatureRef`/`TileRef`/`ItemRef`/`ContainerRef`/`ItemTypeRef`/`PositionRef`/`CombatRef`/`VocationRef`; `CreatureRef` latent `Creature`-table bug fixed via `Player → Creature` chain; tests `gap7b_lua_class_method_callable_via_userdata`, `gap7b_creature_ref_reaches_creature_table`, `gap7b_native_method_wins_over_lua_override`, `class_index_lookup_walks_chain_first_hit_wins`, `class_index_lookup_returns_nil_for_missing_or_non_table_global`). Scope corrected 2026-08-10 — the earlier guess ("likely affects `Spell`, `Weapon`, `Condition`") was **wrong in both directions**. Verified by grepping `data/lib/**` and `data/scripts/**` for `function <Class>[:.]` and probing global kinds after bootstrap.

**Superseded 2026-08-13:** the claim below that "nothing in the data pack extends `Spell`, `Weapon`, or `Condition`" is **wrong** — `data/scripts/lib/helper_constructors.lua:1-5` consumes `getmetatable(class).__call` for `{Action, CreatureEvent, Spell, TalkAction, MoveEvent, GlobalEvent, Weapon}`. `Spell`/`Weapon` were converted anyway (correctly); `Condition` and the five revscript ctor globals still need it — Gap 7c.

The classes 7a covered — **the `Registered as` / `Result` columns describe the pre-7a tree; all of these now load**:

| Global | Registered as | Extended by | Result |
|---|---|---|---|
| `Tile` | function (`userdata/tile.rs:411`) | `lib/core/tile.lua` (6 methods incl. `relocateTo`) | **fails** |
| `Position` | function (`userdata/position.rs:239`) | `lib/core/position.lua` (3 methods incl. `moveUpstairs`) | **fails** |
| `Combat` | function (`userdata/combat.rs:345`) | `lib/core/combat.lua` (2 methods) | **fails** |
| `ItemType` | function (`userdata/item_type.rs:146`) | `lib/core/itemtype.lua`, `item.lua:111` | **fails** |
| `Party` | **not registered** | `lib/core/party.lua`, `events/scripts/party.lua` | **fails** |
| `Teleport` | **not registered** | `lib/core/teleport.lua` | **fails** |
| `Vocation` | **not registered** | `lib/core/vocation.lua` | **fails** |
| `Player`, `Creature`, `Item`, `Container`, `Game` | table (`runtime.rs:1215-1227`) | many | OK |

**Problem:** a bare constructor function can be called but not indexed, so `function Tile.relocateTo(self, …)` raises `attempt to index global 'Tile' (a function value)`. TVP's `registerClass` (`luascript.cpp`) creates a **table** with a `__call` metamethod — callable *and* extensible.

**Root cause is architectural, not a list of 7 omissions.** (Line references in this subsection describe the **pre-7a** tree, i.e. before `69dbaf0`.) Three mechanisms independently write class globals, and their outcome is decided by line ordering in `LuaRuntime::new` (`runtime.rs:120-145`):

1. `userdata/*.rs` → `globals.set("Tile", tile_new)` — a bare **function** (`tile.rs:411`, `position.rs:239`, `combat.rs:345`, `item_type.rs:146`, `spell.rs:144`, `weapon.rs:88`)
2. `register_event_script_bootstrap` → `globals.set(name, table)` for a **hardcoded list of 8** (`runtime.rs:1215-1227`)
3. Hand-written `__call` merge blocks — only for `Player` (`runtime.rs:1519-1534`) and `Creature` (`1541-1558`), two near-identical ~18-line copies

(1) clobbers (2), acknowledged in-tree at `runtime.rs:139`:

```rust
register_event_script_bootstrap(&lua)?;
// Overwrite empty `Tile` / `Game` stubs from bootstrap with real constructors.
register_tile_constructor(&lua)?;
```

So the clobbering that breaks `tile.lua` is **deliberate and documented** — the design, not an oversight. Whether a class ends up callable, extensible, both, or `nil` depends on where its registration sits in a 40-line init function. `Position` is a function because it registers *before* the bootstrap and isn't on the list; `Party` is `nil` because nobody added it. Converting 7 classes by hand reproduces this with a longer hardcoded list and **9 copies** of the `__call` block — already at 2, and `TFS-code-hygiene` says extract before the third.

**Correction (2026-08-10, second probe): a class table alone is NOT sufficient.** The first version of this fix would have cleared the 9 load errors and still left `tile:relocateTo(pos)` broken at call time. Verified:

```
Tile made a class table, `function Tile.luaSideMethod` defined,
  TileRef userdata :luaSideMethod()   → Err "attempt to call method (a nil value)"
CreatureRef :creatureOnlyMethod()     → Err "attempt to call method (a nil value)"   (Creature table)
CreatureRef :playerOnlyMethod()       → Ok("player")                                  (Player table)
```

Two separate mechanisms are required, and only the second one makes the methods *callable*:

| | Fixes | Mechanism |
|---|---|---|
| **7a** class table | *load* time — `function Tile.relocateTo(…)` stops erroring | `Tile` global is a table with `__call` |
| **7b** userdata `__index` fallback | *call* time — `tile:relocateTo(pos)` actually resolves | the userdata metatable chains `__index` → the class table |

**7b done 2026-08-10** — all 8 userdata types now have the fallback via the shared `class_index_lookup` helper (`class_registry.rs`). The previous state (only `CreatureRef`, hardcoded to `Player` only) is replaced.

**Latent bug fixed:** `CreatureRef`'s old fallback reached `Player` but **not `Creature`**, so all 15 methods in `data/lib/core/creature.lua` (`getPlayer`, `isPlayer`, `setMonsterOutfit`, `addSummon`, `addDamageCondition`, `canAccessPz`, …) plus `functions.lua:530` `Creature:addAttributeCondition` were unreachable from Lua — independent of the tools work. The new `Player → Creature` chain fixes this (regression guard: `gap7b_creature_ref_reaches_creature_table`). TFS's `registerClass` takes a **base class** and chains the hierarchy; the shared helper mirrors that with a `&'static [&'static str]` chain per userdata.

**Fix — one primitive pair, not seven conversions:**

Introduce `register_class` as the *only* way a class global is created:

```rust
/// Get-or-create the class table for `name`, optionally attaching a `__call`
/// constructor. Idempotent and order-independent — never replaces an existing
/// table, so registration sequence stops mattering.
/// C++ reference: `luascript.cpp` `LuaScriptInterface::registerClass`.
fn register_class(lua: &Lua, name: &str, ctor: Option<Function>) -> Result<Table, mlua::Error>
```

Then:
- Every `userdata/*.rs` swaps `globals.set(Name, ctor_fn)` → `register_class(lua, Name, Some(ctor_fn))?`.
- `Party` / `Teleport` / `Vocation` → `register_class(lua, Name, None)?` (table-only; no constructor needed for the lib files to attach methods).
- The hardcoded 8-name list in `register_event_script_bootstrap` **is deleted** — a class exists because something registered it, not because it appears on a list.
- The two `__call` blocks collapse into `register_class`; `Player` / `Creature` become ordinary call sites.
- Delete the `runtime.rs:139` overwrite comment and the ordering constraint it documents.

**7b — the matching userdata-side helper (done 2026-08-10).** One shared `class_index_lookup(lua, chain, key)` free function in `class_registry.rs`; each userdata declares its chain as a `pub(crate) const &'static [&'static str]` and adds a one-line `MetaMethod::Index`:

| Userdata | Chain (first hit wins, after native Rust methods) |
|---|---|
| `CreatureRef` | `Player` → `Creature` |
| `TileRef` | `Tile` |
| `ItemRef` | `Item` |
| `ContainerRef` | `Container` → `Item` |
| `ItemTypeRef` | `ItemType` |
| `PositionRef` | `Position` |
| `CombatRef` | `Combat` |
| `VocationRef` | `Vocation` |

`ContainerRef → [Container, Item]` mirrors C++ `Container extends Item` (the data pack defines `Container.createLootItem` and `Item.getType`). `VocationRef` is included because `data/lib/core/vocation.lua` defines `Vocation.getBase(self)` — a real consumer. `Spell`/`Weapon`/`Condition`/`Npc`/`Group`/`MonsterType` are intentionally absent — no `function <Class>:method(...)` consumers in `data/` (the 7a scope correction: speculative fallbacks with no consumer). **Revisit `MonsterType` under 7c**: `register_monster_type.lua` defines `MonsterType.register`, though the only userdata-side caller is a `#`-prefixed (skipped) example file.

Native Rust methods keep priority — mlua 0.12's generated `__index` (`mlua-0.12.0/src/userdata/util.rs:311-333`) checks field getters → registered methods → the user `__index` function **last**, so a Lua override cannot silently shadow an engine method by construction. Verified by `gap7b_native_method_wins_over_lua_override` (native `getId` wins over a Lua override on `ItemType`/`Vocation`). This also means adding `MetaMethod::Index` to `PositionRef` (which has `add_fields` for x/y/z) is safe — field getters are checked first.

Gap 7a/7b are two small helpers plus ~10 mechanical call-site edits. Gap 7c closed the remaining `globals.set(Name, ctor_fn)` bypasses (`Action`, `TalkAction`, `MoveEvent`, `Channel`, `Condition`, `Variant`, `MonsterType`) and added `CreatureEvent`/`GlobalEvent`. The table-driven `all_class_globals_are_tables` test enumerates every `register_class` name so the bug class cannot recur for those globals. (`NpcType` / `NpcDialogue` are still bare functions — out of 7c scope; add them the same way if a lib file starts indexing them.)

**Tests (all passing):**
- every class global is a `table`, and still callable where it has a constructor (7a: `register_class_*` tests)
- for each userdata type, a Lua-defined method on its class table is **callable through a live userdata instance** — `gap7b_lua_class_method_callable_via_userdata` (all 8 types; the check that would have caught the 7a-only plan)
- `CreatureRef` reaches the `Creature` table (not just `Player`) — `gap7b_creature_ref_reaches_creature_table` (latent-bug guard)
- a native Rust method still wins over a same-named Lua method on the class table — `gap7b_native_method_wins_over_lua_override`
- chain walk + fall-through + best-effort skip of missing/non-table globals — `class_index_lookup_walks_chain_first_hit_wins`, `class_index_lookup_returns_nil_for_missing_or_non_table_global`

**Scope:** was a **prerequisite for Gap 5a and Gap 3**. Nine core lib files were failing to load because of this; they now load (7a) and their methods are callable through userdata (7b), so Gap 3's inventory could be re-audited against a fully-loaded lib ([re-audit-2026-08-13.md](re-audit-2026-08-13.md#gap-3--re-audit-result-supersedes-the-gap-3-correction-table)). Gap 5a **landed 2026-08-13**.

## Gap 7c — revscript constructor globals ✅ done 2026-08-13

Routed the 7 remaining bare-function class globals through `register_class(lua, name, Some(ctor))`, added `CreatureEvent` / `GlobalEvent` (plain tables with `__call`, matching `Action`), ported `createFunctions` into `data/lib/core/create_functions.lua` (not the full compat layer — [resolved decision #3](decisions.md#resolved-decisions) / new decision #11). `data/scripts/lib` loads 5/5. Unblocked Gap 5a (landed 2026-08-13). Details and probe output: [re-audit-2026-08-13.md](re-audit-2026-08-13.md#gap-7c--gap-7a-is-not-complete-new).

**Decisions made:**
- `MonsterType` is a class table (7a-style) so `register_monster_type.lua:12` (`MonsterType.register = function(self, mask)`) loads. **No** `MonsterTypeRef` `__index` chain (7b-style): the only `mType:register(...)` call is in `data/monster/lua/#example.lua`, which the loader skips (`#` prefix).
- `{Action, CreatureEvent, Spell, TalkAction, MoveEvent, GlobalEvent, Weapon}` all have `__call` so `helper_constructors.lua` can wrap `getmetatable(class).__call`. `CreatureEvent`/`GlobalEvent` constructors follow the `Action` plain-table pattern (`:type` / `:register` push into `_pending_*`; drained by a future content-stage loader).
- `createFunctions`: **ported** into `data/lib/core/create_functions.lua` (also dofile'd from `core.lua`). Do not load `compat.lua`.

**Tests:**
- `all_class_globals_are_tables` — every `register_class` name on a real `LuaRuntime` is a `table`; `__call` present iff a ctor was attached; `REQUIRED_CLASS_GLOBALS` must all be in the registry (catches a `globals.set` bypass); `HELPER_CTOR_CLASSES` must all be callable.
- `scripts_lib_files_load_with_zero_failures` — `data/scripts/lib/**` loads clean.
