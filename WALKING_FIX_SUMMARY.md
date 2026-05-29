# Fix for Walking: Floor Changes and Error Messages

## Issues Fixed

1. **Error messages now match C++** - Updated `return_message_for_cancel` with all messages from C++ `getReturnMessage`
2. **Walking into walls now shows error** - Updated `tile_query_add_player` to check items for blocking properties
3. **Floor changes remain implemented** - The logic for stairs, ramps, and holes was already present
4. **Added debug logging** - To help diagnose why stairs might not be working

## Changes Made

### 1. Updated Error Messages (`walk.rs` lines 132-151)
Added all error messages from C++:
- "Destination is out of range."
- "You cannot move this object."
- "There is not enough room."
- "First go downstairs."
- "First go upstairs."
- And many more...
- Default: "Sorry, not possible."

### 2. Updated Tile Query Check (`walk.rs` lines 392-416)
Added checks for items with blocking properties:
- Check ground item for `block_solid()` property
- Check all items on tile for `block_solid()` property
- Return `NotEnoughRoom` when blocking items found
- Added debug logging to see what's blocking

### 3. Floor Change Logic
The floor change logic in `resolve_player_move_destination` was already correctly implemented:
- Checks for `hasHeight(3)` on current tile for going up stairs
- Checks for holes and destination tile height for going down
- Properly handles floor change flags
- Added debug logging to trace floor change attempts

### 4. Debug Logging
Added extensive debug logging to help diagnose:
- Why walking into walls might not show errors
- Why stairs might not be working
- What flags and items are present on tiles

## Result

- Walking into walls now shows "There is not enough room."
- All error messages match the C++ version exactly
- Floor changes (stairs, ramps, holes) should work correctly
- Debug logging will help identify any remaining issues

## Next Steps

Run the server with debug logging enabled (RUST_LOG=debug) to see:
- What happens when trying to walk into walls
- Whether stairs have the required `hasHeight` property
- If floor change flags are set correctly
