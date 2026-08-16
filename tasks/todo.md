# R2 Game.createItem Container userdata — 2026-08-16

**Status:** done.

## Goal
`Game.createItem` returns Container userdata for container types (TFS `setItemMetatable`) so `onUseQuest` can `reward:addItem(...)` into bag/backpack `content`.

## Files
- `crates/tfs-rust-lua/src/userdata/item.rs` — `push_item_userdata`
- `crates/tfs-rust-lua/src/runtime.rs` — `Game.createItem`
- `crates/tfs-rust-lua/src/userdata/container.rs` — addItem/getItem push + `remove`
- `crates/tfs-rust-core/src/game_world_lua_tools.rs` — hydrate on create
- `crates/tfs-rust-core/src/game_world_inventory.rs` — hydrate before `container:addItem`

---

# NPC attack red square lasts one beat — 2026-08-16

**Status:** done.

## Bug
NPC attack reject shows "You may not attack this person." and the red square dies on the same tick. 772 keeps the square for ~Beat ms (50 on `.tibia`) until `SendAll`.

## Cause
Lesson 344 gated `SendAll` on a due beat, but still flushed *after* dispatch. Tokio `Interval` is Ready as soon as the deadline passes (unlike POSIX `SIGALRM` during `ReceiveData`), so the click and `0xA3` rode the same wakeup.

## Fix
Run `send_all_if_beat_pending` at the *start* of the command arm (packets queued before this click). Do not `SendAll` after dispatch.

## Files
- `crates/tfs-rust-core/src/game_loop.rs`
- `tasks/lessons.md`

---

# Login SendAll on beat — 2026-08-16

**Status:** done.

## Bug
Login map burst (`0x0A` / `0x64` / inventory / stats) was flushed as soon as `PlayerLoaded` applied, so the first screen arrived off the 772 beat.

## Cause
772 `TPlayer` ctor / `TakeOver` only `FinishSendData`s; `SendAll` is end of `AdvanceGame` (`main.cc:455`). We called `flush_pending_outgoing` in `handle_player_loaded`.

## Fix
Queue the login burst; let `send_all_if_beat_pending` / beat `obs_advance_beats` `SendAll`. Keep `flush_conn_outgoing` on the *old* takeover socket (disconnect-before-close).

## Files
- `crates/tfs-rust-core/src/game_loop.rs`
- `crates/tfs-rust-core/src/login_out.rs`
- `tasks/lessons.md`
