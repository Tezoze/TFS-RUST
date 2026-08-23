# Data-pack scripts (772 fork)

This tree is **not** stock TFS.

- Monster loot is rolled **once, natively, at spawn**. Death only moves that inventory onto the corpse. `onSpawn` may mutate the living monster's items (rarity). Death must never generate loot.
- Script-registry XML (`creaturescripts.xml`, `events.xml`, `movements.xml`, empty action/spell/talkaction/weapon indexes) is **gone**. Everything script-shaped lives here and self-registers.
- The engine loads `eventcallbacks/` and `creaturescripts/` through a **Rust allowlist** (`crates/tfs-rust-lua/src/scripts_interface.rs`). Adding a file to this folder does not enable it. That allowlist defends against **re-import** of upstream generators (`default_onDropLoot`, `droploot`, look doubles, stamina regen), not against the files we ship.
- `globalevents/` XML is Phase 7; do not reintroduce an index file to decide which scripts load.

If an upstream pull brings those generators back, they stay inert until someone adds them to the allowlist — and that addition is a review event.
