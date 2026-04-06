# Phase F — Chat & Social

**Goal:** Chat channels, private messages, party channel, guild channel, NPC channel all work.
**C++ ref files:** `chat.cpp`, `chat.h`, `game.cpp` (playerSay, internalCreatureSay)
**Estimated effort:** 2–3 days

> Can be parallelised with Phases D/E if desired.

---

- [ ] **F.1** `ChatChannelManager` — register default channels (Local, World, Trade, Help, etc.), per-guild, per-party, private.
- [ ] **F.2** Handle `RequestChannels` / `OpenChannel` / `CloseChannel` / `CreatePrivateChannel` / `ChannelInvite` / `ChannelExclude`.
- [ ] **F.3** `Say` handler — route by `speak_class` to correct channel or viewport broadcast. Private messages.
- [ ] **F.4** Fix `sendCreatureSay` vs `sendToChannel` vs `sendChannelMessage` — use correct packet format per context (from GAPS.md #13).
- [ ] **F.5** NPC channel — open on NPC interact, close on walk away, route speech to NPC script.
- [ ] **F.6** VIP system — load from DB, `sendVIPEntries` with real data, online/offline notifications.

---

# Phase G — Conditions & Timed Effects

**Goal:** All condition types tick correctly — poison damage, regen, haste speed, drunk stumble, spell cooldowns expire.
**C++ ref files:** `condition.cpp`, `condition.h`
**Estimated effort:** 2–3 days

> Can be parallelised with Phases D/E if desired.

---

- [ ] **G.1** Condition tick system — periodic damage (poison/fire/energy ticks), regen (health/mana per tick).
- [ ] **G.2** Speed conditions — `ConditionSpeed` recalculate player speed on add/remove/tick.
- [ ] **G.3** Drunk condition — random walk direction offset.
- [ ] **G.4** Duration expiry — remove conditions when ticks reach 0.
- [ ] **G.5** Condition icons — `sendPlayerIcons` (0xA2) update on condition change.
- [ ] **G.6** Deferred condition add/remove during walk step (paralyze ↔ haste).

---

# Phase H — Spells & Runes

**Goal:** Players can cast spells and use runes with correct effects.
**C++ ref files:** `spells.cpp`, `spells.h`, `luascript.cpp` (spell Lua bindings)
**Estimated effort:** 3–4 days

> **Depends on:** Phase E (combat) and Phase G (conditions).

---

- [ ] **H.1** Spell execution pipeline — deduct mana/soul, apply cooldowns, execute combat/condition/area effect.
- [ ] **H.2** Instant spells — healing, haste, light, invisible, find person, etc.
- [ ] **H.3** Rune spells — target validation, rune charge consumption, area/single-target.
- [ ] **H.4** Conjure spells — create rune/item, deduct soul/mana.
- [ ] **H.5** `sendSpellCooldown` / `sendSpellGroupCooldown` packets.
