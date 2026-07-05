//! Player subsystem — inventory, stats, flags, depot, ping, and combat dispatch.
//!
//! This module consolidates the formerly flat `player_*.rs` / `game_world_player.rs`
//! files at the crate root into one `player/` directory, mirroring the Phase 4
//! `monster_ai.rs` → `monster_ai/` split (`REFACTOR_AUDIT.md` §1/§4). The move is
//! behavior-preserving; per-file `//!` C++ reference headers are kept verbatim.
//!
//! New player combat code (PC-1/PC-2/PC-3/PC-4 per `tasks/player-combat-plan.md`)
//! lands inside `player/combat/` rather than as new crate-root `player_*.rs` files.
//!
//! Compatibility re-exports keep the `crate::player::…` surface stable; crate-root
//! `use player::… as <old_name>` aliases in `lib.rs` keep legacy
//! `crate::player_flags::…` / `crate::player_inventory_util::…` / etc. call sites
//! resolving unchanged until they are repointed opportunistically.

pub(crate) mod combat;
pub(crate) mod depot;
pub(crate) mod flags;
pub(crate) mod inventory;
pub(crate) mod ping;
pub(crate) mod stats;

// Re-exports so `crate::player::<item>` resolves for new code. Legacy
// `crate::player_<name>::<item>` paths are kept alive by aliases in `lib.rs`.
// `#[allow(unused_imports)]` — these are populated as later phases (PC-1+) add
// `crate::player::…` call sites; the pure-move phase (PM) keeps them unused.
#[allow(unused_imports)]
pub(crate) use combat::*;
#[allow(unused_imports)]
pub(crate) use flags::*;
#[allow(unused_imports)]
pub(crate) use inventory::*;
