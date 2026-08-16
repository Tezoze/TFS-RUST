# Cancel-target on attack deny — 2026-08-16

**Status:** done.

## Bug
After "You may not attack this person/creature", the client's red attack square stays. Server is not sending cancel-target (`0xA3`).

## Cause
772 `TCombat::StopAttack(0)` (`crcombat.cc:513-518`) always `SendClearTarget` for players, even when `AttackDest` was already 0. The stock client paints the red square on click before the server answers.

Rust `combat_stop_attack_with_conn` gated `encode_clear_target` on `was_attacking` (`attack_target.is_some()`). First-click deny (NPC, rook PvP, PZ, `NO_ATTACK`) never armed dest, so only the `SendResult` text went out.

## Fix
Send codec `0xA3` for every player `StopAttack(0)`, matching the decompile. Do not use `outgoing_extra::send_cancel_target` (extra `u32(0)`).

## Files
- `crates/tfs-rust-core/src/player/combat/mod.rs`
- `tasks/lessons.md`
