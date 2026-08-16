# Ground addThing + getTopVisibleThing — 2026-08-16

**Status:** done.

## Goal
TFS `Tile::addThing` set/replace ground; `getTopVisibleThing` returns ground `Item*`.

## Work
- `internal_add_item_to_tile`: `isGroundTile` → set or replace `ground`/`ground_item`.
- `tile_get_top_visible_thing`: `LookTarget::Ground` → `ground_item` userdata.
- Tests: createItem drawbridge on empty tile; replace bank; always-on-top dirt does not replace; getTopVisibleThing on hydrated ground.

Dirt 4797/4799 stay OTB group NONE + always-on-top, so rat-bridge overlays are unchanged.
