//! Weapon definitions loaded from `data/scripts/weapons/*.lua` via the TFS Lua
//! `Weapon(WEAPON_*)` API.
//!
//! PC-2b: the Lua `Weapon` userdata accumulates config fields during script loading
//! (`:id`, `:level`, `:mana`, `:element`, `:damage`, `:vocation`, `:register`). The
//! `:register()` call pushes a `PendingWeapon` into the Lua runtime's pending buffer,
//! which is drained into a `WeaponRegistry` after all weapon scripts load.
//!
//! C++ reference: `weapons.cpp` `Weapons::load`, `weapons.h` `Weapon` / `WeaponWand` /
//! `WeaponDistance` / `WeaponMelee`, `luascript.cpp:3209-3246` `luaCreateWeapon`.

use std::collections::HashMap;

use tfs_rust_common::enums::CombatType;

/// A wand/rod definition loaded from `wands.lua` / `rods.lua`.
///
/// C++ `WeaponWand` — `weapons.h:200-260`. Wands and rods are both `WEAPON_WAND` in
/// the TFS API; rods are simply wands restricted to druid vocations.
#[derive(Debug, Clone, Default)]
pub struct WandDef {
    /// Item type id (e.g. 2190 = Wand of Vortex).
    pub item_id: u16,
    /// Minimum level to wield.
    pub level: u32,
    /// Mana cost per attack.
    pub mana_cost: u32,
    /// Damage element — maps `weapon:element("energy"/"fire"/"earth"/...)`.
    pub element: CombatType,
    /// Minimum damage per hit.
    pub damage_min: u32,
    /// Maximum damage per hit.
    pub damage_max: u32,
    /// Vocation names that can wield (e.g. `["Sorcerer", "Master Sorcerer"]`).
    /// `true` = allowed, `false` = explicitly disallowed (TFS `vocation(name, bool)`).
    pub vocations: HashMap<String, bool>,
}

/// A distance weapon definition (bow/crossbow + ammo pairing).
/// C++ `WeaponDistance` — `weapons.h:160-199`. PC-3 scope; PC-2b loads the struct.
#[derive(Debug, Clone, Default)]
pub struct DistanceWeaponDef {
    pub item_id: u16,
    pub level: u32,
    pub magic_level: u32,
    pub mana_cost: u32,
    pub vocations: HashMap<String, bool>,
    pub hit_chance: i32,
    pub shoot_range: u32,
    pub element: CombatType,
    pub extra_element: CombatType,
}

/// A melee weapon definition (sword/club/axe with scripted config).
/// C++ `WeaponMelee` — `weapons.h:120-159`. PC-2b loads the struct; melee weapon
/// attack/defense values come from `items.otb`/`items.xml`, not the Lua script.
#[derive(Debug, Clone, Default)]
pub struct MeleeWeaponDef {
    pub item_id: u16,
    pub level: u32,
    pub magic_level: u32,
    pub vocations: HashMap<String, bool>,
    pub element: CombatType,
    pub extra_element: CombatType,
}

/// Union of weapon definition types loaded from `data/scripts/weapons/*.lua`.
#[derive(Debug, Clone)]
pub enum WeaponDef {
    Wand(WandDef),
    Distance(DistanceWeaponDef),
    Melee(MeleeWeaponDef),
}

impl WeaponDef {
    pub fn item_id(&self) -> u16 {
        match self {
            WeaponDef::Wand(w) => w.item_id,
            WeaponDef::Distance(d) => d.item_id,
            WeaponDef::Melee(m) => m.item_id,
        }
    }
}

/// Registry of all weapon definitions loaded from Lua scripts.
/// Mirrors `VocationRegistry` — keyed by item id for O(1) lookup.
#[derive(Debug, Clone, Default)]
pub struct WeaponRegistry {
    pub wands: HashMap<u16, WandDef>,
    pub distance: HashMap<u16, DistanceWeaponDef>,
    pub melee: HashMap<u16, MeleeWeaponDef>,
}

impl WeaponRegistry {
    /// Look up a wand/rod definition by item id.
    pub fn get_wand(&self, item_id: u16) -> Option<&WandDef> {
        self.wands.get(&item_id)
    }

    /// Look up a distance weapon definition by item id.
    pub fn get_distance(&self, item_id: u16) -> Option<&DistanceWeaponDef> {
        self.distance.get(&item_id)
    }

    /// Total number of registered weapons across all types.
    pub fn len(&self) -> usize {
        self.wands.len() + self.distance.len() + self.melee.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Parse a vocation element string into a `CombatType`.
/// C++ `luaWeaponElement` — `luascript.cpp:17747-17777`.
pub fn parse_element_string(s: &str) -> CombatType {
    match s.to_ascii_lowercase().as_str() {
        "earth" | "poison" => CombatType::Earth,
        "ice" => CombatType::Ice,
        "energy" => CombatType::Energy,
        "fire" => CombatType::Fire,
        "death" => CombatType::Death,
        "holy" => CombatType::Holy,
        "physical" => CombatType::Physical,
        "undefined" => CombatType::Undefined,
        "lifedrain" => CombatType::LifeDrain,
        "manadrain" => CombatType::ManaDrain,
        "healing" => CombatType::Healing,
        "drown" => CombatType::Drown,
        _ => CombatType::Physical,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_element_string_covers_all_wand_elements() {
        assert_eq!(parse_element_string("energy"), CombatType::Energy);
        assert_eq!(parse_element_string("fire"), CombatType::Fire);
        assert_eq!(parse_element_string("earth"), CombatType::Earth);
        assert_eq!(parse_element_string("ice"), CombatType::Ice);
        assert_eq!(parse_element_string("holy"), CombatType::Holy);
        assert_eq!(parse_element_string("death"), CombatType::Death);
        // Unknown defaults to Physical (safe fallback).
        assert_eq!(parse_element_string("unknown"), CombatType::Physical);
    }

    #[test]
    fn wand_registry_lookup() {
        let mut reg = WeaponRegistry::default();
        reg.wands.insert(
            2190,
            WandDef {
                item_id: 2190,
                level: 7,
                mana_cost: 2,
                element: CombatType::Energy,
                damage_min: 8,
                damage_max: 18,
                vocations: HashMap::new(),
            },
        );
        let w = reg.get_wand(2190).expect("wand must be registered");
        assert_eq!(w.level, 7);
        assert_eq!(w.damage_min, 8);
        assert_eq!(w.damage_max, 18);
        assert!(reg.get_wand(9999).is_none());
    }
}
