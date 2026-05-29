//! C++ `struct Abilities` and items.xml `abilities.*` fields — `src/items.h` (lines 160–192),
//! parsed in `Items::parseItemNode` (`src/items.cpp` ~860–1158, ~1304–1338).

use tfs_rust_common::enums::{CombatType, Skill};

/// `ConditionType_t` bit masks in `src/enums.h` (used with `Abilities::conditionSuppressions` in C++).
// C++: `src/enums.h` `ConditionType_t` — do not conflate with Rust's sequential `ConditionType` enum in `tfs_rust_common`.
pub const CONDITION_POISON: u32 = 1 << 0;
pub const CONDITION_FIRE: u32 = 1 << 1;
pub const CONDITION_ENERGY: u32 = 1 << 2;
pub const CONDITION_BLEEDING: u32 = 1 << 3;
pub const CONDITION_DRUNK: u32 = 1 << 11;
pub const CONDITION_DROWN: u32 = 1 << 15;
pub const CONDITION_FREEZING: u32 = 1 << 20;
pub const CONDITION_DAZZLED: u32 = 1 << 21;
pub const CONDITION_CURSED: u32 = 1 << 22;

/// C++ `stats_t` / `std::array<..., STAT_LAST + 1>` (`src/enums.h`).
pub const STAT_MAXHITPOINTS: usize = 0;
pub const STAT_MAXMANAPOINTS: usize = 1;
pub const STAT_SOULPOINTS: usize = 2;
pub const STAT_MAGICPOINTS: usize = 3;

/// C++ `SpecialSkills_t` order (`src/enums.h`).
pub const SPECIAL_CRITICALHITCHANCE: usize = 0;
pub const SPECIAL_CRITICALHITAMOUNT: usize = 1;
pub const SPECIAL_LIFELEECHCHANCE: usize = 2;
pub const SPECIAL_LIFELEECHAMOUNT: usize = 3;
pub const SPECIAL_MANALEECHCHANCE: usize = 4;
pub const SPECIAL_MANALEECHAMOUNT: usize = 5;

/// C++ `COMBAT_COUNT` = 12; index matches `combatTypeToIndex` in `src/tools.cpp` and
/// [CombatType] discriminant 0..=11 in [tfs_rust_common::enums::CombatType].
pub const COMBAT_ABSORB_COUNT: usize = 12;

/// `tools.cpp` `combatTypeToIndex(CombatType_t)` for non-`COMBAT_NONE` values.
#[inline]
pub fn combat_absorb_index(ct: CombatType) -> usize {
    ct as usize
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemAbilities {
    pub health_gain: u32,
    pub health_ticks: u32,
    pub mana_gain: u32,
    pub mana_ticks: u32,
    /// C++ `conditionImmunities` — not set from items.xml in 1.4.2; kept for struct parity.
    pub condition_immunities: u32,
    pub condition_suppressions: u32,
    /// C++ `stats[STAT_*]` (length `STAT_LAST + 1` = 4).
    pub stats: [i32; 4],
    pub stats_percent: [i32; 4],
    /// C++ `skills[SKILL_FIST..=SKILL_FISHING]` (length 7).
    pub skills: [i32; 7],
    /// C++ `specialSkills[SPECIALSKILL_FIRST..=SPECIALSKILL_LAST]`.
    pub special_skills: [i32; 6],
    /// Equipment walk-speed bonus from items.xml `speed` (not OTB `ItemType::speed`).
    pub speed: i32,
    pub field_absorb_percent: [i16; COMBAT_ABSORB_COUNT],
    pub absorb_percent: [i16; COMBAT_ABSORB_COUNT],
    pub element_damage: u16,
    /// `None` = C++ `elementType == COMBAT_NONE`.
    pub element_type: Option<CombatType>,
    pub mana_shield: bool,
    pub invisible: bool,
    pub regeneration: bool,
}

impl Default for ItemAbilities {
    fn default() -> Self {
        Self {
            health_gain: 0,
            health_ticks: 0,
            mana_gain: 0,
            mana_ticks: 0,
            condition_immunities: 0,
            condition_suppressions: 0,
            stats: [0; 4],
            stats_percent: [0; 4],
            skills: [0; 7],
            special_skills: [0; 6],
            speed: 0,
            field_absorb_percent: [0; COMBAT_ABSORB_COUNT],
            absorb_percent: [0; COMBAT_ABSORB_COUNT],
            element_damage: 0,
            element_type: None,
            mana_shield: false,
            invisible: false,
            regeneration: false,
        }
    }
}

fn ability_parse_bool(value: &str) -> Option<bool> {
    let v = value.to_ascii_lowercase();
    match v.as_str() {
        "1" | "true" | "yes" => Some(true),
        "0" | "false" | "no" => Some(false),
        _ => None,
    }
}

/// C++ `pugi::xml_attribute::as_bool()`-style: known tokens or `0` = false, non-zero = true; unknown → `false`.
fn ability_as_bool(value: &str) -> bool {
    if let Some(b) = ability_parse_bool(value) {
        return b;
    }
    value.parse::<i32>().ok().is_some_and(|n| n != 0)
}

/// C++: `Items::parseItemNode` — `src/items.cpp` (abilities `switch` body).
/// Returns `true` if `k` is an item ability key handled there (and updated `a`).
pub fn apply_ability_attribute(a: &mut ItemAbilities, k: &str, value: &str) -> bool {
    match k {
        "invisible" => a.invisible = ability_as_bool(value),
        "speed" => {
            if let Ok(n) = value.parse::<i32>() {
                a.speed = n;
            }
        }
        "healthgain" => {
            a.regeneration = true;
            if let Ok(n) = value.parse::<u32>() {
                a.health_gain = n;
            }
        }
        "healthticks" => {
            a.regeneration = true;
            if let Ok(n) = value.parse::<u32>() {
                a.health_ticks = n;
            }
        }
        "managain" => {
            a.regeneration = true;
            if let Ok(n) = value.parse::<u32>() {
                a.mana_gain = n;
            }
        }
        "manaticks" => {
            a.regeneration = true;
            if let Ok(n) = value.parse::<u32>() {
                a.mana_ticks = n;
            }
        }
        "manashield" => {
            a.mana_shield = ability_as_bool(value);
        }
        "skillsword" => {
            if let Ok(n) = value.parse::<i32>() {
                a.skills[Skill::Sword as usize] = n;
            }
        }
        "skillaxe" => {
            if let Ok(n) = value.parse::<i32>() {
                a.skills[Skill::Axe as usize] = n;
            }
        }
        "skillclub" => {
            if let Ok(n) = value.parse::<i32>() {
                a.skills[Skill::Club as usize] = n;
            }
        }
        "skilldist" => {
            if let Ok(n) = value.parse::<i32>() {
                a.skills[Skill::Distance as usize] = n;
            }
        }
        "skillfish" => {
            if let Ok(n) = value.parse::<i32>() {
                a.skills[Skill::Fishing as usize] = n;
            }
        }
        "skillshield" => {
            if let Ok(n) = value.parse::<i32>() {
                a.skills[Skill::Shield as usize] = n;
            }
        }
        "skillfist" => {
            if let Ok(n) = value.parse::<i32>() {
                a.skills[Skill::Fist as usize] = n;
            }
        }
        "criticalhitamount" => {
            if let Ok(n) = value.parse::<i32>() {
                a.special_skills[SPECIAL_CRITICALHITAMOUNT] = n;
            }
        }
        "criticalhitchance" => {
            if let Ok(n) = value.parse::<i32>() {
                a.special_skills[SPECIAL_CRITICALHITCHANCE] = n;
            }
        }
        "manaleechamount" => {
            if let Ok(n) = value.parse::<i32>() {
                a.special_skills[SPECIAL_MANALEECHAMOUNT] = n;
            }
        }
        "manaleechchance" => {
            if let Ok(n) = value.parse::<i32>() {
                a.special_skills[SPECIAL_MANALEECHCHANCE] = n;
            }
        }
        "lifeleechamount" => {
            if let Ok(n) = value.parse::<i32>() {
                a.special_skills[SPECIAL_LIFELEECHAMOUNT] = n;
            }
        }
        "lifeleechchance" => {
            if let Ok(n) = value.parse::<i32>() {
                a.special_skills[SPECIAL_LIFELEECHCHANCE] = n;
            }
        }
        "maxhitpoints" => {
            if let Ok(n) = value.parse::<i32>() {
                a.stats[STAT_MAXHITPOINTS] = n;
            }
        }
        "maxhitpointspercent" => {
            if let Ok(n) = value.parse::<i32>() {
                a.stats_percent[STAT_MAXHITPOINTS] = n;
            }
        }
        "maxmanapoints" => {
            if let Ok(n) = value.parse::<i32>() {
                a.stats[STAT_MAXMANAPOINTS] = n;
            }
        }
        "maxmanapointspercent" => {
            if let Ok(n) = value.parse::<i32>() {
                a.stats_percent[STAT_MAXMANAPOINTS] = n;
            }
        }
        "magicpoints" | "magiclevelpoints" => {
            if let Ok(n) = value.parse::<i32>() {
                a.stats[STAT_MAGICPOINTS] = n;
            }
        }
        "magicpointspercent" => {
            if let Ok(n) = value.parse::<i32>() {
                a.stats_percent[STAT_MAGICPOINTS] = n;
            }
        }
        "fieldabsorbpercentenergy" => {
            if let Ok(d) = value.parse::<i16>() {
                let i = combat_absorb_index(CombatType::Energy);
                a.field_absorb_percent[i] = a.field_absorb_percent[i].wrapping_add(d);
            }
        }
        "fieldabsorbpercentfire" => {
            if let Ok(d) = value.parse::<i16>() {
                let i = combat_absorb_index(CombatType::Fire);
                a.field_absorb_percent[i] = a.field_absorb_percent[i].wrapping_add(d);
            }
        }
        "fieldabsorbpercentpoison" | "fieldabsorbpercentearth" => {
            if let Ok(d) = value.parse::<i16>() {
                // C++: `ITEM_PARSE_FIELDABSORBPERCENTPOISON` / duplicate earth key -> `COMBAT_EARTHDAMAGE` (`src/items.cpp`).
                let i = combat_absorb_index(CombatType::Earth);
                a.field_absorb_percent[i] = a.field_absorb_percent[i].wrapping_add(d);
            }
        }
        "absorbpercentall" | "absorbpercentallelements" => {
            if let Ok(d) = value.parse::<i16>() {
                for i in 0..COMBAT_ABSORB_COUNT {
                    a.absorb_percent[i] = a.absorb_percent[i].wrapping_add(d);
                }
            }
        }
        "absorbpercentelements" => {
            if let Ok(d) = value.parse::<i16>() {
                for ct in [CombatType::Energy, CombatType::Fire, CombatType::Earth, CombatType::Ice] {
                    let i = combat_absorb_index(ct);
                    a.absorb_percent[i] = a.absorb_percent[i].wrapping_add(d);
                }
            }
        }
        "absorbpercentmagic" => {
            if let Ok(d) = value.parse::<i16>() {
                for ct in [
                    CombatType::Energy,
                    CombatType::Fire,
                    CombatType::Earth,
                    CombatType::Ice,
                    CombatType::Holy,
                    CombatType::Death,
                ] {
                    let i = combat_absorb_index(ct);
                    a.absorb_percent[i] = a.absorb_percent[i].wrapping_add(d);
                }
            }
        }
        "absorbpercentenergy" => {
            if let Ok(d) = value.parse::<i16>() {
                let i = combat_absorb_index(CombatType::Energy);
                a.absorb_percent[i] = a.absorb_percent[i].wrapping_add(d);
            }
        }
        "absorbpercentfire" => {
            if let Ok(d) = value.parse::<i16>() {
                let i = combat_absorb_index(CombatType::Fire);
                a.absorb_percent[i] = a.absorb_percent[i].wrapping_add(d);
            }
        }
        "absorbpercentpoison" | "absorbpercentearth" => {
            if let Ok(d) = value.parse::<i16>() {
                let i = combat_absorb_index(CombatType::Earth);
                a.absorb_percent[i] = a.absorb_percent[i].wrapping_add(d);
            }
        }
        "absorbpercentice" => {
            if let Ok(d) = value.parse::<i16>() {
                let i = combat_absorb_index(CombatType::Ice);
                a.absorb_percent[i] = a.absorb_percent[i].wrapping_add(d);
            }
        }
        "absorbpercentholy" => {
            if let Ok(d) = value.parse::<i16>() {
                let i = combat_absorb_index(CombatType::Holy);
                a.absorb_percent[i] = a.absorb_percent[i].wrapping_add(d);
            }
        }
        "absorbpercentdeath" => {
            if let Ok(d) = value.parse::<i16>() {
                let i = combat_absorb_index(CombatType::Death);
                a.absorb_percent[i] = a.absorb_percent[i].wrapping_add(d);
            }
        }
        "absorbpercentlifedrain" => {
            if let Ok(d) = value.parse::<i16>() {
                let i = combat_absorb_index(CombatType::LifeDrain);
                a.absorb_percent[i] = a.absorb_percent[i].wrapping_add(d);
            }
        }
        "absorbpercentmanadrain" => {
            if let Ok(d) = value.parse::<i16>() {
                let i = combat_absorb_index(CombatType::ManaDrain);
                a.absorb_percent[i] = a.absorb_percent[i].wrapping_add(d);
            }
        }
        "absorbpercentdrown" => {
            if let Ok(d) = value.parse::<i16>() {
                let i = combat_absorb_index(CombatType::Drown);
                a.absorb_percent[i] = a.absorb_percent[i].wrapping_add(d);
            }
        }
        "absorbpercentphysical" => {
            if let Ok(d) = value.parse::<i16>() {
                let i = combat_absorb_index(CombatType::Physical);
                a.absorb_percent[i] = a.absorb_percent[i].wrapping_add(d);
            }
        }
        "absorbpercenthealing" => {
            if let Ok(d) = value.parse::<i16>() {
                let i = combat_absorb_index(CombatType::Healing);
                a.absorb_percent[i] = a.absorb_percent[i].wrapping_add(d);
            }
        }
        "absorbpercentundefined" => {
            if let Ok(d) = value.parse::<i16>() {
                let i = combat_absorb_index(CombatType::Undefined);
                a.absorb_percent[i] = a.absorb_percent[i].wrapping_add(d);
            }
        }
        "suppressdrunk" => {
            if ability_as_bool(value) {
                a.condition_suppressions |= CONDITION_DRUNK;
            }
        }
        "suppressenergy" => {
            if ability_as_bool(value) {
                a.condition_suppressions |= CONDITION_ENERGY;
            }
        }
        "suppressfire" => {
            if ability_as_bool(value) {
                a.condition_suppressions |= CONDITION_FIRE;
            }
        }
        "suppresspoison" => {
            if ability_as_bool(value) {
                a.condition_suppressions |= CONDITION_POISON;
            }
        }
        "suppressdrown" => {
            if ability_as_bool(value) {
                a.condition_suppressions |= CONDITION_DROWN;
            }
        }
        "suppressphysical" => {
            if ability_as_bool(value) {
                a.condition_suppressions |= CONDITION_BLEEDING;
            }
        }
        "suppressfreeze" => {
            if ability_as_bool(value) {
                a.condition_suppressions |= CONDITION_FREEZING;
            }
        }
        "suppressdazzle" => {
            if ability_as_bool(value) {
                a.condition_suppressions |= CONDITION_DAZZLED;
            }
        }
        "suppresscurse" => {
            if ability_as_bool(value) {
                a.condition_suppressions |= CONDITION_CURSED;
            }
        }
        "elementice" => {
            if let Ok(n) = value.parse::<u16>() {
                a.element_damage = n;
            }
            a.element_type = Some(CombatType::Ice);
        }
        "elementearth" => {
            if let Ok(n) = value.parse::<u16>() {
                a.element_damage = n;
            }
            a.element_type = Some(CombatType::Earth);
        }
        "elementfire" => {
            if let Ok(n) = value.parse::<u16>() {
                a.element_damage = n;
            }
            a.element_type = Some(CombatType::Fire);
        }
        "elementenergy" => {
            if let Ok(n) = value.parse::<u16>() {
                a.element_damage = n;
            }
            a.element_type = Some(CombatType::Energy);
        }
        "elementdeath" => {
            if let Ok(n) = value.parse::<u16>() {
                a.element_damage = n;
            }
            a.element_type = Some(CombatType::Death);
        }
        "elementholy" => {
            if let Ok(n) = value.parse::<u16>() {
                a.element_damage = n;
            }
            a.element_type = Some(CombatType::Holy);
        }
        _ => return false,
    }
    true
}
