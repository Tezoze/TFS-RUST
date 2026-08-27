# Switch floors transform even with quest aid

**Status:** complete.

Mapped switch floors press/unpress even when the item has a quest aid (demon helmet 3022/3023). Aid Lua still runs for wall removal.

## Verify

`rtk cargo test -p tfs-rust-core --lib -- stepping_tile`
