# Australis Rust Port — Audit & Next-Steps Plan

**Date:** 2026-04-05
**Scope:** Full audit of ported Rust crates vs TFS 1.4.2 C++ source, with prioritised roadmap.

---

## Part 1 — Audit: What Exists Today

### Architecture (Solid)

| Crate | Role | Status |
|---|---|---|
| `tfs-rust-common` | Shared types, enums, `Position`, `GamePacket`, opcodes | ✅ Complete |
| `tfs-rust-content` | OTB/OTBM loaders, items.xml, monsters.xml, vocations, mounts, outfits, spawns, groups | ✅ Complete |
| `tfs-rust-db` | SQLx (MariaDB) — accounts, players, houses, items, market | ✅ Queries ported, wiring partial |
| `tfs-rust-net` | TCP server, XTEA, RSA, packet framing, client→server parse, server→client encode | 🟡 Functional but many wire-format bugs (see GAPS.md) |
| `tfs-rust-core` | Game loop, `GameWorld`, walk, combat, creatures, spells, conditions, death, party, guild | 🟡 Foundation exists, most game logic is stub/skeleton |
| `tfs-rust-lua` | Lua scripting runtime | ❌ Empty scaffold |

### Subsystem Breakdown

#### ✅ Done & Working
- **Game loop** — Tokio `select!` with biased ordering, 50ms tick, walk-wake channel, `nextAction` gating.
- **Networking** — XTEA encrypt/decrypt, RSA handshake (raw modpow), Adler32, packet framing.
- **Login flow** — Full initial packet sequence (self-appear, map, stats, skills, light, VIP stub, fight modes).
- **Map loading** — OTBM tile/item parse, OTB item type database, quadtree spatial index.
- **Pathfinding** — A* with ground-speed weighting.
- **Line of sight** — Bresenham-based.
- **Player walking** — Complete for players: single step, diagonal, auto-walk, speed system, walk delay, cancel. Phases 1–2 of `walk-fix-todo.md` done (timing fixes, `forceUpdateFollowPath`, `onWalkComplete`, `cleanup()`).
- **Client→Server parsing** — All TFS 1.4.2 opcodes parsed into `GamePacket` enum (60+ variants).
- **Creature model** — `SlotMap<CreatureId, CreatureKind>` with `Player`/`Monster`/`Npc` variants. `CreatureBase` has health, speed, outfit, conditions, walk queue, damage map.

#### 🟡 Partial / Skeleton
- **Combat** — HP/mana delta application, condition apply/dispel, PvP zone/protection/skull checks. **Missing:** formulas (`getAttackDamage`, `getDefense`, `getArmor`), area effects, projectiles, hit/magic animations, melee/distance/magic dispatch.
- **Conditions** — Merge rules for 10 condition data variants. **Missing:** tick processing (periodic damage, regen, haste speed recalc, drunk walk offset), duration expiry, deferred add/remove during walk (C++ `creature.cpp` ~1278).
- **Spells** — Cast gating (level/mana/soul/vocation/cooldown). **Missing:** actual spell execution, area targeting, rune use, conjure, house spells.
- **Death** — XP distribution from damage map, corpse item scheduling. **Missing:** player death penalty (XP/skill loss, bless), loot generation, corpse items.
- **Party** — Create/join/leave/transfer, shared XP split formula. **Missing:** party buff, vocation bonus, shared loot, party channel.
- **Guild** — Registry + war tracker. **Missing:** DB load/save, rank management, guild channel.
- **Houses** — Owner/subowner/guest sets. **Missing:** door permissions, rent, kick teleport, house items, DB persistence.
- **Monster** — `think_tick` placeholder (idle/chase/flee/return enum). **Missing:** target selection, spell casting, flee logic, loot tables, summon/convince.
- **NPC** — Bare struct + event trait. **Missing:** dialogue system, keyword matching, shop/trade, focus/idle, walkback.
- **Spawns** — Manager exists, `tick()` called. **Missing:** actual creature instantiation from spawn definitions.
- **Decay** — Manager + schedule/cancel. **Missing:** wired to item transforms.
- **World light** — Struct exists. **Missing:** day/night cycle calculation.
- **Player save/load** — DB queries exist. **Missing:** wired to login (partial) / logout / periodic auto-save.

#### ❌ Not Started
- **Items (runtime)** — Only `Item { item_type: u16, count: u8 }`. No attribute map, no container tree, no equipment logic, no item actions/movements, no fluid handling, no readable/writable items, no doors, no beds, no teleport items.
- **Containers** — No open/close, no parent/child hierarchy, no item move between containers.
- **Inventory** — Stub (sends empty slots). No equip/unequip, no capacity checks, no slot validation.
- **Item movement (`Throw`)** — Parsed but handler is `trace!()` only.
- **Item use (`UseItem`/`UseItemEx`)** — Parsed but no handler.
- **Chat system** — Only `broadcast_creature_say_viewport`. No channel registry, no private messages, no channel open/close/invite/exclude, no NPC channel.
- **Trade** — Parsed but no handler (player-to-player or NPC shop).
- **Market** — DB queries exist, parsing done, no game logic.
- **Quests** — No implementation.
- **Outfits** — Content loaded, no `SetOutfit` handler.
- **VIP** — Always sends empty list; no online/offline notification.
- **Floor changes** — `MoveUpCreature` (0xBE) / `MoveDownCreature` (0xBF) not implemented; falls back to full `sendMapDescription`.
- **Lua scripting** — Empty crate. No `mlua` bindings, no event hooks, no script loading.
- **Graceful shutdown** — Stub (no player save, no house save).
- **GM commands / Talkactions** — None.
- **Skull timing** — Enum exists, no unjustified kill tracking or skull assignment.
- **Raids / Global events** — None.

### Known Protocol Bugs (from GAPS.md — still open)

| # | Bug | Severity | C++ Ref |
|---|---|---|---|
| 1 | `write_item_template` missing trailing `0x00` duration byte | 🔴 Critical | `networkmessage.cpp:114` |
| 2 | Fluid/splash items never encoded (missing `fluidMap[count & 7]`) | 🔴 Critical | `networkmessage.cpp:101` |
| 3 | `sendChangeSpeed` sends 1×u32 instead of 2×u16 (baseSpeed/2 + speed/2) | 🔴 Critical | `protocolgame.cpp:2505` |
| 4 | `sendChannelMessage` field order wrong | 🔴 Critical | `protocolgame.cpp:1730` |
| 5 | `send_creature_turn` completely wrong wire format (missing 0x63 sub-header) | 🔴 Critical | `protocolgame.cpp:2404` |
| 6 | OTClient/OTCv8 detection completely missing | 🟠 High | `protocolgame.cpp:171,469` |
| 7 | `sendCancelTarget` missing `u32(0)` | 🟠 High | — |
| 8 | `send_cancel_walk` hardcodes direction 0 instead of player's actual direction | 🟠 High | — |
| 9 | Known creature eviction doesn't check visibility | 🟡 Medium | `protocolgame.cpp:744` |
| 10 | `MoveUpCreature`/`MoveDownCreature` not implemented (full map reload used) | 🟡 Medium | `protocolgame.cpp` |
| 11 | Creature visibility (ghost/invisible) not filtered in map sends | 🟡 Medium | — |
| 12 | `send_update_tile_end` non-empty path missing `GetTileDescription` | 🟡 Medium | `protocolgame.cpp:2683` |

---

## Part 2 — Roadmap: Prioritised Next Steps

Strategy: Fix protocol correctness first (the world must *render* correctly), then build the item/container foundation (everything depends on items), then layer game systems bottom-up.

---

### Phase A — Protocol Correctness (Wire-format parity)
**Goal:** OTClient renders the world correctly — items, creatures, floor changes, chat, speed.
**C++ ref files:** `networkmessage.cpp`, `protocolgame.cpp`, `protocolgame.h`
**Estimated effort:** 1–2 days

- [ ] **A.1** Fix `write_item_template` — append `0x00` duration byte (`item_encode.rs`)
- [ ] **A.2** Add fluid/splash sub-type byte to `write_item_template` (`item_encode.rs`, needs `fluidMap` table)
- [ ] **A.3** Fix `sendChangeSpeed` — 2×u16 (baseSpeed/2, speed/2) not 1×u32 (`outgoing_extra.rs`)
- [ ] **A.4** Fix `sendChannelMessage` field order to match C++ (`outgoing_extra.rs`)
- [ ] **A.5** Rewrite `send_creature_turn` — full `0x6B` + position/ffff + `0x63` known-creature sub-header + direction + walkthrough (`outgoing_extra.rs`)
- [ ] **A.6** Fix `sendCancelTarget` — add `u32(0)` after opcode (`outgoing_extra.rs`)
- [ ] **A.7** Fix `send_cancel_walk` — pass and write actual player direction (`outgoing_extra.rs`)
- [ ] **A.8** Implement `MoveUpCreature` (0xBE) / `MoveDownCreature` (0xBF) for z-axis transitions (`map_description.rs` / new outgoing)
- [ ] **A.9** Parse and store OS / OTCv8 detection from first packet (`game_first_packet.rs`, `pending_login.rs`)
- [ ] **A.10** Fix known creature eviction — visibility-aware (`map_description.rs`)

---

### Phase B — Item Runtime & Containers
**Goal:** Items exist as rich objects; containers can be opened, items can be moved.
**C++ ref files:** `item.cpp`, `item.h`, `items.cpp`, `container.cpp`, `container.h`, `cylinder.cpp`
**Estimated effort:** 3–5 days

- [ ] **B.1** Expand `Item` struct — attribute map (`ItemAttrMap`), unique ID, `Cylinder` parent reference (tile/container/player slot). Mirror `item.h` fields.
- [ ] **B.2** Port `Container` — child items vec, capacity, parent container, open-by tracking. C++ `container.cpp`.
- [ ] **B.3** Implement `Cylinder` trait (Tile, Container, Player) — `addThing`, `removeThing`, `updateThing`, `queryAdd`, `queryRemove`. C++ `cylinder.h`.
- [ ] **B.4** Port `Game::internalMoveItem` / `internalAddItem` / `internalRemoveItem` chain (`game.cpp` ~1400–1700).
- [ ] **B.5** Handle `Throw` (item move) — validate source/dest, capacity, stackable merge/split.
- [ ] **B.6** Implement server→client: `sendAddTileItem` (0x6A), `sendUpdateTileItem` (0x6B), `sendRemoveTileItem` (0x6C), `sendContainer` (0x6E), `sendAddContainerItem` (0x70), `sendUpdateContainerItem` (0x71), `sendRemoveContainerItem` (0x72).
- [ ] **B.7** Handle `CloseContainer`, `UpArrowContainer`, `UpdateContainer`, `SeekInContainer` game packets.
- [ ] **B.8** Port fluid container / splash encoding for live items (not just templates).

---

### Phase C — Inventory & Equipment
**Goal:** Players can equip/unequip items, inventory renders in client, capacity works.
**C++ ref files:** `player.cpp` (slots), `player.h`, `game.cpp` (playerEquipItem)
**Estimated effort:** 2–3 days

- [ ] **C.1** Real inventory slot model on `Player` — 10 slots + store inbox, each holds an `ItemId` or container.
- [ ] **C.2** `sendInventoryItem` (0x78) / `sendInventoryItemRemove` (0x79) — real items not stubs.
- [ ] **C.3** Port `Player::onEquipItem` / `onDeEquipItem` — stat modifiers, condition apply (e.g. life ring regen).
- [ ] **C.4** Capacity system — `Player::getFreeCapacity`, weight calculation on move/equip.
- [ ] **C.5** `EquipObject` handler (quick-equip from hotbar).
- [ ] **C.6** Item description / `LookAt` handler — build description string, send `sendTextMessage`.

---

### Phase D — Monster & NPC Walking + AI Foundation
**Goal:** Monsters and NPCs walk, chase, flee, return to spawn. Creatures interact on the map.
**C++ ref files:** `creature.cpp`, `monster.cpp`, `npc.cpp`, `spawn.cpp`
**Estimated effort:** 3–4 days

- [x] **D.1** Extend walk system to `CreatureKind::Monster` and `CreatureKind::Npc` (Phase 3 items 8–10 from `walk-fix-todo.md`).
- [ ] **D.2** Port `Creature::onCreatureMove` — `localMapCache` shifting for pathfinding.
- [ ] **D.3** Port follow-creature walk update on target move (`creature.cpp` ~619–656).
- [ ] **D.4** Spawn system — instantiate monsters from `SpawnManager` definitions, respawn timers.
- [ ] **D.5** Monster AI `onThink` — target selection (nearest hostile in range), chase pathfinding, flee at low HP, return to spawn.
- [ ] **D.6** NPC idle walk, focus system (face speaker), walkback to spawn.
- [ ] **D.7** Port deferred condition add/remove during walk — haste/paralyze interaction (Phase 4 item 11 from `walk-fix-todo.md`).

---

### Phase E — Combat System
**Goal:** Melee, distance, and magic attacks work with correct formulas. Creatures can fight and die.
**C++ ref files:** `combat.cpp`, `combat.h`, `weapons.cpp`, `weapons.h`, `game.cpp` (combatChange*)
**Estimated effort:** 4–6 days

- [ ] **E.1** Port combat formulas — `Player::getAttackDamage`, `getDefense`, `getArmor`, critical hit chance. C++ `combat.cpp`, `player.cpp`.
- [ ] **E.2** Melee attack dispatch — `Game::playerAutoAttack` / `checkCreatureAttack` cycle with attack speed timer.
- [ ] **E.3** Distance attack — projectile validation (ammo, range, line of sight), `sendDistanceShoot`.
- [ ] **E.4** Magic damage — element types, resistance/absorption.
- [ ] **E.5** Area combat — `MatrixArea` application, multi-target hit.
- [ ] **E.6** Monster attack/spell — port `Monster::doAttacking`, monster spell list from XML.
- [ ] **E.7** Death penalty — player XP/skill loss, bless reduction, amulet of loss. C++ `Player::onDeath`.
- [ ] **E.8** Loot generation — roll monster loot table, create corpse container with items.
- [ ] **E.9** Skull system — unjustified kill tracking, white/yellow/red/black skull assignment + timing. C++ `player.cpp` skull methods.
- [ ] **E.10** Send combat packets — `sendMagicEffect` (0x83), `sendDistanceShoot` (0x85), `sendCreatureHealth` (0x8C), damage `sendTextMessage` (0xB4 with position).

---

### Phase F — Chat & Social
**Goal:** Chat channels, private messages, party channel, guild channel, NPC channel all work.
**C++ ref files:** `chat.cpp`, `chat.h`, `game.cpp` (playerSay, internalCreatureSay)
**Estimated effort:** 2–3 days

- [ ] **F.1** `ChatChannelManager` — register default channels (Local, World, Trade, Help, etc.), per-guild, per-party, private.
- [ ] **F.2** Handle `RequestChannels` / `OpenChannel` / `CloseChannel` / `CreatePrivateChannel` / `ChannelInvite` / `ChannelExclude`.
- [ ] **F.3** `Say` handler — route by `speak_class` to correct channel or viewport broadcast. Private messages.
- [ ] **F.4** Fix `sendCreatureSay` vs `sendToChannel` vs `sendChannelMessage` — use correct packet format per context (from GAPS.md #13).
- [ ] **F.5** NPC channel — open on NPC interact, close on walk away, route speech to NPC script.
- [ ] **F.6** VIP system — load from DB, `sendVIPEntries` with real data, online/offline notifications.

---

### Phase G — Conditions & Timed Effects
**Goal:** All condition types tick correctly — poison damage, regen, haste speed, drunk stumble, spell cooldowns expire.
**C++ ref files:** `condition.cpp`, `condition.h`
**Estimated effort:** 2–3 days

- [ ] **G.1** Condition tick system — periodic damage (poison/fire/energy ticks), regen (health/mana per tick).
- [ ] **G.2** Speed conditions — `ConditionSpeed` recalculate player speed on add/remove/tick.
- [ ] **G.3** Drunk condition — random walk direction offset.
- [ ] **G.4** Duration expiry — remove conditions when ticks reach 0.
- [ ] **G.5** Condition icons — `sendPlayerIcons` (0xA2) update on condition change.
- [ ] **G.6** Deferred condition add/remove during walk step (paralyze ↔ haste).

---

### Phase H — Spells & Runes
**Goal:** Players can cast spells and use runes with correct effects.
**C++ ref files:** `spells.cpp`, `spells.h`, `luascript.cpp` (spell Lua bindings)
**Estimated effort:** 3–4 days

- [ ] **H.1** Spell execution pipeline — deduct mana/soul, apply cooldowns, execute combat/condition/area effect.
- [ ] **H.2** Instant spells — healing, haste, light, invisible, find person, etc.
- [ ] **H.3** Rune spells — target validation, rune charge consumption, area/single-target.
- [ ] **H.4** Conjure spells — create rune/item, deduct soul/mana.
- [ ] **H.5** `sendSpellCooldown` / `sendSpellGroupCooldown` packets.

---

### Phase I — NPC Trade & Player Trade
**Goal:** Players can buy/sell from NPCs and trade with each other.
**C++ ref files:** `npc.cpp`, `game.cpp` (playerBuyItem, playerSellItem, playerRequestTrade)
**Estimated effort:** 2–3 days

- [ ] **I.1** NPC shop — `sendShop` (0x7A), `sendSaleItemList` (0x7B), handle `PlayerPurchase`/`PlayerSale`/`CloseShop`.
- [ ] **I.2** Player-to-player trade — `RequestTrade` / `LookInTrade` / `AcceptTrade` / `CloseTrade` handlers, trade window packets.
- [ ] **I.3** Gold handling — coin stacking (gold/platinum/crystal), pay-from-containers.

---

### Phase J — Lua Scripting Engine
**Goal:** `tfs-rust-lua` crate provides full script bindings; existing TFS Lua data scripts load and execute.
**C++ ref files:** `luascript.cpp`, `luascript.h`, `baseevents.cpp`, all `*event*.cpp`
**Estimated effort:** 5–8 days (largest single phase)

- [ ] **J.1** `mlua` runtime init — create Lua VM, register global functions, load `data/lib/` libraries.
- [ ] **J.2** Core Lua API — `Creature`, `Player`, `Monster`, `Npc`, `Item`, `Tile`, `Position`, `Game` metatables.
- [ ] **J.3** Event system — `CreatureEvent` (onLogin, onLogout, onDeath, onKill, onAdvance, onThink), `MoveEvent`, `Action`, `TalkAction`, `GlobalEvent`.
- [ ] **J.4** Script loading pipeline — scan `data/scripts/`, `data/creaturescripts/`, `data/actions/`, etc.
- [ ] **J.5** Wire Lua events into `EventDispatcher` trait — replace `NullEventDispatcher`.
- [ ] **J.6** Extended opcode handler — Lua `PacketHandler` for OTClient custom packets.
- [ ] **J.7** NPC Lua scripts — dialogue, keyword, trade callbacks.

---

### Phase K — Player Persistence & Shutdown
**Goal:** Players save on logout & periodic auto-save. Server shuts down cleanly.
**C++ ref files:** `iologindata.cpp`, `game.cpp` (saveGameState)
**Estimated effort:** 2–3 days

- [ ] **K.1** Player save on logout — position, health, mana, skills, experience, inventory, depot, conditions.
- [ ] **K.2** Periodic auto-save — every N minutes, save all online players.
- [ ] **K.3** `Logout` handler — save, remove creature, disconnect.
- [ ] **K.4** Graceful shutdown — save all players, save houses, close DB pool, broadcast "Server shutting down".
- [ ] **K.5** House save — owner, access lists, house items.

---

### Phase L — Remaining Game Systems
**Goal:** Everything else needed for a playable server.
**Estimated effort:** 5–7 days total

- [ ] **L.1** Outfit change — `RequestOutfit` / `SetOutfit` handlers, `sendOutfitWindow` (0xC8) with addon/mount lists.
- [ ] **L.2** Quest system — `QuestLog` / `QuestLine` handlers, quest/mission storage values.
- [ ] **L.3** Houses full — door permissions, rent system, house tile protection, beds, `sendTextWindow` for house lists.
- [ ] **L.4** Market — `sendMarketEnter`, browse, create/cancel/accept offers wired to DB.
- [ ] **L.5** Modal windows — `sendModalWindow` (0xFA) / `ModalWindowAnswer` handler.
- [ ] **L.6** GM commands / Talkactions — `/kick`, `/ban`, `/goto`, `/summon`, `/broadcast`, etc.
- [ ] **L.7** World light day/night cycle — `Game::checkLight` periodic update.
- [ ] **L.8** Raids / Global events.
- [ ] **L.9** Highscores query.

---

## Part 3 — Recommended Execution Order

```
Phase A (Protocol Fixes)          ← DO FIRST — unblocks visual correctness
  ↓
Phase B (Items & Containers)      ← Foundation for everything
  ↓
Phase C (Inventory & Equipment)   ← Players can hold things
  ↓
Phase D (Monster/NPC Walk + AI)   ← World comes alive
  ↓
Phase E (Combat)                  ← Players can fight
  ↓
Phase F (Chat & Social)           ← Players can communicate
  ↓
Phase G (Conditions)              ← Combat effects tick correctly
  ↓
Phase H (Spells & Runes)          ← Magic works
  ↓
Phase I (NPC & Player Trade)      ← Economy works
  ↓
Phase J (Lua Scripting)           ← Custom content works
  ↓
Phase K (Persistence & Shutdown)  ← Server is durable
  ↓
Phase L (Remaining Systems)       ← Feature-complete
```

**Note:** Phases F and G can be parallelised with D/E if desired. Phase J (Lua) is the largest single effort and could be started earlier in a reduced form (just `onLogin`/`onDeath` hooks) to unblock script-dependent content.

---

## Part 4 — Quick Wins (Can Do Right Now, <30 min each)

1. **A.1** — Add `0x00` duration byte to `write_item_template` (1 line fix, unblocks map rendering)
2. **A.2** — Add fluid sub-type byte (10 lines + lookup table)
3. **A.3** — Fix `sendChangeSpeed` to 2×u16 (3 lines)
4. **A.6** — Fix `sendCancelTarget` add `u32(0)` (1 line)
5. **A.7** — Fix `send_cancel_walk` direction (2 lines)
6. **L.7** — Dynamic world light (5 lines in `world_light.rs`)

---

## Part 5 — Metrics Summary

| Category | Ported | Total (est.) | % |
|---|---|---|---|
| Client→Server opcodes parsed | 60+ | ~60 | **~100%** |
| Server→Client opcodes implemented | ~18 | ~40 | **~45%** |
| Game packet handlers (with real logic) | 6 (walk, turn, auto-walk, stop, ping, extended) | ~50 | **~12%** |
| C++ source lines (`.cpp` only) | — | ~1.6M chars / ~45k lines | — |
| Rust source lines (all crates) | — | ~12k lines | — |
| Major systems functional | 3 (networking, walking, map loading) | ~20 | **~15%** |

**Bottom line:** The foundation is solid — architecture, networking, map loading, and player walking are well-engineered and battle-tested. The next highest-leverage work is **Phase A** (protocol wire-format fixes) because it unblocks the client from rendering the world correctly, followed by **Phase B** (items/containers) which is the prerequisite for nearly every remaining game system.
