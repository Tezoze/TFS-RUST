# Phase D — Monster & NPC Walking + AI Foundation

**Goal:** Monsters and NPCs walk, chase, flee, return to spawn. Creatures interact on the map.
**C++ ref files:** `creature.cpp`, `monster.cpp`, `npc.cpp`, `spawn.cpp`
**Estimated effort:** 3–4 days

---

- [x] **D.1** Extend walk system to `CreatureKind::Monster` and `CreatureKind::Npc` (Phase 3 items 8–10 from `walk-fix-todo.md`).
- [x] **D.2** Creature think cadence + dispatch — `Game::checkCreatures` → `Creature::onThink` at 1 Hz for monsters/NPCs (`creature_think.rs`). See [PHASE_D_IMPLEMENTATION.md](../docs/PHASE_D_IMPLEMENTATION.md) for canonical numbering.
- [ ] **D.2b** Port `Creature::onCreatureMove` — `localMapCache` shifting for pathfinding *(legacy checklist item; not in implementation guide)*.
- [ ] **D.3** Port follow-creature walk update on target move (`creature.cpp` ~619–656).
- [ ] **D.4** Spawn system — instantiate monsters from `SpawnManager` definitions, respawn timers.
- [ ] **D.5** Monster AI `onThink` — target selection (nearest hostile in range), chase pathfinding, flee at low HP, return to spawn.
- [ ] **D.6** NPC idle walk, focus system (face speaker), walkback to spawn.
- [ ] **D.7** Port deferred condition add/remove during walk — haste/paralyze interaction (Phase 4 item 11 from `walk-fix-todo.md`).

---

# Phase E — Combat System

**Goal:** Melee, distance, and magic attacks work with correct formulas. Creatures can fight and die.
**C++ ref files:** `combat.cpp`, `combat.h`, `weapons.cpp`, `weapons.h`, `game.cpp` (combatChange*)
**Estimated effort:** 4–6 days

> **Depends on:** Phase D (creatures must be alive and moving before combat makes sense).

---

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
