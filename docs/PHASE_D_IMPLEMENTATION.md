# Phase D — Monster & NPC Walking + AI Foundation (Implementation Guide)

**Status:** 🟡 In progress — **D.1–D.5 complete** (walk engine; think cadence; spawn lifecycle; monster AI onThink; follow-on-target-move repath); D.6–D.7 pending.
**Goal:** Bring the world alive. Monsters and NPCs are instantiated from spawn
definitions, walk on the map, chase/flee/return-to-spawn (monsters), idle-walk and
face speakers (NPCs), and respawn on timers — all with 1:1 TFS 1.4.2 parity.
**Target client:** OTClient v8, protocol 10.98.

**Primary C++ references (all present under `src/`):**

| C++ file | Functions of interest |
|----------|-----------------------|
| `src/spawn.cpp` | `Spawn::startup`, `Spawn::checkSpawn` (~353), `Spawn::spawnMonster` (~276/~314), `Spawn::startSpawnCheck` (~240), `Spawn::findPlayer` (~256), `Spawn::isInSpawnZone` (~271), `Spawn::cleanup` (~389) |
| `src/monster.cpp` | `Monster::onThink` (~732), `Monster::onThinkTarget` (~923), `Monster::searchTarget` (~517), `Monster::getNextStep` (~1224), `Monster::isInSpawnRange` (~1931), `Monster::updateLookDirection` (~1967), `Monster::pushItems` (~1146), `Monster::canUseAttack` |
| `src/npc.cpp` | `Npc::onThink`, `Npc::getNextStep`, `Npc::getRandomStep`, `Npc::turnToCreature`, focus / `Npc::onPlayerEnter` |
| `src/creature.cpp` | `Creature::onThink` (~292), `Creature::onCreatureMove` (~485), `Creature::getNextStep` (~200), `Creature::goToFollowCreature` / `getPathTo`, `Creature::onCreatureDisappear`, `Creature::addEventWalk` |
| `src/game.cpp` | `Game::checkCreatureWalk` (~3773), `Game::checkCreatures` (creature think bucket), `Game::placeCreature` / `internalPlaceCreature` / `addCreatureCheck` |
| `src/map.cpp` | `Map::moveCreature` (~295), `Map::getPathMatching` |

> **Dependency note:** Phase E (combat) depends on Phase D — creatures must be alive,
> placed, and moving before attack cycles make sense. Keep combat hooks as
> `attack_target` reads only; do **not** wire damage here.

---

## 1. Executive summary

The walk **engine** (A* pathfinding, step timing, `addEventWalk`/`onWalk`/
`checkCreatureWalk`, floor-change resolution, move-packet emission) is complete but
**hard-coded to `CreatureKind::Player`**. Spawns are **parsed** (`tfs-rust-content`)
and held in a `SpawnManager`, but `SpawnManager::tick` is a no-op — nothing is ever
placed on the map. Monster AI is a placeholder `think_tick` that only flips an enum.
NPCs are a bare struct + trait.

Phase D has four pillars:

1. **Generalize the walk engine** off `Player` so monsters and NPCs reuse the exact
   same timing/packet code (no behavior change for players).
2. **Spawn lifecycle** — instantiate `Monster`/`Npc` from `SpawnManager` zones at
   startup and on respawn timers; broadcast appearance; track spawn ↔ creature links.
3. **Monster AI `onThink`** — target search, chase pathfinding, flee at low HP,
   return-to-spawn, idle random-step, look direction.
4. **NPC AI** — idle random-step inside spawn radius, focus/turn-to-speaker.

Each pillar is a vertical slice that compiles and is testable on its own.

---

## 2. Current state in the Rust codebase

### What already exists (reuse — do not rewrite)

| Area | Location | Notes |
|------|----------|-------|
| Spawn XML parse | `crates/tfs-rust-content/src/spawns.rs` | `SpawnZone { center, radius, entries }`, `SpawnEntry::{Monster, Monsters, Npc}`, spawntime in ms. |
| Monster type DB | `crates/tfs-rust-content/src/monsters.rs` | `MonsterDatabase::load_dir`, `MonsterType { name, speed, health_now/max, loot, attack_spells, defenses }`. **Missing fields** — see §5.1. |
| `SpawnManager` | `crates/tfs-rust-core/src/spawn.rs` | Holds `zones`; `tick` is a **stub**. |
| Creature enum | `crates/tfs-rust-core/src/creature/kind.rs` | `CreatureKind::{Player, Monster, Npc}`, `base()/base_mut()`. |
| Shared fields | `crates/tfs-rust-core/src/creature/base.rs` | `CreatureBase` has `walk_queue`, `last_step*`, `next_walk_check`, `walk_timer`, `attack_target`, `follow_target`, `master`, `position`, `direction`, `speed`. |
| Monster struct | `crates/tfs-rust-core/src/creature/monster.rs` | `Monster { base, spawn_position, ai_phase, think_interval_ms, last_think_tick, registered_events }`. AI is a placeholder. |
| NPC struct | `crates/tfs-rust-core/src/creature/npc.rs` | `Npc { base, npc_type_id }` + `NpcEventsHandler` trait. |
| A* pathfinding | `crates/tfs-rust-core/src/pathfinding.rs` | `get_path_matching(map, start, target, fpp, can_walk_to, tile_walk_cost)`. Fully generic over callbacks — **ready for monsters**. |
| Walk timing/packets | `crates/tfs-rust-core/src/walk.rs` | `add_event_walk`, `on_walk`, `check_creature_walk`, `internal_move_player_step`, `move_creature_on_map`, step-duration math. **Player-only today.** |
| Walk scheduler | `crates/tfs-rust-core/src/walk.rs` + `game_loop.rs` | `walk_wake_tx` one-shot Tokio timers; fallback `process_walk_deadlines`. |
| Tick loop | `crates/tfs-rust-core/src/game_loop.rs`, `game_world.rs::on_tick` | 50 ms tick; already iterates monsters calling `think_tick`. |
| Creature place pattern | `crates/tfs-rust-core/src/login.rs::login_player` | Reference for: insert into `creatures`, `register_creature_index`, `tile.add_creature`. |
| Creature wire-encode | `crates/tfs-rust-net/src/creature_encode.rs`, `map_description.rs` | Known/unknown (`0x61/0x62`) creature encoding used by map description. |

### What is missing (the work)

1. Walk engine generalized to monsters/NPCs (currently every `on_walk`/
   `add_event_walk`/`check_creature_walk` branch matches only `CreatureKind::Player`).
2. `Creature::onThink` bucketing — a `checkCreatures` cadence (every ~`EVENT_CREATURE_THINK_INTERVAL` = 1000 ms) distinct from the walk scheduler. **Done (D.2):** `creature_think.rs` flat 1 Hz sweep for monsters/NPCs.
3. Spawn instantiation + respawn timers in `SpawnManager`.
4. A creature **place/appear broadcast** path (no `sendAddCreature`-equivalent for
   non-players exists yet — players appear via initial login packets only).
5. Monster AI: `searchTarget`, chase, flee, return-to-spawn, idle step,
   `updateLookDirection`.
6. NPC AI: idle random step within radius, focus/turn-to-speaker.
7. `MonsterType` field gaps required by AI (see §5.1).

---

## 3. Architecture & threading

All Phase D logic runs **on the game thread** (single-threaded sim) per
`.cursor/rules/TFS-threading.mdc`. No `tokio::spawn` touching `GameWorld`.

- **Think cadence:** add a creature-think pass to `GameWorld::on_tick`. C++ runs
  `Game::checkCreatures` on a bucketed scheduler at `EVENT_CHECK_CREATURE_INTERVAL`
  (1000 ms, split into 10 buckets of 100 ms). We will mirror the 1000 ms interval per
  creature; bucketing is an optimization we can defer (note it, don't block on it).
- **Walk scheduling:** monsters reuse the existing `next_walk_check` + `walk_wake_tx`
  one-shot timer machinery. The think pass decides a *direction*; the walk engine
  performs the *step* with correct timing.
- **Spawn cadence:** `SpawnManager::tick(now)` checks each zone's blocks against their
  respawn deadline (TFS `Spawn::checkSpawn` runs every `interval` = min spawntime).

```
on_tick (50ms)
  ├─ process_walk_deadlines / walk_wake          (existing, now also monsters/npcs)
  ├─ check_creatures (≈1000ms cadence)  ← NEW     → per-creature on_think → set walk_queue
  ├─ spawns.tick(now)                   ← NEW body → instantiate/respawn monsters
  ├─ decay.tick / lua_gc_step / pings             (existing)
```

---

## 4. Work breakdown

> Order is chosen so each step compiles and is independently testable. D.1 first
> because everything else depends on creatures being able to take a step.

### D.1 — Generalize the walk engine to all creature kinds - Complete

**C++ ref:** `creature.cpp` `Creature::getNextStep`/`onWalk`/`addEventWalk` are on the
**base** `Creature`; only `Player::getNextStep` / `Monster::getNextStep` override the
*direction source*. The timing pipeline is shared.

**Problem:** In `crates/tfs-rust-core/src/walk.rs` every helper matches
`Some(CreatureKind::Player(p))`. Examples: `add_event_walk` (~1030), `on_walk`
(~1156), `commit_next_walk_deadline` (~654), `sync_walk_timer_arm` (~662),
`stop_event_walk` (~1095), `schedule_walk_followup_deadline` (~993),
`internal_move_player_step` (~1304).

**Plan:**

1. Move walk-state reads/writes from the `Player` arm to **`CreatureBase`** via
   `base()/base_mut()`. The fields are already on `CreatureBase` (`walk_queue`,
   `next_walk_check`, `walk_timer`, `last_step*`, `cancel_next_walk`,
   `force_update_follow_path`). Replace
   `Some(CreatureKind::Player(p)) => p.base.…` with `k.base_mut().…` where the logic is
   identical for all kinds.
2. Keep **player-only side effects** behind a kind check at the call site:
   - `send_cancel_walk` / `send_text_message_simple` / move packets to the owning
     connection (`conn_for_creature`) — only players have a `ConnId`.
   - `auto_close_containers_for_player`, `next_action_until`, `last_activity`,
     `clear_player_walk_action`, ghost-mode checks.
3. Rename/duplicate the step performer:
   - Extract the **map mutation + segment building** part of
     `internal_move_player_step` into a generic `internal_move_creature_step(cid, dir)`
     that returns `Result<Vec<MoveSegment>, ReturnValue>` and is kind-agnostic
     (uses `tile_query_add_*` appropriate to the kind).
   - The **packet emission** (player move packets, spectator broadcast) stays a
     separate step driven by the move result, gated on kind.
4. Add `tile_query_add_monster` / `tile_query_add_npc` (or parametrize
   `tile_query_add_player`) — monsters honor extra tile flags. **C++ ref:**
   `tile.cpp` `Tile::queryAdd` monster branch (~520–560): monsters are blocked by
   `TILESTATE_BLOCKPATH`, fields they're not immune to, `TILESTATE_FLOORCHANGE`
   while pathfinding, and PZ unless `master` is a player (summons). NPCs: blocked by
   `TILESTATE_BLOCKSOLID`, can't enter PZ/houses.
5. Spectator broadcast for non-player moves: monsters/NPCs have **no owning conn**,
   so only `broadcast_spectator_move` applies (no `emit_move_packet`). Verify
   `send_move_creature_spectator` works keyed by the creature's protocol id
   (monsters/NPCs use `creatureId` in the 0x6D move packet, not a player guid).

**Acceptance:** a unit test places a monster on a test map, pushes a direction into
its `walk_queue`, drives `check_creature_walk`, and asserts the creature's
`position` changed and a spectator move packet was queued. No change to existing
player walk tests.

**Risk:** This is the highest-risk step (touches the audited player walk timing).
Mitigation: refactor *without behavior change* for players first, run the full
walk test suite, then add monster paths.

---

### D.2 — `Creature::onThink` cadence + dispatch

**C++ ref:** `Game::checkCreatures` → `Creature::onThink(interval)` →
`Monster::onThink` / `Npc::onThink`. Runs ~every 1000 ms per creature.

**Plan:**

1. Add to `GameWorld`:
   ```rust
   /// Wall-clock of last creature-think sweep (TFS EVENT_CHECK_CREATURE_INTERVAL = 1000 ms).
   last_creature_check: Option<Instant>,
   ```
2. In `on_tick`, when `now - last_creature_check >= 1000 ms`, run a sweep:
   - Collect `Vec<CreatureId>` of monsters/NPCs (collect-then-iterate per
     `TFS-entity-storage.mdc` — never mutate `creatures` while iterating it).
   - For each, call a kind dispatch: `self.monster_on_think(cid, interval)` /
     `self.npc_on_think(cid, interval)`.
3. `interval` passed to think = elapsed ms since last sweep (C++ passes the scheduler
   interval). Use the actual delta clamped to a sane range.
4. Remove the placeholder `m.think_tick(tick, cid)` loop currently in `on_tick`
   (`game_world.rs` ~421–425) — replaced by the real sweep.

**Acceptance:** think sweep fires at ~1 Hz in a stepped test harness; counts match
expected calls over N ticks.

---

### D.3 — Spawn instantiation & respawn timers

**C++ ref:** `spawn.cpp` `Spawn::startup` (~344, force-spawns all blocks), `checkSpawn`
(~353), `spawnMonster` (~276/~314), `findPlayer` (~256, suppress respawn if a player is
within the zone and `spawn-monster` config gate), `isInSpawnZone` (~271).

**Data model — extend `SpawnManager`** (`crates/tfs-rust-core/src/spawn.rs`):

```rust
pub struct SpawnSlot {
    pub zone_index: usize,
    pub entry_index: usize,
    pub position: Position,
    pub spawntime_ms: u64,
    /// Live creature occupying this slot, if any.
    pub current: Option<CreatureId>,
    /// Earliest Instant this slot may respawn (set on death).
    pub respawn_at: Option<Instant>,
}

pub struct SpawnManager {
    pub zones: Vec<SpawnZone>,
    pub slots: Vec<SpawnSlot>,
    pub last_check: Option<Instant>,
    pub started: bool,
}
```

**Plan:**

1. Build `slots` from `zones` in `from_zones` (one slot per `SpawnEntry`; expand
   `SpawnEntry::Monsters` weighted lists at spawn time, not slot-build time).
2. `SpawnManager::tick` should **not** mutate the world directly (borrow conflict —
   it lives inside `GameWorld`). Instead make it produce a plan:
   ```rust
   /// Returns slots whose creature is dead/absent and whose respawn timer elapsed.
   pub fn due_spawns(&mut self, now: Instant) -> Vec<SpawnRequest>;
   ```
   Then `GameWorld` consumes `SpawnRequest`s and performs placement (needs `&mut
   self.creatures`, `&mut self.map`, `monsters_db`). Use `std::mem::take`/scoped split
   per `TFS-code-hygiene.mdc` if needed.
3. `GameWorld::startup_spawns()` — called once after map+spawn load in
   `run_server.rs` (~145, where `SpawnManager::from_zones` is built): force-spawn every
   slot (TFS `startup=true` ignores `findPlayer`/interval).
4. `GameWorld::spawn_monster(name, pos, dir, spawn_pos) -> Option<CreatureId>`:
   - Look up `MonsterType` from a new `monsters_db: Arc<MonsterDatabase>` field on
     `GameWorld` (add it; load in `run_server.rs`).
   - Build `CreatureBase` from `MonsterType` (health, speed, outfit, name).
   - `Monster::new(base, spawn_pos)`; insert into `creatures`; register on tile +
     spatial index (mirror `login_player` ~225–229).
   - **Broadcast appear** (D.3a below).
   - Link `slot.current = Some(cid)`; record `cid → slot` for death cleanup.
5. Respawn: when a monster dies (`apply_creature_death` →
   `remove_creature`), find its spawn slot and set `respawn_at = now +
   spawntime_ms`. `due_spawns` re-spawns once `respawn_at` elapses **and** (config
   gate) no player is inside the zone radius (`findPlayer`).
6. NPCs (`SpawnEntry::Npc`) are spawned once at startup and **never respawn** (TFS NPCs
   are placed by `Npcs::reload`/`Game::placeCreature`, persistent). Track but don't
   timer them.

**D.3a — Creature appear broadcast (NEW packet path):**

**C++ ref:** `Game::internalPlaceCreature` → `SpectatorVec` → `spectator->
sendAddCreature(creature, pos, stackpos, isLogin)`; wire is `0x6A` AddTileThing with
creature encoding (`protocolgame.cpp` `sendAddCreature` / `AddCreature`).

- Add `tfs-rust-net::outgoing_extra::send_add_tile_creature(...)` that writes `0x6A` +
  position + stackpos + the known/unknown creature block (reuse
  `creature_encode.rs`). It must update the viewer's `known_creatures_by_conn` set
  exactly like map-description does.
- `GameWorld::broadcast_creature_appear(cid, pos)` enqueues this to every spectator
  conn (`spectator_conns(pos)`), per-viewer known-set handling.
- Pair with `broadcast_creature_disappear` on death/removal (`0x6C` RemoveTileThing or
  the existing remove path) — verify `remove_creature` already notifies spectators; if
  not, add it here.

**Acceptance:** integration test: load a small map + one spawn zone, run
`startup_spawns`, assert a `Monster` exists at the spawn position and a logged-in
spectator received an add-creature packet. Kill it, advance time past `spawntime_ms`,
assert it respawns.

---

### D.4 — Monster AI `onThink`

**C++ ref:** `monster.cpp` `Monster::onThink` (~732): updates target list, calls
`onThinkTarget`/`onThinkYell`/`onThinkDefense`, then `setFollowCreature` and movement.
`searchTarget` (~517), `isInSpawnRange` (~1931), `updateLookDirection` (~1967).

**`Monster` field additions** (`creature/monster.rs`):

```rust
pub follow_creature: Option<CreatureId>,   // chase target (TFS followCreature)
pub is_idle: bool,
pub idle_ticks: u32,
// from MonsterType (copied at spawn, or read via monsters_db):
pub can_push_creatures: bool,
pub target_distance: i32,    // <flags targetdistance=...>  (1 = melee, >1 = keep distance)
pub run_away_health: i32,    // <flags runonhealth=...>
pub static_attack_chance: u32,
pub health_max: i32,         // already on base
```

(Several come from `MonsterType` flags not yet parsed — see §5.1.)

**State machine (mirrors TFS, not a literal enum in C++ but equivalent):**

1. **Target search** (`searchTarget`, `TARGETSEARCH_NEAREST` default for chase):
   - Iterate spectators of monster pos that are valid hostile targets (players, or
     creatures whose `master` is a player). Filter by line-of-sight + range.
   - Pick nearest (Chebyshev). Set `follow_creature` + `base.attack_target`.
   - C++ also respects `target_list`/`friend_list` from XML; honor if present.
2. **Chase** (has target): compute path with `get_path_matching` using
   `FindPathParams { min_target_dist, max_target_dist, clear_sight, allow_diagonal,
   max_search_dist }`:
   - Melee monster (`target_distance <= 1`): `min=0, max=1` (walk adjacent).
   - Distance monster (`target_distance > 1`): keep `target_distance` — path to a tile
     at that range; if too close, step away (TFS `getDistanceStep`).
   - Push the first direction into `walk_queue`; the walk engine (D.1) executes it.
3. **Flee** (`run_away_health > 0 && health <= run_away_health`): walk **away** from
   target (TFS `getDanceStep` / inverted distance step). Set a flee flag so look
   direction stays toward the target.
4. **Return to spawn** (no target && `!isInSpawnRange(pos)`): path back toward
   `spawn_position` (`min=0,max=0`); when in range, idle.
5. **Idle random step** (no target && in spawn range): TFS `getRandomStep` — small
   chance per think to step to a random adjacent walkable tile inside the radius.
   Gated by `walkable` + spawn radius check.
6. **`updateLookDirection`** (~1967): face the target (or movement direction). Emit
   `internal_creature_turn_with_broadcast`-style `0x6B` only when direction changes.

**Important parity details:**

- `onThink` interval gates how often the monster re-targets/re-paths; movement speed
  is governed by the **walk engine** step duration, not the think interval.
- `forceUpdateFollowPath` / `force_update_follow_path` (already on `CreatureBase`):
  when a step fails, re-path next think (TFS sets this in `internalMoveCreature`
  failure).
- Summons (`base.master.is_some()`) follow their master, not search hostiles — out of
  scope for Phase D core but leave the branch stubbed.

**Acceptance:** unit tests on a deterministic map:
- monster with a player in range acquires target and steps toward it;
- monster at low HP with `run_away_health` steps away;
- monster off-spawn with no target steps toward `spawn_position`;
- idle monster occasionally steps within radius and never leaves it.

---

### D.5 — Follow-on-target-move (path refresh) — Complete

**C++ ref:** `creature.cpp` `Creature::onCreatureMove` (~485) → if the moved creature is
our `followCreature`, set `forceUpdateFollowPath`; `goToFollowCreature` recomputes the
path (~619–656 region cited in the task list).

**Implemented in Rust:**

- `move_creature_on_map` → `monster_dispatch_creature_move` → `monster_on_creature_move` follow branch (`monster_ai.rs`, `walk.rs`).
- Instant repath via `monster_follow_repath_now` (C++ 0 ms `goToFollowCreature` scheduler task), gated on `has_follow_path` per `creature.cpp` ~619.
- Failed-step repath in `walk.rs`; 200 ms periodic fallback in `creature_think.rs`.
- Acceptance test: `monster_repaths_when_follow_target_moves` in `monster_ai.rs` `world_tests`.

**Plan (reference):**

- In `move_creature_on_map` / the post-move hook, when any creature finishes a step,
  notify monsters that are following it: set `force_update_follow_path = true` so the
  next think re-paths immediately rather than waiting a full interval.
- Keep it cheap: only scan monsters whose `follow_creature == moved_cid`. Maintain a
  reverse index if profiling shows the scan is hot (defer; note it).

**Acceptance:** moving the target one tile causes the chasing monster to re-path on the
next think (assert path recomputed / direction updates).

---

### D.6 — NPC idle walk + focus

**C++ ref:** `npc.cpp` `Npc::onThink` (random walk via `getRandomStep` when
`walkTicks`/`floorChange` allow), `Npc::turnToCreature` (focus speaker), masterRadius /
spawn position.

**`Npc` field additions** (`creature/npc.rs`):

```rust
pub spawn_position: Position,
pub master_radius: i32,        // <parameter key="walkradius"> (default ~5)
pub can_walk: bool,            // <walkinterval> > 0
pub walk_interval_ms: u32,
pub focus: Option<CreatureId>, // creature the NPC is currently turned toward
```

**Plan:**

1. NPC `on_think`: if `can_walk` and no active focus interaction, with TFS probability,
   `getRandomStep` within `master_radius` of `spawn_position` (reuse the same idle-step
   helper as monsters, parametrized by center+radius).
2. Focus: when a player says something to the NPC (Phase F chat will call in), or
   enters range, `turnToCreature` → `0x6B` turn broadcast. For Phase D, expose
   `Npc::set_focus(cid)` + a turn helper; full speech routing is Phase F. Implement the
   *turn-to-speaker mechanic* now so it's ready.
3. NPCs do **not** chase or flee; they never leave `master_radius`.

**Acceptance:** NPC idles within radius and never exceeds it; `set_focus` turns it to
face the target and emits a single `0x6B`.

---

### D.7 — Deferred condition add/remove during walk (haste/paralyze)

**C++ ref:** `creature.cpp` condition list mutation guarded during `onWalk` to avoid
invalidating iterators; haste/paralyze (`ConditionSpeed`) modify `varSpeed` which feeds
`getStepDuration`. (Phase 4 item 11 from `walk-fix-todo.md`.)

**Plan:**

- `creature_effective_speed_for_step` (walk.rs ~103) already sums
  `ConditionData::Speed { flat_delta }`. Confirm monster/NPC steps run through the same
  function after D.1 (they will, since timing reads `CreatureBase`).
- Ensure adding/removing a speed condition mid-walk re-arms `next_walk_check` correctly
  (TFS recomputes on the next `addEventWalk`). No deferred-mutation queue is needed in
  Rust because we don't hold a borrow across the condition mutation — but **document**
  that adding a condition must call the walk re-schedule path, and add a test for
  haste-while-walking changing step cadence.

**Acceptance:** a monster under a haste condition has shorter step duration; removing it
restores the base cadence on the next step.

---

## 5. Supporting changes

### 5.1 `MonsterType` field gaps (`tfs-rust-content/src/monsters.rs`)

AI needs flags currently **not parsed**. Extend `MonsterType` + `parse_monster_file` to
read `<flags>` and base look/movement attributes:

| Field | XML source | C++ ref | Used by |
|-------|-----------|---------|---------|
| `outfit: Outfit` | `<look type=.. head/body/legs/feet/addons/typeex/mount>` | `monsters.cpp` `loadMonster` look block | spawn appearance |
| `target_distance: i32` | `<flags><flag targetdistance=..>` | `Monster::getDistanceStep` | chase/flee distance |
| `run_away_health: i32` | `<flags><flag runonhealth=..>` | flee gate | flee |
| `can_push_creatures: bool` | `<flags><flag canpushcreatures=..>` | `Monster::pushCreatures` | step blocking |
| `can_push_items: bool` | `<flags><flag canpushitems=..>` | `Monster::pushItems` | step blocking |
| `static_attack_chance: u32` | `<flags><flag staticattack=..>` | `onThinkTarget` | (Phase E mostly) |
| `health_max` already present | `<health max=..>` | — | spawn |
| `speed` already present | attr `speed` | — | step timing |

Keep new fields additive with sensible defaults so existing loader tests pass.

### 5.2 `GameWorld` field additions

```rust
pub monsters_db: Arc<tfs_rust_content::monsters::MonsterDatabase>, // load in run_server.rs
last_creature_check: Option<Instant>,
/// Reverse link for respawn + follow re-path; created creature → spawn slot.
spawn_slot_by_creature: HashMap<CreatureId, usize>,
```

Wire `MonsterDatabase::load_dir(data/monster, &items_db)` in `run_server.rs` alongside
the existing spawn/items loads, and pass into `GameWorld::new`.

### 5.3 Removing the placeholder

Delete `Monster::think_tick` placeholder usage in `on_tick` and the `MonsterAiPhase`
no-op transitions; replace with the real `monster_on_think`. Keep `MonsterAiPhase` if
useful for debugging/telemetry, but drive it from real logic.

---

## 6. Suggested file layout

| New / changed file | Contents |
|--------------------|----------|
| `crates/tfs-rust-core/src/creature_think.rs` | **Done (D.2)** — `check_creatures`, `creature_on_think`, monster/NPC dispatch stubs. |
| `crates/tfs-rust-core/src/walk.rs` | Generalize helpers to `CreatureBase`; add `internal_move_creature_step`, monster/NPC `tile_query_add_*`. |
| `crates/tfs-rust-core/src/creature/monster.rs` | Real AI fields + `Monster` helpers (pure, no world borrow). |
| `crates/tfs-rust-core/src/creature/npc.rs` | NPC idle/focus fields + helpers. |
| `crates/tfs-rust-core/src/monster_ai.rs` *(new)* | `GameWorld::monster_on_think`, `searchTarget`, chase/flee/return/idle step selection. |
| `crates/tfs-rust-core/src/npc_ai.rs` *(new)* | `GameWorld::npc_on_think`, focus/turn. |
| `crates/tfs-rust-core/src/spawn.rs` | `SpawnSlot`, `due_spawns`, respawn timing. |
| `crates/tfs-rust-core/src/game_world.rs` | `startup_spawns`, `spawn_monster`, `broadcast_creature_appear/disappear`, think sweep in `on_tick`, new fields. |
| `crates/tfs-rust-net/src/outgoing_extra.rs` | `send_add_tile_creature` (0x6A + creature block). |
| `crates/tfs-rust-content/src/monsters.rs` | `MonsterType` flag/outfit parsing (§5.1). |
| `crates/tfs-rust-core/src/run_server.rs` | Load `MonsterDatabase`; pass to world; call `startup_spawns`. |

Keep `monster_ai.rs` / `npc_ai.rs` as `impl GameWorld` blocks (methods), with **pure**
decision helpers on `Monster`/`Npc` so they're unit-testable without a world.

---

## 7. Parity checklist (cite C++ in each port)

Per `.cursor/rules/TFS-cpp-references.mdc`, every ported fn needs a `// C++ reference:`
comment. Confirm before completing:

- [x] Walk timing identical for players after D.1 refactor (run existing walk tests).
- [x] `searchTarget` nearest/random selection matches `monster.cpp` ~517–600.
- [x] `getDistanceStep` keep-distance behavior for ranged monsters.
- [x] `isInSpawnRange` uses spawn center + radius (Chebyshev) like `monster.cpp` ~1931.
- [x] `updateLookDirection` face rules match `monster.cpp` ~1967.
- [x] `Spawn::checkSpawn` interval = min spawntime; `findPlayer` suppresses respawn.
- [x] Startup force-spawns ignore interval/findPlayer (`Spawn::startup`).
- [x] Appear/disappear packets match `sendAddCreature`/`RemoveTileThing` byte layout.
- [x] Step cost / floor-change for monsters reuses the player path (no divergence).
- [x] Follow target move triggers instant repath when `has_follow_path` (`creature.cpp` ~619–637).

---

## 8. Testing strategy

`crates/tfs-rust-core/src/test_world.rs` already builds a headless world with an empty
`SpawnManager` — extend it for Phase D:

1. **Unit (pure):** `Monster`/`Npc` decision helpers (target pick, distance step,
   random step bounds) with hand-built positions — no world.
2. **World-level (stepped clock):** place monster + fake player, drive `on_tick`
   manually, assert positions/packets. Use the polling `process_walk_deadlines`
   fallback (set `walk_wake_tx = None`) for deterministic stepping.
3. **Spawn lifecycle:** startup spawn count, death → respawn after `spawntime_ms`,
   `findPlayer` suppression.
4. **Regression:** full existing player walk suite must stay green (D.1 is a refactor).

Prefer deterministic RNG seams: thread a small RNG or accept a `uniform_random` shim so
idle/random-step tests are reproducible (TFS uses `uniform_random`/`normal_random`).

---

## 9. Verification commands

```bash
cargo check --workspace
cargo clippy --workspace -- -D warnings
cargo test --workspace
SQLX_OFFLINE=true cargo test --workspace   # CI-equivalent (no DB)
```

Manual smoke (OTClient 10.98 → `127.0.0.1:7171`): walk near a spawn, confirm monsters
appear, chase, and respawn after being out of view.

---

## 10. Risks & sequencing notes

- **D.1 is load-bearing and risky.** The player walk timing is heavily audited
  (`tasks/walk-audit.md`, `walk-smoothness-audit.md`). Refactor to `CreatureBase`
  **without behavior change**, prove with tests, *then* add monster/NPC paths.
- **Borrow splitting:** spawn placement and think sweeps must collect IDs first, then
  mutate (`TFS-entity-storage.mdc`). Reuse scoped helpers; avoid ad-hoc
  `std::mem::take` proliferation (`TFS-code-hygiene.mdc`).
- **No combat here.** Set `attack_target`/`follow_creature` only; damage, attack speed,
  and death-loot are Phase E. Keep the seam clean.
- **Bucketed `checkCreatures`** is an optimization — a flat 1 Hz sweep is fine for
  Phase D; note the bucket design for later if creature counts grow.
- **Update `tasks/lessons.md`** with any C++ behavior that deviated from assumptions
  (per workflow rule), and tick off `tasks/04-phase-D-E.md` items D.1–D.7 as completed.

---

## 11. Definition of done

- Monsters spawn from `*-spawn.xml` at startup and respawn on timers.
- Monsters search/chase players, flee at low HP, return to spawn, idle-walk in range,
  and face their target — all with TFS-parity timing reusing the walk engine.
- NPCs idle-walk within radius and can turn to face a speaker.
- Player walking behavior is byte-for-byte unchanged.
- All `cargo check/clippy/test` pass; new behavior covered by tests; C++ references
  present on every ported function.
