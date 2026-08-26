# Depot locker open freeze

**Status:** complete.

Virtual locker `Item.parent` is `None`. Open packet `has_parent` used `discover_item_parent` → full-map `find_item_position` (~518 ms). Skip that scan for unmapped virtual roots.

## Verify

`rtk cargo test -p tfs-rust-core --lib -- virtual_depot_locker_parent_is_none_without_tile`

Restart the server and open a depot locker; it should no longer hitch ~0.5 s.
