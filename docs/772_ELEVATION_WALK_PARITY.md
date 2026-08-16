# Elevation Walk Parity — stacked `HEIGHT` objects

Status: **plan**. Owner: walk / tile domain.

Two separable subjects, both built on the same `ELEVATION` primitive:

| Part | Era | Status |
|---|---|---|
| **A — step-up limit** ("parcel/box walls" block movement) | **7.4** | Not present in 772 — the mechanic was **removed** by 7.72. Spec kept for a future 7.4 shard. |
| **B — `>= 24` floor climb** (3 stacked objects → change floor) | 772 (and 7.4) | Live in this repo, with four real defects — §4/§5. |

**Nothing in Part A applies to the 772 profile.** Walking from plain ground onto a 2-stack of
parcels **must succeed** on 772; that is current behavior and is correct. Part A exists only so a
`Classic74` profile can switch it on later.

References: `tibia-game-master/src/` (772 outcomes), `tvp-772/gameserver/src/` (wire / TFS shape),
repo-root `src/` (TFS 1.4.2 domain). No 7.4 reference sources are vendored in this repo yet — see §8.

---

## 1. The `ELEVATION` primitive (shared by both parts)

All 772 `HEIGHT` object types carry `Elevation = 8` (verified: 357/357 in
`reference/cipsoft-772/runtime/dat/objects.srv`), and **no** `BANK` type carries `HEIGHT`
(0 matches) — so ground contributes nothing and a tile's elevation is `8 × (stacked HEIGHT objects)`.

`GetHeight(x,y,z)` (`info.cc:689`) sums `ELEVATION` over every `HEIGHT`-flagged object on the
field. Our equivalents live in `walk/walk_tile.rs`: `tile_elevation_sum` (sum) and
`tile_has_height_n` (count). Because every elevation is exactly `8`, `hasHeight(3)` and
`GetHeight() >= 24` are interchangeable on 772 data — TVP and TFS use the count, the decompile
uses the sum.

---

## 2. Part A — 7.4 step-up limit (future shard)

Target behavior as observed on 7.4:

| Source tile | Destination tile | Expected |
|---|---|---|
| ground (0) | 1 parcel (8) | walk |
| ground (0) | 2 parcels (16) | **blocked** |
| 1 parcel (8) | 2 parcels (16) | walk |
| 2 parcels (16) | 3 parcels (24) | walk |
| ground (0) | 3 parcels (24) | **blocked** |
| 2 parcels (16) | ground (0) | walk (stepping down is unrestricted) |
| 3 parcels (24) | anything, cardinal | floor climb (`z-1`) — Part B |

**Candidate rule:** block the step when `elevation(dest) - elevation(src) > 8` — equivalently, the
destination has ≥ 2 more `HEIGHT` objects than the source. One-directional (only stepping up).
Consistent with how box-climbing works in practice: you cannot walk *up* onto a 3-stack, you build
the stack under yourself and then step off it.

### 2.1 Proof it is absent from 772

`GetHeight` has exactly six references in the decompile, and every one is the Part B climb gate:

| Site | Purpose |
|---|---|
| `info.cc:689` / `info.hh:35` | `GetHeight` definition / declaration |
| `cract.cc:421,426` | `TCreature::GoExec` climb ±1 floor when `GetHeight >= 24` |
| `operate.cc:500,504` | `CheckMapDestination` creature-container arm, **floor-change only** |

Movement permission has no elevation term at all:

- `crmain.cc:883` `TCreature::MovePossible` → `JumpPossible` or `BANK && !UNPASS` (+ `AVOID` when `!Execute`)
- `crplayer.cc:363` `TPlayer::MovePossible` → base + `EarliestProtectionZoneRound` gate
- `crnonpl.cc:2141` `TMonster::MovePossible` → home/radius, PZ/house, kick loop over `UNPASS`/`AVOID`
- `crnonpl.cc:1672` `TNPC::MovePossible` → `BANK && !UNPASS && !AVOID && z == startz && radius && !IsHouse`
- `operate.cc:493-532` `CheckMapDestination` — elevation checked **only** for `DestZ == OrigZ ± 1`

Parcels/boxes/crates/chairs are `Avoid`+`Height` but never `Unpass`, so no flag blocks them either
(verified in our `items.otb`: parcel `2595`, box `1738`, crate `1739`, chair `1650` →
`FLAG_BLOCK_PATHFIND | FLAG_HAS_HEIGHT`, no `FLAG_BLOCK_SOLID`).

TVP matches: no `ELEVATION` attribute exists at all, only `bool hasHeight` (`items.h:341`), and its
`Tile::queryAdd` creature branches contain no height logic. Its `hasHeight(n)` (`tile.cpp:62`) is
used at `game.cpp:849,886,908` (climb + return remap), `npc.cpp:468` (TVP-only NPC avoidance),
`tile.cpp:654,685` (item placement) and `tile.cpp:1519,1545` (quest-chest `actionId` hack).

### 2.2 Design for a `Classic74` profile

Tier-1 knob in `MechanicsProfile` (`formulas.rs`):

```rust
/// Max elevation a creature may step **up** in one move, in `ELEVATION` units.
/// `0` disables the gate. 7.4 blocks a step of more than one object (8); the rule was
/// removed by 7.72, so 772 and 1098 both use `0`.
/// See `docs/772_ELEVATION_WALK_PARITY.md`.
pub elevation_step_limit: i32,
```

- `data/formulas/772.lua` → `elevationStepLimit = 0`
- `data/formulas/1098.lua` → `elevationStepLimit = 0`
- future `data/formulas/74.lua` → `elevationStepLimit = 8`

Placement — `internal_move_creature_step` (`walk/mod.rs:2042`), **after**
`resolve_player_move_destination` and **before** `tile_query_add_creature`, so a resolved floor
change bypasses it:

```rust
if is_player && dest_pos.z == current_pos.z && self.mechanics.profile.elevation_step_limit > 0 {
    if elevation_step_blocked(self, current_pos, dest_pos) {
        return Err(ReturnValue::NotPossible); // MOVENOTPOSSIBLE → "Sorry, not possible."
    }
}
```

`elevation_step_blocked` sits next to `tile_elevation_sum` in `walk/walk_tile.rs` — a plain
`dest_sum - src_sum > limit` comparison.

Deliberately **not** in `tile_query_add_player`: `queryAdd` has no source position and is shared by
pushes, teleports, spawn placement and pathfinding probes, none of which the rule governs.

Open design points to resolve against a 7.4 reference before implementing:

- **Player-only or all creatures?** Whether 7.4 box walls stopped monsters decides if the gate goes
  in the walk step (player-only) or into `MovePossible` for every creature. Do not guess.
- **Pushes / throws** — 7.4's `CheckMapDestination` equivalent may or may not gate same-floor
  pushes onto a stack.
- **Server-side pathfinding** — if the gate exists, the A* edge filter (`path_cost`) should honour
  it, or map-click paths will route into a wall and stall.
- **Diagonals** — assumed same limit; unverified.

---

## 3. Part B — what is live today

`resolve_player_move_destination` (`walk/walk_tile.rs:164-241`) implements the `hasHeight(3)`
±1-floor climb, TFS-shaped (`game.cpp:797-841`), called from `internal_move_creature_step`
(`walk/mod.rs:2069`). Confirmed working in-game. `check_push_destination`
(`game_world_player_throw.rs:296-331`) implements the `operate.cc:499-507` push-across-floors gate
on `tile_elevation_sum >= 24`.

Correct today, **do not** "fix": no height gate in `tile_query_add_monster` /
`tile_query_add_npc` (TVP's `npc.cpp:468` `hasHeight(1)` is TVP-only; our NPC arm already rejects
those tiles via `BLOCKPATH`/`AVOID`, which is the right reason), and `IMMOVABLEBLOCKSOLID` on the
climb target (matches `JumpPossible`'s `UNPASS && UNMOVE`; TVP's `BLOCKSOLID` is stricter than 772).

---

## 4. Defects (Part B — 772, ship independently of Part A)

| # | Defect | Location |
|---|---|---|
| **G1** | `ItemType::elevation` is `0` for all 4990 items — `items.otb` carries no elevation attribute (present attr ids: `0x10,0x11,0x14,0x20,0x21,0x22,0x23,0x2A,0x2B`) and `data/items/items.xml` has zero `elevation` keys. `tile_elevation_sum` is therefore constant `0` and `check_push_destination`'s `elev < 24` gate can never pass — pushing a creature up/down a floor over a 3-stack is impossible, though `operate.cc:499-507` allows it. Unit tests hand-build `ItemType`s with `elevation` set, masking it. **Also a prerequisite for Part A.** | `crates/tfs-rust-content/src/items.rs:809`, `game_world_player_throw.rs:296-331`, `walk/walk_tile.rs:124` |
| **G2** | The climb is attempted **before** the flat step is validated. `cract.cc:415` only climbs when `MovePossible(dest, z, true, false)` fails; TVP implements exactly that with a `canGoUp` probe (`game.cpp:850-855`, `875-880`). Ours climbs whenever the source tile has `hasHeight(3)` and the tile above the destination has ground — so standing on a 3-stack next to a *walkable* tile can teleport the player up a floor. | `walk/walk_tile.rs:177-238` |
| **G3** | Floor bounds are TFS's `currentPos.z != 8` / `!= 7`; the decompile uses `DestZ > 0` / `DestZ < 15` (`cract.cc:421,426`, same in `magic.cc:1668,1674`). Climbing `z=8 → z=7` and stepping down `z=7 → z=8` onto a stack are wrongly refused. | `walk/walk_tile.rs:178,210` |
| **G4** | A blocked walk propagates the raw `ReturnValue`, so `NotEnoughRoom` → *"There is not enough room."* `GoExec` only ever throws `MOVENOTPOSSIBLE` → *"Sorry, not possible."* (`sending.cc:339`; `NOROOM`'s string is `sending.cc:296` and walking never throws it). TVP patches only the `hasHeight(3)` case (`game.cpp:908`). | `walk/mod.rs:1549`, `walk/mod.rs:2093-2096`, `return_value.rs` |
| **G5** | 19 object types are `HEIGHT` in `objects.srv` but lack `FLAG_HAS_HEIGHT` in `items.otb` (client ids `1990,1991,2470,2479,2543-2546,2549,2551,2553,2554,2558-2562,2564,2565` → server ids `4348,4358-4382`). All are `Unmove` and all but client `2470` already have `FLAG_BLOCK_SOLID`, so they can never form a climbable stack. Cosmetic. | `data/items/items.otb` |

---

## 5. Implementation order

**Phase 1 — `elevation` becomes real (G1).** `items.otb` has no elevation attribute and never will
(not in the OTB spec). Two options:

- **1a (preferred)** — derive it: `elevation()` returns `8` when `has_height()` and no explicit
  `items.xml` override exists. Exact for 772 (all 357 types are `Elevation = 8`), zero data churn,
  keeps the override path for 1098 / custom content.
- **1b** — emit `<attribute key="elevation" value="8"/>` into `items.xml` for the 338 `HAS_HEIGHT`
  types via the objects.srv→OTB tooling (`docs/772_OBJECTS_SRV_TO_OTB_LOOKUP.md`). More faithful to
  "data drives mechanics", larger diff, still needs 1a for the 19 G5 types.

Ship **1a**; leave 1b as a data-pipeline follow-up.

**Phase 2 — climb ordering (G2).** Port TVP's probe with decompile semantics: climb only when the
flat step fails. `resolve_player_move_destination` is pure (`map`/`items_db`/`items`) while
`tile_query_add_creature` needs `&GameWorld` + `CreatureId`, so either pass those in or hoist the
probe into `internal_move_creature_step` and call the resolver only on failure. Prefer the latter —
it keeps the resolver pure and makes the `cract.cc:415` order explicit at the call site. Probe with
`flags = 0`, matching TVP (`queryAdd(..., 1, 0)`).

**Phase 3 — floor bounds (G3).** Replace `z != 8` / `z != 7` with `dest_pos.z > 0` /
`dest_pos.z < 15`. If 1098 depends on the TFS bounds, drive them from the profile rather than
forking the function.

**Phase 4 — blocked-walk message (G4).** Map every walk rejection to `MOVENOTPOSSIBLE` semantics in
`on_walk_step_rejected` (or remap in `internal_move_creature_step`), since `GoExec` has exactly one
throw. Verify no other caller depends on `NotEnoughRoom` leaking out of the walk path; split into
its own change if it reaches unrelated flows.

**Phase 5 — G5 (optional).** Add the missing `HAS_HEIGHT` to the 19 quest-chest types if the OTB
tooling is being touched anyway.

**Phase 6 — Part A**, only when the 7.4 shard lands and §2.2's open points are answered from a 7.4
reference.

---

## 6. Tests

Part B (now):

| Test | Asserts |
|---|---|
| `elevation_loaded_from_item_db` | loaded DB reports `elevation == 8` for parcel `2595`, box `1738`, crate `1739`, chair `1650` (G1 — current tests hand-build `ItemType`s and cannot catch this) |
| `push_up_floor_over_three_stack_allowed` | G1 regression — `elev >= 24` is reachable again |
| `climb_not_taken_when_flat_move_succeeds` | G2 |
| `climb_up_from_z8` / `step_down_from_z7_onto_stack` | G3 |
| `blocked_walk_sends_sorry_not_possible` | G4 |
| `two_stack_walkable_from_ground_on_772` | **772 regression guard** — the 7.4 rule must never leak into the 772 profile |
| `monster_not_blocked_by_two_stack` | 772 parity — box walls do not stop monsters |

Part A (with the 7.4 shard): the §2 table as a parameterised case set, plus
`gate_disabled_on_772_and_1098_profiles`.

Synthetic tiles: reuse the `has_height` + `elevation` `ItemType` helper in
`game_world_player_throw.rs:1080` / `sim_harness`.

## 7. Verification

```bash
rtk cargo check
rtk cargo clippy --all-targets
rtk cargo test -p tfs-rust-core walk::
rtk cargo test -p tfs-rust-core throw
rtk cargo test -p tfs-rust-content items
```

Live check with the real 7.72 client: walking from ground onto a 2-stack **must work**; stack 3
parcels under yourself and step off — must climb `z-1`; a monster must still path over a 2-stack.

## 8. Prerequisites for the 7.4 shard

- No 7.4 reference is vendored under `reference/` — Part A cannot be implemented from the 772
  decompile, which is exactly where the rule was deleted.
- `objects.srv` / `.dat` for 7.4 to confirm `Elevation` values are still uniformly `8`.
- A wire codec entry for the 7.4 protocol (`TFS-protocol-versioning`), plus a `Classic74`
  `MechanicsProfile` variant and `data/formulas/74.lua`.
- Decide whether the 7.4 rule was client-side, server-side, or both — it changes whether the gate
  belongs in the walk step or in `MovePossible`.
