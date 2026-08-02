//! Era-tunable combat / skill / condition / spell formulas (Track B, Phase B4).
//!
//! Pure functions over [`MechanicsProfile`] (Tier-1 constants) + [`FormulaHooks`] (Tier-2 Lua
//! overrides). The combat *execution* loop is still a skeleton (`combat/mod.rs`, design §12.7/§12.9);
//! this module is the math it will call once wired, and is fully unit-testable today.
//!
//! **C++ reference — behavior/outcomes (772, clean-room R12):**
//! - Weapon damage `((rand%100+rand%100)/2) * Max * / 10000`, `Max = attack*(skill*5+50)` —
//!   `tibia-game-master/src/crskill.cc:535` `TSkillProbe::ProbeValue`, `crcombat.cc:219` `GetAttackDamage`.
//! - Fight modes: offensive `+20%` atk / `−40%` def; defensive `−40%` atk / `+80%` def —
//!   `crcombat.cc:222–227` (`GetAttackDamage`), `:250–256` (`GetDefendDamage`).
//! - Melee `Damage = max(0, Attack − Defense)`; randomized armor `(A/2)+rand%(A/2)` when `A>=2` —
//!   `crcombat.cc:649–653`, `:302–304` `GetArmorStrength`.
//! - Attack/defense cadence: 2000 ms each — `crcombat.cc:145,640` `DelayAttack(2000)`, `:241` defense gate.
//! - Level exp `(((L-6)*L+17)*L-12)/6 * Delta` — `crskill.cc:352` `TSkillLevel::GetExpForLevel`.
//! - Skill tries geometric `Delta * (b^(act-min) ... )`, `b = FactorPercent/1000` — `crskill.cc:483–499`.
//! - Spell damage `(2*level + 3*magicLevel)` % multiplier, flag clamps — `magic.cc:784` `ComputeDamage`.
//! - Exp distribution 20-slot proportional; PvP MaxLevel scale `(L*11)/10` — `crcombat.cc:891–934`.
//! - Condition ticks fire 10/8, energy 25/10 — `crskill.cc:1064,1090`.
//!
//! **C++ reference — structure (TFS 1.4.2 / 10.98):** repo-root `src/weapons.cpp`,
//! `creature.cpp:500–533` (`blockHit`), `vocation.cpp`, `condition.cpp:1330`, `spells.cpp`,
//! `player.cpp` (`getExpForLevel`).

use crate::formulas::{
    ArmorReduction, ConditionTicks, DamageFormula, DamageProbeTuning, FightModes, FormulaHooks,
    LevelExpModel, MechanicsProfile,
};
use crate::sim_glibc_rand::GlibcRngState;

/// Player fight stance (`ATTACK_MODE_*` in CipSoft, `fightMode_t` in TFS).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FightMode {
    Offensive,
    #[default]
    Balanced,
    Defensive,
}

impl FightMode {
    /// Wire byte from `0xA7` `FIGHT_MODES` (`raw_fight_mode`): 1 = offensive, 2 = balanced, 3 = defensive.
    /// Opcode is `0xA7` client→server (`receiving.cc` `FIGHT_MODES`); `0xA0` is the *server→client*
    /// `AddPlayerStats` opcode (see `tasks/player-combat-plan.md` §0.2).
    pub fn from_wire(raw: u8) -> Self {
        match raw {
            1 => FightMode::Offensive,
            3 => FightMode::Defensive,
            _ => FightMode::Balanced,
        }
    }

    /// Integer code passed to Tier-2 hooks (mirrors 772 `ATTACK_MODE_*`: 1/2/3).
    pub(crate) fn code(self) -> i32 {
        match self {
            FightMode::Offensive => 1,
            FightMode::Balanced => 2,
            FightMode::Defensive => 3,
        }
    }
}

// ---------------------------------------------------------------------------
// B4.1 — attack / defense cadence
// ---------------------------------------------------------------------------

/// Milliseconds between attacks.
///
/// `attack_speed_ms == 0` ⇒ use the vocation/weapon `vocation_attack_speed_ms` (`vocations.xml`
/// / TFS `getAttackSpeed`) for both eras.
/// A registered Tier-2 `getAttackSpeed(attacker_speed)` overrides both.
pub fn attack_speed_ms(
    profile: &MechanicsProfile,
    hooks: &FormulaHooks,
    vocation_attack_speed_ms: i32,
) -> i32 {
    if let Some(ms) = hooks.attack_speed(vocation_attack_speed_ms) {
        return ms.max(0);
    }
    if profile.attack_speed_ms == 0 {
        vocation_attack_speed_ms.max(0)
    } else {
        profile.attack_speed_ms as i32
    }
}

/// Defense re-roll gate in ms (`crcombat.cc:241` `EarliestDefendTime = LastDefendTime + 2000`).
pub fn defense_gate_ms(profile: &MechanicsProfile) -> i32 {
    profile.defense_gate_ms as i32
}

// ---------------------------------------------------------------------------
// B4.4 — fight-mode modifiers
// ---------------------------------------------------------------------------

/// Apply fight-mode scaling with 772 integer tenths (`crcombat.cc:222–227,250–256`).
///
/// `Max ± (Max * k) / 10` where `k = round((factor - 1) * 10)`. Matches decompile truncation
/// for classic ±20%/±40%/±80% ratios (e.g. defensive atk 7 → 5, not `floor(7*0.6)=4`).
/// Era-tunable `FightModes` floats still select `k` (1098 `defensive_atk=0.80` → −2/10).
fn apply_mode_integer_tenths(max_value: i32, factor: f64) -> i32 {
    let tenths = ((factor - 1.0) * 10.0).round() as i32;
    max_value.saturating_add(max_value.saturating_mul(tenths) / 10)
}

/// Apply the fight-mode attack multiplier to a max attack value.
fn apply_attack_mode(modes: &FightModes, mode: FightMode, max_value: i32) -> i32 {
    let f = match mode {
        FightMode::Offensive => modes.offensive_atk,
        FightMode::Defensive => modes.defensive_atk,
        FightMode::Balanced => 1.0,
    };
    apply_mode_integer_tenths(max_value, f)
}

/// Apply the fight-mode defense multiplier to a max defense value.
fn apply_defense_mode(modes: &FightModes, mode: FightMode, max_value: i32) -> i32 {
    let f = match mode {
        FightMode::Offensive => modes.offensive_def,
        FightMode::Defensive => modes.defensive_def,
        FightMode::Balanced => 1.0,
    };
    apply_mode_integer_tenths(max_value, f)
}

// ---------------------------------------------------------------------------
// B4.2 — weapon (attack) damage
// ---------------------------------------------------------------------------

/// Probe damage roll loaded from startup tuning (`formulas.damageTuning`).
///
/// Random factor source: sim harness glibc (`TFS_SIM_SEED`) when enabled, else `parity`
/// (per-world [`GlibcRngState`] — sole production stream).
pub fn probe_value(
    skill: i32,
    attack: i32,
    tuning: DamageProbeTuning,
    parity: &GlibcRngState,
) -> i32 {
    let random_factor = {
        #[cfg(any(test, feature = "sim"))]
        if crate::sim_glibc_rand::sim_glibc_rng_enabled() {
            crate::sim_glibc_rand::sim_probe_random_factor()
        } else {
            parity.probe_random_factor(tuning.random_max)
        }
        #[cfg(not(any(test, feature = "sim")))]
        {
            parity.probe_random_factor(tuning.random_max)
        }
    };
    let max_value = attack.max(0)
        * (skill
            .max(0)
            .saturating_mul(tuning.skill_mult.max(0))
            .saturating_add(tuning.skill_base.max(0)));
    (random_factor * max_value) / 10000
}

/// Deterministic ceiling of [`probe_value`] when `RandomFactor == damageTuning.randomMax`.
///
/// Useful for diagnostics / Modern-adjacent max queries. **772 `COMBAT_FORMULA_SKILL`
/// does not use this** — it rolls one [`classic_probe_sample`] like `GetAttackDamage`.
pub fn probe_damage_ceiling(
    skill: i32,
    attack: i32,
    mode: FightMode,
    profile: &MechanicsProfile,
) -> i32 {
    let modified_attack = apply_attack_mode(&profile.fight_modes, mode, attack);
    classic_probe_sample_raw(skill, modified_attack, profile.damage_probe, profile.damage_probe.random_max.max(0))
}

/// One `ProbeValue` sample with a pre-rolled `random_factor` (`crskill.cc:535-546`).
///
/// `attack` should already include fight-mode scaling ([`apply_attack_mode`] via
/// [`classic_probe_sample`]).
pub fn classic_probe_sample_raw(
    skill: i32,
    attack: i32,
    tuning: DamageProbeTuning,
    random_factor: i32,
) -> i32 {
    let max_value = attack.max(0)
        * (skill
            .max(0)
            .saturating_mul(tuning.skill_mult.max(0))
            .saturating_add(tuning.skill_base.max(0)));
    (random_factor.max(0) * max_value) / 10000
}

/// Fight-mode-scaled ClassicProbe sample — 772 `GetAttackDamage` without skill Increase.
pub fn classic_probe_sample(
    profile: &MechanicsProfile,
    skill: i32,
    attack: i32,
    mode: FightMode,
    random_factor: i32,
) -> i32 {
    let modified_attack = apply_attack_mode(&profile.fight_modes, mode, attack);
    classic_probe_sample_raw(skill, modified_attack, profile.damage_probe, random_factor)
}

/// TFS `COMBAT_FORMULA_SKILL` weapon-max term — era-gated.
///
/// - **772 / ClassicProbe** — [`probe_damage_ceiling`] (diagnostic max only).
/// - **1098 / Modern** — TFS `Weapons::getMaxWeaponDamage` (`0.085×skill×attack×d + level/5`).
pub fn formula_skill_weapon_max(
    profile: &MechanicsProfile,
    skill: i32,
    attack: i32,
    mode: FightMode,
    level: u32,
    attack_factor: f64,
) -> i32 {
    match profile.damage_formula {
        DamageFormula::ClassicProbe => probe_damage_ceiling(skill, attack, mode, profile),
        DamageFormula::Modern => {
            let factor = if attack_factor > 0.0 {
                attack_factor
            } else {
                1.0
            };
            crate::weapon::max_weapon_damage_melee(level, skill, attack, factor)
        }
    }
}

/// `setFormula(COMBAT_FORMULA_SKILL, …)` → `(lo, hi)` magnitudes for AoE.
///
/// - **772 / ClassicProbe** — one [`weapon_damage`] / ProbeValue sample (decompile
///   `GetAttackDamage`), then `× maxa + maxb`. Returns `(v, v)` so AoE does not
///   re-roll uniformly over a ceiling range.
/// - **1098 / Modern** — TFS `normal_random(minb, fma(weaponMax, maxa, maxb))` bounds.
pub fn formula_skill_damage_bounds(
    profile: &MechanicsProfile,
    hooks: &FormulaHooks,
    skill: i32,
    attack: i32,
    mode: FightMode,
    level: u32,
    attack_factor: f64,
    min_b: f64,
    max_a: f64,
    max_b: f64,
    parity: &GlibcRngState,
) -> (i32, i32) {
    match profile.damage_formula {
        DamageFormula::ClassicProbe => {
            let rolled =
                weapon_damage(profile, hooks, skill, attack, mode, level as i32, parity);
            let v = (f64::from(rolled) * max_a + max_b).round() as i32;
            (v, v)
        }
        DamageFormula::Modern => {
            let weapon_max =
                formula_skill_weapon_max(profile, skill, attack, mode, level, attack_factor);
            let lo = min_b as i32;
            let hi = (f64::from(weapon_max) * max_a + max_b).round() as i32;
            (lo, hi.max(lo))
        }
    }
}

/// Rolled weapon damage for the active era (B4.2).
///
/// - Tier-2 `getWeaponDamage` hook wins when registered (`772.lua` / `1098.lua`).
/// - [`DamageFormula::ClassicProbe`] (772) — fight-mode-scaled `attack`, then probe roll.
/// - [`DamageFormula::Modern`] (1098) — TFS `getMaxWeaponDamage` then triangular melee roll
///   (15%‥max), matching `WeaponMelee::getWeaponDamage`.
///
/// Always uses per-world glibc via `parity` (sim harness overrides when enabled).
pub fn weapon_damage(
    profile: &MechanicsProfile,
    hooks: &FormulaHooks,
    skill: i32,
    attack: i32,
    mode: FightMode,
    level: i32,
    parity: &GlibcRngState,
) -> i32 {
    if let Some(v) = hooks.weapon_damage(skill, attack, mode.code(), level) {
        return v.max(0);
    }
    match profile.damage_formula {
        DamageFormula::ClassicProbe => {
            let modified_attack = apply_attack_mode(&profile.fight_modes, mode, attack);
            probe_value(skill, modified_attack, profile.damage_probe, parity).max(0)
        }
        DamageFormula::Modern => {
            // TFS `Player::getAttackFactor` — offensive 1.0 / balanced 0.75 / defense 0.5.
            let attack_factor = match mode {
                FightMode::Offensive => 1.0,
                FightMode::Balanced => 0.75,
                FightMode::Defensive => 0.5,
            };
            let max_value = crate::weapon::max_weapon_damage_melee(
                level.max(0) as u32,
                skill,
                attack,
                attack_factor,
            );
            let max_value = (max_value as f64).floor() as i32;
            let min_value = (max_value as f64 * 0.15_f64).floor() as i32;
            // Magnitude only — callers treat this as a positive roll like ClassicProbe.
            crate::combat::rng::triangular_random_glibc(parity, min_value, max_value).max(0)
        }
    }
}

/// Rolled defense value (`crcombat.cc:236` `GetDefendDamage`): fight-mode-scaled defense through
/// `ProbeValue`. Tier-2 `getDefense(skill, defense, mode)` overrides.
pub fn defense_value(
    profile: &MechanicsProfile,
    hooks: &FormulaHooks,
    skill: i32,
    defense: i32,
    mode: FightMode,
    parity: &GlibcRngState,
) -> i32 {
    if let Some(v) = hooks.defense(skill, defense, mode.code()) {
        return v.max(0);
    }
    let modified_defense = apply_defense_mode(&profile.fight_modes, mode, defense);
    probe_value(skill, modified_defense, profile.damage_probe, parity).max(0)
}

// ---------------------------------------------------------------------------
// B4.3 — armor reduction
// ---------------------------------------------------------------------------

/// Effective armor mitigation (B4.3).
///
/// - Tier-2 `getArmorReduction(armor)` wins when registered (`772.lua` / `1098.lua`).
/// - [`ArmorReduction::Full`] (1098) — subtract the full armor value (`creature.cpp` ~532).
/// - [`ArmorReduction::Randomized`] (772) — `(A/2)+rand%(A/2)` when `A >= minArmorForRandom`.
///
/// Always uses per-world glibc via `parity` (sim harness overrides when enabled).
pub fn armor_reduction(
    profile: &MechanicsProfile,
    hooks: &FormulaHooks,
    armor: i32,
    parity: &GlibcRngState,
) -> i32 {
    if let Some(v) = hooks.armor_reduction(armor) {
        return v.max(0);
    }
    match profile.armor {
        ArmorReduction::Full => armor.max(0),
        ArmorReduction::Randomized => {
            let min_armor = profile.armor_random.min_armor_for_random.max(0);
            let div = profile.armor_random.divisor.max(1);
            if armor >= min_armor {
                let half = (armor / div).max(1);
                {
                    #[cfg(any(test, feature = "sim"))]
                    if crate::sim_glibc_rand::sim_glibc_rng_enabled() {
                        half + crate::sim_glibc_rand::sim_rand_mod(half as u32) as i32
                    } else {
                        half + parity.armor_rand_extra(half)
                    }
                    #[cfg(not(any(test, feature = "sim")))]
                    {
                        half + parity.armor_rand_extra(half)
                    }
                }
            } else {
                armor.max(0)
            }
        }
    }
}

/// Final melee damage to HP: `max(0, attack − defense)`, then armor (`crcombat.cc:649` `CloseAttack`).
/// Returns a non-negative magnitude (caller negates for an HP delta).
pub fn melee_damage_after_defense_and_armor(attack: i32, defense: i32, armor: i32) -> i32 {
    let after_defense = (attack - defense).max(0);
    (after_defense - armor.max(0)).max(0)
}

// ---------------------------------------------------------------------------
// B4.8 — distance hit probe (PC-3)
// ---------------------------------------------------------------------------

/// 772 `TSkillProbe::Probe` — `crskill.cc:549` hit-probe for distance attacks.
///
/// Always uses per-world glibc via `parity` (sim harness overrides when enabled).
pub fn probe_hit(skill: i32, diff: i32, prob: i32, parity: &GlibcRngState) -> bool {
    if diff == 0 {
        return true;
    }
    let (diff_roll, chance_roll) = {
        #[cfg(any(test, feature = "sim"))]
        {
            if crate::sim_glibc_rand::sim_glibc_rng_enabled() {
                (
                    crate::sim_glibc_rand::sim_rand_mod(diff.max(1) as u32) as i32,
                    crate::sim_glibc_rand::sim_rand_mod(100) as i32,
                )
            } else {
                (
                    parity.rand_mod(diff.max(1) as u32) as i32,
                    parity.rand_mod(100) as i32,
                )
            }
        }
        #[cfg(not(any(test, feature = "sim")))]
        {
            (
                parity.rand_mod(diff.max(1) as u32) as i32,
                parity.rand_mod(100) as i32,
            )
        }
    };
    if skill < diff_roll {
        return false;
    }
    // `(rand() % 100) <= Prob` — Prob=100 always hits; Prob=0 hits only on 0 (1%).
    chance_roll <= prob
}

// ---------------------------------------------------------------------------
// B4.7 — spell damage
// ---------------------------------------------------------------------------

/// 772 `ComputeDamage` (`magic.cc:784`): `damage * (level_mult*level + magic_mult*magicLevel) / 100`.
/// `clamp_min_100` / `clamp_max_100` mirror the spell flag bits (`& 4` caps at 100%, `& 8` floors at 100%).
/// Tier-2 `getSpellDamage(level, magicLevel, base)` overrides.
pub fn spell_damage(
    profile: &MechanicsProfile,
    hooks: &FormulaHooks,
    level: i32,
    magic_level: i32,
    base: i32,
    clamp_max_100: bool,
    clamp_min_100: bool,
) -> i32 {
    if let Some(v) = hooks.spell_damage(level, magic_level, base) {
        return v;
    }
    let mult = spell_formula_multiplier(
        profile,
        level,
        magic_level,
        clamp_max_100,
        clamp_min_100,
    );
    (base * mult) / 100
}

/// Level/magic multiplier after optional 100% clamps (`magic.cc:784-792`).
///
/// Tuned by `formulas.spell.levelMult` / `magicMult` (`772.lua` / `1098.lua`).
pub fn spell_formula_multiplier(
    profile: &MechanicsProfile,
    level: i32,
    magic_level: i32,
    clamp_max_100: bool,
    clamp_min_100: bool,
) -> i32 {
    let mut mult =
        profile.spell_coeff.level_mult * level + profile.spell_coeff.magic_mult * magic_level;
    if clamp_max_100 && mult > 100 {
        mult = 100;
    }
    if clamp_min_100 && mult < 100 {
        mult = 100;
    }
    mult
}

/// `Player:computeDamage` / `computeHealing` range — decompile variation as bounds, then scale.
///
/// Returns **positive** magnitudes `(lo, hi)` for `damage±variation` after the spell formula.
/// Lua damage scripts negate; healing keeps the sign.
pub fn spell_damage_range(
    profile: &MechanicsProfile,
    hooks: &FormulaHooks,
    level: i32,
    magic_level: i32,
    damage: i32,
    variation: i32,
    clamp_max_100: bool,
    clamp_min_100: bool,
) -> (i32, i32) {
    let lo = spell_damage(
        profile,
        hooks,
        level,
        magic_level,
        damage - variation,
        clamp_max_100,
        clamp_min_100,
    );
    let hi = spell_damage(
        profile,
        hooks,
        level,
        magic_level,
        damage + variation,
        clamp_max_100,
        clamp_min_100,
    );
    (lo.min(hi), lo.max(hi))
}

// ---------------------------------------------------------------------------
// B4.5 — experience & skills
// ---------------------------------------------------------------------------

/// Cumulative experience required to *be* `level` (B4.5).
///
/// Both eras use the same polynomial `(((L-6)*L+17)*L-12)/6 * level_exp_delta`:
/// - [`LevelExpModel::Tfs`] (1098) — TFS `Player::getExpForLevel` with `delta = 100` (`player.h:171`).
/// - [`LevelExpModel::DeltaPoly`] (772) — 772 `TSkillLevel::GetExpForLevel` (`crskill.cc:352`).
///
/// Tier-2 `getExperienceForLevel(level)` overrides.
pub fn experience_for_level(profile: &MechanicsProfile, hooks: &FormulaHooks, level: i64) -> i64 {
    if let Some(v) = hooks.experience_for_level(level as i32) {
        return v;
    }
    match profile.level_exp {
        // TFS 1.4.2 `Player::getExpForLevel` (`player.h:171`) is the *same* polynomial as CipSoft
        // (`crskill.cc:352`) with `Delta = 100`: `(((L-6)*L+17)*L-12)/6 * delta`. The eras differ
        // only in the `Delta` default (both 100 for the level curve; 772 varies it per skill).
        LevelExpModel::Tfs | LevelExpModel::DeltaPoly => {
            experience_for_level_poly(level, profile.level_exp_delta)
        }
    }
}

/// Pure level-exp polynomial `(((L-6)*L+17)*L-12)/6 * delta` (`crskill.cc:352`).
///
/// Shared by [`experience_for_level`] and [`crate::creature::vocation::total_experience_for_level`].
#[inline]
pub fn experience_for_level_poly(level: i64, delta: i64) -> i64 {
    if level <= 1 {
        return 0;
    }
    let l = level;
    (((l - 6) * l + 17) * l - 12) / 6 * delta
}

/// Triangular 20-slot proportional split of `total_exp` across `damage_shares` (B4.5).
///
/// 772 distributes experience proportionally to damage dealt across the (up to 20-entry)
/// `CombatList` (`crcombat.cc:891–905`). Returns each sharer's exp in input order; integer
/// floor per share (remainder is dropped, matching integer C++ division).
pub fn distribute_experience(total_exp: u64, damage_shares: &[u64]) -> Vec<u64> {
    let total_damage: u64 = damage_shares.iter().sum();
    if total_damage == 0 || total_exp == 0 {
        return vec![0; damage_shares.len()];
    }
    damage_shares
        .iter()
        .map(|&dmg| (total_exp as u128 * dmg as u128 / total_damage as u128) as u64)
        .collect()
}

/// 772 PvP kill share scale after proportional damage split (`crcombat.cc:927–934`).
///
/// `MaxLevel = (victim_level * num) / den` (profile `pvpExpCap`, default `11/10`).
/// Returns 0 when `attacker_level >= MaxLevel`; else
/// `((MaxLevel - attacker_level) * amount) / victim_level`.
pub fn pvp_kill_experience_amount(
    profile: &MechanicsProfile,
    victim_level: i32,
    attacker_level: i32,
    amount: u64,
) -> u64 {
    if amount == 0 || victim_level <= 0 || profile.pvp_exp_cap_den == 0 {
        return 0;
    }
    let max_level = (victim_level as i64 * profile.pvp_exp_cap_num as i64)
        / profile.pvp_exp_cap_den as i64;
    if attacker_level as i64 >= max_level {
        return 0;
    }
    (((max_level - attacker_level as i64) as u128 * amount as u128) / victim_level as u128) as u64
}

/// Skill tries required to reach `level` (B4.5).
///
/// Geometric curve shared by both eras: `skill_base * multiplier^(level - (min_level + 1))`. The
/// base/multiplier are era data (TFS `vocations.xml` `skillBase`/`skillMultiplier`, `vocation.cpp:146`;
/// 772 `Delta`/`FactorPercent`, `crskill.cc:483`). `min_level` is the first trainable level
/// (TFS `MINIMUM_SKILL_LEVEL` = 10). Tier-2 `getReqSkillTries(skill, level)` overrides the whole curve.
pub fn req_skill_tries(
    hooks: &FormulaHooks,
    skill: i32,
    level: i32,
    skill_base: u64,
    multiplier: f64,
    min_level: i32,
) -> u64 {
    if let Some(v) = hooks.req_skill_tries(skill, level) {
        return v.max(0) as u64;
    }
    let exp = level - (min_level + 1);
    (skill_base as f64 * multiplier.powi(exp)).floor() as u64
}

// ---------------------------------------------------------------------------
// B4.6 — condition ticks
// ---------------------------------------------------------------------------

/// Fire/energy DoT element selector for [`condition_tick`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DotElement {
    Fire,
    Energy,
}

impl DotElement {
    fn code(self) -> i32 {
        match self {
            DotElement::Fire => 0,
            DotElement::Energy => 1,
        }
    }
}

/// DoT Event `(damage, MaxCount interval)` for `element` at combat `round` (B4.6).
///
/// Native default reads `profile.conditions` (fire dmg=10 MaxCount=8, energy 25/10 —
/// `crskill.cc:1064,1090` / `crmain.cc:600,610`). `ticks` is the ProcessSkills gap between
/// Events, not Cycle length.
/// Tier-2 `getConditionTick(type, round)` overrides and may vary by round.
pub fn condition_tick(
    profile: &MechanicsProfile,
    hooks: &FormulaHooks,
    element: DotElement,
    round: i32,
) -> (i32, i32) {
    if let Some(t) = hooks.condition_tick(element.code(), round) {
        return t;
    }
    let ConditionTicks { fire, energy, .. } = profile.conditions;
    match element {
        DotElement::Fire => (fire.dmg, fire.ticks),
        DotElement::Energy => (energy.dmg, energy.ticks),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formulas::Mechanics;
    use crate::sim_glibc_rand::GlibcRngState;
    use tfs_rust_common::ProtocolVersion;

    fn p772() -> Mechanics {
        Mechanics::for_version(ProtocolVersion::V772)
    }
    fn p1098() -> Mechanics {
        Mechanics::for_version(ProtocolVersion::V1098)
    }

    #[test]
    fn attack_speed_uses_vocation_when_profile_is_zero() {
        let m772 = p772();
        let m1098 = p1098();
        // 772: attack_speed_ms == 0 ⇒ vocation/weapon value passes through.
        assert_eq!(attack_speed_ms(&m772.profile, &m772.hooks, 1500), 1500);
        // 1098: attack_speed_ms == 0 ⇒ vocation/weapon value passes through.
        assert_eq!(attack_speed_ms(&m1098.profile, &m1098.hooks, 1500), 1500);
    }

    #[test]
    fn fight_mode_modifiers_match_772_integer_shape() {
        let m = p772();
        // Offensive +20% atk: 100 -> 120 (772 `+ (v*2)/10`).
        assert_eq!(
            apply_attack_mode(&m.profile.fight_modes, FightMode::Offensive, 100),
            120
        );
        // Defensive -40% atk: 100 -> 60.
        assert_eq!(
            apply_attack_mode(&m.profile.fight_modes, FightMode::Defensive, 100),
            60
        );
        // Offensive -40% def: 100 -> 60.
        assert_eq!(
            apply_defense_mode(&m.profile.fight_modes, FightMode::Offensive, 100),
            60
        );
        // Defensive +80% def: 100 -> 180.
        assert_eq!(
            apply_defense_mode(&m.profile.fight_modes, FightMode::Defensive, 100),
            180
        );
        // Balanced is neutral.
        assert_eq!(
            apply_attack_mode(&m.profile.fight_modes, FightMode::Balanced, 100),
            100
        );
        // Truncation parity vs `Max ± (Max*k)/10` for small weapon values (not f64 floor).
        for v in 1..=20 {
            assert_eq!(
                apply_attack_mode(&m.profile.fight_modes, FightMode::Defensive, v),
                v - (v * 4) / 10,
                "defensive atk tenths for {v}"
            );
            assert_eq!(
                apply_attack_mode(&m.profile.fight_modes, FightMode::Offensive, v),
                v + (v * 2) / 10,
                "offensive atk tenths for {v}"
            );
            assert_eq!(
                apply_defense_mode(&m.profile.fight_modes, FightMode::Offensive, v),
                v - (v * 4) / 10,
                "offensive def tenths for {v}"
            );
            assert_eq!(
                apply_defense_mode(&m.profile.fight_modes, FightMode::Defensive, v),
                v + (v * 8) / 10,
                "defensive def tenths for {v}"
            );
        }
    }

    #[test]
    fn probe_value_matches_classic_formula_bounds() {
        // ProbeValue is bounded by Max/100: max factor 99 -> (99 * Max)/10000.
        // skill=10, attack=50 -> Max = 50*(10*5+50) = 50*100 = 5000.
        // Max possible roll: (99 * 5000)/10000 = 49. Min: 0.
        let parity = GlibcRngState::seed(42);
        let mut max_seen = 0;
        for _ in 0..10_000 {
            let v = probe_value(10, 50, p772().profile.damage_probe, &parity);
            assert!((0..=49).contains(&v), "probe value {v} out of [0,49]");
            max_seen = max_seen.max(v);
        }
        assert!(
            max_seen >= 40,
            "expected high rolls to approach the cap, saw {max_seen}"
        );
    }

    #[test]
    fn melee_damage_subtracts_defense_then_armor() {
        // crcombat.cc CloseAttack: Damage = max(0, Attack-Defense), then armor in TCreature::Damage.
        assert_eq!(melee_damage_after_defense_and_armor(100, 30, 20), 50);
        assert_eq!(melee_damage_after_defense_and_armor(40, 50, 10), 0); // defense exceeds attack
        assert_eq!(melee_damage_after_defense_and_armor(100, 0, 200), 0); // armor exceeds remainder
    }

    #[test]
    fn armor_full_vs_randomized_bounds() {
        let m1098 = p1098();
        let m772 = p772();
        let parity = GlibcRngState::seed(7);
        // 1098 full: armor returned verbatim.
        assert_eq!(
            armor_reduction(&m1098.profile, &m1098.hooks, 30, &parity),
            30
        );
        // 772 randomized: in [A/2, A-1] for A>=2 → [15, 29] for A=30.
        for _ in 0..1000 {
            let r = armor_reduction(&m772.profile, &m772.hooks, 30, &parity);
            assert!(
                (15..=29).contains(&r),
                "randomized armor {r} out of [15,29]"
            );
        }
        // A=1 returns 1 in both.
        assert_eq!(
            armor_reduction(&m772.profile, &m772.hooks, 1, &parity),
            1
        );
    }

    #[test]
    fn tier2_get_armor_reduction_overrides_native() {
        use crate::formulas::FormulaHooks;
        let lua = mlua::Lua::new();
        lua.load("function getArmorReduction(armor) return 42 end")
            .exec()
            .unwrap();
        let hooks = FormulaHooks::from_lua_for_test(lua);
        let parity = GlibcRngState::seed(1);
        assert_eq!(
            armor_reduction(&p772().profile, &hooks, 30, &parity),
            42
        );
    }

    #[test]
    fn spell_damage_multiplier_and_clamps() {
        let m = p772();
        // 2*level + 3*magicLevel: level=50, ml=30 -> 100+90 = 190%. base 100 -> 190.
        assert_eq!(
            spell_damage(&m.profile, &m.hooks, 50, 30, 100, false, false),
            190
        );
        // clamp_max_100 (flag & 4): capped to 100%.
        assert_eq!(
            spell_damage(&m.profile, &m.hooks, 50, 30, 100, true, false),
            100
        );
        // clamp_min_100 (flag & 8): low multiplier floored to 100%. level=1, ml=1 -> 5% -> 100%.
        assert_eq!(
            spell_damage(&m.profile, &m.hooks, 1, 1, 100, false, true),
            100
        );
    }

    #[test]
    fn spell_damage_range_matches_compute_damage() {
        let m = p772();
        // level=20, magic=10 → mult=70; damage=45, variation=10 → (24, 38)
        let (lo, hi) =
            spell_damage_range(&m.profile, &m.hooks, 20, 10, 45, 10, false, false);
        assert_eq!((lo, hi), (24, 38));
    }

    #[test]
    fn level_exp_curves_per_era() {
        let m1098 = p1098();
        let m772 = p772();
        // TFS getExpForLevel = (((L-6)*L+17)*L-12)/6 * 100: lvl 1 = 0, lvl 2 = 100, lvl 8 = 4200.
        assert_eq!(experience_for_level(&m1098.profile, &m1098.hooks, 1), 0);
        assert_eq!(experience_for_level(&m1098.profile, &m1098.hooks, 2), 100);
        assert_eq!(experience_for_level(&m1098.profile, &m1098.hooks, 8), 4200);
        // 772 uses the same polynomial with Delta=100 → identical anchors.
        assert_eq!(experience_for_level(&m772.profile, &m772.hooks, 1), 0);
        assert_eq!(experience_for_level(&m772.profile, &m772.hooks, 8), 4200);
    }

    #[test]
    fn experience_distribution_is_proportional() {
        // 1000 exp split across damage 30/70 → 300/700.
        assert_eq!(distribute_experience(1000, &[30, 70]), vec![300, 700]);
        // No damage → no exp.
        assert_eq!(distribute_experience(1000, &[0, 0]), vec![0, 0]);
        // Single sharer takes all (integer floor).
        assert_eq!(distribute_experience(999, &[5]), vec![999]);
    }

    #[test]
    fn pvp_kill_experience_max_level_11_10() {
        let m = p772();
        // Victim L100 → MaxLevel 110. Attacker L100 → ((110-100)*1000)/100 = 100.
        assert_eq!(
            pvp_kill_experience_amount(&m.profile, 100, 100, 1000),
            100
        );
        // Attacker at/above MaxLevel → 0.
        assert_eq!(pvp_kill_experience_amount(&m.profile, 100, 110, 1000), 0);
        assert_eq!(pvp_kill_experience_amount(&m.profile, 100, 120, 1000), 0);
        // Odd half: L10 MaxLevel 11; atk L8 → ((11-8)*100)/10 = 30.
        assert_eq!(pvp_kill_experience_amount(&m.profile, 10, 8, 100), 30);
        assert_eq!(pvp_kill_experience_amount(&m.profile, 10, 8, 0), 0);
    }

    #[test]
    fn req_skill_tries_geometric_curve() {
        let m = p1098();
        // TFS sword: skillBase 50, multiplier 1.1, MINIMUM_SKILL_LEVEL 10.
        // level 11 → exp 0 → 50 tries; level 12 → exp 1 → 55; level 13 → 60 (floor of 60.5).
        assert_eq!(req_skill_tries(&m.hooks, 2, 11, 50, 1.1, 10), 50);
        assert_eq!(req_skill_tries(&m.hooks, 2, 12, 50, 1.1, 10), 55);
        assert_eq!(req_skill_tries(&m.hooks, 2, 13, 50, 1.1, 10), 60);
    }

    #[test]
    fn req_skill_tries_772_human_mon_deltas() {
        let m = p772();
        let st = m.profile.skill_tries;
        // Sword Delta=50, Base=2.0 (none vocation) — L11→50, L12→100, L13→200.
        assert_eq!(
            req_skill_tries(&m.hooks, 2, 11, st.skill_base[2], 2.0, st.min_level[2]),
            50
        );
        assert_eq!(
            req_skill_tries(&m.hooks, 2, 12, st.skill_base[2], 2.0, st.min_level[2]),
            100
        );
        assert_eq!(
            req_skill_tries(&m.hooks, 2, 13, st.skill_base[2], 2.0, st.min_level[2]),
            200
        );
        // Dist Delta=30, Base=2.0 — L11→30, L12→60.
        assert_eq!(
            req_skill_tries(&m.hooks, 4, 11, st.skill_base[4], 2.0, st.min_level[4]),
            30
        );
        assert_eq!(
            req_skill_tries(&m.hooks, 4, 12, st.skill_base[4], 2.0, st.min_level[4]),
            60
        );
        // Shielding Delta=100, Base=1.5 — L11→100, L12→150.
        assert_eq!(
            req_skill_tries(&m.hooks, 5, 11, st.skill_base[5], 1.5, st.min_level[5]),
            100
        );
        assert_eq!(
            req_skill_tries(&m.hooks, 5, 12, st.skill_base[5], 1.5, st.min_level[5]),
            150
        );
        // Fishing Delta=20, Base=1.1 — L11→20, L12→22.
        assert_eq!(
            req_skill_tries(&m.hooks, 6, 11, st.skill_base[6], 1.1, st.min_level[6]),
            20
        );
        assert_eq!(
            req_skill_tries(&m.hooks, 6, 12, st.skill_base[6], 1.1, st.min_level[6]),
            22
        );
        // Magic skill_base=1600, min=0, mult=3.0 — L1→1600, L2→4800.
        assert_eq!(
            req_skill_tries(
                &m.hooks,
                -1,
                1,
                st.magic_skill_base,
                3.0,
                st.magic_min_level
            ),
            1600
        );
        assert_eq!(
            req_skill_tries(
                &m.hooks,
                -1,
                2,
                st.magic_skill_base,
                3.0,
                st.magic_min_level
            ),
            4800
        );
    }

    #[test]
    fn experience_for_level_poly_matches_total() {
        assert_eq!(experience_for_level_poly(1, 100), 0);
        assert_eq!(
            experience_for_level_poly(2, 100) as u64,
            crate::creature::vocation::total_experience_for_level(2)
        );
        assert_eq!(
            experience_for_level_poly(8, 100) as u64,
            crate::creature::vocation::total_experience_for_level(8)
        );
    }

    #[test]
    fn tier2_req_skill_tries_overrides_native() {
        use crate::formulas::FormulaHooks;
        let lua = mlua::Lua::new();
        lua.load("function getReqSkillTries(skill, level) return 4242 end")
            .exec()
            .unwrap();
        let hooks = FormulaHooks::from_lua_for_test(lua);
        assert_eq!(req_skill_tries(&hooks, 2, 50, 50, 1.1, 10), 4242);
    }

    #[test]
    fn condition_ticks_from_profile() {
        let m772 = p772();
        assert_eq!(
            condition_tick(&m772.profile, &m772.hooks, DotElement::Fire, 0),
            (10, 8)
        );
        assert_eq!(
            condition_tick(&m772.profile, &m772.hooks, DotElement::Energy, 0),
            (25, 10)
        );
    }

    #[test]
    fn startup_damage_tuning_changes_probe_shape() {
        let mut profile = p772().profile;
        profile.damage_probe.skill_mult = 1;
        profile.damage_probe.skill_base = 1;
        profile.damage_probe.random_max = 10;
        let hooks = FormulaHooks::default();
        let parity = GlibcRngState::seed(1);
        let v = weapon_damage(&profile, &hooks, 10, 50, FightMode::Offensive, 8, &parity);
        assert!(v >= 0);
    }

    /// ClassicProbe FORMULA_SKILL: one ProbeValue sample → `(v, v)`, not ceiling range.
    #[test]
    fn formula_skill_classic_probe_rolls_once() {
        let m = p772();
        let parity = GlibcRngState::seed(42);
        let (lo, hi) = formula_skill_damage_bounds(
            &m.profile,
            &m.hooks,
            80,
            40,
            FightMode::Balanced,
            50,
            1.0,
            0.0,
            1.0,
            0.0,
            &parity,
        );
        assert_eq!(lo, hi, "772 FORMULA_SKILL must not re-roll over a range");
        let ceiling = probe_damage_ceiling(80, 40, FightMode::Balanced, &m.profile);
        assert!(
            (0..=ceiling).contains(&lo),
            "probe sample {lo} must be in 0..={ceiling}"
        );
    }

    /// Modern FORMULA_SKILL max = TFS `getMaxWeaponDamage`.
    #[test]
    fn formula_skill_modern_tfs_closed_form() {
        let profile = p1098().profile;
        // floor(0.085*80*40*1) + floor(50/5) = 272 + 10 = 282
        assert_eq!(
            formula_skill_weapon_max(&profile, 80, 40, FightMode::Balanced, 50, 1.0),
            282
        );
        let parity = GlibcRngState::seed(1);
        let (lo, hi) = formula_skill_damage_bounds(
            &profile,
            &FormulaHooks::default(),
            80,
            40,
            FightMode::Balanced,
            50,
            1.0,
            0.0,
            1.0,
            0.0,
            &parity,
        );
        assert_eq!((lo, hi), (0, 282));
    }
}
