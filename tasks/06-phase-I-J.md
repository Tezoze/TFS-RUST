# Phase I — NPC Trade & Player Trade

**Goal:** Players can buy/sell from NPCs and trade with each other.
**C++ ref files:** `npc.cpp`, `game.cpp` (playerBuyItem, playerSellItem, playerRequestTrade)
**Estimated effort:** 2–3 days

> **Depends on:** Phase B/C (items must exist as real objects).

---

- [ ] **I.1** NPC shop — `sendShop` (0x7A), `sendSaleItemList` (0x7B), handle `PlayerPurchase` / `PlayerSale` / `CloseShop`.
- [ ] **I.2** Player-to-player trade — `RequestTrade` / `LookInTrade` / `AcceptTrade` / `CloseTrade` handlers, trade window packets.
- [ ] **I.3** Gold handling — coin stacking (gold/platinum/crystal), pay-from-containers.

---

# Phase J — Lua Scripting Engine

**Goal:** `tfs-rust-lua` crate provides full script bindings; existing TFS Lua data scripts load and execute.
**C++ ref files:** `luascript.cpp`, `luascript.h`, `baseevents.cpp`, all `*event*.cpp`
**Estimated effort:** 5–8 days (largest single phase)

> **Note:** Could be started earlier in a reduced form (just `onLogin`/`onDeath` hooks) to unblock script-dependent content sooner.

---

- [ ] **J.1** `mlua` runtime init — create Lua VM, register global functions, load `data/lib/` libraries.
- [ ] **J.2** Core Lua API — `Creature`, `Player`, `Monster`, `Npc`, `Item`, `Tile`, `Position`, `Game` metatables.
- [ ] **J.3** Event system — `CreatureEvent` (onLogin, onLogout, onDeath, onKill, onAdvance, onThink), `MoveEvent`, `Action`, `TalkAction`, `GlobalEvent`.
- [ ] **J.4** Script loading pipeline — scan `data/scripts/`, `data/creaturescripts/`, `data/actions/`, etc.
- [ ] **J.5** Wire Lua events into `EventDispatcher` trait — replace `NullEventDispatcher`.
- [ ] **J.6** Extended opcode handler — Lua `PacketHandler` for OTClient custom packets.
- [ ] **J.7** NPC Lua scripts — dialogue, keyword, trade callbacks.
