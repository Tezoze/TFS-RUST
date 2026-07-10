//! `Weapon` userdata for Lua — PC-2b.
//!
//! C++ reference:
//! - `luascript.cpp:3209-3246` — `luaCreateWeapon` / Weapon metatable registration.
//! - `luascript.cpp:17556-17586` — `luaWeaponRegister`.
//! - `luascript.cpp:17729-17745` — `luaWeaponWandDamage`.
//! - `luascript.cpp:17747-17777` — `luaWeaponElement`.
//! - `weapons.h:53-293` — `Weapon` / `WeaponWand` / `WeaponDistance` / `WeaponMelee`.
//!
//! The Lua `Weapon` is a config bag: `Weapon(WEAPON_WAND)` creates a `WeaponBuilder`,
//! `:id`/`:level`/`:mana`/`:element`/`:damage`/`:vocation` populate it, and
//! `:register()` pushes a `PendingWeapon` into the runtime's pending buffer.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use mlua::{Lua, UserData, UserDataMethods};

use tfs_rust_common::enums::CombatType;
use tfs_rust_content::weapons::parse_element_string;

/// Weapon type constants — mirrors `const.h:143-150`.
const WEAPON_NONE: i32 = 0;
const WEAPON_SHIELD: i32 = 4;

/// A pending weapon registration — drained from the Lua runtime into a `WeaponRegistry`.
/// Stored as a `PendingWeapon` (not `WeaponBuilder`) in the `_pending_weapons` table
/// so the loader can iterate without holding `Rc` references to the builder.
#[derive(Debug, Clone, Default)]
pub struct PendingWeapon {
    pub weapon_type: i32,
    pub item_id: u16,
    pub level: u32,
    pub magic_level: u32,
    pub mana_cost: u32,
    pub element: CombatType,
    pub damage_min: u32,
    pub damage_max: u32,
    /// Vocation name → allowed (TFS `vocation(name, bool)`).
    pub vocations: HashMap<String, bool>,
}

impl PendingWeapon {
    pub fn new(weapon_type: i32) -> Self {
        Self {
            weapon_type,
            element: CombatType::Physical,
            ..Default::default()
        }
    }
}

/// Lua-facing `Weapon(type)` builder — newtype wrapper around `Rc<RefCell<PendingWeapon>>`
/// to satisfy Rust's orphan rule.
#[derive(Clone)]
pub struct WeaponBuilder(pub Rc<RefCell<PendingWeapon>>);

/// Register the `Weapon` metatable + constructor.
pub fn register_weapon_metatable(lua: &Lua) -> Result<(), mlua::Error> {
    lua.register_userdata_type::<WeaponBuilder>(|_registry| {})?;
    lua.register_userdata_type::<PendingWeapon>(|_registry| {})?;

    // `Weapon(type)` constructor — C++ `luaCreateWeapon` (`luascript.cpp:17474`).
    let weapon_new = lua.create_function(|_, weapon_type: i32| {
        // C++ returns `nil` for unsupported types (NONE/SHIELD).
        if matches!(weapon_type, WEAPON_NONE | WEAPON_SHIELD) {
            return Ok(None::<WeaponBuilder>);
        }
        Ok(Some(WeaponBuilder(Rc::new(RefCell::new(
            PendingWeapon::new(weapon_type),
        )))))
    })?;
    lua.globals().set("Weapon", weapon_new)?;

    Ok(())
}

impl UserData for WeaponBuilder {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // `weapon:id(id)` — `luascript.cpp:17829`.
        methods.add_method_mut("id", |_, this, id: u16| {
            this.0.borrow_mut().item_id = id;
            Ok(true)
        });

        // `weapon:level(level)` — sets minimum wield level.
        methods.add_method_mut("level", |_, this, level: u32| {
            this.0.borrow_mut().level = level;
            Ok(true)
        });

        // `weapon:magicLevel(level)` — sets minimum magic level.
        methods.add_method_mut("magicLevel", |_, this, level: u32| {
            this.0.borrow_mut().magic_level = level;
            Ok(true)
        });

        // `weapon:mana(mana)` — sets mana cost per attack.
        methods.add_method_mut("mana", |_, this, mana: u32| {
            this.0.borrow_mut().mana_cost = mana;
            Ok(true)
        });

        // `weapon:element(combatType)` — `luascript.cpp:17747-17777`.
        // Accepts a string ("energy"/"fire"/"earth"/...) or a numeric COMBAT_* constant.
        methods.add_method_mut("element", |_, this, value: mlua::Value| {
            let element = match value {
                mlua::Value::String(s) => {
                    let s = s.to_str()?.to_string();
                    parse_element_string(&s)
                }
                mlua::Value::Integer(n) => bitflag_to_combat_type(n as i32),
                _ => return Err(mlua::Error::runtime("element: expected string or integer")),
            };
            this.0.borrow_mut().element = element;
            Ok(true)
        });

        // `weapon:damage(min, max)` — `luascript.cpp:17729-17745`. Wand-only.
        methods.add_method_mut("damage", |_, this, (min, max): (u32, u32)| {
            let mut b = this.0.borrow_mut();
            b.damage_min = min;
            b.damage_max = max;
            Ok(true)
        });

        // `weapon:vocation(name, allowed)` — adds a vocation filter entry.
        methods.add_method_mut("vocation", |_, this, (name, allowed): (String, bool)| {
            this.0.borrow_mut().vocations.insert(name, allowed);
            Ok(true)
        });

        // `weapon:register()` — `luascript.cpp:17556-17586`.
        // Pushes a snapshot of this weapon's config into the `_pending_weapons` global
        // table for later draining by the script loader.
        methods.add_method("register", |lua, this, ()| {
            let globals = lua.globals();
            let pending: mlua::Table = globals.get("_pending_weapons")?;
            let len = pending.len()?;
            let snapshot = this.0.borrow().clone();
            pending.set(len + 1, snapshot)?;
            Ok(true)
        });
    }
}

/// Map a Lua bit-flag combat type (1=physical, 2=energy, 4=earth, ...) to the Rust enum.
fn bitflag_to_combat_type(value: i32) -> CombatType {
    match value {
        1 => CombatType::Physical,
        2 => CombatType::Energy,
        4 => CombatType::Earth,
        8 => CombatType::Fire,
        16 => CombatType::Undefined,
        32 => CombatType::LifeDrain,
        64 => CombatType::ManaDrain,
        128 => CombatType::Healing,
        _ => CombatType::Physical,
    }
}

// `PendingWeapon` must be UserData so it can be stored in the `_pending_weapons` Lua table.
impl UserData for PendingWeapon {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weapon_constructor_creates_wand_builder() {
        let lua = Lua::new();
        register_weapon_metatable(&lua).expect("registration must succeed");
        crate::combat_enums::register_combat_enums(&lua).expect("enum registration must succeed");
        // Initialize _pending_weapons table (normally done by the script loader).
        lua.globals()
            .set("_pending_weapons", lua.create_table().unwrap())
            .unwrap();

        let result: mlua::AnyUserData = lua
            .load(
                r#"
                local weapon = Weapon(WEAPON_WAND)
                weapon:level(7)
                weapon:mana(2)
                weapon:element("energy")
                weapon:damage(8, 18)
                weapon:vocation("Sorcerer", true)
                weapon:vocation("Master Sorcerer", false)
                weapon:id(2190)
                return weapon
            "#,
            )
            .eval()
            .expect("weapon setup must succeed");
        let w_ref = result
            .borrow::<WeaponBuilder>()
            .expect("must be WeaponBuilder");
        let w = w_ref.0.borrow();
        assert_eq!(w.item_id, 2190);
        assert_eq!(w.level, 7);
        assert_eq!(w.mana_cost, 2);
        assert_eq!(w.element, CombatType::Energy);
        assert_eq!(w.damage_min, 8);
        assert_eq!(w.damage_max, 18);
        assert_eq!(w.vocations.get("Sorcerer"), Some(&true));
        assert_eq!(w.vocations.get("Master Sorcerer"), Some(&false));
    }

    #[test]
    fn weapon_register_pushes_to_pending_table() {
        let lua = Lua::new();
        register_weapon_metatable(&lua).expect("registration must succeed");
        crate::combat_enums::register_combat_enums(&lua).expect("enum registration must succeed");
        lua.globals()
            .set("_pending_weapons", lua.create_table().unwrap())
            .unwrap();

        lua.load(
            r#"
            local weapon = Weapon(WEAPON_WAND)
            weapon:id(2190)
            weapon:level(7)
            weapon:mana(2)
            weapon:element("energy")
            weapon:damage(8, 18)
            weapon:register()
        "#,
        )
        .exec()
        .expect("weapon register must succeed");

        let pending: mlua::Table = lua.globals().get("_pending_weapons").unwrap();
        assert_eq!(pending.len().unwrap(), 1);
    }
}
