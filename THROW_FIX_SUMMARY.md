# Fix for Throwing on Blocked Tiles

## Problem
Players could throw items onto tiles that should be blocked, specifically tiles with the BLOCK_PROJECTILE flag or containing items that block projectiles.

## Root Cause
The throwing mechanism was only checking intermediate tiles in the path (via `can_throw_item_between`), but not checking if the destination tile itself could accept thrown items. The destination tile was only checked later during the move operation via `internal_move_item`.

## Solution
Added a new function `can_throw_to_tile` that checks if a destination tile can accept thrown items by verifying:
1. The tile exists
2. The tile doesn't have the BLOCK_PROJECTILE flag
3. The ground item doesn't block projectiles
4. All items on the tile don't block projectiles

This check is now performed after the path check but before attempting the move, preventing throws to blocked tiles.

## Changes Made

### File: `crates/tfs-rust-core/src/game_world.rs`

1. Added `can_throw_to_tile` function (lines 977-1019)
2. Updated throwing logic to check destination tile (lines 945-949)

## Testing
The fix has been implemented and the code compiles successfully. The throwing mechanism now properly prevents throwing onto tiles that are flagged as blocked or contain blocking items.

## Compatibility
This fix maintains compatibility with the TFS 1.4.2 behavior, where the destination tile check happens in `internalMoveItem` via `queryAdd`. The Rust implementation now performs this check earlier, providing better feedback to the player before attempting the move.
