//! B5 — validate the shipped `data/formulas/{772,1098}.lua` load into the expected era profiles.
//!
//! These exercise the real files a server deploys (not synthetic Lua), confirming the Tier-1 loader
//! overlay matches the built-in `MechanicsProfile::for_version` defaults end-to-end. If someone edits
//! a formulas file in a way that drifts from the era's 772 / TFS constants, this fails loudly.

use std::path::PathBuf;

use tfs_rust_core::formulas::{
    load_mechanics,     ArmorReduction, DistanceKeep, MechanicsProfile, PathCostModel,
    PathSearchModel, SpawnNearPlayer,
    StepSpeedModel, WeakestTargetMetric,
};
use tfs_rust_common::ProtocolVersion;

/// Workspace `data/` dir (two levels up from this crate's manifest).
fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("data")
}

#[test]
fn shipped_1098_formulas_match_era_defaults() {
    let dir = data_dir();
    if !dir.join("formulas").join("1098.lua").is_file() {
        eprintln!("skipping: data/formulas/1098.lua not present");
        return;
    }
    let m = load_mechanics(&dir, ProtocolVersion::V1098);
    let p = &m.profile;
    // The shipped file is a readable mirror of the built-in defaults — they must agree exactly.
    assert_eq!(*p, MechanicsProfile::for_version(ProtocolVersion::V1098));
    assert_eq!(p.beat_ms, 50);
    assert_eq!(p.armor, ArmorReduction::Full);
    assert_eq!(p.path_cost, PathCostModel::Fixed);
    assert_eq!(p.weakest_target_metric, WeakestTargetMetric::MaxHp);
    assert_eq!(p.distance_keep, DistanceKeep::PerType);
    assert_eq!(p.spawn_near_player, SpawnNearPlayer::Block);
    assert_eq!(p.attack_speed_ms, 0);
    assert_eq!(p.step_speed, StepSpeedModel::TfsLog);
    assert_eq!(p.step_beat_ms, 50);
}

#[test]
fn shipped_772_formulas_match_profile_defaults() {
    let dir = data_dir();
    if !dir.join("formulas").join("772.lua").is_file() {
        eprintln!("skipping: data/formulas/772.lua not present");
        return;
    }
    let m = load_mechanics(&dir, ProtocolVersion::V772);
    let p = &m.profile;
    let defaults = MechanicsProfile::for_version(ProtocolVersion::V772);
    // Shipped lua overlays `playerSpeed`; other Tier-1 keys mirror built-in 772 defaults.
    assert_eq!(p.beat_ms, defaults.beat_ms);
    assert_eq!(p.path_cost, defaults.path_cost);
    assert_eq!(p.path_search, defaults.path_search);
    assert_eq!(p.follow_repath_without_path, defaults.follow_repath_without_path);
    assert_eq!(p.beat_ms, 200);
    assert_eq!(p.attack_speed_ms, 0);
    assert_eq!(p.armor, ArmorReduction::Randomized);
    assert_eq!(p.path_cost, PathCostModel::TerrainWeighted);
    assert_eq!(p.path_search, PathSearchModel::Reverse);
    assert_eq!(p.weakest_target_metric, WeakestTargetMetric::CurrentHp);
    assert_eq!(p.distance_keep, DistanceKeep::PerType);
    assert_eq!(p.spawn_near_player, SpawnNearPlayer::RadiusShrink);
    assert_eq!(p.step_speed, StepSpeedModel::LinearGo);
    assert_eq!(p.step_beat_ms, 50);
    assert_eq!(p.conditions.fire.dmg, 10);
    assert_eq!(p.conditions.fire.ticks, 8);
    assert_eq!(p.conditions.energy.dmg, 25);
    assert_eq!(p.conditions.energy.ticks, 10);
    assert_eq!(p.fight_modes.offensive_atk, 1.20);
    assert_eq!(p.fight_modes.defensive_def, 1.80);
    // Tier-2 hooks are unset by default in the shipped file → native fast path.
    assert!(m.hooks.weapon_damage(10, 50, 1, 8).is_none());
}
