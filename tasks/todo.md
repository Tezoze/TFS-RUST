# 772 client crash on map teleporters

**Status:** complete.

Walk `NotifyGo` must precede teleport `0x64`. Specials ran inside `internal_move_creature_step` and crashed official 772.

## Done

1. Defer `apply_tile_creature_specials` until after walk packets
2. FX after the teleport move
3. Spectator notify on teleport path
4. Test: `0x6D` before dest `0x64`

## Verify

`rtk cargo test -p tfs-rust-core --lib -- stepping_on_teleport teleport_772`

Restart the server, then step on a forcefield.
