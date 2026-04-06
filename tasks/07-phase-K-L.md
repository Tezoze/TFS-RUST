# Phase K — Player Persistence & Shutdown

**Goal:** Players save on logout & periodic auto-save. Server shuts down cleanly.
**C++ ref files:** `iologindata.cpp`, `game.cpp` (saveGameState)
**Estimated effort:** 2–3 days

---

- [ ] **K.1** Player save on logout — position, health, mana, skills, experience, inventory, depot, conditions.
- [ ] **K.2** Periodic auto-save — every N minutes, save all online players.
- [ ] **K.3** `Logout` handler — save, remove creature, disconnect.
- [ ] **K.4** Graceful shutdown — save all players, save houses, close DB pool, broadcast "Server shutting down".
- [ ] **K.5** House save — owner, access lists, house items.

---

# Phase L — Remaining Game Systems

**Goal:** Everything else needed for a playable server.
**Estimated effort:** 5–7 days total

---

- [ ] **L.1** Outfit change — `RequestOutfit` / `SetOutfit` handlers, `sendOutfitWindow` (0xC8) with addon/mount lists.
- [ ] **L.2** Quest system — `QuestLog` / `QuestLine` handlers, quest/mission storage values.
- [ ] **L.3** Houses full — door permissions, rent system, house tile protection, beds, `sendTextWindow` for house lists.
- [ ] **L.4** Market — `sendMarketEnter`, browse, create/cancel/accept offers wired to DB.
- [ ] **L.5** Modal windows — `sendModalWindow` (0xFA) / `ModalWindowAnswer` handler.
- [ ] **L.6** GM commands / Talkactions — `/kick`, `/ban`, `/goto`, `/summon`, `/broadcast`, etc.
- [ ] **L.7** World light day/night cycle — `Game::checkLight` periodic update. *(Quick win: ~5 lines in `world_light.rs`)*
- [ ] **L.8** Raids / Global events.
- [ ] **L.9** Highscores query.
