# Item Decay / Expire — Implementation Plan

**Status:** Phase 5 done — ready for Phase 6  
**Date:** 2026-07-18  
**Goal:** 772 decompile **expire outcomes** on a **TFS/TVP-style data pack** (`items.xml` `duration` / `decayto` / `stopduration`), implemented as **idiomatic Rust** (deadline scheduler, not a C++ transliteration of either reference).

---

## 1. Three-layer framing

| Layer | Source of truth | What we match |
|-------|-----------------|---------------|
| **Outcomes** | `tibia-game-master` Expire + Cron | When an item starts/stops timing, transform target, pause/resume (`ExpireStop`), container-empty-before-transform, second-scale deadlines |
| **Domain shape** | TFS / TVP `gameserver` + `data/items/items.xml` | `duration`, `decayto`, `stopduration`, `DecayState`, `item:decay()`, cylinder `startDecaying`, attribute blob Duration/DecayingState |
| **Implementation** | Rust idioms | `ItemId` + deadline map (already closer to decompile Cron heap than TFS 4-bucket lists); no `*_772` forks; era knobs only if numbers truly diverge |

**Conflict rule:** data-pack attribute **names and units** stay TFS (`duration` = seconds in XML → ms on the item). Timing **resolution and transform semantics** follow the decompile (RoundNr-second Cron). Do not require `objects.srv` at runtime — OTB + `items.xml` already carry the same numbers (validated against `merged_objects.srv`).

---

## 2. Authority map (decompile ↔ TFS ↔ data pack)

| Decompile (`objects.srv` / `map.cc` / `operate.cc`) | TFS / TVP | `items.xml` / instance attrs |
|----------------------------------------------------|-----------|------------------------------|
| Flag `Expire` | `ItemType.decayTime != 0` | `<attribute key="duration" value="N"/>` (seconds) |
| `TotalExpireTime` | `decayTime` | same `duration` value |
| `ExpireTarget` | `ItemType.decayTo` | `decayto` (0 / absent → remove) |
| Flag `ExpireStop` | `ItemType.stopTime` | `stopduration` = true |
| Instance `SavedExpireTime` | remaining `ITEM_ATTRIBUTE_DURATION` while stopped | duration attr kept, decaying false |
| Instance `RemainingExpireTime` (look) | `getDuration()` / 1000 | showduration UI |
| `CronExpire` / `CronSet` | `Game::startDecay` → `toDecayItems` | schedule |
| `CronStop` / `CronInfo` | leave decay list + keep remaining ms | cancel / pause |
| `ChangeObject` → `CronExpire` | `Item::setID` / `transformItem` + `startDecay` | transform + reschedule |
| `ProcessCronSystem` → `Change(ExpireTarget)` | `checkDecay` → `internalDecayItem` | fire |
| Container expire empties first | (TFS transforms; content handling via cylinder) | preserve loot until final remove |

### 2.1 Decompile timing (outcomes)

```
// main.cc AdvanceGame
CronTimeCounter >= 1500 → ProcessCronSystem()   // ~1 Hz after warmup
OtherTimeCounter >= 1000 → RoundNr += 1         // logical second

// map.cc
CronSet(Obj, Delay) → Entry.RoundNr = RoundNr + Delay
CronExpire(Obj, -1) → Delay = TOTALEXPIRETIME (seconds)
CronExpire(Obj, saved) → resume with SavedExpireTime

// operate.cc ProcessCronSystem
ExpireTarget = EXPIRETARGET
if CONTAINER: Empty(Obj, remainderCapacity)
Change(Obj, ExpireTarget, 0)   // may chain into another Expire via ChangeObject
```

Create/transform always goes through `ChangeObject` → unconditional `CronExpire` at the end (`map.cc`). New `Expire` types get a full `TotalExpireTime`; transitioning **from** `Expire` **into** `ExpireStop` stores remaining into `SavedExpireTime`; transitioning **from** `ExpireStop` restores that delay.

### 2.2 TFS/TVP domain (data pack)

```
// items.cpp parse
duration     → it.decayTime (seconds)
decayto      → it.decayTo (-1 default; 0 = vanish)
stopduration → it.stopTime

// item.cpp
CreateItem → setDefaultDuration() = decayTime * 1000 (ms on instance)
startDecaying → Game::startDecay
canDecay: not removed, decayTo >= 0, decayTime != 0, no uniqueId,
          actionId not in [1000,2000], depot gate optional

// game.cpp
startDecay: if duration > 0 → DECAYING_TRUE + queue; else internalDecayItem
internalDecayItem: decayTo != 0 → transformItem + startDecay; else remove
checkDecay: 250ms × 4 buckets (implementation detail — not required in Rust)
```

**Data-pack parity check (no srv at runtime):**  
`merged_objects.srv` `TotalExpireTime=300,ExpireTarget=293` ↔ `items.xml` `duration=300` / `decayto=293`. Same for fields, corpses, lit lamps (`ExpireStop` ↔ `stopduration`).

---

## 3. Current Rust audit

### 3.1 What already exists

| Piece | Location | Notes |
|-------|----------|-------|
| `DecayManager` deadline map | `decay.rs` | Good shape (Cron-like); fields named `deadline_tick` but fed `server_ms` |
| Cron subsystem fire | `game_world_tick.rs` `fired.cron` | Matches `CronTimeCounter` stagger |
| Equip transform + schedule | `equip_abilities.rs` | Rings/boots: `duration` sec × 1000 → deadline; **only path that applies expiry** |
| Corpse / splash schedule | `monster_inventory.rs`, `death.rs` | Schedules into `DecayManager` |
| Attr blob Duration / DecayingState / DecayTo | `item_attributes.rs`, `item_blob.rs` | DB serialize path present |
| Lua `item:decay()` | `lua_mutation` → `LuaMutation::ItemDecay` | **Wired** → `start_decay` (Phase 2) |
| XML keys recognized | `items_xml_keys.rs` | `duration` / `decayto` / `stopduration` still mostly raw `xml_attributes` |
| Profile corpse offset | `corpse_decay_offset_ms` | 772 = 30s, 1098 = 600ms (death placeholder) |

### 3.2 Critical gaps

1. **Expiry apply is equipment-only.** — **fixed Phase 2**  
   Cron → `process_decay_expiry` dispatches equip / tile / container.

2. **No general `start_decay` / `internal_decay_item`.** — **fixed Phase 2 (API)**  
   Create-on-tile, cylinder add, map load, login hydrate hooks still Phase 3/4; Lua `item:decay()` wired.

3. **Duration unit bug (corpse/splash).** — **fixed Phase 1**  
   Was `duration * 50` ms; now `decay_deadline_ms` (seconds × 1000).

4. **No `stopduration` / ExpireStop.** — **fixed Phase 3**  
   Lit → unlit / deequip pause remaining via `change_item_type`; re-light / re-equip resumes.

5. **No typed `ItemType` decay fields.** — **fixed Phase 1**  
   `decay_time` / `decay_to` / `stop_time` / `show_duration` on `ItemType`; XML still mirrored in `xml_attributes`.

6. **No `can_decay` guards.**  
   uniqueId, quest actionId band, removed item, depot policy (TVP flag) — not enforced.

7. **No chain transform for map items.**  
   Equip path re-schedules via `transform_equipped_item(..., start_decay)`. Tile path needs the same: transform → if new type has duration → schedule again (decompile `Change` → `CronExpire`).

8. **Container expire empty.**  
   Decompile empties container into map (capacity remainder) before `Change` to target. **Required** — ~139 decaying corpses in `items.xml` are containers with loot (e.g. dead troll 2806 → `containersize=7`, `duration=1800`, `decayto=2810`).

9. **Login / map load `DECAYING_PENDING`.**  
   Blob already maps non-false decaying → Pending on load; nothing re-queues Pending into `DecayManager` after load (TFS `startDecaying` on iologin/iomap).

10. **Special death offset vs XML chain.**  
    `death.rs` / `corpse_decay_offset_ms` is a placeholder. Decompile places the race corpse via `Create` → `ChangeObject` → `CronExpire(TotalExpireTime)` with **no** extra delay (`crmain.cc` destructor). **Decision:** race corpse clock = XML `duration` only; remove offset as the corpse timer (see §8).

### 3.3 What we should **not** copy

| Avoid | Prefer |
|-------|--------|
| TFS `EVENT_DECAYINTERVAL` × 4 buckets | Deadline / min-heap keyed by `server_ms` (current `DecayManager`, tighten API) |
| Decompile Cron hash + heap C++ layout | `HashMap<ItemId, DecayEntry>` (+ optional binary heap for O(log n) pop) |
| Runtime `objects.srv` Expire flags | `items.xml` (+ OTB where needed) |
| Parallel `expire_*` API beside `decay_*` | One TFS-named domain: `start_decay`, `DecayState`, `duration` |

---

## 4. Target design

### 4.1 Domain API (TFS-shaped)

```text
ItemType { decay_time_sec, decay_to, stop_time, show_duration, ... }

Item {
  duration_ms / decaying / decay_to override (attrs)
}

GameWorld::start_decay(item_id)           // Item::startDecaying / CronExpire(-1 or saved)
GameWorld::stop_decay(item_id) -> remaining_ms  // CronStop
GameWorld::can_decay(item_id) -> bool
GameWorld::internal_decay_item(item_id)   // transform or remove + maybe start_decay
GameWorld::process_decay_expiry(expired)  // cron apply — all cylinders
```

`DecayManager` stays the scheduler only (schedule / cancel / tick). Transform, cylinder notify, and equip ability strip live in `GameWorld` (or a small `decay_apply.rs` helper module).

### 4.2 Clock (772 outcomes)

- Store instance duration as **milliseconds** (TFS blob / `setDefaultDuration`).
- Schedule deadline = `server_ms + remaining_ms` (equip path already correct).
- Cron subsystem (~1s) pops due entries — matches `ProcessCronSystem` cadence; do not require RoundNr integer seconds if `server_ms` deadlines are exact.
- **Unify** all call sites to `duration_sec * 1000` (delete the `* 50` corpse unit).

1098: same domain; if any duration scale differs, put it in `MechanicsProfile` — today XML seconds are shared.

### 4.3 Transform / stopduration semantics

Mirror decompile `ChangeObject` expire branch using TFS field names:

```text
on type change old → new:
  if old.decay_time > 0 (was timing):
      remaining = stop_decay(item)   // cancel cron, read remaining
  if new.stop_time:
      keep duration = remaining      // SavedExpireTime
      decaying = False
  else if new.decay_time > 0:
      if remaining was 0: duration = new.decay_time * 1000
      start_decay(item)
  else:
      clear duration / decaying
```

Equip `transformEquipTo` / `transformDeEquipTo` must go through this helper so rings and lamps share one path.

### 4.4 Apply paths (where expired items live)

| Location | Action |
|----------|--------|
| Player equipment slot | existing `process_equipment_decay_expiry` (abilities strip) |
| Tile | `transform_item` / remove + spectator tile update |
| Container / depot / inbox | transform or remove + container UI refresh |
| Not found / already removed | cancel only (idempotent) |

After successful transform to a decaying type → `start_decay` again (chain).

### 4.5 Hooks that must call `start_decay`

| Event | TFS / decompile ref |
|-------|---------------------|
| Item created with default duration | `Item` ctor `setDefaultDuration` + cylinder `startDecaying` |
| Added to tile/container/inventory | `startDecaying` / `ChangeObject`→`CronExpire` |
| `transform_item` result | `transformItem` duration block / `ChangeObject` |
| Map load / house load | `iomap` / `iomapserialize` |
| Player login inventory | `iologindata` |
| Lua `item:decay()` | `luaItemDecay` |
| Splash / field create | `Game::startDecay(splash)` / field create |

### 4.6 `can_decay` policy

Port TFS checks (domain): not removed, `decay_time != 0`, `decay_to >= 0`, no uniqueId, actionId not in `[1000, 2000]`.

**Depot (locked):** config key `itemsDecayInsideDepots` (TVP name), **default `false`** — items inside a depot locker do not decay unless the flag is enabled (`configmanager.cpp` default matches). Add to `config.lua` when wiring `can_decay`.

---

## 5. Implementation phases

### Phase 0 — Spec lock (this doc) — **done**

- [x] Map Expire ↔ duration/decayto/stopduration  
- [x] Audit Rust gaps  
- [x] Death/corpse: XML `duration` only (no extra offset) — see §8.1  
- [x] Container Empty on expire: required (~139 corpse containers) — see §8.3  
- [x] Depot: `itemsDecayInsideDepots` default `false` — see §8.2  
- [x] 1098: shared XML seconds / shared path — see §8.4  

### Phase 1 — Content + clock hygiene — **done**

**Files:** `tfs-rust-content` `items.rs` / `otb.rs`; `decay.rs`; `monster_inventory.rs`; `equip_abilities.rs`.

- [x] Promote typed `decay_time`, `decay_to`, `stop_time`, `show_duration` on `ItemType` from XML.
- [x] Fix schedule math to `server_ms + decay_time_sec * 1000`.
- [x] Central helper: `decay_deadline_ms(now, duration_sec)`.
- [x] Unit tests: firefield 200s, candelabrum 3000s, dead troll 1800s; `item_decay_schedule` typed path.

### Phase 2 — Core decay apply — **done**

**Files:** `decay.rs` (API clarify), `decay_apply.rs`, `game_world_tick.rs`, `lua_scope.rs`, `equip_abilities.rs`.

- [x] `start_decay` / `stop_decay` / `can_decay` / `internal_decay_item`.
- [x] `process_decay_expiry`: dispatch by location (equip / tile / container).
- [x] Cron tick calls **general** apply (equip path becomes a branch).
- [x] Wire `LuaMutation::ItemDecay` to `start_decay`.

### Phase 3 — Create / transform / cylinder hooks — **done**

**Files:** `decay_apply.rs` (`change_item_type`), equip/lua transform, tile/container/inventory add+remove.

- [x] Every create/transform that yields a decaying type calls `start_decay` (via cylinder add + `change_item_type`).
- [x] Implement stopduration pause/resume in the shared type-change helper.
- [x] Cancel decay on `Destroy` / item remove (`DestroyObject` CronStop).

### Phase 4 — Persistence — **done**

**Files:** `item_blob.rs`, `game_world_save.rs`, `player/inventory/load.rs`.

- [x] On load: `DecayState::Pending` → `start_decay` with remaining ms (`restart_pending_decay_for_player`).
- [x] Save remaining duration via `write_item_blob_with_duration` + `DecayManager::remaining_ms`.
- [x] House `tile_store` apply still absent — reuse `restart_pending_decay_*` when that path lands.

### Phase 5 — Container empty + polish — **done**

- [x] Decompile `Empty` before expire transform when type is container (`empty_container_for_expire`).
- [x] Look text / `showduration` (`item_look.rs` + look call site remaining ms).
- [x] Depot policy + quest actionId / uniqueId guards (`itemsDecayInsideDepots` default false).
- [x] Magic field expire unit smoke via cron `process_decay_expiry`.

### Phase 6 — Cleanup special cases

- Fold corpse/splash ad-hoc schedulers into `start_decay`.
- Remove `corpse_decay_offset_ms` / formula knob once race corpses use XML `start_decay` (Phase 0: not a decompile timer).
- Remove CH-6 “no-op” comments; update `pc3a-spell-gaps.md` / lessons.

---

## 6. Suggested module layout

```text
crates/tfs-rust-content/src/items.rs     # typed decay_* on ItemType
crates/tfs-rust-core/src/decay.rs        # DecayManager only (schedule/cancel/tick)
crates/tfs-rust-core/src/decay_apply.rs  # start/stop/can/internal/process_expiry
                                         # //! Domain: TFS game.cpp startDecay/checkDecay
                                         # //! Outcomes: map.cc Cron*, operate.cc ProcessCronSystem
```

Keep equip ability strip in `equip_abilities.rs` but call into `decay_apply` for type change + reschedule.

---

## 7. Verification

### Commands

```bash
rtk cargo test -p tfs-rust-core --lib decay
rtk cargo test -p tfs-rust-core --lib equip_abilities
rtk cargo test -p tfs-rust-content --lib
rtk cargo check -p tfs-rust-core
```

### Tests to add

| Test | Expectation |
|------|-------------|
| `duration_sec_to_deadline` | 300 → +300_000 ms on `server_ms` |
| Tile item decays to `decayto` | type changes; spectators updated; new type rescheduled if duration > 0 |
| `decayto` 0 / absent | item removed from tile |
| Equip ring chain | abilities strip + transform (existing) still passes |
| `stopduration` lamp | lit → unlit keeps remaining; re-lit resumes (~same remaining) |
| `item:decay()` Lua mutation | schedules non-zero deadline |
| Load Pending | after hydrate, cron fires at remaining |
| `can_decay` uniqueId | refuses schedule |
| Corpse stage times | match XML seconds (not ×50); e.g. 2806 → 1800s → 2810 |
| Corpse Empty | loot dumped to tile (or kept in next-stage container per Empty remainder) before stage transform |
| Field expire | poison/fire field → next id or remove per XML |
| Depot `can_decay` | item in depot with `itemsDecayInsideDepots=false` does not schedule |

### Live smoke

- Kill monster: corpse stages through XML chain, then gone; loot survives stages (Empty).  
- Cast fire field: expires through field stages.  
- Equip time ring: expires to inactive; re-equip resumes duration behavior per XML.  
- Light candelabrum: burns out to used / stopduration unlit.  
- Put decaying item in depot: does not expire (default config).

---

## 8. Phase 0 decisions (locked)

### 8.1 Death / corpse timer

**Decision:** race corpse uses XML `duration` / `decayto` only. Remove `corpse_decay_offset_ms` as the corpse clock.

**Evidence:** `crmain.cc` `~TCreature` → `Create(Con, CorpseType, 0)` → `ChangeObject` → `CronExpire(TotalExpireTime)`. No separate pre-corpse delay. Example: dead troll `2806` → `duration=1800`, `decayto=2810` (`items.xml`; matches srv Expire attrs).

**Rust follow-up:** place race corpse → `start_decay`; delete generic “vanish after offset” path in `death.rs` once wired. Profile knob / `772.lua` entry removed in Phase 6.

### 8.2 Depot decay

**Decision:** config `itemsDecayInsideDepots` (TVP), **default `false`**.

**Evidence:** `configmanager.cpp` `getGlobalBoolean(L, "itemsDecayInsideDepots", false)`. Not yet in our `config.lua` — add when implementing `can_decay`.

### 8.3 Container Empty on expire

**Decision:** implement decompile `Empty` before expire transform for container types.

**Evidence:** `operate.cc` `ProcessCronSystem` empties when `CONTAINER` before `Change(ExpireTarget)`. Active pack has **~139** decaying corpse containers (and ~158 decaying+container attrs total). Without Empty, loot would vanish or stuck-transform incorrectly.

### 8.4 1098

**Decision:** one shared decay path; XML `duration` stays seconds → ms. Do not port TFS 4-bucket scheduling for “1098 feel.”

**Evidence:** same `items.xml` keys for both eras; bucket list is TFS implementation, not an era outcome. If a true number delta appears later, put it in `MechanicsProfile` — none known today.

---

## 9. Lessons to capture (after implement)

- Expire (decompile) ≡ duration/decayto (TFS); one scheduler.  
- Never scale XML `duration` by beat unit (50 ms) — seconds → ms only.  
- Cron tick must apply **all** locations, not only equipment.  
- `stopduration` is pause/resume, not “no decay”.  
- Race corpse timer is type `TotalExpireTime` / XML `duration` — not `corpse_decay_offset_ms`.  
- Decaying corpses are containers → Empty before stage change.

---

## 10. Out of scope

- Condition / poison “decay” (`process_skills` FactorPercent) — different system.  
- Map refresh / sector refresh.  
- Reintroducing runtime `objects.srv` for Expire flags.
