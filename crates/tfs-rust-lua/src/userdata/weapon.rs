//! `Weapon` userdata for Lua — PC-2b / PC-3a.
//!
//! C++ reference:
//! - `luascript.cpp:3209-3246` — `luaCreateWeapon` / Weapon metatable registration.
//! - `luascript.cpp:17556-17586` — `luaWeaponRegister`.
//! - `luascript.cpp:17729-17745` — `luaWeaponWandDamage`.
//! - `luascript.cpp:17747-17777` — `luaWeaponElement`.
//! - `weapons.cpp:485` — `Weapon::executeUseWeapon` / `onUseWeapon(player, var)`.
//! - `weapons.h:53-293` — `Weapon` / `WeaponWand` / `WeaponDistance` / `WeaponMelee`.
//!
//! The Lua `Weapon` is a config bag: `Weapon(WEAPON_WAND)` creates a `WeaponBuilder`,
//! `:id`/`:level`/`:mana`/`:element`/`:damage`/`:vocation` populate it, and
//! `:register()` pushes a `PendingWeapon` into the runtime's pending buffer.
//! `function weapon.onUseWeapon(...)` is captured like `spell.onCastSpell`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use mlua::{Lua, RegistryKey, UserData, UserDataMethods, Value};

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
    /// `weapon:breakChance(n)` — 0..=100; used with `action("move")`.
    pub break_chance: u8,
    /// `weapon:action("move"|"removecount"|"removecharge")`.
    pub consume_action: tfs_rust_content::weapons::WeaponConsumeAction,
    /// True when an `onUseWeapon` callback was captured for this registration.
    pub has_on_use: bool,
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

/// Lua-facing `Weapon(type)` builder.
///
/// `on_use_fn` holds the Lua callback captured via `__newindex` /
/// `:onUseWeapon(fn)` (`function weapon.onUseWeapon(player, variant[, hit])`).
#[derive(Clone)]
pub struct WeaponBuilder {
    pub weapon: Rc<RefCell<PendingWeapon>>,
    pub on_use_fn: Rc<RefCell<Option<RegistryKey>>>,
}

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
        Ok(Some(WeaponBuilder {
            weapon: Rc::new(RefCell::new(PendingWeapon::new(weapon_type))),
            on_use_fn: Rc::new(RefCell::new(None)),
        }))
    })?;
    lua.globals().set("Weapon", weapon_new)?;

    Ok(())
}

impl UserData for WeaponBuilder {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // `weapon:id(id)` — `luascript.cpp:17829`.
        methods.add_method_mut("id", |_, this, id: u16| {
            this.weapon.borrow_mut().item_id = id;
            Ok(true)
        });

        // `weapon:level(level)` — sets minimum wield level.
        methods.add_method_mut("level", |_, this, level: u32| {
            this.weapon.borrow_mut().level = level;
            Ok(true)
        });

        // `weapon:magicLevel(level)` — sets minimum magic level.
        methods.add_method_mut("magicLevel", |_, this, level: u32| {
            this.weapon.borrow_mut().magic_level = level;
            Ok(true)
        });

        // `weapon:mana(mana)` — sets mana cost per attack.
        methods.add_method_mut("mana", |_, this, mana: u32| {
            this.weapon.borrow_mut().mana_cost = mana;
            Ok(true)
        });

        // `weapon:element(combatType)` — `luascript.cpp:17747-17777`.
        methods.add_method_mut("element", |_, this, value: mlua::Value| {
            let element = match value {
                mlua::Value::String(s) => {
                    let s = s.to_str()?.to_string();
                    parse_element_string(&s)
                }
                mlua::Value::Integer(n) => bitflag_to_combat_type(n as i32),
                _ => return Err(mlua::Error::runtime("element: expected string or integer")),
            };
            this.weapon.borrow_mut().element = element;
            Ok(true)
        });

        // `weapon:damage(min, max)` — `luascript.cpp:17729-17745`. Wand-only.
        methods.add_method_mut("damage", |_, this, (min, max): (u32, u32)| {
            let mut b = this.weapon.borrow_mut();
            b.damage_min = min;
            b.damage_max = max;
            Ok(true)
        });

        // `weapon:vocation(name, allowed)` — adds a vocation filter entry.
        methods.add_method_mut("vocation", |_, this, (name, allowed): (String, bool)| {
            this.weapon.borrow_mut().vocations.insert(name, allowed);
            Ok(true)
        });

        // `weapon:breakChance(chance)` — `luascript.cpp` `luaWeaponBreakChance`.
        methods.add_method_mut("breakChance", |_, this, chance: u8| {
            this.weapon.borrow_mut().break_chance = chance.min(100);
            Ok(true)
        });

        // `weapon:action(action)` — `luascript.cpp` `luaWeaponAction`.
        methods.add_method_mut("action", |_, this, action: String| {
            use tfs_rust_content::weapons::WeaponConsumeAction;
            let consume = match action.to_ascii_lowercase().as_str() {
                "move" => WeaponConsumeAction::Move,
                "removecharge" => WeaponConsumeAction::RemoveCharge,
                _ => WeaponConsumeAction::RemoveCount,
            };
            this.weapon.borrow_mut().consume_action = consume;
            Ok(true)
        });

        // `weapon:onUseWeapon(fn)` — TFS / compat.lua when `function weapon.onUseWeapon`.
        methods.add_method_mut("onUseWeapon", |lua, this, value: Value| {
            if let Value::Function(func) = value {
                let registry_key = lua.create_registry_value(func)?;
                *this.on_use_fn.borrow_mut() = Some(registry_key);
                this.weapon.borrow_mut().has_on_use = true;
            }
            Ok(true)
        });

        // `weapon:register()` — `luascript.cpp:17556-17586`.
        methods.add_method("register", |lua, this, ()| {
            let globals = lua.globals();
            let pending: mlua::Table = globals.get("_pending_weapons")?;
            let len = pending.len()?;
            let idx = len + 1;
            let snapshot = this.weapon.borrow().clone();
            pending.set(idx, snapshot)?;

            let callback_key = this.on_use_fn.borrow_mut().take();
            if let Some(key) = callback_key {
                let callbacks: mlua::Table = globals.get("_pending_weapon_callbacks")?;
                let func: mlua::Function = lua.registry_value(&key)?;
                callbacks.set(idx, func)?;
            }

            Ok(true)
        });

        // `__newindex` — captures `function weapon.onUseWeapon(...)` without compat.lua.
        methods.add_meta_method(
            mlua::MetaMethod::NewIndex,
            |lua, this, (key, value): (String, Value)| {
                if key == "onUseWeapon" && let Value::Function(func) = value {
                    let registry_key = lua.create_registry_value(func)?;
                    *this.on_use_fn.borrow_mut() = Some(registry_key);
                    this.weapon.borrow_mut().has_on_use = true;
                }
                Ok(())
            },
        );
    }
}

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

impl UserData for PendingWeapon {}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_pending(lua: &Lua) {
        lua.globals()
            .set("_pending_weapons", lua.create_table().unwrap())
            .unwrap();
        lua.globals()
            .set("_pending_weapon_callbacks", lua.create_table().unwrap())
            .unwrap();
    }

    #[test]
    fn weapon_constructor_creates_wand_builder() {
        let lua = Lua::new();
        register_weapon_metatable(&lua).expect("registration must succeed");
        crate::combat_enums::register_combat_enums(&lua).expect("enum registration must succeed");
        setup_pending(&lua);

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
        let w = w_ref.weapon.borrow();
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
        setup_pending(&lua);

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

    #[test]
    fn weapon_break_chance_and_action_methods() {
        let lua = Lua::new();
        register_weapon_metatable(&lua).expect("registration must succeed");
        crate::combat_enums::register_combat_enums(&lua).expect("enum registration must succeed");
        setup_pending(&lua);

        let result: mlua::AnyUserData = lua
            .load(
                r#"
                local weapon = Weapon(WEAPON_DISTANCE)
                weapon:id(2389)
                weapon:action("move")
                weapon:breakChance(7)
                return weapon
            "#,
            )
            .eval()
            .expect("weapon setup must succeed");
        let w_ref = result
            .borrow::<WeaponBuilder>()
            .expect("must be WeaponBuilder");
        let w = w_ref.weapon.borrow();
        assert_eq!(w.item_id, 2389);
        assert_eq!(w.break_chance, 7);
        assert_eq!(
            w.consume_action,
            tfs_rust_content::weapons::WeaponConsumeAction::Move
        );
    }

    #[test]
    fn weapon_on_use_weapon_captured_via_newindex() {
        let lua = Lua::new();
        register_weapon_metatable(&lua).expect("registration must succeed");
        crate::combat_enums::register_combat_enums(&lua).expect("enum registration must succeed");
        setup_pending(&lua);

        lua.load(
            r#"
            local weapon = Weapon(WEAPON_AMMO)
            function weapon.onUseWeapon(player, variant, hit)
                return true
            end
            weapon:id(2546)
            weapon:action("removecount")
            weapon:register()
        "#,
        )
        .exec()
        .expect("burst-style register must succeed");

        let pending: mlua::Table = lua.globals().get("_pending_weapons").unwrap();
        let ud: mlua::AnyUserData = pending.get(1).unwrap();
        let pw = ud.borrow::<PendingWeapon>().unwrap();
        assert!(pw.has_on_use);
        assert_eq!(pw.item_id, 2546);

        let callbacks: mlua::Table = lua.globals().get("_pending_weapon_callbacks").unwrap();
        assert!(callbacks.get::<mlua::Function>(1).is_ok());
    }
}
