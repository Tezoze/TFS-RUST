# Data-pack Lua — 772 corpus policy

**Status:** Phase 1 shipped 2026-08-23 (allowlisted scripts-interface scan). Native spawn-roll and native player `LoseInventory` already exist. Most hooks below are **not** dispatched yet (Phase 2+).  
**Execution plan:** [tasks/data-pack-lua-implementation-plan.md](../tasks/data-pack-lua-implementation-plan.md).  
**Corpus:** 772 behaviour for **every** `clientVersion`. One pack, one timing. No 1098 death-time loot roll, no stamina-empty corpses, no `data/1098/` tree.  
**Companions:** [DATA_FORMAT_MIGRATION.md](DATA_FORMAT_MIGRATION.md), [tasks/monsters-lua-plan.md](../tasks/monsters-lua-plan.md).

This is the policy for **all** of these trees (not only loot/death):

| Tree | Role |
|------|------|
| `data/lib/` | Lua SDK on engine userdata (keep thin; core outcomes stay Rust) |
| `data/scripts/lib/` | EventCallback bus, revscript constructors |
| `data/scripts/eventcallbacks/` | Extra event bodies (`onSpawn` rarity, report bug, …) |
| `data/scripts/creaturescripts/` | Canonical CreatureEvents (login, firstlogin, playerdeath) |
| `data/scripts/globalevents/` | Startup / save / record timers, if any survive as Lua |

**Target: one tree, zero script-registry XML.** Everything script-shaped lives under `data/scripts/**` and self-registers. The parallel `data/<kind>/{lib,scripts}/` trees and their index files — `creaturescripts.xml`, `events.xml`, `globalevents.xml`, `movements.xml`, and the four empty ~60-byte stubs (`actions`, `spells`, `talkactions`, `weapons`) — are **deleted**, not migrated one-for-one. The enable bits are replaced by registration itself: `EventCallbackData` *is* the enable set, `hasEventCallback(type)` is the query. Sequencing and the one real migration (`movements.xml`) are in the [implementation plan](../tasks/data-pack-lua-implementation-plan.md) Phases 5–6.

Content-data XML (`data/items/items.xml`, `data/XML/*`, `raids/`, `world/*-spawn.xml`, `npc/archive/`) is a **different job** — see [DATA_FORMAT_MIGRATION.md](DATA_FORMAT_MIGRATION.md). Do not conflate the two.

**Not this document** (already have loaders; same Lua-vs-native rule): `data/scripts/actions`, `movements`, `spells`, `weapons`, `talkactions`, `data/npc`, `data/monster/*.lua` (Lua-as-**data**, not hooks). See monsters-lua-plan for monster defs.

---

## Hard rules

1. **Monster loot is generated once, at spawn.** Rust `roll_monster_spawn_loot`. Summons skip. Death only **moves** that inventory onto the corpse.
2. **Lua mutates at spawn**, after the roll, while items are on the living monster (bag / equip / body). Rarity, attributes, transform, extra items belong here.
3. **Death must not create monster loot.** No `createLootItem`, no `mType:getLoot()` roll, no `default_onDropLoot` generate branch, no second chance table.
   Read rule 1 precisely: the ban is on a **second roll against the monster definitions** and on **any** generation at death. `onSpawn` adding items is the rarity feature, not a violation — those are pack policy, decided after the native roll, visible on the living monster.
4. **Player gear drop is native 772** (`LoseInventory` SOME / ALL / AoL). Not `droploot.lua`.
5. **One script tree, no XML index.** Canonical revscripts: `data/scripts/**`, self-registering. The old `data/<kind>/{lib,scripts}/` trees and every script-registry `*.xml` are deleted. Never reintroduce an index file to say which scripts load — a Rust allowlist decides that, and registration decides what dispatches.

---

## Lua vs native (why `data/lib` exists)

TFS put a lot of **Player/Game helpers in Lua** so C++ only bound primitives (`getBankBalance`, `addItem`) and the pack composed `depositMoney`, `getPremiumDays`, `sendCancelMessage`. That is not “core mechanics in a scripting language.” It is a **script SDK**.

| Put in **Rust** | Put in **Lua** |
|-----------------|----------------|
| 772 outcomes: loot roll, corpse move, `LoseInventory`, exp/skill death, food, combat, walk | Pack policy: welcome text, first items, shop cost, rarity, GM talkactions |
| Anything per-tick or must not silently change if someone edits `data/` | Convenience that is only compositions of bound methods (`getPremiumDays` = math on timestamps) |
| Invariants (summons never drop, one loot roll) | Hooks (`onSpawn`, `onLogin`, `onDeath` message/DB) |

**Worth keeping as Lua**

- NPC bank/shop **orchestration** once the primitives exist (`depositMoney` is already native; `removeTotalMoney` should become native or a thin Lua on `removeItem` + `setBankBalance`).
- Talkactions / first login / premium buy — shards change these.
- Tiny predicates (`Tile.isTeleport`, `isInRange`) — not worth duplicating in Rust.

**Not worth Lua (hardcode / already native)**

- Death penalty, monster loot chance, player gear drop, look description, XP share.
- Duplicates of userdata: Lua `depositMoney`, Lua `sendCancelMessage`, Lua `Tile.isWalkable` when Rust already has them. Native wins; the Lua copy is dead weight — delete when cleaning, don’t maintain two.
- `createLootItem` / `getLossPercent` as engine paths.

**Where the line falls:** boot-time tables in Lua are fine — `data/formulas/772.lua`, monster defs, `data/lib` constants are read once by Rust and create no second code path. Per-event mutation of a core outcome is not. That is what separates the formulas pack from an `onGainExperience` hook, and it is why "mechanics live in `772.lua`" is not a precedent for "mechanics live in event hooks".

**Reload stance (Phase 1.3, 2026-08-23):** **(a) re-runnable from the start.** Each scripts-interface scan clears `EventCallback` data and replaces the pending CreatureEvent / GlobalEvent tables. There is still no `/reload` talkaction; Lua's day-to-day advantage is pack authorship and portability. When `/reload` lands, Phase 2's registry must be a name-keyed replaceable map (not an append-only buffer) so per-player `registerEvent` sets re-resolve by name.

**Point of Lua here:** edit the **data pack** without a rebuild, and keep TFS script call sites (`player:isPremium()`, `player:removeTotalMoney(cost)`). The point is **not** to implement the server in Lua. If a helper is load-bearing 772 behaviour, it belongs next to `death.rs` / `monster_inventory.rs`, with a Lua binding only if scripts must call it.

Gap list = **bind or native-implement the primitive**, then keep the Lua one-liner if scripts use that name. The gap is wider than the convenience helpers: the scripts we intend to **keep** also need `Player:registerEvent`, `sendOutfitWindow`, `getLastLoginSaved`, `getLastLogout`, `setVocation`, `getOutfit` / `setOutfit`, `setDirection`, `getSkull`, `getGuild`, `Vocation:getPromotion` / `getDemotion`, and a `db` global. Full triage: implementation plan Phase 3.

---

## Monster loot sequence

```
spawn
  → native roll from data/monster/*.lua loot blocks
  → Lua Monster:onSpawn (or EventCallback onSpawn) — mutate inventory
death
  → native drop_monster_corpse (move inventory, splash, decay)
  → optional Lua: loot *message* from corpse contents only (no items created)
  → optional creaturescript onDeath / onKill (stats, quests) — still no loot roll
```

`events.xml` today has `onDropLoot` enabled and `onSpawn` disabled. Rather than inverting the bits, the file is **deleted**: register an `onSpawn` callback and give `onDropLoot` no Rust call site. Registration is the enable bit.

Shard rarity (new file, not the TFS default):

```lua
local ec = EventCallback
ec.onSpawn = function(monster, position, startup, artificial)
  -- walk monster inventory / bag; setAttribute / transform / addItem
  return true
end
ec:register()
```

Do **not** put rarity on `onDropLoot`.

---

## Two packs of the same scripts

| Tree | Style | Loaded today |
|------|--------|----------------|
| `data/creaturescripts/scripts/*.lua` | Classic `function onLogin` | Only if listed in XML — XML is **empty**, so **nothing** |
| `data/scripts/creaturescripts/*.lua` | Revscript `CreatureEvent("Name"):register()` | **Allowlisted scan** (Phase 1). `droploot.lua` / `regeneratestamina.lua` skipped. Handlers still do not fire (Phase 2). |

Rust `load_creaturescripts` only registers **login/logout** from XML anyway; death is skipped. Login/logout on `LuaEventDispatcher` are wired but have no XML entries, so they do not run either.

**Canonical going forward:** load `data/scripts/creaturescripts/*.lua` (and `data/scripts/eventcallbacks/**` under the scripts interface). **Delete `data/creaturescripts/` outright** — index, `lib/`, and all eight scripts. Leaving it unused is not good enough: two files named `login.lua` with different contents is a trap that costs someone an afternoon.

---

## Verdicts — creaturescripts

Legend: **Lua** = keep and dispatch. **Native** = engine; do not also run the Lua. **Drop** = 1098/TFS-only or would fight spawn-loot; do not register.

### `data/scripts/creaturescripts/` (canonical)

| Script | TFS job | 772 corpus | Verdict |
|--------|---------|------------|---------|
| `login.lua` | Welcome, last visit, outfit window, promotion, premium loss, GM light, `registerEvent` Death+DropLoot, stamina table | Welcome / last visit / first-outfit window / promotion **yes**. Premium-expire teleport to Thais/Rook, premium-outfit strip, `FullLight`, `nextUseStaminaTime` are later-era. **Must not** `registerEvent("DropLoot")`. | **Lua**, slim to 772. Strip DropLoot register and stamina/premium-town bits (or leave as no-ops if APIs missing). |
| `logout.lua` | Clear `nextUseStaminaTime` | Harmless if stamina unused | **Lua** (tiny). Keep. |
| `firstlogin.lua` | First-login axe, torch, sex-dependent coat, backpack+food, default outfit, face south | Rook starter kit is pack content; 772-appropriate | **Lua**. This is the starter kit (prefer over XML `firstitems.lua`). |
| `playerdeath.lua` | “You are dead.”, death SQL, guild-war kills, blessing storage 101–105 | Death list + message are fine. Storages 101–105 are TFS blessing **items**, not 772. | **Lua**, keep message + DB. **Drop** the five `setStorageValue(101..105)` lines. |
| `droploot.lua` | Move player slots to corpse (AoL, skull, `getLossPercent` / `CLASSIC_PLAYER_LOOTDROP`) | Already native `player_death_drop_inventory` | **Drop.** Do not register. Native only. |
| `regeneratestamina.lua` | Offline stamina regen (8.x+ / TFS) | No 772 stamina loot/exp rules | **Drop.** Do not register. |
| `extendedopcode.lua` | OTClient language opcode | Optional OTC | **Lua** if OTC clients; no-op is fine. |
| `killstatistics.lua` | `onKill` / `onDeath` counters + shutdown SQL | Not 772 official; useful shard/website | **Lua optional.** Not loot. Register only if you want `/kill` stats. |

### `data/creaturescripts/scripts/` (XML duplicate — do not load)

| Script | vs revscript | Verdict |
|--------|----------------|---------|
| `login.lua` | Smaller; still registers DropLoot | Unused duplicate |
| `logout.lua` | Same idea | Unused duplicate |
| `firstitems.lua` | Simpler kit (torch, club, coat, bag) vs slotted `firstlogin.lua` | Unused; **one** starter kit lives in revscript `firstlogin.lua` |
| `playerdeath.lua` | No blessing-storage reset | Unused duplicate |
| `droploot.lua` | AoL + red/black + `getLossPercent` + refill bag | Unused; native player drop |
| `offlinetraining.lua` | 10.x offline training | **Drop** even if XML were loaded — not 772 |
| `regeneratestamina.lua` | Same as revscript | **Drop** |
| `extendedopcode.lua` | Same | Unused duplicate |

`login.lua` (XML) does **not** register firstitems; TFS XML `creaturescripts.xml` normally listed FirstItems + Login + DropLoot separately. Empty XML means none of this ran on TFS either unless revscripts loaded.

---

## How Lua is supposed to run (target)

TFS has three boot stages. We keep the first two and **drop the third** — the `events.xml` + `data/events/scripts/` stage exists only to map method names to bodies, which the revscript bus already does.

```
boot  load_data_lib
        data/lib/core/**          — helpers on Item/Container/Player (already loaded)
        data/scripts/lib/**       — EventCallback table, revscript ctors (already loaded)
        data/scripts/*.lua        — tools helpers (already loaded)

boot  scripts interface (isScriptsInterface = true)   — allowlisted scan (Phase 1)
        data/scripts/eventcallbacks/**   — only `default_onReportBug.lua` today; `rarity.lua` when authored
        data/scripts/creaturescripts/**  — allowlisted CreatureEvents (login, firstlogin, …); not dispatched yet (Phase 2)
        data/scripts/globalevents/**     — not scanned (Phase 7)

      (no third stage: events.xml and data/events/ are deleted.
       Today player.lua is exec'd, creature/monster/party.lua are not loaded,
       and the loader registers only Player + onInventoryUpdate — a method
       absent from events.xml — so *no* Player event body dispatches at all,
       including the eleven enabled="1" entries. That dead stage is the thing
       being removed, not fixed.)

sim   spawn monster
        native roll_monster_spawn_loot
        Monster:onSpawn → EventCallback onSpawn   -- mutate inventory

sim   death
        native corpse move / player LoseInventory
        creaturescript onDeath (playerdeath, optional kill stats)
        -- no Monster:onDropLoot item create
```

`EventCallback:register` is gated on `isScriptsInterface()` (`data/scripts/lib/event_callbacks.lua:81,102`). Rust creates that global at VM init (`runtime.rs`) as a `Cell<bool>` read; `load_scripts_interface` sets it true for the allowlisted scan and a `ScriptsInterfaceGuard` resets it on drop. Outside the pass, `ec:register()` is a no-op, not a nil call. `CreatureEvent:register` is **not** gated — the allowlist is the only control for which creaturescripts execute.

`event_callbacks.lua` itself loads fine today: the two gated call sites only run on `register()` / callback assignment, and the file's tail calls `EventCallback:clear()`, which populates the per-type tables so `hasEventCallback` returns `false` rather than erroring.

Do **not** load `data/globalevents/scripts/startup.lua` / `serversave.lua` as the source of truth for DB cleanup and save — those are engine. Optional: `record.lua` after `Game.broadcastMessage` exists.

Rust stubs `hasEventCallback` / `EventCallback` to always-false in `runtime.rs` **before** lib load. `event_callbacks.lua` then **replaces** those globals. `isScriptsInterface` is **not** replaced by the lib file. Dispatch still never calls `Monster:onSpawn` (Phase 4).

---

## Verdicts — `data/scripts/lib/` (loaded today)

| File | Job | 772 corpus | Verdict |
|------|-----|------------|---------|
| `event_callbacks.lua` | IDs, `EventCallback` metatable, `__call` chain | **Need.** This is how `onSpawn` rarity registers | **Keep.** Do not stub over it after load. `onDropLoot` stays in the enum so old files do not error; we simply never register a generator |
| `helper_constructors.lua` | Table-ctor for Action / Spell / CreatureEvent / … | Need for revscripts | **Keep** |
| `defaults_move_event.lua` | Default step/equip always-true | Movements loader | **Keep** |
| `create_functions.lua` | `getX`/`setX` aliases on MonsterType / Spell | Used at load; not our monster **data** path | **Keep** (harmless) |
| `register_monster_type.lua` | TFS `mType:register(mask)` + `Loot()` / `addLoot` | **Wrong tool** — defs are `data/monster/*.lua` serde, not `createMonsterType` | **Keep as dead stub.** Do not author monsters through it. Do not call `registerMonsterType.loot` |

---

## Verdicts — `data/scripts/eventcallbacks/` (allowlisted scan; Phase 1)

These only run after a scripts-interface scan **and** Rust calling the matching `Player:` / `Monster:` method.

| File | Job | 772 corpus | Verdict |
|------|-----|------------|---------|
| `monster/default_onDropLoot.lua` | Death `createLootItem` + stamina skip + “Loot of …” | Forbidden generator | **Do not load** (or delete generate + stamina; message-only only if we explicitly want announce). Rarity is **not** this file |
| *(new)* `monster/rarity.lua` or `onSpawn` callback | Mutate spawn inventory | **The** loot Lua | **Add** when we want rarity. `ec.onSpawn = function(monster, pos, startup, artificial)` |
| `player/default_onLook.lua` | `"You see " .. getDescription` + GM ids | Native look already matches 772 wire | **Do not dispatch until** look is a single path. Prefer **native**; do not also run this (double “You see”). Load later only if Lua becomes the only look builder |
| `player/default_onLookInBattleList.lua` | Same for battle list | Native creature look | **Same as onLook** — native wins |
| `player/default_onMoveItem.lua` | No-op (`return true`) | Dead | **Do not load** |
| `player/default_onReportBug.lua` | Write `bugs/<name> report.txt` | Fine as GM tool | **Lua** when `Player:onReportBug` is dispatched |

Dispatch decisions, formerly `events.xml` bits: table immediately below.

---

Dispatch sites, replacing the old `events.xml` enable bits. "Rust call site" is the only switch: no call site means a registered callback is inert, and that is a **boot warning**.

| Method | xml said | Rust call site | Why |
|--------|---------|--------|-----|
| `Monster:onSpawn` | 0 | **yes** | Loot mutate after native roll |
| `Monster:onDropLoot` | 1 | **no** | Never generate loot on death |
| `Party:onShareExperience` | 1 | **no** | Native 772 XP |
| `Player:onLook` / battle / trade | 1 | **no** | Native look |
| `Player:onGainExperience` / `onGainSkillTries` | 1 | **no** | Native XP / rates |
| `Player:onMoveItem` / `onItemMoved` | 1 | **yes** | aid 1000–2000 / candelabrum / trap 2579 — real pack policy, rehomed out of `events/scripts/player.lua` |
| `Player:onReportBug` | 1 | **yes** | `default_onReportBug.lua` |
| Creature / other Player / Party | 0 | no | Until a shard needs them |

The `xml said` column is kept only to show what is being discarded. None of it ran: the loader hardcoded `Player` + `onInventoryUpdate`, a method absent from `events.xml`, so every enabled bit was inert. That gap is the argument for deleting the file rather than fixing it — two sources of truth where one silently won.

---

## Verdicts — `data/globalevents/`

**Loaded today:** no. `EventDispatcher::on_startup` / `on_shutdown` are no-ops. `GlobalEvent` ctor exists (Gap 7c) but XML `globalevents.xml` is not scanned. Kill-statistics’ `GlobalEvent("KillStatistics_Flush")` would only register if that creaturescript file is loaded under scripts interface.

| File | Job | Need | Works today | Verdict |
|------|-----|------|-------------|---------|
| `globalevents.xml` | startup, record, ServerSave 04:30 | Index only | Not loaded | **Delete.** Replace with Rust timers; anything that stays Lua becomes a self-registering revscript under `data/scripts/globalevents/` |
| `lib/globalevents.lua` | empty | No | n/a | Ignore |
| `scripts/startup.lua` | Truncate `players_online`, expire bans/wars, house auctions, insert `towns` | Some of this is **ops** | **No** (`db`, `Game.getTowns`, `House`, `doRemoveOfflinePlayerMoney`) | **Prefer Rust** (sqlx / house module). Not 772-critical Lua. House auction / `HOUSES_BANKSYSTEM` is later-era |
| `scripts/serversave.lua` | Warn, `GAME_STATE_CLOSED`, shutdown or `saveServer()` at 04:30 | Shard ops | **No** (`Game.broadcastMessage`, `setGameState`, `saveServer`) | **Rust or config** for save/restart. Lua only if you want editable warn text without rebuild — still needs those Game bindings |
| `scripts/record.lua` | “New record: N players” | Optional flavor | Needs `Game.broadcastMessage` | **Lua optional** once broadcast exists |

772 did not ship this XML pack. Treat save/startup as **engine**, record as **pack**. Registration is by `GlobalEvent(name)` constructor call, not by file location — `killstatistics.lua` already declares one from the creaturescripts tree.

---

## Verdicts — `data/events/scripts/` (engine event bodies)

**The whole tree is deleted.** `creature.lua` / `monster.lua` / `party.lua` are pure `EventCallback(...)` pass-throughs that add nothing once Rust calls the bus directly. `player.lua` is the exception and must be audited line by line first — it is the only file in the XML trees carrying behaviour that is neither native nor forbidden. Its keepers (`onMoveItem` aid 1000–2000, `onItemMoved` candelabrum + trap 2579) move to `data/scripts/eventcallbacks/player/moveitem.lua`; its GM tool block folds into `data/scripts/actions/tools/*.lua`. Verdicts below describe what each body *did*, to justify where it lands.

| File | Job | Verdict |
|------|-----|---------|
| `monster.lua` `onSpawn` | Forwards EventCallback; `false` can cancel spawn | **Call from Rust after loot roll.** Return value: allow spawn (TFS). Mutation happens inside callbacks |
| `monster.lua` `onDropLoot` | Forwards EventCallback | **Do not call** (or call only a message callback we control). Never a roller |
| `player.lua` `onLook` / battle / trade | Forwards + a few hardcoded aid checks | Native look; **do not double**. `onMoveItem` aid 1000–2000 / candelabrum **is** pack policy — wire **that** method or port those rules native; the EventCallback no-op `default_onMoveItem` is useless |
| `player.lua` `onGainExperience` | Stages, stamina 1.5/0.5, soul condition | **Do not call** — XP is native `death.rs` / 772 soul. Stamina multiplier is not 772 |
| `player.lua` `onGainSkillTries` | `RATE_MAGIC` / `RATE_SKILL` | Native / config rates; **do not double** |
| `party.lua` `onShareExperience` | +20% vocation mix | **Do not call** — native 772 share |
| `creature.lua` | outfit / area / target / hear | xml already 0; leave off until needed |

---

## Verdicts — `data/lib/`

TFS `global.lua` does `dofile('data/lib/lib.lua')` → core + compat + debugging. **This server does not:** `load_data_lib` scans `data/lib/core/**` only (skips `core.lua` / `lib.lua`). `compat/` and `debugging/` are **not** loaded.

Status vs **current** Rust Lua surface (`lua-defs/engine.d.lua`). Native userdata methods beat Lua class-table methods (`__index` only on miss).

Legend: **Need** = 772 pack should keep it. **Works** = would run today if a script called it. **Gap** = file is wanted but missing bindings.

### Dispatchers (Rust does not execute)

| File | Need | Works | Notes |
|------|------|-------|-------|
| `lib.lua` / `core/core.lua` | No | n/a | Scan replaces dofile chain. Leave on disk. |

### `core/` — loaded today

| File | Need | Works today | Notes |
|------|------|-------------|-------|
| `constants.lua` | **Yes** | **Yes** | `CONTAINER_POSITION`, damage-list ids. Tiny. |
| `storages.lua` | **Yes** (promotion key) | **Yes** | Pure table. Achievement ranges unused until we want 8.6+ achievements. |
| `actionids.lua` | **Yes** | **Yes** | Merges onto TVP `actionIds`. |
| `create_functions.lua` | Yes (Spell/MonsterType aliases) | **Yes** | Same name as `scripts/lib/create_functions.lua`, not the same file (32 lines vs 2). |
| `position.lua` | **Yes** | **Mostly** | Native already has `getNextPosition` / `moveUpstairs` / `+`. Lua `isInRange` works (Position add exists). |
| `teleport.lua` | Yes | **Yes** | `isTeleport` predicate only. |
| `vocation.lua` | **Yes** (login / `getBase`) | **No** | `Vocation:getId` only. **`getDemotion` / `getPromotion` not bound** — `getBase` and login promotion Lua will fail. |
| `tile.lua` | **Yes** | **Mostly** | Native `isWalkable`, `getThing`, `getItems`, `hasProperty`. Lua `relocateTo` should work (moveTo / teleportTo / fluid / isMovable exist). Type predicates are Lua. |
| `itemtype.lua` | Yes | **No** | `usesSlot` needs **`ItemType:getSlotPosition`** — not bound. |
| `item.lua` | Helpers yes; Lua look **no** | **Helpers yes; desc no** | Predicates + `getType` work. `getDescription` needs `getAbilities`, `getWeaponType`, Spell rune API, `getCombatName`, … Keep **`LUA_ITEM_DESC` off**; native look stays. |
| `container.lua` | `isContainer` yes; `createLootItem` **no** | `isContainer` yes | **Stub `createLootItem`.** `getEmptySlots` / `addItemEx` / `Game.createItem` exist if someone called it. |
| `party.lua` | Optional (loot message) | **No** | `Party` userdata has **no** `getLeader` / `getMembers`. |
| `combat.lua` | Yes if spell scripts call it | **Partial** | `Combat:execute` / `setCallback` exist. `Condition:addDamage` used elsewhere, not here. |
| `game.lua` | **Yes** (map/tools) | **Partial** | Tile helpers (`setMapItemActionId`, remove/transform/effect) **work**. Pure Lua: reverse direction, skill-from-weapon, RAM `Game.getStorageValue`. **`Game.getPlayers` / `getReturnMessage` missing** → `broadcastMessage` broken. Native `sendCancelMessage` already maps return values. |
| `creature.lua` | **Yes** (illusion, closest tile) | **Partial** | `getClosestFreePosition` works for usual calls (`getPathTo` only if `mustBeReachable`). `setMonsterOutfit` / `setItemOutfit` likely work (Condition + MonsterType outfit). **`addSummon` Lua path broken** (`setTarget` / `setDropLoot` / `setMaster` missing); native `Creature:addSummon` exists. **`addDamageCondition` broken** (`isImmune`, `Condition:addDamage` missing). `canAccessPz` works. |
| `achievements.lua` | **No** (not 772) | Loads | Do not hook. Optional drop from scan. |
| `player.lua` | **Yes** (see below) | **Mixed** | Script library, not death. |

### `data/lib/core/player.lua` — need vs works

Native already: `hasFlag`, `isPremium`, `sendCancelMessage`, `feed`, `getBankBalance`, `getMoney`, `depositMoney`, `withdrawMoney`, `addSkillTries`, `addManaSpent`, `getDepotLocker`, `addItem` / `removeItem`, `getFreeCapacity`, `getSlotItem`, `setPremiumEndsAt`. Lua only runs when those names are **not** on userdata.

| Function | Need for 772 pack | Works today | Why |
|----------|-------------------|-------------|-----|
| `hasFlag` | Yes | **Yes** | Native. |
| `sendCancelMessage` | Yes | **Yes** | Native (maps `RETURNVALUE_*`). Lua wrap unused. |
| `isPremium` + premium time/days | Yes (`buypremium`, `changesex`, NPC shops) | **Partial** | Native `isPremium` / `getPremiumEndsAt` / `setPremiumEndsAt`. Lua `getPremiumDays` **works** (math on those). |
| `getClosestFreePosition` | Yes (god teleport) | **Yes** | Lua; Tile APIs exist. |
| `getDepotItems` | Rare | **Yes** | `getDepotLocker` + `getItemHoldingCount`. |
| `depositMoney` / `withdrawMoney` | Yes (bank NPC) | **Yes** | **Native** wins over Lua redefinitions. |
| `removeTotalMoney` | Yes (NPC shops) | **Gap** | Needs `removeMoney` + `setBankBalance` (not bound). Native deposit/withdraw do not replace “inventory then bank”. |
| `canCarryMoney` / `transferMoneyTo` | Bank NPC (withdraw check / transfer) | **Gap** | `transferMoneyTo` needs `Player(guid)`, `db.query`, `setBankBalance`. No `db` global. |
| `addSkillTries` wrap | Yes (fishing) | **Yes** | Wraps native; `APPLY_SKILL_MULTIPLIER` is Lua state. |
| `addLevel` / `addSkill` / `addMagicLevel` | Yes (god `add_skill`) | **Gap** | Need `Game.getExperienceForLevel`, `addExperience` / `removeExperience`, `getSkillLevel`, `getRequiredSkillTries`, `getBaseMagicLevel`, `removeManaSpent`, … |
| `getWeaponType` | Optional | **Gap** | `ItemType:getWeaponType` not bound. |
| `getLossPercent` | **No** | Would need `hasBlessing` | Item-drop table. Engine uses native `LoseInventory`. |
| `isUsingOtClient` / `sendExtendedOpcode` | Only if OTC | **Gap** | `getClient`, `NetworkMessage` not bound. |

**Use now:** flags, cancel, premium days, closest tile, depot count, native bank deposit/withdraw, fishing tries wrap.

**Need later (bindings):** shop `removeTotalMoney`, bank transfer, god add-skill, OTC opcodes.

**Do not drive:** death/loot/food/XP.

### Not loaded today

| File | Need | Works | Notes |
|------|------|-------|-------|
| `compat/compat.lua` | **No** | Would paper over old `doPlayer*` | Do not load. |
| `debugging/dump.lua` / `lua_version.lua` | No | n/a | **Not on disk.** `lib.lua` still dofiles them, but `lib.lua` is skipped by the scan, so the dangling reference is inert. Clean up the dofile. |

### `data/global.lua` (adjacent)

Not fully dofiled. `getLootRandom` — **don't need**. `rateLoot` in Rust spawn roll. TVP `actionIds` 4000–4005 stay.

---

## Player death (not monster loot)

Native 772 for **all** versions:

- AoL 2173 on exact lethal → keep inventory, consume amulet
- Red skull → drop all
- Else SOME: containers always + 10% per other slot
- Corpse 3128

Then Lua `playerdeath` for text + `player_deaths` row. Never Lua `droploot`.

---

## What to dispatch (target)

| Hook | When | Who |
|------|------|-----|
| `onLogin` | Player login | Slim `login.lua` + `firstlogin.lua` + optional kill-stats |
| `onLogout` | Logout | `logout.lua` |
| `Monster:onSpawn` | After native loot roll | EventCallback **onSpawn** (rarity). `events/scripts/monster.lua` |
| `onDeath` | After native corpse + player drop | `playerdeath.lua`; optional kill-stats |
| `onKill` | Killer credited | Optional kill-stats only |
| `Player:onReportBug` | Client bug report | `default_onReportBug.lua` |
| `Player:onLook` / battle list | Look | **Native** — do not also run `default_onLook*` |
| `Player:onGainExperience` / skill / `Party:onShareExperience` | XP | **Native** — do not call |
| `Monster:onDropLoot` | — | **Do not call** |

`EventDispatcher::on_death` is a no-op. `on_monster_spawn` is a bool allow-gate, not the Lua mutate hook.

---

## Current vs target

| Piece | Now | Target |
|-------|-----|--------|
| Monster loot roll | Spawn, Rust | Unchanged; `rateLoot` in **Rust** |
| Lua mutate | None | **onSpawn** after roll |
| Death loot generate | File exists, not called | **Never**; do not load `default_onDropLoot.lua` |
| `createLootItem` / `getLootRandom` | In lib | Remove or stub |
| `event_callbacks.lua` | Loaded; Rust stubs overwritten | Keep; register under scripts interface |
| `eventcallbacks/**` | Not loaded | Load **except** drop-loot, no-op move, look (until look is Lua-only) |
| `register_monster_type.lua` | Loaded stub | Stay unused for defs |
| Creaturescripts | Two trees; XML empty; revscripts not loaded | **One** tree: load `data/scripts/creaturescripts/`, delete `data/creaturescripts/`. No DropLoot / stamina / offline-training |
| Globalevents | Not loaded | Startup/save → **Rust**; survivors are revscripts under `data/scripts/globalevents/`; delete the XML |
| `onDropLoot` / `onSpawn` enable bits | `events.xml` 1 / 0 | **No `events.xml`** — registration is the enable bit, Rust call site is the switch |
| `movements.xml` | Parsed for field + equip bindings | **Deleted**; bindings derived from item data |
| Script-registry XML overall | 8 index files, 8 parallel trees | **Zero.** One `data/scripts/**` tree |
| Player drop | Native 772 | Unchanged |
| Look / XP EventCallbacks | xml on; native already | Native; do not double |

---

## Implementation sketch

1. `isScriptsInterface(true)` then scan `data/scripts/eventcallbacks/**` and `data/scripts/creaturescripts/*.lua` against a Rust `const` **allowlist** — a manifest of files that may load, not a skip list. A blocklist fails open on the next file imported from upstream TFS; an allowlist fails closed. Keeping it in Rust means editing `data/` cannot re-enable a death-time generator.
2. After `roll_monster_spawn_loot`, call `Monster:onSpawn` with mutation scope. **This needs new bindings first:** `Monster` is a bare table (`class_registry.rs`) with no userdata, live creatures are `CreatureRef` with no inventory accessor, and `get_player_inventory_item` returns `None` for non-players. Expose `MonsterInventory` (`bag` / `equipment` / `body`) as the *same* `ItemId`-backed `Item` / `Container` userdata the rest of the pack uses — a snapshot-and-apply layer would silently drop mutations and reproduce the corpse-only-rarity anti-pattern. Recompute monster combat stats after the callback in case equipment changed, and **scope the handle to the spawn hook**: `recompute_monster_combat_from_equipment` is `pub(crate)` and runs exactly once, so a handle that outlives `onSpawn` lets a later script strip a weapon off a living monster while its combat stats stay frozen at spawn.
3. Do not call `Monster:onDropLoot`. Do not call `Player:onGainExperience` / party share.
4. Stub or delete `Container.createLootItem` in `data/lib/core/container.lua`. Slim `login.lua` (no DropLoot). Do not start loading `compat.lua`. Keep `LUA_ITEM_DESC` off.
5. Delete `data/events/` — after rehoming the `onMoveItem` / `onItemMoved` rules from `player.lua` into a revscript eventcallback and its GM tool block into `data/scripts/actions/tools/`. Guard each Rust dispatch site with `hasEventCallback`; warn at boot when a registration has no call site.
6. Derive the `movements.xml` field and equip bindings from item data, golden-diff the derived set against the XML parser output, then delete the file and `MoveEvents::load_from_xml`.
7. Delete the eight legacy trees and their index files. CI-check that no new `*.xml` appears under `data/` outside the content-data allowlist.
8. Tests: spawn → mutated attrs on body and on corpse; death adds no extra rolled ids; loading default_onDropLoot must not be required for loot to exist.
9. Globalevents: do not wire `startup.lua` / `serversave.lua` until the same jobs exist in Rust; optional `record.lua` after broadcast, as a revscript.

---

## Pattern check: native core + Lua mutates after

This is **only** valid when Rust already produced the result and Lua **edits that result**. It is **not** valid when Lua would roll, recompute, or replace the core.

Columns: **Core** = Rust. **Mutate later** = a hook that may change the core output. **Stock TFS Lua** = the file we have — often a *second generator*, so **no**.

### Engine events (`data/events/scripts/`)

| Function | Native core? | Lua mutate later? | Stock TFS Lua |
|----------|--------------|-------------------|---------------|
| `Monster:onSpawn` | Spawn + **loot roll** | **Yes** — walk inventory, rarity, extra items | Forwarder only; enable. Add shard `ec.onSpawn`, not TFS drop-loot |
| `Monster:onDropLoot` | Corpse = spawn inventory (move) | Message only, optional | **`default_onDropLoot` generates — do not use** |
| `Player:onLook` / `onLookInBattleList` / `onLookInTrade` | Native look string | Only if hook **appends** to that string (GM ids) | **`default_onLook*` rebuilds “You see” — do not run** (double) |
| `Player:onUseItem` | Actions/weapons native + action scripts | Action scripts already are the pack | EventCallback unused; `events/scripts/player.lua` GM tool block is pack |
| `Player:onMoveItem` | Cylinder move native | **Yes** — cancel/transform (aid 1000–2000, candelabrum) | Use **`events/scripts/player.lua` body**, not empty `default_onMoveItem` |
| `Player:onItemMoved` | Native post-move | **Yes** — trap 2579 transform in that file | Same |
| `Player:onMoveCreature` | Native walk | Optional cancel | xml 0; forwarder only |
| `Player:onReportBug` | None (no native bug log) | n/a — **Lua is the feature** | Keep `default_onReportBug` |
| `Player:onTurn` / trade trio | Native trade | Optional veto/extra | xml 0; forwarder only |
| `Player:onGainExperience` | Native 772 XP | Only a hook that **multiplies after** native | **Stock applies stages + stamina — do not call** |
| `Player:onLoseExperience` | Native death exp loss | Same idea, unused | xml 0 |
| `Player:onGainSkillTries` | Native / config rates | Only multiply **after** native | **Stock RATE_* — do not double** |
| `Party:onShareExperience` | Native 772 share | Shard bonus **after** native only | **Stock +20% voc mix — do not call** |
| `Party:onJoin/Leave/Disband` | Native party | Optional veto | xml 0 |
| `Creature:onChangeOutfit` / area / target / hear | Native combat/outfit | Optional veto | xml 0 |

### Creaturescripts (`data/scripts/creaturescripts/` — XML copies: ignore)

| Function | Native core? | Lua mutate later? | Stock file |
|----------|--------------|-------------------|------------|
| `onLogin` (login.lua) | Login, load player | **Yes** — welcome, outfit window, promotion (needs `getPromotion` bind) | Slim; strip DropLoot / stamina / premium-town |
| `onLogin` (firstlogin.lua) | None for starter kit | n/a — **Lua is the kit** | Keep |
| `onLogout` | Native logout | Tiny cleanup | Keep |
| `onDeath` (playerdeath.lua) | Native death + corpse + exp | **Yes** — message, DB row | Keep; strip blessing 101–105 |
| `onDeath` (droploot.lua) | Native `LoseInventory` | Must **not** also drop gear | **Drop file** |
| `onLogin` (regeneratestamina) | No 772 stamina core we want | — | **Drop** |
| `onExtendedOpcode` | None | n/a — OTC pack | Optional |
| `onKill` / `onDeath` (killstatistics) | Native kill credit | **Yes** — counters | Optional |
| `onShutdown` (kill stats flush) | Native shutdown | Persist table | Optional; needs `db` |

XML `offlinetraining.lua` / `droploot.lua` / dupes: **do not load**.

### EventCallbacks (stock files)

| Function | Fits pattern? |
|----------|----------------|
| `ec.onSpawn` (new rarity) | **Yes** — mutate spawn loot |
| `ec.onDropLoot` (`default_onDropLoot`) | **No** — generator |
| `ec.onLook` / `onLookInBattleList` | **No** as written (rebuild). Yes only as post-append |
| `ec.onMoveItem` (default) | **No** — no-op; real rules are in `events/scripts/player.lua` |
| `ec.onReportBug` | Lua-only feature, not a mutate of native |

### Globalevents

| Function | Native core? | Lua mutate? |
|----------|--------------|-------------|
| `onStartup` | Prefer **Rust** sqlx / houses | Do not use stock `startup.lua` as core |
| `onTime` (serversave) | Prefer **Rust** save/restart | Lua only for warn text, after Game APIs exist |
| `onRecord` | Native player count | **Yes** — announce | `record.lua` |

### `data/lib` (not hooks — SDK)

These are **not** “core + mutate.” Either native primitive, Lua one-liner, or dead duplicate.

| Fits “native core, Lua mutates”? | Functions |
|----------------------------------|-----------|
| **No — already native, Lua copy unused** | `depositMoney`, `withdrawMoney`, `hasFlag`, `sendCancelMessage`, `isPremium` (base), `addSkillTries` (native; Lua only wraps multiplier flag), `Tile.isWalkable`, `Position:getNextPosition` / `moveUpstairs` |
| **No — must stay native, never Lua roller** | `createLootItem`, `getLossPercent` (item drop), `getLootRandom`, Lua `Item.getDescription` |
| **Lua convenience / pack calls (not mutate of sim)** | `getPremiumDays`, `getClosestFreePosition`, `getDepotItems`, `isInRange`, `relocateTo`, type predicates, `Game.setMapItemActionId` and other map helpers |
| **Want native primitive, then scripts call it** | `removeTotalMoney`, `transferMoneyTo`, `addSkill`/`addLevel`, `Vocation.getBase` (`getPromotion`), `Game.broadcastMessage`, `Game.getPlayers` |
| **Lua-only content we don’t want** | `achievements.lua` entire API |

**Bottom line:** the spawn-loot pattern (**Rust rolls → Lua may edit items**) is the model. Same shape is OK for look/XP **only** if the hook receives the native result and tweaks it. Stock TFS `onDropLoot` / `onLook` / `onGainExperience` / `droploot.lua` **do not** do that — they replace the core. Do not wire those as-is.

---

## Completeness

**In this document (policy decided):** loot timing; Lua vs native; one script tree with zero script-registry XML; `scripts/lib`; eventcallbacks; event bodies and where their content lands; `data/lib` need/works; globalevents; player.lua API; dispatch list; anti-patterns.

**Still implementation, not more design:** `isScriptsInterface` global + scoped loader; **Monster inventory Lua surface** (prerequisite for `Monster:onSpawn`, does not exist yet); `CreatureEvent` registry drain + `Player:registerEvent`; slim login; stub `createLootItem`; `hasEventCallback`-guarded dispatch sites replacing `events.xml`; `movements.xml` derivation + golden diff; bind gaps (`getPromotion`, `registerEvent`, `sendOutfitWindow`, `removeTotalMoney`, …) as **Rust primitives**. Phasing and dependency order: [implementation plan](../tasks/data-pack-lua-implementation-plan.md).

**Other loaders (same Lua-vs-native rule, not catalogued here):** actions, movements, spells, weapons, talkactions, NPCs, monster **defs**.

---

## Anti-patterns

- Death-time `createLootItem` / `mType:getLoot()` / loading `default_onDropLoot.lua` as-is.
- Loading eventcallbacks **without** `isScriptsInterface` — today that is a hard Lua error (nil call), not a silent skip.
- Running `default_onLook.lua` on top of native look.
- Calling `Player:onGainExperience` on top of native XP.
- Defining monsters via `register_monster_type.lua` / `Loot()`.
- Using `Player.getLossPercent` for exp/skill loss (that is `death.rs`).
- `LUA_ITEM_DESC = true` plus native look.
- dofile `lib.lua` (second path: compat + full TFS load).
- `MONSTERS_SPAWN_WITH_LOOT = false` “so TFS scripts work”. It is not a Rust config key at all — `configKeys` exposes only `FREE_PREMIUM`, and unknown boolean keys read `false`, so `default_onDropLoot.lua` would take its **generate** branch. There is no switch that turns the native spawn roll off.
- Registering XML **and** revscript login/death. (After the cleanup there is no XML path — but do not reintroduce one.)
- Adding a new `*.xml` index to say which scripts load. A Rust allowlist decides loading; registration decides dispatch. An index file is a third source of truth and will silently disagree with both.
- Translating `movements.xml` into a hand-maintained Lua id list instead of deriving the bindings from item data — same lossy copy, new syntax.
- Rarity only on the corpse so equipped loot looks vanilla until death.
- `droploot.lua` plus native player drop.
- Wiring TFS `startup.lua` / `serversave.lua` as the real save/DB path.

---

## Related engine files

- Spawn roll / corpse: `crates/tfs-rust-core/src/creature/monster_inventory.rs`
- Player corpse: `crates/tfs-rust-core/src/game_world_lifecycle.rs` (`player_death_drop_inventory`)
- XP / bless / `on_death` stub: `crates/tfs-rust-core/src/death.rs`
- Dispatch: `event_dispatcher.rs`, `lua_event_dispatcher.rs`
- Lib load (core + `scripts/lib`, **not** eventcallbacks): `crates/tfs-rust-lua/src/actions.rs` `load_data_lib`
- EventCallback stubs then overwritten by Lua: `crates/tfs-rust-lua/src/runtime.rs`
- Death skip in XML loader: `crates/tfs-rust-lua/src/script_loader.rs`
- Globalevents: not loaded; `on_startup` no-op in `event_dispatcher.rs`
