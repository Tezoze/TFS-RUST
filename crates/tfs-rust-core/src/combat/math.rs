//! Era-tunable combat / skill / condition / spell formulas (Track B, Phase B4).
//!
//! Pure functions over [`MechanicsProfile`] (Tier-1 constants) + [`FormulaHooks`] (Tier-2 Lua
//! overrides). The combat *execution* loop is still a skeleton (`combat/mod.rs`, design §12.7/§12.9);
//! this module is the math it will call once wired, and is fully unit-testable today.
//!
//! **C++ reference — behavior/outcomes (CipSoft 7.72, clean-room R12):**
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
//! - Exp distribution 20-slot proportional, PvP cap `11/10`, 60-round window — `crcombat.cc:891–905`.
//! - Condition ticks fire 10/8, energy 25/10 — `crskill.cc:1064,1090`.
//!
//! **C++ reference — structure (TFS 1.4.2 / 10.98):** repo-root `src/weapons.cpp`,
//! `creature.cpp:500–533` (`blockHit`), `vocation.cpp`, `condition.cpp:1330`, `spells.cpp`,
//! `player.cpp` (`getExpForLevel`).

use rand::Rng;

use crate::formulas::{
    ArmorReduction, ConditionTicks, DamageFormula, FightModes, FormulaHooks, LevelExpModel,
    MechanicsProfile,
};

/// Player fight stance (`ATTACK_MODE_*` in CipSoft, `fightMode_t` in TFS).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FightMode {
    Offensive,
    Balanced,
    Defensive,
}

impl FightMode {
    /// Wire byte from `0xA0` `parseFightModes` (`raw_fight_mode`): 1 = offensive, 2 = balanced, 3 = defensive.
    pub fn from_wire(raw: u8) -> Self {
        match raw {
            1 => FightMode::Offensive,
            3 => FightMode::Defensive,
            _ => FightMode::Balanced,
        }
    }

    /// Integer code passed to Tier-2 hooks (mirrors CipSoft `ATTACK_MODE_*`: 1/2/3).
    fn code(self) -> i32 {
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
/// 772: flat `profile.attack_speed_ms` (2000 ms, `crcombat.cc` `DelayAttack(2000)`). 1098:
/// `attack_speed_ms == 0` ⇒ use the vocation/weapon `vocation_attack_speed_ms` (TFS `getAttackSpeed`).
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

/// Apply the fight-mode attack multiplier to a max attack value.
/// CipSoft uses integer `±(v*2)/10` / `±(v*4)/10`; we model the fraction in `FightModes` and floor.
fn apply_attack_mode(modes: &FightModes, mode: FightMode, max_value: i32) -> i32 {
    let f = match mode {
        FightMode::Offensive => modes.offensive_atk,
        FightMode::Defensive => modes.defensive_atk,
        FightMode::Balanced => 1.0,
    };
    ((max_value as f64) * f).floor() as i32
}

/// Apply the fight-mode defense multiplier to a max defense value.
fn apply_defense_mode(modes: &FightModes, mode: FightMode, max_value: i32) -> i32 {
    let f = match mode {
        FightMode::Offensive => modes.offensive_def,
        FightMode::Defensive => modes.defensive_def,
        FightMode::Balanced => 1.0,
    };
    ((max_value as f64) * f).floor() as i32
}

// ---------------------------------------------------------------------------
// B4.2 — weapon (attack) damage
// ---------------------------------------------------------------------------

/// CipSoft `TSkillProbe::ProbeValue` (`crskill.cc:535`): `((rand%100 + rand%100)/2) * Max / 10000`,
/// where `Max = attack * (skill*5 + 50)`. Returns the rolled damage magnitude (non-negative).
pub fn probe_value<R: Rng + ?Sized>(rng: &mut R, skill: i32, attack: i32) -> i32 {
    let random_factor = (rng.gen_range(0..100) + rng.gen_range(0..100)) / 2;
    let max_value = attack.max(0) * (skill.max(0) * 5 + 50);
    (random_factor * max_value) / 10000
}

/// Rolled weapon damage for the active era (B4.2).
///
/// - [`DamageFormula::ClassicProbe`] (772) — fight-mode-scaled `attack`, then `probe_value`.
/// - [`DamageFormula::Modern`] (1098) — TFS classic-formula shape (same `ProbeValue` math today;
///   diverges once the modern weapon formula lands). Tier-2 `getWeaponDamage` overrides either.
pub fn weapon_damage<R: Rng + ?Sized>(
    profile: &MechanicsProfile,
    hooks: &FormulaHooks,
    rng: &mut R,
    skill: i32,
    attack: i32,
    mode: FightMode,
    level: i32,
) -> i32 {
    if let Some(v) = hooks.weapon_damage(skill, attack, mode.code(), level) {
        return v.max(0);
    }
    let modified_attack = apply_attack_mode(&profile.fight_modes, mode, attack);
    match profile.damage_formula {
        DamageFormula::ClassicProbe | DamageFormula::Modern => {
            probe_value(rng, skill, modified_attack).max(0)
        }
    }
}

/// Rolled defense value (`crcombat.cc:236` `GetDefendDamage`): fight-mode-scaled defense through
/// `ProbeValue`. Tier-2 `getDefense(skill, defense, mode)` overrides.
pub fn defense_value<R: Rng + ?Sized>(
    profile: &MechanicsProfile,
    hooks: &FormulaHooks,
    rng: &mut R,
    skill: i32,
    defense: i32,
    mode: FightMode,
) -> i32 {
    if let Some(v) = hooks.defense(skill, defense, mode.code()) {
        return v.max(0);
    }
    let modified_defense = apply_defense_mode(&profile.fight_modes, mode, defense);
    probe_value(rng, skill, modified_defense).max(0)
}

// ---------------------------------------------------------------------------
// B4.3 — armor reduction
// ---------------------------------------------------------------------------

/// Effective armor mitigation (B4.3).
///
/// - [`ArmorReduction::Full`] (1098) — subtract the full armor value (`creature.cpp` ~532).
/// - [`ArmorReduction::Randomized`] (772) — `(A/2) + rand%(A/2)` when `A >= 2`, else `A`
///   (`crcombat.cc:302–304`). Tier-2 `getArmorReduction(armor)` overrides.
pub fn armor_reduction<R: Rng + ?Sized>(
    profile: &MechanicsProfile,
    hooks: &FormulaHooks,
    rng: &mut R,
    armor: i32,
) -> i32 {
    if let Some(v) = hooks.armor_reduction(armor) {
        return v.max(0);
    }
    match profile.armor {
        ArmorReduction::Full => armor.max(0),
        ArmorReduction::Randomized => {
            if armor >= 2 {
                (armor / 2) + rng.gen_range(0..(armor / 2))
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
// B4.7 — spell damage
// ---------------------------------------------------------------------------

/// CipSoft `ComputeDamage` (`magic.cc:784`): `damage * (level_mult*level + magic_mult*magicLevel) / 100`.
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
    let mut mult = profile.spell_coeff.level_mult * level + profile.spell_coeff.magic_mult * magic_level;
    if clamp_max_100 && mult > 100 {
        mult = 100;
    }
    if clamp_min_100 && mult < 100 {
        mult = 100;
    }
    (base * mult) / 100
}

// ---------------------------------------------------------------------------
// B4.5 — experience & skills
// ---------------------------------------------------------------------------

/// Cumulative experience required to *be* `level` (B4.5).
///
/// Both eras use the same polynomial `(((L-6)*L+17)*L-12)/6 * level_exp_delta`:
/// - [`LevelExpModel::Tfs`] (1098) — TFS `Player::getExpForLevel` with `delta = 100` (`player.h:171`).
/// - [`LevelExpModel::CipSoftPoly`] (772) — CipSoft `TSkillLevel::GetExpForLevel` (`crskill.cc:352`).
///
/// Tier-2 `getExperienceForLevel(level)` overrides.
pub fn experience_for_level(profile: &MechanicsProfile, hooks: &FormulaHooks, level: i64) -> i64 {
    if let Some(v) = hooks.experience_for_level(level as i32) {
        return v;
    }
    match profile.level_exp {
        // TFS 1.4.2 `Player::getExpForLevel` (`player.h:171`) is the *same* polynomial as CipSoft
        // (`crskill.cc:352`) with `Delta = 100`: `(((L-6)*L+17)*L-12)/6 * delta`. The eras differ
        // only in the `Delta` default (both 100 for the level curve; CipSoft varies it per skill).
        LevelExpModel::Tfs | LevelExpModel::CipSoftPoly => {
            let l = level;
            (((l - 6) * l + 17) * l - 12) / 6 * profile.level_exp_delta
        }
    }
}

/// Triangular 20-slot proportional split of `total_exp` across `damage_shares` (B4.5).
///
/// CipSoft distributes experience proportionally to damage dealt across the (up to 20-entry)
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

/// PvP experience cap: when killing a player, exp is capped to `num/den` of the victim's value
/// (CipSoft `11/10`, `crcombat.cc:900`). Returns the capped exp.
pub fn pvp_exp_cap(profile: &MechanicsProfile, raw_exp: u64) -> u64 {
    if profile.pvp_exp_cap_den == 0 {
        return raw_exp;
    }
    let cap = (raw_exp as u128 * profile.pvp_exp_cap_num as u128 / profile.pvp_exp_cap_den as u128) as u64;
    raw_exp.min(cap)
}

/// Skill tries required to reach `level` (B4.5).
///
/// Geometric curve shared by both eras: `skill_base * multiplier^(level - (min_level + 1))`. The
/// base/multiplier are era data (TFS `vocations.xml` `skillBase`/`skillMultiplier`, `vocation.cpp:146`;
/// CipSoft `Delta`/`FactorPercent`, `crskill.cc:483`). `min_level` is the first trainable level
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

/// DoT tick `(damage_per_tick, tick_count)` for `element` at combat `round` (B4.6).
///
/// Native default reads `profile.conditions` (fire 10/8, energy 25/10 — `crskill.cc:1064,1090`).
/// Tier-2 `getConditionTick(type, round)` overrides and may vary by round (e.g. poison decay).
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
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use tfs_rust_common::ProtocolVersion;

    fn p772() -> Mechanics {
        Mechanics::for_version(ProtocolVersion::V772)
    }
    fn p1098() -> Mechanics {
        Mechanics::for_version(ProtocolVersion::V1098)
    }

    #[test]
    fn attack_speed_flat_2000_for_772_vocation_for_1098() {
        let m772 = p772();
        let m1098 = p1098();
        // 772: flat 2000 regardless of vocation value.
        assert_eq!(attack_speed_ms(&m772.profile, &m772.hooks, 1500), 2000);
        // 1098: attack_speed_ms == 0 ⇒ vocation/weapon value passes through.
        assert_eq!(attack_speed_ms(&m1098.profile, &m1098.hooks, 1500), 1500);
    }

    #[test]
    fn fight_mode_modifiers_match_cipsoft_integer_shape() {
        let m = p772();
        // Offensive +20% atk: 100 -> 120 (CipSoft `+ (v*2)/10`).
        assert_eq!(apply_attack_mode(&m.profile.fight_modes, FightMode::Offensive, 100), 120);
        // Defensive -40% atk: 100 -> 60.
        assert_eq!(apply_attack_mode(&m.profile.fight_modes, FightMode::Defensive, 100), 60);
        // Offensive -40% def: 100 -> 60.
        assert_eq!(apply_defense_mode(&m.profile.fight_modes, FightMode::Offensive, 100), 60);
        // Defensive +80% def: 100 -> 180.
        assert_eq!(apply_defense_mode(&m.profile.fight_modes, FightMode::Defensive, 100), 180);
        // Balanced is neutral.
        assert_eq!(apply_attack_mode(&m.profile.fight_modes, FightMode::Balanced, 100), 100);
    }

    #[test]
    fn probe_value_matches_cipsoft_formula_bounds() {
        // ProbeValue is bounded by Max/100: max factor 99 -> (99 * Max)/10000.
        // skill=10, attack=50 -> Max = 50*(10*5+50) = 50*100 = 5000.
        // Max possible roll: (99 * 5000)/10000 = 49. Min: 0.
        let mut rng = StdRng::seed_from_u64(42);
        let mut max_seen = 0;
        for _ in 0..10_000 {
            let v = probe_value(&mut rng, 10, 50);
            assert!((0..=49).contains(&v), "probe value {v} out of [0,49]");
            max_seen = max_seen.max(v);
        }
        assert!(max_seen >= 40, "expected high rolls to approach the cap, saw {max_seen}");
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
        let mut rng = StdRng::seed_from_u64(7);
        // 1098 full: armor returned verbatim.
        assert_eq!(armor_reduction(&m1098.profile, &m1098.hooks, &mut rng, 30), 30);
        // 772 randomized: in [A/2, A-1] for A>=2 → [15, 29] for A=30.
        for _ in 0..1000 {
            let r = armor_reduction(&m772.profile, &m772.hooks, &mut rng, 30);
            assert!((15..=29).contains(&r), "randomized armor {r} out of [15,29]");
        }
        // A=1 returns 1 in both.
        assert_eq!(armor_reduction(&m772.profile, &m772.hooks, &mut rng, 1), 1);
    }

    #[test]
    fn spell_damage_multiplier_and_clamps() {
        let m = p772();
        // 2*level + 3*magicLevel: level=50, ml=30 -> 100+90 = 190%. base 100 -> 190.
        assert_eq!(spell_damage(&m.profile, &m.hooks, 50, 30, 100, false, false), 190);
        // clamp_max_100 (flag & 4): capped to 100%.
        assert_eq!(spell_damage(&m.profile, &m.hooks, 50, 30, 100, true, false), 100);
        // clamp_min_100 (flag & 8): low multiplier floored to 100%. level=1, ml=1 -> 5% -> 100%.
        assert_eq!(spell_damage(&m.profile, &m.hooks, 1, 1, 100, false, true), 100);
    }

    #[test]
    fn level_exp_curves_per_era() {
        let m1098 = p1098();
        let m772 = p772();
        // TFS getExpForLevel = (((L-6)*L+17)*L-12)/6 * 100: lvl 1 = 0, lvl 2 = 100, lvl 8 = 4200.
        assert_eq!(experience_for_level(&m1098.profile, &m1098.hooks, 1), 0);
        assert_eq!(experience_for_level(&m1098.profile, &m1098.hooks, 2), 100);
        assert_eq!(experience_for_level(&m1098.profile, &m1098.hooks, 8), 4200);
        // CipSoft uses the same polynomial with Delta=100 → identical anchors.
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
    fn pvp_exp_cap_11_10() {
        let m = p772();
        // Cap = raw * 11/10; raw is below the cap so returned unchanged.
        assert_eq!(pvp_exp_cap(&m.profile, 1000), 1000);
        // The cap only ever reduces; raw never exceeds raw*11/10, so identity for the cap direction.
        assert_eq!(pvp_exp_cap(&m.profile, 0), 0);
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
        assert_eq!(condition_tick(&m772.profile, &m772.hooks, DotElement::Fire, 0), (10, 8));
        assert_eq!(condition_tick(&m772.profile, &m772.hooks, DotElement::Energy, 0), (25, 10));
    }

    #[test]
    fn tier2_weapon_damage_overrides_native() {
        use crate::formulas::{FormulaHooks, MechanicsProfile};
        let lua = mlua::Lua::new();
        lua.load("function getWeaponDamage(skill, attack, mode, level) return 777 end")
            .exec()
            .unwrap();
        let profile = MechanicsProfile::for_version(ProtocolVersion::V772);
        let hooks = FormulaHooks::from_lua_for_test(lua);
        let mut rng = StdRng::seed_from_u64(1);
        assert_eq!(weapon_damage(&profile, &hooks, &mut rng, 10, 50, FightMode::Offensive, 8), 777);
    }
}
