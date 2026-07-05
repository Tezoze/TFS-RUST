//! Player inventory cylinder queries, load, notifications, and weapon/util helpers.
//!
//! Consolidates the formerly flat `player_inventory_*.rs` files per
//! `tasks/player-combat-plan.md` Phase PM / `REFACTOR_AUDIT.md` §6 "module
//! fragmentation" recommendation. Pure file relocation — no logic edits.

pub(crate) mod load;
pub(crate) mod notifications;
pub(crate) mod query_add;
pub(crate) mod util;

// `#[allow(unused_imports)]` — populated as later phases add `crate::player::inventory::…`
// call sites; the pure-move phase (PM) keeps them unused.
#[allow(unused_imports)]
pub(crate) use notifications::*;
#[allow(unused_imports)]
pub(crate) use query_add::*;
#[allow(unused_imports)]
pub(crate) use util::*;
