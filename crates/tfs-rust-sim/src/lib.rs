//! Headless chase/kite scenario harness — depends on `tfs-rust-core`, never the reverse.
//!
//! C++ reference: `chase_kite_scenario.cc`; `crmain.cc` `MoveCreatures`.

pub mod chase_jsonl;
pub mod clock;
pub mod scenario;
pub mod world;

#[cfg(test)]
mod fillmap_tests;

pub use scenario::*;
