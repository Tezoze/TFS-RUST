# Australis Rust Port — Audit

**Date:** 2026-04-05
**Scope:** Full audit of ported Rust crates vs TFS 1.4.2 C++ source.

---

## Architecture

| Crate | Role | Status |
|---|---|---|
| `tfs-rust-common` | Shared types, enums, `Position`, `GamePacket`, opcodes | ✅ Complete |
| `tfs-rust-content` | OTB/OTBM loaders, items.xml, monsters.xml, vocations, mounts, outfits, spawns, groups | ✅ Complete |
| `tfs-rust-db` | SQLx (MariaDB) — accounts, players, houses, items, market | ✅ Queries ported, wiring partial |
| `tfs-rust-net` | TCP server, XTEA, RSA, packet framing, client→server parse, server→client encode | 🟡 Functional but many wire-format bugs (see `01-protocol-bugs.md`) |
| `tfs-rust-core` | Game loop, `GameWorld`, walk, combat, creatures, spells, conditions, death, party, guild | 🟡 Foundation exists, most game logic is stub/skeleton |
| `tfs-rust-lua` | Lua scripting runtime | ❌ Empty scaffold |

---

## Subsystem Breakdown

### ✅ Done & Working

- **Game loop** — Tokio `select!` with biased ordering, 50ms tick, walk-wake channel, `nextAction` gating.
- **Networking** — XTEA encrypt/decrypt, RSA handshake (raw modpow), Adler32, packet framing.
- **Login flow** — Full initial packet sequence (self-appear, map, stats, skills, light, VIP stub, fight modes).
- **Map loading** — OTBM tile/item parse, OTB item type database, quadtree spatial index.
- **Pathfinding** — A* with ground-speed weighting.
- **Line of sight** — Bresenham-based.
- **Player walking** — Complete for players: single step, diagonal, auto-walk, speed system, walk delay, cancel. Phases 1–2 of `walk-fix-todo.md` done (timing fixes, `forceUpdateFollowPath`, `onWalkComplete`, `cleanup()`).
- **Client→Server parsing** — All TFS 1.4.2 opcodes parsed into `GamePacket` enum (60+ variants).
- **Creature model** — `SlotMap<CreatureId, CreatureKind>` with `Player`/`Monster`/`Npc` variants. `CreatureBase` has health, speed, outfit, conditions, walk queue, damage map.

---

### 🟡 Partial / Skeleton

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

---

### ❌ Not Started

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
