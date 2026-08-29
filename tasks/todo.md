# Spectator 0x6D after StepIn doRelocate (bug0000017)

**Status:** complete.

Walk self NotifyGo `A→B`, then StepIn `doRelocate`/`teleportTo` with `dz≠0` was broadcasting spectator `0x6D` `B→C` before the walk’s `A→live`. Spectators still had the walker at `A` → stock 772 `Communication.cpp:1879`.

Fix: `flushing_step_creature` — skip `broadcast_spectator_move` for that mover during their deferred StepIn/Out. Self packets unchanged. Outer walk still sends origin → live.
