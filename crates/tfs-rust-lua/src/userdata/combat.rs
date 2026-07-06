//! `Combat` userdata + `createCombatArea` global for Lua — PC-2b.
//!
//! C++ reference:
//! - `luascript.cpp:2855-2871` — `Combat` metatable registration.
//! - `luascript.cpp:3545-3575` — `luaCreateCombatArea`.
//! - `luascript.cpp:13015-13021` — `luaCombatCreate`.
//! - `luascript.cpp:13032-13052` — `luaCombatSetParameter`.
//! - `luascript.cpp:13093-13119` — `luaCombatSetArea`.
//! - `combat.h:118` — `Combat` class.
//!
//! The Lua `Combat` is a config bag: `Combat()` creates a `CombatDef`, `:setParameter`
//! / `:setArea` / `:setCallback` / `:setFormula` populate it, and `:execute` dispatches
//! to the core combat execution layer. The area matrix is stored as a `Vec<Vec<u8>>`
//! where `3` = caster origin, `1` = affected, `0` = unaffected.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use mlua::{Lua, UserData, UserDataMethods};

use tfs_rust_common::enums::CombatType;

/// Area combat matrix — the Rust side of `createCombatArea(areaMatrix)`.
/// C++ `AreaCombat` — `combat.h`. The matrix is a 2D grid where each cell is:
/// - `0` = unaffected
/// - `1` = affected
/// - `3` = caster origin (center)
#[derive(Debug, Clone, Default)]
pub struct AreaCombat {
    /// Row-major matrix of cells (`matrix[row][col]`).
    pub matrix: Vec<Vec<u8>>,
    /// Caster origin row (index of the row containing `3`).
    pub center_row: usize,
    /// Caster origin col (index of the col containing `3`).
    pub center_col: usize,
}

impl AreaCombat {
    /// Parse a Lua 2D table into an `AreaCombat`.
    /// C++ `LuaScriptInterface::getArea` — `luascript.cpp`.
    pub fn from_matrix(matrix: Vec<Vec<u8>>) -> Self {
        let mut center_row = 0;
        let mut center_col = 0;
        for (r, row) in matrix.iter().enumerate() {
            for (c, &cell) in row.iter().enumerate() {
                if cell == 3 {
                    center_row = r;
                    center_col = c;
                }
            }
        }
        Self {
            matrix,
            center_row,
            center_col,
        }
    }

    /// Returns the relative offsets of affected tiles (cell value `1` or `3`),
    /// relative to the caster origin.
    pub fn affected_offsets(&self) -> Vec<(i32, i32)> {
        let mut offsets = Vec::new();
        for (r, row) in self.matrix.iter().enumerate() {
            for (c, &cell) in row.iter().enumerate() {
                if cell != 0 {
                    let dr = r as i32 - self.center_row as i32;
                    let dc = c as i32 - self.center_col as i32;
                    offsets.push((dr, dc));
                }
            }
        }
        offsets
    }
}

/// Combat parameter keys — mirrors `CombatParam_t` (`enums.h:113-124`).
/// Kept as `i32` to match the Lua-facing enum values.
const COMBAT_PARAM_TYPE: i32 = 0;
const COMBAT_PARAM_EFFECT: i32 = 1;
const COMBAT_PARAM_DISTANCEEFFECT: i32 = 2;
const COMBAT_PARAM_BLOCKSHIELD: i32 = 3;
const COMBAT_PARAM_BLOCKARMOR: i32 = 4;
const COMBAT_PARAM_AGGRESSIVE: i32 = 7;

/// Callback parameter keys — mirrors `CallbackParam_t` (`enums.h:128-131`).
const CALLBACK_PARAM_LEVELMAGICVALUE: i32 = 0;
const CALLBACK_PARAM_SKILLVALUE: i32 = 1;
const CALLBACK_PARAM_TARGETTILE: i32 = 2;
const CALLBACK_PARAM_TARGETCREATURE: i32 = 3;

/// Formula type — mirrors `formulaType_t` (`enums.h:244-247`).
#[derive(Debug, Clone, Copy, Default)]
pub enum FormulaType {
    #[default]
    Undefined,
    LevelMagic,
    Skill,
    Damage,
}

/// A formula definition set via `combat:setFormula(type, mina, minb, maxa, maxb)`.
/// C++ `Combat::setPlayerCombatValues` — `combat.cpp`.
#[derive(Debug, Clone, Default)]
pub struct FormulaDef {
    pub formula_type: FormulaType,
    pub min_a: f64,
    pub min_b: f64,
    pub max_a: f64,
    pub max_b: f64,
}

/// A combat callback — `combat:setCallback(param, functionName)`.
/// C++ `Combat::setCallback` — `combat.cpp`. The callback name is a Lua global
/// function name resolved at execution time.
#[derive(Debug, Clone, Default)]
pub struct CombatCallback {
    pub param: i32,
    pub function_name: String,
}

/// The Rust-side `Combat` definition — a config bag accumulated during script loading.
/// C++ `Combat` class — `combat.h:118`.
#[derive(Debug, Clone, Default)]
pub struct CombatDef {
    /// `COMBAT_PARAM_TYPE` → combat damage type (bit-flag value).
    pub combat_type: i32,
    /// `COMBAT_PARAM_EFFECT` → magic effect id (CONST_ME_*).
    pub effect: i32,
    /// `COMBAT_PARAM_DISTANCEEFFECT` → shoot type (CONST_ANI_*).
    pub distance_effect: i32,
    /// `COMBAT_PARAM_BLOCKSHIELD` → whether defense is applied.
    pub block_shield: bool,
    /// `COMBAT_PARAM_BLOCKARMOR` → whether armor is applied.
    pub block_armor: bool,
    /// `COMBAT_PARAM_AGGRESSIVE` → whether the combat is aggressive (PZ lock).
    pub aggressive: bool,
    /// Optional area matrix set via `combat:setArea(areaId)`.
    pub area: Option<Rc<RefCell<AreaCombat>>>,
    /// Optional formula set via `combat:setFormula`.
    pub formula: Option<FormulaDef>,
    /// Callbacks keyed by `CallbackParam_t`.
    pub callbacks: HashMap<i32, CombatCallback>,
}

impl CombatDef {
    pub fn new() -> Self {
        Self::default()
    }

    /// `Combat::setParam` — `combat.cpp`. Dispatches on the `COMBAT_PARAM_*` key.
    /// C++ `luascript.cpp:13032-13052`.
    pub fn set_parameter(&mut self, key: i32, value: i32) {
        match key {
            k if k == COMBAT_PARAM_TYPE => self.combat_type = value,
            k if k == COMBAT_PARAM_EFFECT => self.effect = value,
            k if k == COMBAT_PARAM_DISTANCEEFFECT => self.distance_effect = value,
            k if k == COMBAT_PARAM_BLOCKSHIELD => self.block_shield = value != 0,
            k if k == COMBAT_PARAM_BLOCKARMOR => self.block_armor = value != 0,
            k if k == COMBAT_PARAM_AGGRESSIVE => self.aggressive = value != 0,
            _ => {} // Unknown params silently ignored (C++ `default: break`).
        }
    }

    /// `Combat::setCallback` — stores the callback function name for a given param.
    pub fn set_callback(&mut self, param: i32, function_name: String) {
        self.callbacks.insert(
            param,
            CombatCallback {
                param,
                function_name,
            },
        );
    }

    /// `Combat::setFormula` — `combat.cpp`.
    pub fn set_formula(&mut self, formula_type: i32, min_a: f64, min_b: f64, max_a: f64, max_b: f64) {
        let ft = match formula_type {
            1 => FormulaType::LevelMagic,
            2 => FormulaType::Skill,
            3 => FormulaType::Damage,
            _ => FormulaType::Undefined,
        };
        self.formula = Some(FormulaDef {
            formula_type: ft,
            min_a,
            min_b,
            max_a,
            max_b,
        });
    }

    /// Resolve the Lua bit-flag combat type to the Rust `CombatType` enum.
    pub fn resolved_combat_type(&self) -> CombatType {
        match self.combat_type {
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
}

/// Make the combat type constants accessible to the spell userdata module.
pub fn is_skill_callback(param: i32) -> bool {
    param == CALLBACK_PARAM_SKILLVALUE
}

pub fn is_levelmagic_callback(param: i32) -> bool {
    param == CALLBACK_PARAM_LEVELMAGICVALUE
}

pub fn is_targettile_callback(param: i32) -> bool {
    param == CALLBACK_PARAM_TARGETTILE
}

pub fn is_targetcreature_callback(param: i32) -> bool {
    param == CALLBACK_PARAM_TARGETCREATURE
}

/// Lua-facing `Combat()` userdata — newtype wrapper around `Rc<RefCell<CombatDef>>`
/// to satisfy Rust's orphan rule (`UserData` is a foreign trait, `Rc<RefCell<T>>` is a
/// foreign type). The wrapper is `Clone` (cheap `Rc` clone) so Lua can hold multiple
/// references to the same combat definition.
#[derive(Clone)]
pub struct CombatRef(pub Rc<RefCell<CombatDef>>);

/// Lua-facing `createCombatArea()` return value — newtype wrapper for the same
/// orphan-rule reason as `CombatRef`.
#[derive(Clone)]
pub struct AreaRef(pub Rc<RefCell<AreaCombat>>);

impl UserData for AreaRef {}

/// Register the `Combat` metatable and `createCombatArea` global.
pub fn register_combat_metatable(lua: &Lua) -> Result<(), mlua::Error> {
    // Register the Combat metatable via register_userdata_type so `Combat()` constructor
    // can create instances. The constructor is registered as a global function below.
    lua.register_userdata_type::<CombatRef>(|_registry| {})?;
    lua.register_userdata_type::<AreaRef>(|_registry| {})?;

    // `Combat()` constructor — C++ `luaCombatCreate` (`luascript.cpp:13015`).
    let combat_new = lua.create_function(|_, ()| {
        Ok(CombatRef(Rc::new(RefCell::new(CombatDef::new()))))
    })?;
    lua.globals().set("Combat", combat_new)?;

    // `createCombatArea(areaMatrix[, extArea])` — C++ `luaCreateCombatArea`
    // (`luascript.cpp:3545-3575`). Returns an `AreaRef` userdata. The optional `extArea`
    // diagonal overlay is accepted but not yet processed (PC-2b scope: matrix only).
    let create_area = lua.create_function(|_, (area, _ext): (mlua::Value, Option<mlua::Value>)| {
        let matrix = parse_area_matrix(&area)?;
        let area_combat = AreaCombat::from_matrix(matrix);
        Ok(AreaRef(Rc::new(RefCell::new(area_combat))))
    })?;
    lua.globals().set("createCombatArea", create_area)?;

    Ok(())
}

/// Parse a Lua 2D table (table of tables of numbers) into a `Vec<Vec<u8>>`.
/// C++ `LuaScriptInterface::getArea` — `luascript.cpp`.
fn parse_area_matrix(value: &mlua::Value) -> Result<Vec<Vec<u8>>, mlua::Error> {
    let table = value.as_table().ok_or_else(|| {
        mlua::Error::runtime("createCombatArea: area argument must be a table")
    })?;
    let mut matrix = Vec::new();
    for pair in table.pairs::<i64, mlua::Value>() {
        let (_, row_value) = pair?;
        let row_table = row_value.as_table().ok_or_else(|| {
            mlua::Error::runtime("createCombatArea: each row must be a table")
        })?;
        let mut row = Vec::new();
        for cell_pair in row_table.pairs::<i64, mlua::Value>() {
            let (_, cell_value) = cell_pair?;
            let cell: i64 = cell_value
                .as_integer()
                .ok_or_else(|| mlua::Error::runtime("createCombatArea: cell values must be integers"))?;
            row.push(cell as u8);
        }
        matrix.push(row);
    }
    Ok(matrix)
}

impl UserData for CombatRef {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // `combat:setParameter(key, value)` — `luascript.cpp:13032-13052`.
        // Accepts boolean or integer value (C++ coerces `true` → 1, `false` → 0).
        methods.add_method_mut("setParameter", |_, this, (key, value): (i32, mlua::Value)| {
            let v = match value {
                mlua::Value::Boolean(b) => if b { 1 } else { 0 },
                mlua::Value::Integer(n) => n as i32,
                _ => return Err(mlua::Error::runtime("setParameter: value must be boolean or integer")),
            };
            this.0.borrow_mut().set_parameter(key, v);
            Ok(true)
        });

        // `combat:setArea(areaId)` — `luascript.cpp:13093-13119`.
        // `areaId` is an `AreaRef` userdata returned by `createCombatArea`. We use
        // `AnyUserData` + `borrow()` because mlua 0.10 doesn't auto-impl `FromLua`
        // for custom `UserData` newtypes.
        methods.add_method_mut("setArea", |_, this, area: mlua::AnyUserData| {
            let area_ref = area.borrow::<AreaRef>()?;
            this.0.borrow_mut().area = Some(area_ref.0.clone());
            Ok(true)
        });

        // `combat:setFormula(type, mina, minb, maxa, maxb)` — `luascript.cpp:13073-13091`.
        methods.add_method_mut("setFormula", |_, this, (ft, mina, minb, maxa, maxb): (i32, f64, f64, f64, f64)| {
            this.0.borrow_mut().set_formula(ft, mina, minb, maxa, maxb);
            Ok(true)
        });

        // `combat:setCallback(param, functionName)` — `luascript.cpp`.
        methods.add_method_mut("setCallback", |_, this, (param, name): (i32, String)| {
            this.0.borrow_mut().set_callback(param, name);
            Ok(true)
        });

        // `combat:execute(creature, variant)` — `luascript.cpp:13198+`.
        // PC-2b scope: the execution dispatch to the core combat layer is a stub that
        // returns `true` (success). Full combat execution (area resolution + damage
        // application via `combat_execute_with_stimulus`) is PC-3a scope. This stub
        // allows spell scripts to load without errors.
        methods.add_method("execute", |_, _this, (_creature, _variant): (mlua::Value, mlua::Value)| {
            // TODO(PC-3a): wire to `combat_execute_lua(world, caster_id, &combat_def, &variant)`.
            Ok(true)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn area_combat_parses_square1x1() {
        // AREA_SQUARE1X1 = { {1,1,1}, {1,3,1}, {1,1,1} }
        let matrix = vec![
            vec![1, 1, 1],
            vec![1, 3, 1],
            vec![1, 1, 1],
        ];
        let area = AreaCombat::from_matrix(matrix);
        assert_eq!(area.center_row, 1);
        assert_eq!(area.center_col, 1);
        let offsets = area.affected_offsets();
        assert_eq!(offsets.len(), 9); // 3x3 = 9 affected tiles (including center)
        assert!(offsets.contains(&(0, 0))); // center
        assert!(offsets.contains(&(-1, -1))); // top-left
        assert!(offsets.contains(&(1, 1))); // bottom-right
    }

    #[test]
    fn combat_def_set_parameter_dispatches_correctly() {
        let mut def = CombatDef::new();
        def.set_parameter(COMBAT_PARAM_TYPE, 1); // COMBAT_PHYSICALDAMAGE
        assert_eq!(def.combat_type, 1);
        assert_eq!(def.resolved_combat_type(), CombatType::Physical);

        def.set_parameter(COMBAT_PARAM_AGGRESSIVE, 1);
        assert!(def.aggressive);

        def.set_parameter(COMBAT_PARAM_BLOCKARMOR, 0);
        assert!(!def.block_armor);
    }

    #[test]
    fn combat_def_set_callback_stores_function() {
        let mut def = CombatDef::new();
        def.set_callback(CALLBACK_PARAM_SKILLVALUE, "onGetFormulaValues".to_string());
        assert_eq!(
            def.callbacks.get(&CALLBACK_PARAM_SKILLVALUE).unwrap().function_name,
            "onGetFormulaValues"
        );
    }

    #[test]
    fn create_combat_area_from_lua() {
        let lua = Lua::new();
        register_combat_metatable(&lua).expect("registration must succeed");
        crate::combat_enums::register_combat_enums(&lua).expect("enum registration must succeed");

        // Test createCombatArea with AREA_SQUARE1X1 equivalent.
        let result: mlua::AnyUserData = lua
            .load(r#"
                local area = createCombatArea({
                    {1, 1, 1},
                    {1, 3, 1},
                    {1, 1, 1}
                })
                return area
            "#)
            .eval()
            .expect("createCombatArea must succeed");
        let area_ref = result.borrow::<AreaRef>().expect("must be AreaRef");
        let area = area_ref.0.borrow();
        assert_eq!(area.center_row, 1);
        assert_eq!(area.center_col, 1);
        assert_eq!(area.matrix.len(), 3);
        assert_eq!(area.matrix[0].len(), 3);
    }

    #[test]
    fn combat_userdata_set_parameter_and_set_area() {
        let lua = Lua::new();
        register_combat_metatable(&lua).expect("registration must succeed");
        crate::combat_enums::register_combat_enums(&lua).expect("enum registration must succeed");

        let result: mlua::AnyUserData = lua
            .load(r#"
                local combat = Combat()
                combat:setParameter(COMBAT_PARAM_TYPE, COMBAT_PHYSICALDAMAGE)
                combat:setParameter(COMBAT_PARAM_EFFECT, CONST_ME_HITAREA)
                combat:setParameter(COMBAT_PARAM_BLOCKARMOR, true)
                combat:setParameter(COMBAT_PARAM_AGGRESSIVE, true)
                local area = createCombatArea({
                    {1, 1, 1},
                    {1, 3, 1},
                    {1, 1, 1}
                })
                combat:setArea(area)
                return combat
            "#)
            .eval()
            .expect("combat setup must succeed");
        let combat_ref = result.borrow::<CombatRef>().expect("must be CombatRef");
        let def = combat_ref.0.borrow();
        assert_eq!(def.combat_type, 1); // COMBAT_PHYSICALDAMAGE
        assert_eq!(def.effect, 10); // CONST_ME_HITAREA
        assert!(def.block_armor);
        assert!(def.aggressive);
        assert!(def.area.is_some());
    }
}
