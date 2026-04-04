//! Property 8: weapon damage formulas match TFS `weapons.cpp` constants.

use proptest::prelude::*;
use tfs_rust_core::{max_melee_damage_monster, max_weapon_damage_melee};

/// Known anchor from C++: `Weapons::getMaxWeaponDamage(100, 100, 50, 1.0f)` → 445.
#[test]
fn max_weapon_damage_anchor() {
    let v = max_weapon_damage_melee(100, 100, 50, 1.0);
    assert_eq!(v, 445);
}

proptest! {
    #[test]
    fn melee_formula_monotonic_in_skill(
        level in 20u32..200,
        skill in 10i32..120,
        attack in 10i32..60,
    ) {
        let a = max_weapon_damage_melee(level, skill, attack, 1.0);
        let b = max_weapon_damage_melee(level, skill + 1, attack, 1.0);
        prop_assert!(b >= a);
    }

    #[test]
    fn monster_melee_non_negative(
        skill in 0i32..130,
        attack in 0i32..100,
    ) {
        let v = max_melee_damage_monster(skill, attack);
        prop_assert!(v >= 0);
    }
}
