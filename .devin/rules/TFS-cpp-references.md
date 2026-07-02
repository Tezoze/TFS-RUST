---
trigger: always_on
description: Mandatory C++ source tracing for all ported logic to ensure exact parity with TFS 1.4.2.
globs:
---

# C++ Source Tracing (Mandatory)

Every Rust file with ported TFS logic MUST include references to the original C++ source.

## Required Header Comment

At the top of every `.rs` file with substantial ported logic:

```rust
//! Brief module description.
// C++ reference: <file> <function/class>
```

Or in doc comments:
```rust
//! Module description.
//! C++ reference: `game.cpp` `Game::playerMove`, `Game::internalMoveCreature`.
```

## Multi-Function Ports

For complex ports spanning multiple C++ functions, list all relevant references:

```rust
//! TFS 1.4.2 walking implementation.
//!
//! - `Game::playerMove` / `playerAutoWalk` — `game.cpp` (~1880, ~2075).
//! - `Creature::startAutoWalk`, `onWalk`, `getNextStep` — `creature.cpp` (~200–322).
//! - `Map::moveCreature` — `map.cpp` (~295–306).
```

## Inline Function References

For critical ported functions, include inline references:

```rust
/// TFS `Creature::getSpeed` — `baseSpeed + varSpeed` from conditions (`creature.h`).
fn creature_effective_speed(base: &CreatureBase) -> i32 { ... }
```

## Constants and Magic Numbers

Document C++ source for all constants:

```rust
const SPEED_A: f64 = 857.36; // creature.h
const FLAG_NOLIMIT: u32 = 1 << 0; // cylinder.h
const PLAYER_MIN_SPEED: i32 = 10; // player.h PLAYER_MIN_SPEED
```

## When Uncertain About C++ Behavior

1. **Stop immediately** — do not guess or assume behavior
2. State the uncertainty clearly
3. Find the reference source by era:
   - **772 (default):** `gameserver/src/` (primary wire), `tibia-game-master/src/` (mechanics + config constants), or repo-root `src/` (cross-ref). Use built-in Grep/Read or `scripts/ref_grep.sh PATTERN` for 772 reference trees.
   - **1098:** built-in Grep on repo-root `src/`
   - Shell fallback for 772 text search: `scripts/ref_grep.sh PATTERN` (not `rtk grep`)
4. If the C++ source is not available in the workspace, ask the user for clarification
5. Document the exact C++ behavior once confirmed, citing which tree was authoritative

## Lessons Learned

Update `tasks/lessons.md` when:
- C++ behavior deviates from initial assumptions
- Edge cases are discovered
- Non-obvious implementation details are found

## Verification

Before completing any port:
1. Confirm all major functions have C++ references
2. Verify constants match C++ values exactly
3. Check that edge case handling mirrors C++
