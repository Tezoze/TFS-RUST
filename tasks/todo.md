# Native Handler Migration

**Status:** complete.

## Phase 2 — EventCallback dispatch (ship first)
- [x] Rust-side `has_event_callback` bitset + direct RegistryKey dispatch
- [x] Sync from `EventCallbackData` at end of `load_scripts_interface`

## Phase 1 — MoveEvent aid native path (3000–3123)
- [x] `aid_move_events.rs` + `aid_move_compile.rs` + dispatch hooks + boot log

## Phase 1b — Native spell combat (pure `combat:execute` scripts)
- [x] `spell_combat_compile.rs` — boot parse Combat specs from spell/rune scripts
- [x] `native_spell_combat.rs` — skip `onCastSpell` VM; call `combat_execute_from_lua` directly
- [x] `fire_on_cast_spell` / `fire_on_cast_rune` try native first; boot log `native_spell_combats`
