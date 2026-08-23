# Data-pack Lua — phased implementation plan

**Source doc:** [docs/DATA_PACK_LUA.md](../docs/DATA_PACK_LUA.md) (policy). This file is the **execution** plan.
**Companions:** [DATA_FORMAT_MIGRATION.md](../docs/DATA_FORMAT_MIGRATION.md), [monsters-lua-plan.md](monsters-lua-plan.md), [lessons.md](lessons.md).

**Corpus:** 772 behaviour for every `clientVersion`. One pack, one timing.

**Invariant carried through every phase:** monster loot is rolled **once, natively, at spawn, from `data/monster/*.lua` definitions**; Lua may only **mutate** what Rust produced. Nothing in this plan adds a death-time generator.

Read the invariant precisely — it is narrower than "one roll". `onSpawn` is *allowed* to add items; that is the rarity feature. What is forbidden is a **second roll against the monster definitions**, and any generation at **death**. Items a shard adds in `onSpawn` are pack policy, decided after the native roll, visible on the living monster. There is therefore no symmetric counterpart to the zero-item-delta assertion on `onDeath` (Phase 8.7), and that asymmetry is deliberate.

**Why native-core is not a trade against scriptability here:** spawn-time loot *is* the decompile outcome — `roll_monster_spawn_loot` cites `TMonster::TMonster` (`crnonpl.cc:2050`), where a 772 monster carries its inventory from construction. TFS's death-time roll is the divergence. The parity choice and the architecture choice coincide; the allowlist in Phase 1 protects a correct outcome rather than papering over a parity bug.

**Where the Lua/Rust line falls generally:** boot-time tables in Lua are fine (`data/formulas/772.lua`, monster defs, `data/lib` constants) — they are read once by Rust and create no second code path. Per-event mutation of a core outcome is not fine. That is the rule that separates the formulas pack from an `onGainExperience` hook.

---

## Target layout — one tree, zero script-registry XML

Everything script-shaped lives under `data/scripts/**` and self-registers. No `*.xml` index files, no parallel `data/<kind>/{lib,scripts}/` trees.

```
data/scripts/
  lib/              EventCallback bus, revscript ctors      (loaded today)
  creaturescripts/  CreatureEvent("Name"):register()        (Phase 1)
  eventcallbacks/   EventCallback ec.onX = … ec:register()  (Phase 1)
  globalevents/     GlobalEvent("Name"):register()          (Phase 7)
  actions/ movements/ spells/ talkactions/ weapons/ chatchannels/   (loaded today)
```

**Deleted outright**, not migrated one-for-one:

| XML | Lines | Loaded today | Replacement |
|---|---:|---|---|
| `data/creaturescripts/creaturescripts.xml` | 4 | yes (empty) | `data/scripts/creaturescripts/` self-registration (Phase 1) |
| `data/events/events.xml` | 35 | yes | **Registration is the enable bit** (Phase 5) |
| `data/globalevents/globalevents.xml` | 9 | no | `data/scripts/globalevents/` or Rust timers (Phase 7) |
| `data/movements/movements.xml` | 119 | **yes** | Derive natively from item data (Phase 6.1) — the only real migration |
| `data/actions/actions.xml` | 60 B | no | already empty stub |
| `data/spells/spells.xml` | 59 B | no | already empty stub |
| `data/talkactions/talkactions.xml` | 69 B | no | already empty stub |
| `data/weapons/weapons.xml` | 61 B | no | already empty stub |

Their eight parent trees (`data/actions/`, `data/creaturescripts/`, `data/events/`, `data/globalevents/`, `data/movements/`, `data/spells/`, `data/talkactions/`, `data/weapons/`, each with `lib/` + `scripts/`) go with them.

**Out of scope — content data, not script wiring.** `data/items/items.xml`, `data/XML/{vocations,outfits,mounts,quests,stages}.xml`, `data/raids/*.xml`, `data/world/*-spawn.xml`, `data/monster/monsters/*.xml`, `data/npc/archive/**`. Those are format migrations tracked in [DATA_FORMAT_MIGRATION.md](../docs/DATA_FORMAT_MIGRATION.md) and [monsters-lua-plan.md](monsters-lua-plan.md). Do not fold them into this plan — "no XML anywhere" and "no XML script registries" are different jobs with different risk.

---

## Phase 0 — Corrections to `docs/DATA_PACK_LUA.md` — **DONE 2026-08-23**

The policy doc was audited against the tree. Policy is sound; **nine factual statements were wrong or understated**, two of which change the shape of Phase 1 and Phase 4. All nine are now patched in `DATA_PACK_LUA.md`; the table below is kept as the audit record.

| # | Doc says | Reality | Impact |
|---|----------|---------|--------|
| 1 | Loading eventcallbacks without `isScriptsInterface` "registers silently fail" | The global **does not exist at all**. `event_callbacks.lua:81,102` calls `isScriptsInterface()`; nothing defines it in Rust or Lua. Loading an eventcallback file today **raises** `attempt to call a nil value` | Phase 1 must *create* the global, not just flip it. Anti-pattern wording changes from "silent" to "hard error" |
| 2 | "Rust must set that flag … TFS `loadScripts(..., isScriptsInterface)`" — implies a flag exists | No `isScriptsInterface` / `is_scripts_interface` / `scripts_interface` anywhere in `crates/` | Same as #1 |
| 3 | Monster loot mutation happens on "inventory userdata" at `Monster:onSpawn` | **There is no `Monster` userdata.** `class_registry.rs:558` registers `Monster` as a bare table for method definitions only. Live creatures are `CreatureRef`, which has no inventory accessor. `get_player_inventory_item` returns `None` for non-players (`game_world_inventory.rs:59`) | Phase 4 is much larger than the doc implies: the rarity hook needs a **new Lua surface** before it can exist |
| 4 | "`player.lua` loaded for inventory update only" | `player.lua` **is** exec'd (`script_loader.rs:166`), but the loader registers only `Player` + `onInventoryUpdate` (`script_loader.rs:186`) and **`onInventoryUpdate` is not in `events.xml`**. Net: **zero** Player event bodies dispatch, including the eleven `enabled="1"` ones | The "invert `events.xml`" task is hygiene/documentation today, not a behaviour change. Do it anyway, but don't expect an observable diff |
| 5 | `data/lib/debugging/dump.lua` / `lua_version.lua` listed as "not loaded today" | Those files **do not exist on disk**. `lib.lua` dofiles them, but `lib.lua` is skipped by `is_dofile_dispatcher` (`actions.rs:219`) so it is harmless | Drop the row, or note the dangling dofile |
| 6 | `MONSTERS_SPAWN_WITH_LOOT = false` "so TFS scripts work" listed only as an anti-pattern | It is **not a Rust config key**. `configKeys` exposes only `FREE_PREMIUM` (`constants.rs:327`); unknown boolean keys return `false` (`runtime.rs:2191`). So `default_onDropLoot.lua:11` reads **false** and would take the **generate** branch | Strengthens "do not load that file". Also means no shard can turn the spawn roll off — say so |
| 7 | `firstlogin.lua` gives "club, torch, armor, bag+food" | Actual kit is **axe**, torch, **coat** (sex-dependent), backpack + food | Cosmetic, but the doc is quoted as the starter-kit spec |
| 8 | Gap list is "vocation promotion, `removeTotalMoney`, god `addSkill`" | The real gap for the scripts we intend to **keep** is far wider — see Phase 3. `registerEvent`, `sendOutfitWindow`, `getLastLoginSaved`, `getLastLogout`, `setVocation`, `getSkull`, `getGuild`, `getOutfit`/`setOutfit`, `setDirection`, `db` are all unbound | Phase 3 exists because of this |
| 9 | `data/lib/core/create_functions.lua` "duplicate of `scripts/lib/create_functions.lua`" | Not a duplicate: 32 lines vs 2 lines | Cosmetic |

**Confirmed correct** (do not re-litigate): spawn-only loot roll with summon skip (`monster_inventory.rs:241-251`, `spawn_lifecycle.rs:397`); death moves inventory only (`game_world_lifecycle.rs:437`, `monster_inventory.rs:507`); native player drop with AoL 2173 / red-skull ALL / SOME = containers + 10% / corpse 3128 (`game_world_lifecycle.rs:658-759`); `on_death` / `on_startup` / `on_shutdown` no-ops; `on_monster_spawn` is a bool allow-gate; `load_data_lib` scans `lib/core/**` + `scripts/lib/**` + `data/scripts/*.lua` only; eventcallbacks and revscript creaturescripts unloaded; `creaturescripts.xml` empty; `globalevents.xml` unscanned; `runtime.rs:1771` stubs replaced by `event_callbacks.lua`; `LUA_ITEM_DESC` effectively false.

**Exit:** met — doc patched.

---

## Phase 1 — Scripts-interface loader — **DONE 2026-08-23**

**Goal:** make `EventCallback:register()` and revscript `CreatureEvent(...):register()` actually reachable, while loading only what the allowlist names.

**Shipped:** `isScriptsInterface` + `ScriptsInterfaceGuard` (`runtime.rs`); `load_scripts_interface` in `crates/tfs-rust-lua/src/scripts_interface.rs` (not `actions.rs` — that hub is already large); allowlist fail-closed; `load_data_lib` idempotent so `load_spell_scripts`'s second call cannot `EventCallback:clear()` the bus; boot call after `assert_required_data_globals`.

### 1.3 Reload — **(a) re-runnable from the start**

Chosen 2026-08-23. Each scan: `EventCallback:clear()` then replace `_pending_creature_events` / `_pending_global_events`. No `/reload` talkaction yet. Phase 2 registry must be a name-keyed replaceable map so per-player `registerEvent` sets re-resolve by name.

### 1.1 `isScriptsInterface` global

Add to `crates/tfs-rust-lua/src/runtime.rs`, next to the `hasEventCallback` / `EventCallback` stubs (`runtime.rs:1771`):

```rust
// TFS `LuaScriptInterface::isScriptsInterface` — true only while the
// scripts-interface pass is running. `data/scripts/lib/event_callbacks.lua`
// gates `EventCallback:register` / `__newindex` on it; outside the pass a
// stray `ec:register()` must be a no-op, not a nil call.
globals.set("isScriptsInterface", lua.create_function(|_, ()| Ok(false))?)?;
```

Back it with a `Cell<bool>` on the runtime plus a scoped guard so the flag cannot leak past the scan:

```rust
pub(crate) struct ScriptsInterfaceGuard<'a> { flag: &'a Cell<bool> }
impl Drop for ScriptsInterfaceGuard<'_> { fn drop(&mut self) { self.flag.set(false); } }
```

**Rationale for the guard rather than a bare setter:** an error mid-scan (`?` in the loader) must not leave the pack in a state where a later `dofile` can register callbacks.

### 1.2 Scan pass

New `load_scripts_interface(&self, data_dir: &Path)` in `crates/tfs-rust-lua/src/actions.rs`, called from `run_server.rs` **after** `load_data_lib` (`event_callbacks.lua` must have replaced the stubs first) and **before** the content loaders.

Scans, with the flag set: `data/scripts/eventcallbacks/**/*.lua` and `data/scripts/creaturescripts/*.lua`.

**Allowlist, not deny list.** A `const &[&str]` manifest in Rust of the files that may load; anything else in those trees is skipped with a boot warning naming the file.

A blocklist fails **open**: it defends against the six known-bad files and silently admits the seventh someone imports from upstream TFS next year — which is precisely the case the list exists for. An allowlist fails **closed**. Both trees are small and hook-shaped (13 files today), so the manifest costs nothing to maintain and every addition becomes a deliberate review. Keeping it in Rust rather than a Lua convention means editing `data/` cannot re-enable a forbidden generator.

| Allowed | Why it is safe |
|---|---|
| `creaturescripts/login.lua` | Welcome / promotion / outfit window, after strip (Phase 6.2) |
| `creaturescripts/firstlogin.lua` | Starter kit — Lua *is* the feature, no native core |
| `creaturescripts/playerdeath.lua` | Message only, after strip |
| `creaturescripts/logout.lua` | Only until Phase 6.2 empties it |
| `creaturescripts/extendedopcode.lua` | OTC only, behind config |
| `creaturescripts/killstatistics.lua` | Shard stats, behind config |
| `eventcallbacks/player/default_onReportBug.lua` | No native bug log exists; Lua is the feature |
| `eventcallbacks/monster/rarity.lua` | Phase 4, when authored |

Everything else in those trees is excluded. For the record, the six that would otherwise be actively harmful — `monster/default_onDropLoot.lua` (death generator; reads `MONSTERS_SPAWN_WITH_LOOT` as `false` so it always generates), `player/default_onLook.lua` and `default_onLookInBattleList.lua` (double "You see"), `player/default_onMoveItem.lua` (no-op masking the real rules in `events/scripts/player.lua`), `creaturescripts/droploot.lua` (duplicates native `player_death_drop_inventory`), `creaturescripts/regeneratestamina.lua` (8.x+ stamina) — are deleted outright in Phase 6.1. After that the manifest is defending against **re-import**, not against the shipped tree.

`data/creaturescripts/**` (the XML tree) is **never** scanned.

### 1.3 Reload: decide now, not later

**There is no `/reload` in the engine** — no match for `reload` anywhere in `crates/*.rs` outside an unrelated OTB tool. Half the stated value of Lua is editing the pack without a rebuild, and today you rebuild anyway. `event_callbacks.lua:127` carries a "can't be overwritten on reloads" comment, so the pack already expects one.

This is a Phase 1 decision because retrofitting reload onto a registry built for single-shot boot is the expensive version:

- **(a) Re-runnable from the start.** The scan is idempotent: `EventCallback:clear()` before re-scan, `CreatureEvent` registry replaced wholesale rather than appended, per-player `registerEvent` sets re-resolved by name against the new registry. Costs a little care in Phase 2's registry design; buys hot iteration on the pack.
- **(b) Single-shot, explicitly.** Accept that Lua's advantage here is authorship ergonomics and content portability, not hot iteration. Document it so nobody builds a workflow assuming reload.

**Recommendation: (a)**, and it mostly falls out of Phase 2 if the registry is a replaceable map keyed by name rather than an append-only buffer. Design it that way even if the `/reload` talkaction lands later.

### 1.4 Verification

- Boot with no eventcallbacks registered → `hasEventCallback(EVENT_CALLBACK_ONSPAWN)` is `false`, no error. (`event_callbacks.lua:128` already calls `EventCallback:clear()` at load, so the per-type tables exist.)
- Boot with a one-line test callback → `hasEventCallback` flips to `true`.
- Drop an unlisted file into `eventcallbacks/` → skipped, boot warning names it.

**Exit:** met — scan runs, guard resets the flag, allowlist enforced by unit tests, reload stance (a) recorded. CreatureEvent handlers still do not fire (Phase 2).

---

## Phase 2 — CreatureEvent registry + dispatch — **DONE 2026-08-23**

Phase 1 only *executes* the revscripts. `CreatureEvent(name)` (`runtime.rs:1628`) buffers; nothing drains it, and `Player:registerEvent` is unbound, so no handler ever fires.

### 2.1 Drain the pending buffer

Mirror the `_pending_global_events` pattern. After the Phase 1 scan, drain pending `CreatureEvent`s into a `HashMap<String, RegisteredCreatureEvent { kind, callback }>` on the Lua side of the dispatcher.

Make the map **replaceable wholesale**, not append-only, and resolve per-player `registerEvent` sets by **name** against the current map rather than caching callbacks. That is what makes Phase 1.3 option (a) cheap later; it costs nothing now.

### 2.2 Bind `Player:registerEvent(name)` / `unregisterEvent(name)`

Per-player event name set on the player entity, mirroring TFS. `login.lua` uses it; `killstatistics.lua` uses it.

### 2.3 Dispatch points

| Hook | Rust call site | Ordering requirement |
|---|---|---|
| `onLogin` | existing login path in `lua_event_dispatcher.rs` | Global `PlayerLogin` runs for every player; `FirstLogin` only when `getLastLoginSaved() == 0` |
| `onLogout` | existing logout path | Return `true`/`false` gates logout, per TFS |
| `onDeath` | `death.rs:281` `events.on_death(victim)` — override in `LuaEventDispatcher` | **After** `player_death_drop_inventory` and `handle_creature_death` XP loss (`game_world_lifecycle.rs:431`). Lua must see the finished state, never precede it |
| `onKill` | killer credit in `death.rs` | Optional; only if kill-stats enabled |

`onDeath` is the highest-risk wiring in this plan: it is the exact seam where TFS shards historically re-drop gear. Add a debug assertion that no item is created inside the `on_death` dispatch window.

### 2.4 Do **not** register

`DropLoot` and `RegenerateStamina` names are not in the registry at all, so a stray `player:registerEvent("DropLoot")` in a shard script is a warn-and-ignore, not a second gear drop.

**Exit:** met — name-keyed replaceable registry; `registerEvent`/`unregisterEvent`; login/logout/death/kill dispatch; `DropLoot`/`RegenerateStamina` excluded. Login welcome text still needs Phase 3 primitives.

---

## Phase 3 — Bind the primitives the kept scripts need — **DONE 2026-08-23**

Phase 2 makes the scripts run; most of them will then **error on line 1** because their APIs are unbound. Every binding below is a Rust primitive, per the policy's "bind or native-implement the primitive, then keep the Lua one-liner" rule.

### 3.1 Blocking — `login.lua`

| Binding | Needed by | Note |
|---|---|---|
| `Player:registerEvent` / `unregisterEvent` | Phase 2 | |
| `Player:getLastLoginSaved()` | Welcome vs first-login branch | |
| `Player:getLastLogout()` | "Last visit" line | |
| `Player:sendOutfitWindow()` | First-outfit window | Needs the 772 outfit packet |
| `Player:getOutfit()` / `setOutfit()` | Default outfit, promotion look | |
| `Player:setVocation(v)` | Promotion / demotion | |
| `Vocation:getPromotion()` / `getDemotion()` | Promotion, `Vocation.getBase` in `data/lib/core/vocation.lua` | Only `getId` is bound (`vocation.rs:29`) |

Already bound and usable: `getStorageValue`/`setStorageValue`, `getVocation`, `getGroup`, `hasFlag`, `getPremiumEndsAt`, `isPremium`, `sendTextMessage`, `teleportTo`, `setTown`, `Town:getTemplePosition`, `addCondition`.

### 3.2 Blocking — `firstlogin.lua`

`Player:addItem` and `Container:addItem` are bound. Missing: `Player:setDirection`, `Player:setOutfit`, `Player:getSex`.

### 3.3 Blocking — `playerdeath.lua`

The `db` global does not exist. Two options; pick one before starting the phase:

- **(a) Bind a narrow `db`** — `db.query` / `storeQuery` / `escapeString` / `asyncQuery` over the existing sqlx pool. Faithful to TFS, but hands arbitrary SQL to the pack.
- **(b) Keep the death row native** and give Lua only `Player:sendTextMessage` for the death text. Smaller surface; diverges from TFS call sites.

**Chosen 2026-08-23: (b).** `playerdeath.lua` keeps the death text only (also drops blessing-storage and guild-war SQL that Phase 6.3 would have removed). No `db` / `result` globals.

Also needed for the trimmed script: `Player:getSkull()` is unbound; only required if guild-war logic is kept — it is not (see Phase 6).

### 3.4 Non-blocking, deferred

`removeTotalMoney` (needs `Player:removeMoney` + `setBankBalance`), `transferMoneyTo`, `addSkill`/`addLevel` (needs `Game.getExperienceForLevel`, `addExperience`, `getSkillLevel`, `getRequiredSkillTries`, `removeManaSpent`), `Game.getPlayers` / `broadcastMessage`, `ItemType:getSlotPosition` / `getWeaponType`, `Party:getLeader` / `getMembers`, `Player:getClient` + `NetworkMessage` (OTC only), `Player:hasBlessing`.

None of these gate the 772 corpus. Bind them when a shard script needs them, not speculatively. `Player:hasBlessing` in particular must **never** be used for `getLossPercent` — exp/skill loss is `death.rs`.

**Exit:** every kept script loads and runs its happy path without a Lua error in the log.

---

## Phase 4 — Monster spawn mutate hook (the rarity feature) — **DONE 2026-08-23**

This is the payload of the whole design and the phase the policy doc understates. Rust already rolls the loot; there is currently **no way for Lua to see it**.

### 4.1 Monster Lua surface (new)

`Monster` today is a table-only class registered for method definitions (`class_registry.rs:558`). Needed:

1. A `MonsterRef` userdata (or extend `CreatureRef`) reachable from the spawn dispatch.
2. Inventory access over `MonsterInventory` (`bag`, `equipment`, `body` — see `monster_inventory.rs:507`), returning existing `Item` / `Container` userdata so `setAttribute`, `transform`, `addItem`, `remove` all work unchanged.
3. `Monster:getType()` → existing `MonsterTypeRef`.

**Design constraint:** hand out the *same* `ItemId`-backed userdata the rest of the pack uses. Do not invent a snapshot/apply layer — a copy would silently drop mutations and reintroduce the "rarity only on the corpse" anti-pattern.

**Stat-staleness hazard — resolve before binding.** `recompute_monster_combat_from_equipment` is `pub(crate)` and called exactly once, at `spawn_lifecycle.rs:403`. The moment Lua holds `ItemId` handles to a *living* monster's equipment, any later script — an action, a talkaction, a GM tool, a movement — can `remove()` or `moveTo()` a weapon mid-combat and the monster keeps fighting on stats computed at spawn. Recomputing after the `onSpawn` callback (§4.2) covers the rarity path and nothing else.

Two ways to close it; pick one and write it into the binding:

- **(a) Scope the accessor to the spawn hook.** The inventory handle is only reachable from the `onSpawn` callback argument and goes dead after it returns. Smallest surface, no staleness possible, but rules out legitimate later uses (GM inspect, quest scripts that seed a specific monster).
- **(b) Bind generally, recompute on mutation.** Any Lua path that changes monster equipment triggers the recompute. Requires making the recompute reachable from the mutation layer and auditing every arm that can touch a monster-held item.

**Recommendation: (a) for this phase.** The rarity feature needs nothing more, and (b)'s audit surface is the kind of thing that looks complete and then misses one arm. Revisit when a script actually needs post-spawn access — at which point (b) is a contained follow-up rather than a speculative widening.

### 4.2 Dispatch

`spawn_lifecycle.rs:397` already runs the roll right after the allow-gate:

```
on_monster_spawn(name, pos, startup)   // existing bool gate — unchanged
  → roll_monster_spawn_loot            // existing
  → recompute_monster_combat_from_equipment
  → NEW: on_monster_spawned(monster_id, pos, startup, artificial)
```

The new hook is **separate from the allow-gate** and returns nothing. Conflating them would let a rarity script cancel a spawn by falling off the end of a function.

It calls `data/events/scripts/monster.lua` `Monster:onSpawn`, which forwards to `EventCallback(EVENT_CALLBACK_ONSPAWN, ...)`. Load `monster.lua` in Phase 1's pass (it is currently not loaded at all).

Ordering: **after** `recompute_monster_combat_from_equipment`, then recompute again if the callback returns having touched equipment — or document that equipment swaps in `onSpawn` require an explicit recompute call. Prefer the automatic second recompute; it is once per spawn and removes a footgun.

### 4.3 Shard rarity file

New `data/scripts/eventcallbacks/monster/rarity.lua`, not shipped enabled:

```lua
local ec = EventCallback
ec.onSpawn = function(monster, position, startup, artificial)
	-- walk monster inventory / bag; setAttribute / transform / addItem
	return true
end
ec:register()
```

### 4.4 `Monster:onDropLoot`

Never dispatched. Keep `EVENT_CALLBACK_ONDROPLOOT` in the enum so old files parse; add no call site.

**Exit:** tests in Phase 8 pass. Rarity is visible on the **living** monster and survives the corpse move.

---

## Phase 5 — Retire `events.xml`: registration is the enable bit — **DONE 2026-08-23**

**Delete `data/events/` entirely** — the XML and all four forwarder bodies. Nothing replaces the enable bits, because the bits are redundant with something that already exists.

### 5.1 Why the file can just go

`events.xml` exists in TFS because C++ needed to know which `Player:onX` methods to look up in a monolithic `player.lua`. The revscript bus makes that lookup unnecessary: `EventCallbackData[type]` **is** the enable set, and `hasEventCallback(type)` (`event_callbacks.lua:75`) is the query. Rust guards each dispatch site with it:

```rust
// No registered callback → no Lua call, no XML consulted.
if runtime.has_event_callback(EventCallbackType::OnSpawn) { … }
```

Nobody registers `onLook` → nothing dispatches. That is the same outcome as `enabled="0"`, with one source of truth instead of two, and it removes the failure mode from Phase 0 #4 where eleven `enabled="1"` entries sat inert because the loader never looked at them.

The dispatch **site** stays a Rust decision, not a pack decision. Hooks the policy forbids — `onDropLoot`, `onGainExperience`, `onGainSkillTries`, `onShareExperience`, `onLook*` — simply have no call site in Rust, so registering one is inert. Warn at boot when a callback is registered for a type with no dispatch site: that is the honest replacement for the enabled-but-undispatched warning, and it catches the same class of mistake from the other end.

### 5.2 Rehome the real logic in `events/scripts/player.lua`

Three of the four forwarders (`creature.lua`, `monster.lua`, `party.lua`) are pure `EventCallback(...)` pass-throughs and are not loaded at all — delete them. `player.lua` is different: 196 lines, and it carries **actual pack content** mixed in with the forwarding.

| Content in `player.lua` | Goes to |
|---|---|
| `onMoveItem` aid 1000–2000 block, `onItemMoved` candelabrum + trap 2579 transform | New `data/scripts/eventcallbacks/player/moveitem.lua` — genuine native-core-then-mutate, keep as Lua |
| `onUseItem` GM tool block (rope / pick / shovel / scythe / machete on aid) | Fold into the existing `data/scripts/actions/tools/*.lua`, where the rest of that behaviour already lives |
| `onGainExperience` stages + stamina, `onGainSkillTries` `RATE_*`, soul condition | **Delete.** Native XP; stages and stamina multipliers are not 772 |
| `onLook*` forwarding | **Delete.** Native look |

This is the step that makes deleting `events.xml` safe: audit `player.lua` line by line before removing it, because it is the one file in the XML trees with behaviour that is neither duplicated natively nor forbidden.

### 5.3 Loader

Delete `ScriptLoader::load_player_events` and the `events.xml` structs (`script_loader.rs:150-164, 223+`), including the `Player` + `onInventoryUpdate` hardcode at `:186`. If inventory-update dispatch is wanted, it becomes an ordinary registered callback like everything else.

**Exit:** `data/events/` gone; dispatch set == registered set ∩ Rust call sites; boot warns on a registration with no call site.

---

## Phase 6 — Retire the XML script trees

Do this **after** Phases 1–5, so every deletion is provably unreferenced. One commit per subsection.

### 6.1 `movements.xml` — the only live migration

119 lines, parsed at `move_events.rs:228`, and the sole script-registry XML with real content still feeding the engine. Two kinds of entry, both of which are **derivable from item data rather than a hand-maintained list**:

- **Fields / campfires** (ids 1423–1425, 1487–1499…): `StepIn` → `onStepInField`, `AddItem` → `onAddField`. Both are natives, not Lua — `game_world_item_cylinder.rs:676` says so explicitly. The set is exactly "items with magic-field behaviour", which `magic_field.rs` and the item data already know.
- **Equip / DeEquip** (rings 2205–2216, helmets 2502/2664, dwarven armor 2503, legs 2504): bound to `onEquipItem` / `onDeEquipItem`, which are defaults that `return true` (`move_events.rs:210`). They exist only to trigger the ability path, which `equip_abilities.rs` already drives from `items.xml` abilities.

**Recommendation: derive both natively and delete the file** rather than translating it into a `data/scripts/movements/` revscript. Translating preserves a hand-maintained id list that is already a lossy copy of item data — the failure mode is a new field item that works in items data but is silently inert because nobody added two XML lines.

**Risk:** the derived set must be *exactly* the current set. Before deleting, dump both — the parsed XML bindings and the derived set — and diff them. Any id present in one and not the other is either a bug in the derivation or a latent bug in the XML; resolve each one explicitly rather than taking the union.

### 6.2 Delete the trees

| Path | Reason |
|---|---|
| `data/events/**` | Phase 5 — XML plus four forwarders |
| `data/creaturescripts/**` | XML tree: empty index, 8 orphan scripts duplicating the revscripts, 1-line `lib/`. Keeping them guarantees someone edits the wrong `login.lua` |
| `data/movements/**` | Phase 6.1 |
| `data/actions/**`, `data/spells/**`, `data/talkactions/**`, `data/weapons/**` | Empty ~60-byte XML stubs plus `lib/` + `scripts/` shells; the live scripts are under `data/scripts/` |
| `data/globalevents/**` | Phase 7 |
| `data/scripts/creaturescripts/droploot.lua` | Native player drop |
| `data/scripts/creaturescripts/regeneratestamina.lua` | Not 772 |
| `data/scripts/eventcallbacks/monster/default_onDropLoot.lua` | Death generator |
| `data/scripts/eventcallbacks/player/default_onMoveItem.lua` | No-op; real rules rehomed in Phase 5.2 |
| `data/scripts/eventcallbacks/player/default_onLook.lua`, `default_onLookInBattleList.lua` | Native look; would double "You see" |

Check each `<kind>/lib/*.lua` for content before deleting the shell — `creaturescripts/lib/creaturescripts.lua` and `globalevents/lib/globalevents.lua` are 1 line and empty respectively, but confirm rather than assume for the other six.

Rust side: delete `ScriptLoader::load_creaturescripts` + its `Death`-skip branch (`script_loader.rs:57-147`), `load_player_events` (`:150-164`), the XML structs (`:216-230`), and `MoveEvents::load_from_xml` (`move_events.rs:220`). `ScriptLoader` may end up empty enough to remove.

**Label the fork.** Once these are gone, the Phase 1 allowlist no longer defends the running server — it defends against **re-import**. This pack is a deliberate divergence from stock TFS: loot is spawn-rolled, death never generates. Add a short `data/scripts/README.md` saying exactly that, so the next upstream pull does not re-litigate it from scratch, and so a contributor who adds a file and sees it skipped knows why.

### 6.3 Strip functions inside kept scripts

| File | Remove | Keep |
|---|---|---|
| `data/scripts/creaturescripts/login.lua` | `registerEvent("DropLoot")`; `nextUseStaminaTime` init; premium-expiry teleport to Thais/Rookgaard + `setTown`; premium-outfit strip; `PlayerFlag_FullLight` GM light | Welcome text, last-visit line, first-outfit window, promotion/demotion, `registerEvent("PlayerDeath")`, `registerEvent("FirstLogin")` |
| `data/scripts/creaturescripts/playerdeath.lua` | `setStorageValue(101..105)` (TFS blessing items, not 772); guild-war kill logging + `isInWar`; `db` death-list insert/trim if Phase 3 option (b) | Death message |
| `data/scripts/creaturescripts/logout.lua` | `nextUseStaminaTime` clear — the whole file becomes empty; delete it and drop `PlayerLogout` from `login.lua` if nothing else lands here | — |
| `data/scripts/creaturescripts/killstatistics.lua` | `GlobalEvent("KillStatistics_Flush")` shutdown SQL until Phase 7 decides on `db` | Counters, if the shard wants them |
| `data/lib/core/container.lua` | `Container.createLootItem` body → hard error stub (`error("createLootItem: loot is rolled natively at spawn")`). A silent `return false` invites "why is my loot empty" | `isContainer` |
| `data/global.lua` | `getLootRandom` | TVP `actionIds` 4000–4005 |
| `data/lib/core/player.lua` | `getLossPercent`; Lua `depositMoney` / `withdrawMoney` / `sendCancelMessage` / `hasFlag` redefinitions (native wins; the Lua copy is dead weight) | `getPremiumDays`, `getClosestFreePosition`, `getDepotItems`, `addSkillTries` multiplier wrap |
| `data/lib/core/achievements.lua` | Whole file — not 772; nothing calls it | — |
| `data/lib/core/item.lua` | Lua `Item.getDescription` branch behind `LUA_ITEM_DESC` | Predicates, `getType` |
| `data/lib/lib.lua` | Dangling dofiles of the non-existent `debugging/` files | — |

`data/lib/compat/compat.lua` stays on disk, stays unloaded. Do not add it to any scan.

### 6.4 Guard against regression

Add a test that fails if any file outside the Phase 1 allowlist appears in the loaded-script set, and CI `rg` checks for (a) `createLootItem` / `getLootRandom` / `getLossPercent` call sites outside their own definition, and (b) **any new `*.xml` under `data/` outside the content-data allowlist** (`items/`, `XML/`, `raids/`, `world/`, `monster/monsters/`, `npc/archive/`). The second one is what stops the trees growing back.

**Exit:** one `login.lua`, one starter kit, no second loot path, no script-registry XML in the tree.

---

## Phase 7 — Globalevents

`globalevents.xml` is unscanned; `GlobalEvent(name)` buffers into `_pending_global_events` (`runtime.rs:225`) and is never drained; `on_startup` / `on_shutdown` have **no call sites at all**.

| Job | Decision |
|---|---|
| `startup.lua` — truncate `players_online`, expire bans/wars, house auctions, insert `towns` | **Rust** (sqlx + house module). Ops, not 772 mechanics. Do not wire the Lua |
| `serversave.lua` — 04:30 warn, `GAME_STATE_CLOSED`, save/shutdown | **Rust** timer + config. Lua only if editable warn text is wanted, and only after `Game.broadcastMessage` / `setGameState` exist |
| `record.lua` — "New record: N players" | **Optional Lua**, after `Game.getPlayers` + `broadcastMessage` are bound |

Either drain the pending-GlobalEvent buffer with a real `onTime` / `onStartup` scheduler, or **remove the `GlobalEvent` constructor** so it stops advertising a capability that does not exist. Do not ship the half-state.

**If the scheduler is built**, globalevents follow the same rule as everything else: scan `data/scripts/globalevents/**` under the scripts interface, self-registering via the existing `GlobalEvent(name)` constructor, on the Phase 1 allowlist. No `globalevents.xml`. `killstatistics.lua` already declares a `GlobalEvent("KillStatistics_Flush")` in the creaturescripts tree, so the drain has to work regardless of which directory the declaration came from — registration is by constructor call, not by file location.

**Exit:** startup/save are Rust; `data/globalevents/` deleted; any surviving globalevent is a revscript under `data/scripts/globalevents/`.

---

## Phase 8 — Tests

Ordered by how load-bearing the invariant is.

1. **Spawn roll is the only roll.** Spawn a monster, snapshot corpse contents after death, assert the multiset of item ids equals the spawn inventory. No extra ids, no missing ids.
2. **Rarity survives death.** `onSpawn` callback sets an attribute on a bag item; assert it is present on the **living** monster and on the corpse.
3. **Summons drop nothing.** Summon → kill → corpse has no rolled items.
4. **No loot without eventcallbacks.** Boot with the whole `eventcallbacks/` tree absent; loot still exists. (Catches any regression where Lua becomes load-bearing for loot.)
5. **Allowlist fails closed.** Write an unlisted file into `eventcallbacks/` at test time and assert it is not loaded — including a file name that did not exist when the manifest was written. That is the case a blocklist would miss.
6. **`isScriptsInterface` scoping.** Registration inside the scan succeeds; a `register()` after the scan is a no-op, not an error.
6b. **Monster inventory handle does not outlive `onSpawn`** (if Phase 4.1 option (a)) — a stashed handle used later errors rather than mutating a live monster.
7. **Player death drop unchanged.** Existing AoL / red-skull / SOME coverage must still pass with `onDeath` Lua registered — assert item count delta from the Lua dispatch window is zero.
8. **Registration/dispatch parity.** A callback registered for a type with no Rust call site produces a boot warning; a type with a call site and no registration performs no Lua call.
8b. **`movements.xml` derivation is exact.** Golden test: the natively derived StepIn/AddItem/Equip/DeEquip binding set equals the set the XML parser produced, id for id. Run it *before* deleting the file, keep it after as a regression guard on the derivation.
8c. **No script-registry XML.** Walk `data/`, assert no `*.xml` outside the content-data allowlist.
9. **1098 regression.** Shared paths touched in Phases 2, 4, 5, 6 must not change 1098 behaviour — `movements.xml` derivation in particular is era-shared.

---

## Dependency order

```
Phase 0  doc fixes                     (no code)  — DONE
Phase 1  isScriptsInterface + scan     ── blocks 2, 4, 5  — DONE
         1.3 reload stance             ── (a) re-runnable; registry shape is Phase 2
Phase 2  CreatureEvent dispatch        ── blocks 3  — DONE
Phase 3  binding gaps                  ── blocks the kept scripts running
Phase 4  Monster surface + onSpawn     ── the feature; needs 1
Phase 5  retire events.xml             ── needs 1, 4
Phase 6  retire the XML script trees   ── needs 1–5 (proves unreferenced)
         6.1 movements.xml derivation  ── independent of 1–5, start early
Phase 7  globalevents                  ── independent
Phase 8  tests                         ── alongside 4, 5, 6
```

Phases 4 and 7 are independent of 2/3 and can run in parallel. **Phase 6.1 is independent of everything** — it is a native derivation plus a golden diff, touches no Lua, and is the only XML removal with real behavioural risk. Start it early rather than saving it for the cleanup commit, so the diff has room to be wrong.

---

## Risk register

| Risk | Mitigation |
|---|---|
| `onDeath` Lua re-drops gear | Phase 2.4 (name not registrable) + Phase 8.7 (zero item delta assertion) |
| Rarity applied to a snapshot, lost on corpse move | Phase 4.1 — hand out real `ItemId`-backed userdata; Phase 8.2 |
| `onSpawn` callback swaps equipment, combat stats go stale | Phase 4.2 — recompute after the callback |
| A *later* script mutates a living monster's equipment; stats stay at spawn values | Phase 4.1 — scope the handle to the spawn hook (option a). `recompute_monster_combat_from_equipment` is `pub(crate)` and runs once |
| Allowlist bypassed by editing `data/` | Manifest is a Rust `const`, enforced by test 8.5 |
| Upstream TFS re-import quietly reintroduces a generator | Allowlist fails closed; `data/scripts/README.md` states the fork (Phase 6.1) |
| Pack authored assuming hot reload that does not exist | Phase 1.3 — decide re-runnable vs single-shot before Phase 2 fixes the registry shape |
| `db` binding turns the pack into an SQL surface | Phase 3.3 option (b); revisit only with explicit approval |
| Silent no-op hooks accumulate | Phase 5 — boot warning when a registration has no Rust call site |
| `movements.xml` derivation silently drops an item id (field stops burning, ring stops applying) | Phase 6.1 — golden diff against the XML parser output before deletion; test 8b keeps it |
| XML script trees grow back | Phase 6.4 — CI check for new `*.xml` outside the content-data allowlist |
| "No XML" scope-creeps into `items.xml` / `vocations.xml` and stalls the phase | Target-layout section names the content-data files as explicitly out of scope; they belong to `DATA_FORMAT_MIGRATION.md` |

---

## Engine files touched

- `crates/tfs-rust-lua/src/runtime.rs` — `isScriptsInterface`, guard, bindings
- `crates/tfs-rust-lua/src/scripts_interface.rs` — `load_scripts_interface`, allowlist manifest
- `crates/tfs-rust-lua/src/actions.rs` — `load_data_lib` idempotent (skip second call)
- `crates/tfs-rust-lua/src/script_loader.rs` — delete `load_creaturescripts`, `load_player_events`, both XML structs; module may disappear
- `crates/tfs-rust-lua/src/move_events.rs` — delete `load_from_xml`; bindings derived from item data
- `crates/tfs-rust-core/src/magic_field.rs` + item data — source of the derived field binding set (Phase 6.1)
- `crates/tfs-rust-lua/src/class_registry.rs`, `userdata/` — Monster surface, player bindings
- `crates/tfs-rust-lua/src/lua_event_dispatcher.rs` — `on_death`, creature events, `on_monster_spawned`
- `crates/tfs-rust-core/src/event_dispatcher.rs` — new `on_monster_spawned` trait method
- `crates/tfs-rust-core/src/spawn_lifecycle.rs` — dispatch after the roll
- `crates/tfs-rust-core/src/death.rs` — `on_death` ordering
