# Fix for Throwing on Trees and Solid Terrain

## Problem
Players could still throw items onto trees and other solid terrain that have the BLOCK_SOLID flag but not BLOCK_PROJECTILE.

## Root Cause
The `can_throw_to_tile` function was only checking for `BLOCK_PROJECTILE` but not `BLOCK_SOLID`. Trees and many terrain items have `BLOCK_SOLID` set which prevents items from being placed on them.

Additionally, the error message for a failed throw was incorrect.

## Solution Implemented

Updated `can_throw_to_tile` in `crates/tfs-rust-core/src/game_world.rs` to check for both flags:

1. Tile flags now check for both `BLOCK_PROJECTILE` and `BLOCK_SOLID`
2. Ground items now check for both `block_projectile()` and `block_solid()`
3. All items on the tile now check for both blocking properties
4. Changed error message from `NotPossible` to `NotEnoughRoom` to match C++

## Code Changes

- Line 991: Added `BLOCK_SOLID` to tile flag check
- Lines 999-1000: Added `block_solid()` check for ground items
- Lines 1011-1012: Added `block_solid()` check for tile items
- Line 947: Changed error message to `ReturnValue::NotEnoughRoom`

This matches the behavior of C++'s `Tile::queryAdd` which returns `RETURNVALUE_NOTENOUGHROOM` when the tile is blocked.

## Result
Throwing is now properly blocked on:
- Walls (BLOCK_PROJECTILE)
- Trees (BLOCK_SOLID)
- Other solid terrain items
- Any combination of blocking flags

The error message "There is not enough room." now matches the C++ version exactly when throwing onto blocked tiles.
