# House door tile_store open/closed restore

**Status:** complete.

Bug: open house door + restart → closed OTBM door remained and open door was added on top.
Fix: TFS `loadItem` door↔door / bed↔bed match + `change_item_type`; unmatched stationary discarded.

## Done

1. `find_matching_stationary` — exact id, else door↔door, else bed↔bed
2. `place_loaded_item` — overlay attrs then `change_item_type`; no add on stationary miss
3. `should_save_house_item` — also save by `is_door()`
4. Tests: open-door restore (single item) + unmatched discard
5. Lesson 382

## Verify

`rtk cargo test -p tfs-rust-core house::serialize`

In-game: open house door → server save / restart → one open door, no closed under it.
