//! Weapon damage formulas (player melee / distance / wand / monster melee).
// C++ reference: `weapons.cpp` `Weapons::getMaxWeaponDamage`, `WeaponMelee::getWeaponDamage`, etc.

/// Players — melee max: `floor(0.085 × skill × attack × d) + floor(level / 5)`, `d = 1/attackFactor`.
#[inline]
pub fn max_weapon_damage_melee(
    level: u32,
    attack_skill: i32,
    attack_value: i32,
    attack_factor: f64,
) -> i32 {
    let d = 1.0 / attack_factor;
    let v = attack_value.max(0);
    (0.085_f64 * attack_skill as f64 * v as f64 * d).floor() as i32
        + (level as f64 / 5.0).floor() as i32
}

/// Monsters — `ceil(skill × attack × 0.05 + attack × 0.5)`.
#[inline]
pub fn max_melee_damage_monster(attack_skill: i32, attack_value: i32) -> i32 {
    let v = attack_value.max(0);
    ((attack_skill as f64 * (v as f64 * 0.05)) + (v as f64 * 0.5)).ceil() as i32
}

/// Distance max inside `getWeaponDamage` (before min/max uniform):  
/// `(floor(0.09 × skill × attack × d) + floor(level / 5)) × distDamageMultiplier`
#[inline]
pub fn max_weapon_damage_distance_core(
    level: u32,
    attack_skill: i32,
    attack_value: i32,
    attack_factor: f64,
    dist_damage_multiplier: f64,
) -> i32 {
    let d = 1.0 / attack_factor;
    let v = attack_value.max(0);
    let inner =
        (0.09_f64 * attack_skill as f64 * v as f64 * d).floor() + (level as f64 / 5.0).floor();
    (inner * dist_damage_multiplier).floor() as i32
}

/// Player melee rolled damage (negative): triangular between `minValue` and `maxValue` with knight floor 15%.
// C++ reference: `WeaponMelee::getWeaponDamage`.
pub fn roll_melee_player_damage<R: rand::Rng + ?Sized>(
    rng: &mut R,
    max_value: i32,
    melee_damage_multiplier: f64,
) -> i32 {
    let max_value = (max_value as f64 * melee_damage_multiplier).floor() as i32;
    let min_value = (max_value as f64 * 0.15_f64).floor() as i32;
    -crate::combat::rng::triangular_random(rng, min_value, max_value)
}

/// Distance physical (negative): uniform between 13% max and max.
// C++ reference: `WeaponDistance::getWeaponDamage`.
pub fn roll_distance_player_damage<R: rand::Rng + ?Sized>(
    rng: &mut R,
    max_value: i32,
    dist_damage_multiplier: f64,
) -> i32 {
    let max_value = (max_value as f64 * dist_damage_multiplier).floor() as i32;
    let min_value = (max_value as f64 * 0.13_f64).floor() as i32;
    -crate::combat::rng::normal_random(rng, min_value, max_value)
}

/// Wand: uniform between min and max (negative values in TFS).
pub fn roll_wand_damage<R: rand::Rng + ?Sized>(
    rng: &mut R,
    min_change: i32,
    max_change: i32,
) -> i32 {
    -crate::combat::rng::normal_random(rng, min_change, max_change)
}
