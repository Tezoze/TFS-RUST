# Elevation Walk Parity — stacked `HEIGHT` objects

Status: **G1–G4 shipping in this change.** Owner: walk / tile domain.

Two separable subjects, both built on the same `ELEVATION` primitive:

| Part | Era | Status |
|---|---|---|
| **A — step-up limit** ("parcel/box walls" block movement) | **7.4** | Not present in 772 — the mechanic was **removed** by 7.72. Spec kept for a future 7.4 shard. |
| **B — `>= 24` floor climb** (3 stacked objects → change floor) | 772 (and 7.4) | Live. **G1–G4 ship in this change**; G5 still cosmetic. |

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
field. Our equivalent is `walk/walk_tile.rs` `tile_elevation_sum`. Because every elevation is
exactly `8`, TFS `hasHeight(3)` and `GetHeight() >= 24` coincide on 772 data; climb and push
both use the sum.

`ItemType::elevation()` (G1): xml override if `elevation != 0`, else `8` when `has_height()`, else
`0`. OTB has no elevation attribute; `items.xml` has no `elevation` keys.

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

Placement — `internal_move_creature_step` (`walk/mod.rs`), **after**
flat/`try_player_elevation_climb` resolution and **before** `tile_query_add_creature`, so a resolved floor
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

**Not implemented.** Do not add `elevationStepLimit` to `MechanicsProfile` until a 7.4 shard lands.

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

`internal_move_creature_step` (`walk/mod.rs`) probes the flat dest with 772
`player_move_possible_push(..., Jump=false)` (`BANK && !UNPASS` + PZ/house). Climb runs only when
that probe is `Ok(false)`, the mover is a player, and the step is not diagonal — matching
`cract.cc:415`. `try_player_elevation_climb` (`walk/walk_tile.rs`) implements up/down with
`GetHeight >= 24`, `!BANK && !UNPASS` air tiles, `DestZ > 0` / `DestZ < 15`, and
`MovePossible(..., Jump=true)` → `JumpPossible` (`info.cc:702`).

`check_push_destination` (`game_world_player_throw.rs`) implements the `operate.cc:499-507`
push-across-floors gate on `tile_elevation_sum >= 24`. With G1 the sum is reachable on live OTB
`HEIGHT` items (field `0` → accessor `8`).

Correct today, **do not** "fix": no height gate in `tile_query_add_monster` /
`tile_query_add_npc` (TVP's `npc.cpp:468` `hasHeight(1)` is TVP-only; our NPC arm already rejects
those tiles via `BLOCKPATH`/`AVOID`, which is the right reason). Climb dest uses `JumpPossible`
(BANK present, no `UNPASS && UNMOVE`), not TFS `FLAG_IGNOREBLOCKITEM | FLAG_IGNOREBLOCKCREATURE`.

---

## 4. Defects (Part B — 772, ship independently of Part A)

| # | Defect | Status |
|---|---|---|
| **G1** | `ItemType::elevation` was `0` for all items — `items.otb` carries no elevation attribute and `items.xml` has zero `elevation` keys. `tile_elevation_sum` was constant `0` and `check_push_destination`'s `elev < 24` gate could never pass on live data. | **Shipping** — `elevation()` returns `8` when `has_height()` and xml is `0`. No 338-key xml dump. |
| **G2** | Climb ran **before** the flat step was validated, so standing on a 3-stack next to walkable ground could change floor. Corpus: climb only when `MovePossible(dest, z, true, false)` fails (`cract.cc:415`). | **Shipping** — probe then `try_player_elevation_climb`. Hole/air is `!BANK && !UNPASS`, not `ground.is_none && !BLOCKSOLID`. |
| **G3** | Floor bounds were TFS `currentPos.z != 8` / `!= 7`; decompile uses `DestZ > 0` / `DestZ < 15` (`cract.cc:421,426`). | **Shipping** — corpus bounds. `z=8 → z=7` and `z=7 → z=8` onto a stack are allowed. |
| **G4** | A blocked walk could leak `NotEnoughRoom` → *"There is not enough room."* `GoExec` only throws `MOVENOTPOSSIBLE` → *"Sorry, not possible."* | **Shipping** — remap `NotEnoughRoom` → `NotPossible` at the walk-step Err site only. `PlayerIsPzLocked` / `PlayerIsNotInvited` pass through. |
| **G5** | 19 object types are `HEIGHT` in `objects.srv` but lack `FLAG_HAS_HEIGHT` in `items.otb` (client ids `1990,1991,2470,2479,2543-2546,2549,2551,2553,2554,2558-2562,2564,2565` → server ids `4348,4358-4382`). All are `Unmove` and all but client `2470` already have `FLAG_BLOCK_SOLID`, so they can never form a climbable stack. Cosmetic. | Open |

---

## 5. Implementation order

**Phase 1 — `elevation` becomes real (G1). SHIPPED.** `items.otb` has no elevation attribute and never will
(not in the OTB spec). Shipped **1a**: `elevation()` returns `8` when `has_height()` and no explicit
`items.xml` override exists. Exact for 772 (all 357 types are `Elevation = 8`), zero data churn.
Leave 1b (xml dump of 338 keys) as a data-pipeline follow-up. Do **not** add `elevationStepLimit`.

**Phase 2 — climb ordering (G2). SHIPPED.** Probe the flat dest with 772 `MovePossible(Jump=false)`
in `internal_move_creature_step`; call `try_player_elevation_climb` only on `Ok(false)` for
non-diagonal players. Climb dest uses `Jump=true` / `JumpPossible`.

**Phase 3 — floor bounds (G3). SHIPPED.** `flat_dest.z > 0` / `flat_dest.z < 15`.

**Phase 4 — blocked-walk message (G4). SHIPPED.** `remap_walk_step_err` in the walk module maps
`NotEnoughRoom` → `NotPossible` on `internal_move_creature_step` Err only.

**Phase 5 — G5 (optional).** Add the missing `HAS_HEIGHT` to the 19 quest-chest types if the OTB
tooling is being touched anyway.

**Phase 6 — Part A**, only when the 7.4 shard lands and §2.2's open points are answered from a 7.4
reference.

---

## 6. Tests

Part B (this change), in `walk/elevation_climb_tests.rs`, `otb.rs`, and `game_world_player_throw.rs`:

| Test | Asserts |
|---|---|
| `elevation_loaded_from_item_db` | loaded OTB reports `elevation() == 8` for parcel `2595`, box `1738`, crate `1739`, chair `1650` |
| `elevation_xml_override_wins` | field `16` still wins over the HEIGHT default |
| `three_default_height_items_sum_to_24` / `p5_three_default_elevation_height_items_pass_gate` | G1 — three default-8 items sum to 24; push height gate passes |
| `p5_push_up_sufficient_elevation_passes_height_gate` | existing P5 regression |
| `climb_not_taken_when_flat_move_succeeds` | G2 |
| `climb_up_when_flat_blocked` | G2 |
| `climb_up_from_z8` / `step_down_from_z7_onto_stack` | G3 |
| `blocked_walk_noroom_is_not_possible` | G4 |
| `two_stack_walkable_from_ground_on_772` | **772 regression guard** — the 7.4 rule must never leak into the 772 profile |

Part A (with the 7.4 shard): the §2 table as a parameterised case set, plus
`gate_disabled_on_772_and_1098_profiles`.

---

## 7. Verification

```bash
rtk cargo test -p tfs-rust-core --lib walk
rtk cargo test -p tfs-rust-content --lib
rtk cargo test -p tfs-rust-core --lib game_world_player_throw
```

Live check with the real 7.72 client: walking from ground onto a 2-stack **must work**; stack 3
parcels under yourself and step off — must climb `z-1`; a monster must still path over a 2-stack.

---

## 8. Prerequisites for the 7.4 shard

- No 7.4 reference is vendored under `reference/` — Part A cannot be implemented from the 772
  decompile, which is exactly where the rule was deleted.
- `objects.srv` / `.dat` for 7.4 to confirm `Elevation` values are still uniformly `8`.
- A wire codec entry for the 7.4 protocol (`TFS-protocol-versioning`), plus a `Classic74`
  `MechanicsProfile` variant and `data/formulas/74.lua`.
- Decide whether the 7.4 rule was client-side, server-side, or both — it changes whether the gate
  belongs in the walk step or in `MovePossible`.
