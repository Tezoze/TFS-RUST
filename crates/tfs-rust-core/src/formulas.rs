//! Era-tunable game-mechanics profile (`MechanicsProfile`) + Lua formula engine.
//!
//! Parallel to the wire `ProtocolCaps`: one struct selected by `clientVersion` carrying every
//! mechanic that diverges between eras (beat quantization, path cost, attack cadence, armor model,
//! fight-mode modifiers, target metric, distance keeping, damage/exp/skill/condition/spell formulas).
//! No `if version` checks scatter through `tfs-rust-core` — the game thread reads this profile.
//!
//! **Two tiers** (design `docs/PROTOCOL_VERSIONING.md` §12.11, §12.13):
//! - **Tier 1** — scalars/tables loaded once into [`MechanicsProfile`] (Copy, zero per-call cost).
//! - **Tier 2** — optional Lua override functions in [`FormulaHooks`]; native fast path when absent.
//!
//! **C++ reference (behavior / outcomes — CipSoft 7.72, clean-room R12):**
//! - Beat 200 ms — `tibia-game-master/src/config.cc` `Beat = 200`.
//! - Speed `GoStrength*2 + 80` — `crmain.cc:445` `TCreature::GetSpeed`.
//! - Step delay `ceil((wp*1000/speed)/Beat)*Beat` — `cract.cc:1462` `TCreature::NotifyGo`.
//! - Path cost terrain-weighted, diagonal 3× — `cract.cc:136–183` `TShortway`.
//! - Attack 2000 ms / defense 2000 ms gate — `crcombat.cc:145,241` `TCombat::DelayAttack`/`GetDefendDamage`.
//! - Melee `max(0, Atk−Def)` then randomized armor `(A/2)+rand(A/2)` — `crcombat.cc:285,649`.
//! - Weapon damage `((rand%100+rand%100)/2)*(skill*5+50)*max/10000` — `crskill.cc:535` `TSkillProbe::ProbeValue`.
//! - Fight modes off `+20%` atk / `−40%` def, def `−40%` atk / `+80%` def — `crcombat.cc:222,250`.
//! - Fire 10 dmg / 8 ticks, energy 25 dmg / 10 ticks — `crskill.cc:1064,1090`.
//! - Spell mult `2*level + 3*magicLevel`, flag clamps — `magic.cc:784` `ComputeDamage`.
//! - Level exp `(((L-6)*L+17)*L-12)/6 * Delta` — `crskill.cc:352` `TSkillLevel::GetExpForLevel`.
//!
//! **C++ reference (structure — TFS 1.4.2 / 10.98 defaults):** repo-root `src/creature.cpp`
//! (`getStepDuration`), `map.cpp` (`getPathMatching` fixed 10/25), `weapons.cpp`, `condition.cpp`,
//! `vocation.cpp`.

use std::path::Path;

use mlua::{Lua, Value};
use tfs_rust_common::ProtocolVersion;

/// A* edge-cost model (`pathfinding.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathCostModel {
    /// TFS 1.4.2 — `MAP_NORMALWALKCOST = 10`, `MAP_DIAGONALWALKCOST = 25` (`map.cpp`).
    Fixed,
    /// CipSoft 7.72 — terrain-speed-weighted waypoints, diagonal costs 3× the tile (`cract.cc` `TShortway`).
    TerrainWeighted,
}

/// A* expansion direction (`pathfinding.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathSearchModel {
    /// TFS 1.4.2 — forward search origin → destination (`map.cpp` `getPathMatching`).
    Forward,
    /// CipSoft 7.72 — reverse search destination → origin (`cract.cc:7` `TShortway`).
    Reverse,
}

/// Armor mitigation model (combat).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmorReduction {
    /// TFS classic — subtract full armor value (`creature.cpp` ~532).
    Full,
    /// CipSoft — `(Armor/2) + rand()%(Armor/2)` when `Armor >= 2` (`crcombat.cc:303`).
    Randomized,
}

/// Monster "weakest target" comparison metric (`monster_ai.rs` target strategy).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeakestTargetMetric {
    /// CipSoft — current HP (`crnonpl.cc` `Strategy`).
    CurrentHp,
    /// TFS — max HP (`monsters.cpp` `<targetstrategy>`).
    MaxHp,
}

/// Monster distance-keeping range (`monster_distance_step.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistanceKeep {
    /// TFS — per-`MonsterType` `targetDistance`.
    PerType,
    /// Fixed override range for all monsters.
    Fixed(i32),
}

/// Weapon-damage formula selector (combat).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageFormula {
    /// CipSoft `ProbeValue` — `((rand%100+rand%100)/2) * max * / 10000` (`crskill.cc:535`).
    ClassicProbe,
    /// TFS modern level/skill weapon formula (`weapons.cpp`).
    Modern,
}

/// Walk/step timing speed model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepSpeedModel {
    /// TFS — per-tile duration from `floor(A*log((stepSpeed/2)+B)+C)` (`creature.cpp` `getStepDuration`).
    TfsLog,
    /// CipSoft / TVP — `GetSpeed = 2*GoStrength+80`, delay `(ground*1000)/speed` (`crmain.cc:445`, `cract.cc:1462`).
    CipSoft,
}

/// Startup-selected player speed scaling policy (loaded once from `formulas.playerSpeed`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerSpeedModel {
    /// Keep era-native behavior (`StepSpeedModel` default for the active protocol version).
    EraDefault,
    /// Classic CipSoft 7.72 linear (`GetSpeed = 2*go + 80`).
    Classic772,
    /// TFS 10.x logarithmic speed curve (`A*ln((go/2)+B)+C`).
    Retail1098,
    /// Logarithmic diminishing-returns curve anchored to classic progression.
    BalancedLog,
}

/// Probe-based physical damage tuning constants (startup-loaded Tier-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DamageProbeTuning {
    /// Multiplier in `attack * (skill * skill_mult + skill_base)`.
    pub skill_mult: i32,
    /// Base term in `attack * (skill * skill_mult + skill_base)`.
    pub skill_base: i32,
    /// Random upper bound (inclusive) for each roll term.
    pub random_max: i32,
}

/// Randomized armor tuning constants (startup-loaded Tier-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArmorRandomTuning {
    /// Minimum armor to use randomized mode.
    pub min_armor_for_random: i32,
    /// Divisor for randomized range (`armor/divisor`).
    pub divisor: i32,
}

/// CipSoft `TCreature::GetSpeed` — `gameserver/src/creature.h` `getSpeed()`.
#[inline]
pub fn cipsoft_effective_speed(go_strength: i32) -> i32 {
    go_strength.saturating_mul(2).saturating_add(80).max(1)
}

/// Spawn behavior when a player is near the spawn point (`spawn_lifecycle.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnNearPlayer {
    /// TFS — block the spawn while a player occupies the block tile (`spawn.cpp`).
    Block,
    /// CipSoft — shrink the spawn radius, still spawn further out (`crnonpl.cc:1414`).
    RadiusShrink,
}

/// Per-level experience curve (`skills` module).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelExpModel {
    /// TFS `Player::getExpForLevel` (`player.h:171`) — `(((L-6)*L+17)*L-12)/6 * 100`. Identical
    /// polynomial to CipSoft with `Delta = 100`.
    Tfs,
    /// CipSoft `(((L-6)*L+17)*L-12)/6 * delta` (`crskill.cc:352`) — same shape, era `Delta`.
    CipSoftPoly,
}

/// Attack/defense fight-mode multipliers (offensive / balanced / defensive).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FightModes {
    /// Attack multiplier when in offensive mode (CipSoft `1.20`, TFS `1.20`).
    pub offensive_atk: f64,
    /// Attack multiplier when in defensive mode (CipSoft `0.60`, TFS `0.80`).
    pub defensive_atk: f64,
    /// Defense multiplier when in offensive mode (CipSoft `0.60`, TFS `0.80`).
    pub offensive_def: f64,
    /// Defense multiplier when in defensive mode (CipSoft `1.80`, TFS `1.20`).
    pub defensive_def: f64,
}

/// A single damage-over-time condition tick (`condition.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TickSpec {
    /// Damage per tick.
    pub dmg: i32,
    /// Number of ticks (cycles).
    pub ticks: i32,
}

/// DoT condition constants by element.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConditionTicks {
    /// CipSoft fire = `{10, 8}` (`crskill.cc:1064`).
    pub fire: TickSpec,
    /// CipSoft energy = `{25, 10}` (`crskill.cc:1090`).
    pub energy: TickSpec,
    /// Poison initial damage decays by `FactorPercent` per tick — start damage anchor (`crskill.cc:969`).
    pub poison_start: i32,
}

/// Spell damage coefficients — `damage * (level_mult*level + magic_mult*magicLevel) / 100` (`magic.cc:784`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellCoeff {
    pub level_mult: i32,
    pub magic_mult: i32,
}

/// Era-tuned mechanics knobs (Tier-1). `Copy` — read freely on the game thread, no per-call cost.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MechanicsProfile {
    /// Scheduler / combat beat (CipSoft 200, TFS 50) — not always walk quantization on 772.
    pub beat_ms: u32,
    /// Per-tile walk delay quantization — 50 ms both shipped eras (TVP `gameserver` for 772).
    pub step_beat_ms: u32,
    /// Per-tile walk duration curve (TFS log vs CipSoft linear speed).
    pub step_speed: StepSpeedModel,
    /// Player speed scaling model loaded once from formulas.
    pub player_speed_model: PlayerSpeedModel,
    /// A* edge-cost model.
    pub path_cost: PathCostModel,
    /// A* expansion direction — forward (TFS) vs reverse (CipSoft `TShortway`).
    pub path_search: PathSearchModel,
    /// Flat attack interval in ms; `0` = use vocation/weapon `getAttackSpeed`.
    pub attack_speed_ms: u32,
    /// Defense re-roll gate in ms (CipSoft 2000).
    pub defense_gate_ms: u32,
    /// Armor mitigation model.
    pub armor: ArmorReduction,
    /// Fight-mode attack/defense multipliers.
    pub fight_modes: FightModes,
    /// Monster weakest-target metric.
    pub weakest_target_metric: WeakestTargetMetric,
    /// Monster distance-keeping range.
    pub distance_keep: DistanceKeep,
    /// Weapon damage formula.
    pub damage_formula: DamageFormula,
    /// Probe-value damage constants.
    pub damage_probe: DamageProbeTuning,
    /// Randomized armor constants.
    pub armor_random: ArmorRandomTuning,
    /// DoT condition constants.
    pub conditions: ConditionTicks,
    /// Spawn-near-player policy.
    pub spawn_near_player: SpawnNearPlayer,
    /// Exp attribution window in combat rounds (CipSoft 60).
    pub exp_attribution_rounds: u32,
    /// PvP exp cap fraction numerator/denominator (CipSoft `11/10`).
    pub pvp_exp_cap_num: u32,
    pub pvp_exp_cap_den: u32,
    /// Spell damage coefficients.
    pub spell_coeff: SpellCoeff,
    /// Per-level experience curve.
    pub level_exp: LevelExpModel,
    /// CipSoft level-exp `Delta` multiplier (TFS uses the same polynomial with `Delta = 100`).
    pub level_exp_delta: i64,
    /// When true, repath on follow-target move even if `has_follow_path` is false (CipSoft).
    /// TFS 1098 requires an active follow path before repathing (`creature.cpp:619`).
    pub follow_repath_without_path: bool,
}

impl MechanicsProfile {
    /// Built-in defaults per era — fallback when `data/formulas/<v>.lua` is absent.
    pub fn for_version(version: ProtocolVersion) -> Self {
        match version.raw() {
            772 => Self {
                beat_ms: 200,
                // TVP `gameserver/src/creature.cpp` — `50 * ((50 + 1000*wp/speed - 1) / 50)`.
                step_beat_ms: 50,
                step_speed: StepSpeedModel::CipSoft,
                player_speed_model: PlayerSpeedModel::BalancedLog,
                path_cost: PathCostModel::TerrainWeighted,
                path_search: PathSearchModel::Reverse,
                // Use vocation/weapon attack speed from `vocations.xml` like 1098.
                attack_speed_ms: 0,
                defense_gate_ms: 2000,
                armor: ArmorReduction::Randomized,
                fight_modes: FightModes {
                    offensive_atk: 1.20,
                    defensive_atk: 0.60,
                    offensive_def: 0.60,
                    defensive_def: 1.80,
                },
                weakest_target_metric: WeakestTargetMetric::CurrentHp,
                distance_keep: DistanceKeep::PerType,
                damage_formula: DamageFormula::ClassicProbe,
                damage_probe: DamageProbeTuning {
                    skill_mult: 5,
                    skill_base: 50,
                    random_max: 99,
                },
                armor_random: ArmorRandomTuning {
                    min_armor_for_random: 2,
                    divisor: 2,
                },
                conditions: ConditionTicks {
                    fire: TickSpec { dmg: 10, ticks: 8 },
                    energy: TickSpec { dmg: 25, ticks: 10 },
                    poison_start: 50,
                },
                spawn_near_player: SpawnNearPlayer::RadiusShrink,
                exp_attribution_rounds: 60,
                pvp_exp_cap_num: 11,
                pvp_exp_cap_den: 10,
                spell_coeff: SpellCoeff {
                    level_mult: 2,
                    magic_mult: 3,
                },
                level_exp: LevelExpModel::CipSoftPoly,
                level_exp_delta: 100,
                follow_repath_without_path: true,
            },
            1098 => Self {
                beat_ms: 50,
                step_beat_ms: 50,
                step_speed: StepSpeedModel::TfsLog,
                player_speed_model: PlayerSpeedModel::Retail1098,
                path_cost: PathCostModel::Fixed,
                path_search: PathSearchModel::Forward,
                attack_speed_ms: 0,
                defense_gate_ms: 2000,
                armor: ArmorReduction::Full,
                fight_modes: FightModes {
                    offensive_atk: 1.20,
                    defensive_atk: 0.80,
                    offensive_def: 0.80,
                    defensive_def: 1.20,
                },
                weakest_target_metric: WeakestTargetMetric::MaxHp,
                distance_keep: DistanceKeep::PerType,
                damage_formula: DamageFormula::Modern,
                damage_probe: DamageProbeTuning {
                    skill_mult: 5,
                    skill_base: 50,
                    random_max: 99,
                },
                armor_random: ArmorRandomTuning {
                    min_armor_for_random: 2,
                    divisor: 2,
                },
                conditions: ConditionTicks {
                    fire: TickSpec { dmg: 10, ticks: 8 },
                    energy: TickSpec { dmg: 25, ticks: 10 },
                    poison_start: 50,
                },
                spawn_near_player: SpawnNearPlayer::Block,
                exp_attribution_rounds: 60,
                pvp_exp_cap_num: 11,
                pvp_exp_cap_den: 10,
                spell_coeff: SpellCoeff {
                    level_mult: 2,
                    magic_mult: 3,
                },
                level_exp: LevelExpModel::Tfs,
                level_exp_delta: 100,
                follow_repath_without_path: false,
            },
            other => unreachable!("unsupported protocol version {other}"),
        }
    }
}

/// Both tiers, threaded onto the game thread. Built by [`load_mechanics`].
pub struct Mechanics {
    /// Tier-1 scalars/tables (Copy).
    pub profile: MechanicsProfile,
    /// Tier-2 optional Lua override functions.
    pub hooks: FormulaHooks,
}

impl Mechanics {
    /// Built-in defaults for `version`, no Tier-2 overrides (used by tests / missing-file fallback).
    pub fn for_version(version: ProtocolVersion) -> Self {
        Self {
            profile: MechanicsProfile::for_version(version),
            hooks: FormulaHooks::default(),
        }
    }
}

/// Tier-2 Lua formula overrides. Each `Option` is `Some` only if the script registered that function.
/// Owns its own `Lua` VM (game-thread only, `!Send`) loaded from `data/formulas/<v>.lua`.
#[derive(Default)]
pub struct FormulaHooks {
    /// Kept alive so the stored [`mlua::Function`]s remain valid.
    _lua: Option<Lua>,
    get_weapon_damage: Option<mlua::Function>,
    get_armor_reduction: Option<mlua::Function>,
    get_defense: Option<mlua::Function>,
    get_attack_speed: Option<mlua::Function>,
    get_step_duration: Option<mlua::Function>,
    get_creature_speed: Option<mlua::Function>,
    get_experience_for_level: Option<mlua::Function>,
    get_req_skill_tries: Option<mlua::Function>,
    get_spell_damage: Option<mlua::Function>,
    get_condition_tick: Option<mlua::Function>,
}

impl std::fmt::Debug for FormulaHooks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FormulaHooks")
            .field("get_weapon_damage", &self.get_weapon_damage.is_some())
            .field("get_armor_reduction", &self.get_armor_reduction.is_some())
            .field("get_defense", &self.get_defense.is_some())
            .field("get_attack_speed", &self.get_attack_speed.is_some())
            .field("get_step_duration", &self.get_step_duration.is_some())
            .field("get_creature_speed", &self.get_creature_speed.is_some())
            .field("get_experience_for_level", &self.get_experience_for_level.is_some())
            .field("get_req_skill_tries", &self.get_req_skill_tries.is_some())
            .field("get_spell_damage", &self.get_spell_damage.is_some())
            .field("get_condition_tick", &self.get_condition_tick.is_some())
            .finish()
    }
}

impl FormulaHooks {
    fn from_lua(lua: Lua) -> Self {
        let f = |name: &str| -> Option<mlua::Function> {
            match lua.globals().get::<Value>(name) {
                Ok(Value::Function(func)) => Some(func),
                _ => None,
            }
        };
        Self {
            get_weapon_damage: f("getWeaponDamage"),
            get_armor_reduction: f("getArmorReduction"),
            get_defense: f("getDefense"),
            get_attack_speed: f("getAttackSpeed"),
            get_step_duration: f("getStepDuration"),
            get_creature_speed: f("getCreatureSpeed"),
            get_experience_for_level: f("getExperienceForLevel"),
            get_req_skill_tries: f("getReqSkillTries"),
            get_spell_damage: f("getSpellDamage"),
            get_condition_tick: f("getConditionTick"),
            _lua: Some(lua),
        }
    }

    /// Test-only: build hooks from an already-loaded VM (mirrors [`Self::from_lua`]).
    #[cfg(test)]
    pub(crate) fn from_lua_for_test(lua: Lua) -> Self {
        Self::from_lua(lua)
    }

    /// Tier-2 `getWeaponDamage(skill, attack, mode, level)` — `None` if unregistered (use native).
    pub fn weapon_damage(&self, skill: i32, attack: i32, mode: i32, level: i32) -> Option<i32> {
        let func = self.get_weapon_damage.as_ref()?;
        func.call::<i32>((skill, attack, mode, level)).ok()
    }

    /// Tier-2 `getArmorReduction(armor)`.
    pub fn armor_reduction(&self, armor: i32) -> Option<i32> {
        let func = self.get_armor_reduction.as_ref()?;
        func.call::<i32>(armor).ok()
    }

    /// Tier-2 `getDefense(skill, defense, mode)`.
    pub fn defense(&self, skill: i32, defense: i32, mode: i32) -> Option<i32> {
        let func = self.get_defense.as_ref()?;
        func.call::<i32>((skill, defense, mode)).ok()
    }

    /// Tier-2 `getAttackSpeed(attacker_speed)` — ms.
    pub fn attack_speed(&self, attacker_speed: i32) -> Option<i32> {
        let func = self.get_attack_speed.as_ref()?;
        func.call::<i32>(attacker_speed).ok()
    }

    /// Tier-2 `getStepDuration(speed, ground, diagonal)` — ms.
    pub fn step_duration(&self, speed: i32, ground: i32, diagonal: bool) -> Option<i64> {
        let func = self.get_step_duration.as_ref()?;
        func.call::<i64>((speed, ground, diagonal)).ok()
    }

    /// Tier-2 `getCreatureSpeed(base, var)`.
    pub fn creature_speed(&self, base: i32, var: i32) -> Option<i32> {
        let func = self.get_creature_speed.as_ref()?;
        func.call::<i32>((base, var)).ok()
    }

    /// Tier-2 `getExperienceForLevel(level)`.
    pub fn experience_for_level(&self, level: i32) -> Option<i64> {
        let func = self.get_experience_for_level.as_ref()?;
        func.call::<i64>(level).ok()
    }

    /// Tier-2 `getReqSkillTries(skill, level)`.
    pub fn req_skill_tries(&self, skill: i32, level: i32) -> Option<i64> {
        let func = self.get_req_skill_tries.as_ref()?;
        func.call::<i64>((skill, level)).ok()
    }

    /// Tier-2 `getSpellDamage(level, magicLevel, base)`.
    pub fn spell_damage(&self, level: i32, magic_level: i32, base: i32) -> Option<i32> {
        let func = self.get_spell_damage.as_ref()?;
        func.call::<i32>((level, magic_level, base)).ok()
    }

    /// Tier-2 `getConditionTick(type, round)` → `(dmg, ticks)`.
    pub fn condition_tick(&self, ctype: i32, round: i32) -> Option<(i32, i32)> {
        let func = self.get_condition_tick.as_ref()?;
        func.call::<(i32, i32)>((ctype, round)).ok()
    }
}

/// Read `formulas.<key>` (a number) from the loaded VM, falling back to `default`.
fn num_or(lua: &Lua, table: &mlua::Table, key: &str, default: i64) -> i64 {
    let _ = lua;
    match table.get::<Value>(key) {
        Ok(Value::Integer(i)) => i,
        Ok(Value::Number(n)) => n as i64,
        _ => default,
    }
}

fn float_or(table: &mlua::Table, key: &str, default: f64) -> f64 {
    match table.get::<Value>(key) {
        Ok(Value::Number(n)) => n,
        Ok(Value::Integer(i)) => i as f64,
        _ => default,
    }
}

fn str_or(table: &mlua::Table, key: &str, default: &str) -> String {
    match table.get::<Value>(key) {
        Ok(Value::String(s)) => s.to_string_lossy(),
        _ => default.to_string(),
    }
}

fn bool_or(table: &mlua::Table, key: &str, default: bool) -> bool {
    match table.get::<Value>(key) {
        Ok(Value::Boolean(b)) => b,
        _ => default,
    }
}

/// Load `data/formulas/<clientVersion>.lua` into [`Mechanics`]. Missing file → built-in
/// [`MechanicsProfile::for_version`] defaults with no Tier-2 hooks.
///
/// Tier-1 constants come from the global `formulas = { … }` table; Tier-2 overrides are global
/// functions (`getWeaponDamage`, …). All values default to the era profile when absent, so a
/// partial script only overrides what it sets — never a magic number buried in Rust (R11).
pub fn load_mechanics(data_dir: &Path, version: ProtocolVersion) -> Mechanics {
    let path = data_dir.join("formulas").join(format!("{}.lua", version.raw()));
    if !path.is_file() {
        tracing::info!(
            file = %path.display(),
            version = %version,
            "no formulas file; using built-in MechanicsProfile defaults"
        );
        return Mechanics::for_version(version);
    }

    let chunk = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(file = %path.display(), error = %e, "read formulas failed; using defaults");
            return Mechanics::for_version(version);
        }
    };

    let lua = Lua::new();
    if let Err(e) = lua.load(&chunk).set_name(path.display().to_string()).exec() {
        tracing::warn!(file = %path.display(), error = %e, "exec formulas failed; using defaults");
        return Mechanics::for_version(version);
    }

    let defaults = MechanicsProfile::for_version(version);
    let profile = parse_profile(&lua, defaults);
    tracing::info!(file = %path.display(), version = %version, "loaded mechanics formulas");
    Mechanics {
        profile,
        hooks: FormulaHooks::from_lua(lua),
    }
}

/// Overlay the `formulas` global table onto `defaults`. Any field not present keeps its era default.
fn parse_profile(lua: &Lua, defaults: MechanicsProfile) -> MechanicsProfile {
    let Ok(Value::Table(formulas)) = lua.globals().get::<Value>("formulas") else {
        return defaults;
    };

    let mut p = defaults;
    p.beat_ms = num_or(lua, &formulas, "beatMs", p.beat_ms as i64).max(1) as u32;
    p.step_beat_ms =
        num_or(lua, &formulas, "stepBeatMs", p.step_beat_ms.max(1) as i64).max(1) as u32;
    p.attack_speed_ms = num_or(lua, &formulas, "attackSpeedMs", p.attack_speed_ms as i64).max(0) as u32;
    p.defense_gate_ms = num_or(lua, &formulas, "defenseGateMs", p.defense_gate_ms as i64).max(0) as u32;
    p.exp_attribution_rounds =
        num_or(lua, &formulas, "expAttributionRounds", p.exp_attribution_rounds as i64).max(1) as u32;

    p.armor = match str_or(&formulas, "armor", "").as_str() {
        "randomized" => ArmorReduction::Randomized,
        "full" => ArmorReduction::Full,
        _ => p.armor,
    };
    p.path_cost = match str_or(&formulas, "pathCost", "").as_str() {
        "terrain" | "terrainWeighted" => PathCostModel::TerrainWeighted,
        "fixed" => PathCostModel::Fixed,
        _ => p.path_cost,
    };
    p.path_search = match str_or(&formulas, "pathSearch", "").as_str() {
        "reverse" | "cipsoft" | "shortway" => PathSearchModel::Reverse,
        "forward" | "tfs" => PathSearchModel::Forward,
        _ => p.path_search,
    };
    p.step_speed = match str_or(&formulas, "stepSpeedModel", "").as_str() {
        "cipsoft" | "cip" => StepSpeedModel::CipSoft,
        "tfs" | "tfsLog" => StepSpeedModel::TfsLog,
        _ => p.step_speed,
    };
    p.player_speed_model = match str_or(&formulas, "playerSpeed", "").as_str() {
        "772" => PlayerSpeedModel::Classic772,
        "retail" | "1098" => PlayerSpeedModel::Retail1098,
        "balanced" => PlayerSpeedModel::BalancedLog,
        _ => p.player_speed_model,
    };
    p.weakest_target_metric = match str_or(&formulas, "weakestTargetMetric", "").as_str() {
        "current" | "currentHp" => WeakestTargetMetric::CurrentHp,
        "max" | "maxHp" => WeakestTargetMetric::MaxHp,
        _ => p.weakest_target_metric,
    };
    p.spawn_near_player = match str_or(&formulas, "spawnNearPlayer", "").as_str() {
        "shrink" | "radiusShrink" => SpawnNearPlayer::RadiusShrink,
        "block" => SpawnNearPlayer::Block,
        _ => p.spawn_near_player,
    };
    p.damage_formula = match str_or(&formulas, "damageFormula", "").as_str() {
        "classic" | "probe" => DamageFormula::ClassicProbe,
        "modern" => DamageFormula::Modern,
        _ => p.damage_formula,
    };
    p.level_exp = match str_or(&formulas, "levelExp", "").as_str() {
        "cipsoft" | "poly" => LevelExpModel::CipSoftPoly,
        "tfs" => LevelExpModel::Tfs,
        _ => p.level_exp,
    };
    p.level_exp_delta = num_or(lua, &formulas, "levelExpDelta", p.level_exp_delta).max(1);
    p.follow_repath_without_path =
        bool_or(&formulas, "followRepathWithoutPath", p.follow_repath_without_path);

    // distanceKeep: integer = Fixed(n); "perType" string keeps per-type.
    match formulas.get::<Value>("distanceKeep") {
        Ok(Value::Integer(i)) => p.distance_keep = DistanceKeep::Fixed(i as i32),
        Ok(Value::Number(n)) => p.distance_keep = DistanceKeep::Fixed(n as i32),
        Ok(Value::String(s)) if s.to_string_lossy() == "perType" => {
            p.distance_keep = DistanceKeep::PerType;
        }
        _ => {}
    }

    if let Ok(Value::Table(fm)) = formulas.get::<Value>("fightModes") {
        p.fight_modes = FightModes {
            offensive_atk: float_or(&fm, "offensiveAtk", p.fight_modes.offensive_atk),
            defensive_atk: float_or(&fm, "defensiveAtk", p.fight_modes.defensive_atk),
            offensive_def: float_or(&fm, "offensiveDef", p.fight_modes.offensive_def),
            defensive_def: float_or(&fm, "defensiveDef", p.fight_modes.defensive_def),
        };
    }

    if let Ok(Value::Table(conds)) = formulas.get::<Value>("conditions") {
        if let Ok(Value::Table(fire)) = conds.get::<Value>("fire") {
            p.conditions.fire = TickSpec {
                dmg: num_or(lua, &fire, "dmg", p.conditions.fire.dmg as i64) as i32,
                ticks: num_or(lua, &fire, "ticks", p.conditions.fire.ticks as i64) as i32,
            };
        }
        if let Ok(Value::Table(energy)) = conds.get::<Value>("energy") {
            p.conditions.energy = TickSpec {
                dmg: num_or(lua, &energy, "dmg", p.conditions.energy.dmg as i64) as i32,
                ticks: num_or(lua, &energy, "ticks", p.conditions.energy.ticks as i64) as i32,
            };
        }
        p.conditions.poison_start =
            num_or(lua, &conds, "poisonStart", p.conditions.poison_start as i64) as i32;
    }

    if let Ok(Value::Table(sp)) = formulas.get::<Value>("spell") {
        p.spell_coeff = SpellCoeff {
            level_mult: num_or(lua, &sp, "levelMult", p.spell_coeff.level_mult as i64) as i32,
            magic_mult: num_or(lua, &sp, "magicMult", p.spell_coeff.magic_mult as i64) as i32,
        };
    }

    if let Ok(Value::Table(dmg)) = formulas.get::<Value>("damageTuning") {
        p.damage_probe = DamageProbeTuning {
            skill_mult: num_or(lua, &dmg, "skillMult", p.damage_probe.skill_mult as i64).max(0) as i32,
            skill_base: num_or(lua, &dmg, "skillBase", p.damage_probe.skill_base as i64).max(0) as i32,
            random_max: num_or(lua, &dmg, "randomMax", p.damage_probe.random_max as i64)
                .clamp(1, i32::MAX as i64) as i32,
        };
    }
    if let Ok(Value::Table(ar)) = formulas.get::<Value>("armorTuning") {
        p.armor_random = ArmorRandomTuning {
            min_armor_for_random: num_or(
                lua,
                &ar,
                "minArmorForRandom",
                p.armor_random.min_armor_for_random as i64,
            )
            .max(0) as i32,
            divisor: num_or(lua, &ar, "divisor", p.armor_random.divisor as i64).max(1) as i32,
        };
    }

    if let Ok(Value::Table(pvp)) = formulas.get::<Value>("pvpExpCap") {
        p.pvp_exp_cap_num = num_or(lua, &pvp, "num", p.pvp_exp_cap_num as i64).max(0) as u32;
        p.pvp_exp_cap_den = num_or(lua, &pvp, "den", p.pvp_exp_cap_den as i64).max(1) as u32;
    }

    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_1098_match_today_constants() {
        let p = MechanicsProfile::for_version(ProtocolVersion::V1098);
        assert_eq!(p.beat_ms, 50);
        assert_eq!(p.path_cost, PathCostModel::Fixed);
        assert_eq!(p.path_search, PathSearchModel::Forward);
        assert_eq!(p.attack_speed_ms, 0);
        assert_eq!(p.armor, ArmorReduction::Full);
        assert_eq!(p.weakest_target_metric, WeakestTargetMetric::MaxHp);
        assert_eq!(p.distance_keep, DistanceKeep::PerType);
        assert_eq!(p.spawn_near_player, SpawnNearPlayer::Block);
        assert_eq!(p.level_exp, LevelExpModel::Tfs);
        assert_eq!(p.step_speed, StepSpeedModel::TfsLog);
        assert!(!p.follow_repath_without_path);
    }

    #[test]
    fn cipsoft_effective_speed_matches_gameserver() {
        assert_eq!(cipsoft_effective_speed(42), 164);
        assert_eq!(cipsoft_effective_speed(0), 80);
    }

    #[test]
    fn defaults_772_match_cipsoft() {
        let p = MechanicsProfile::for_version(ProtocolVersion::V772);
        assert_eq!(p.beat_ms, 200);
        assert_eq!(p.path_cost, PathCostModel::TerrainWeighted);
        assert_eq!(p.path_search, PathSearchModel::Reverse);
        assert_eq!(p.attack_speed_ms, 0);
        assert_eq!(p.armor, ArmorReduction::Randomized);
        assert_eq!(p.weakest_target_metric, WeakestTargetMetric::CurrentHp);
        assert_eq!(p.distance_keep, DistanceKeep::PerType);
        assert_eq!(p.spawn_near_player, SpawnNearPlayer::RadiusShrink);
        assert_eq!(p.level_exp, LevelExpModel::CipSoftPoly);
        assert_eq!(p.step_speed, StepSpeedModel::CipSoft);
        assert_eq!(p.step_beat_ms, 50);
        assert_eq!(p.conditions.fire, TickSpec { dmg: 10, ticks: 8 });
        assert_eq!(p.conditions.energy, TickSpec { dmg: 25, ticks: 10 });
        assert_eq!(p.fight_modes.defensive_def, 1.80);
        assert!(p.follow_repath_without_path);
    }

    #[test]
    fn missing_file_falls_back_to_defaults() {
        let dir = std::env::temp_dir().join("tfs_formulas_missing_test");
        let m = load_mechanics(&dir, ProtocolVersion::V1098);
        assert_eq!(m.profile, MechanicsProfile::for_version(ProtocolVersion::V1098));
        assert!(m.hooks.weapon_damage(10, 50, 0, 8).is_none());
    }

    #[test]
    fn partial_table_overlays_onto_defaults() {
        let lua = Lua::new();
        lua.load(r#"formulas = { beatMs = 100, armor = "randomized" }"#)
            .exec()
            .unwrap();
        let p = parse_profile(&lua, MechanicsProfile::for_version(ProtocolVersion::V1098));
        assert_eq!(p.beat_ms, 100);
        assert_eq!(p.armor, ArmorReduction::Randomized);
        // Untouched fields keep their 1098 default.
        assert_eq!(p.path_cost, PathCostModel::Fixed);
        assert_eq!(p.distance_keep, DistanceKeep::PerType);
    }

    #[test]
    fn tier2_hook_used_when_registered() {
        let lua = Lua::new();
        lua.load(
            r#"
            function getWeaponDamage(skill, attack, mode, level)
                return skill + attack + mode + level
            end
            "#,
        )
        .exec()
        .unwrap();
        let hooks = FormulaHooks::from_lua(lua);
        assert_eq!(hooks.weapon_damage(1, 2, 3, 4), Some(10));
        // Unregistered hook stays native (None).
        assert_eq!(hooks.spell_damage(1, 2, 3), None);
    }

    #[test]
    fn nested_condition_table_parses() {
        let lua = Lua::new();
        lua.load(
            r#"formulas = { conditions = { fire = {dmg=12, ticks=9}, energy = {dmg=30, ticks=12} } }"#,
        )
        .exec()
        .unwrap();
        let p = parse_profile(&lua, MechanicsProfile::for_version(ProtocolVersion::V772));
        assert_eq!(p.conditions.fire, TickSpec { dmg: 12, ticks: 9 });
        assert_eq!(p.conditions.energy, TickSpec { dmg: 30, ticks: 12 });
    }
}
