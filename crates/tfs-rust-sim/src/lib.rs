//! Headless chase/kite scenario harness — depends on `tfs-rust-core`, never the reverse.
//!
//! C++ reference: `chase_kite_scenario.cc`; `crmain.cc` `MoveCreatures`.

pub mod chase_jsonl;
pub mod clock;
pub mod population;
pub mod scenario;
pub mod sweep;
pub mod world;

#[cfg(test)]
mod fillmap_tests;
#[cfg(test)]
mod sweep_tests;

pub use scenario::*;
