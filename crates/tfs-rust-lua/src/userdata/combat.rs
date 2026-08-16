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

use mlua::{Lua, MetaMethod, UserData, UserDataMethods, Value};

use tfs_rust_common::enums::CombatType;

use crate::context::{CURRENT_CTX, CreatureRef};
use crate::instruction_budget::with_lua_instruction_budget;
use crate::lua_mutation::{CombatExecuteRequest, call_combat_execute};
use crate::userdata::position::PositionRef;

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
    /// Optional diagonal overlay (`createCombatArea(area, extArea)`).
    /// C++ `AreaCombat::setupExtArea` — `combat.cpp:1426`.
    pub ext_matrix: Option<Vec<Vec<u8>>>,
    pub ext_center_row: usize,
    pub ext_center_col: usize,
}

impl AreaCombat {
    /// Parse a Lua 2D table into an `AreaCombat`.
    /// C++ `LuaScriptInterface::getArea` — `luascript.cpp`.
    pub fn from_matrix(matrix: Vec<Vec<u8>>) -> Self {
        let (center_row, center_col) = find_matrix_center(&matrix);
        Self {
            matrix,
            center_row,
            center_col,
            ext_matrix: None,
            ext_center_row: 0,
            ext_center_col: 0,
        }
    }

    /// Attach diagonal `extArea` overlay — C++ `setupExtArea`.
    pub fn with_ext_area(mut self, ext: Vec<Vec<u8>>) -> Self {
        let (r, c) = find_matrix_center(&ext);
        self.ext_matrix = Some(ext);
        self.ext_center_row = r;
        self.ext_center_col = c;
        self
    }

    /// Returns the relative offsets of affected tiles (cell value `1` or `3`),
    /// relative to the caster origin.
    ///
    /// Returns `(dx, dy)` = `(col_delta, row_delta)` where `dx` maps to the
    /// x-axis (east-west) and `dy` maps to the y-axis (north-south). The matrix
    /// is row-major (`matrix[row][col]`), so row maps to y and col maps to x.
    pub fn affected_offsets(&self) -> Vec<(i32, i32)> {
        matrix_affected_offsets(&self.matrix, self.center_row, self.center_col)
    }

    /// Offsets from the diagonal overlay matrix (NW orientation raw).
    pub fn ext_affected_offsets(&self) -> Option<Vec<(i32, i32)>> {
        self.ext_matrix
            .as_ref()
            .map(|m| matrix_affected_offsets(m, self.ext_center_row, self.ext_center_col))
    }

    pub fn has_ext_area(&self) -> bool {
        self.ext_matrix.is_some()
    }
}

fn find_matrix_center(matrix: &[Vec<u8>]) -> (usize, usize) {
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
    (center_row, center_col)
}

fn matrix_affected_offsets(
    matrix: &[Vec<u8>],
    center_row: usize,
    center_col: usize,
) -> Vec<(i32, i32)> {
    let mut offsets = Vec::new();
    for (r, row) in matrix.iter().enumerate() {
        for (c, &cell) in row.iter().enumerate() {
            if cell != 0 {
                let dy = r as i32 - center_row as i32;
                let dx = c as i32 - center_col as i32;
                offsets.push((dx, dy));
            }
        }
    }
    offsets
}

/// Combat parameter keys — mirrors `CombatParam_t` (`enums.h:113-124`).
/// Kept as `i32` to match the Lua-facing enum values.
const COMBAT_PARAM_TYPE: i32 = 0;
const COMBAT_PARAM_EFFECT: i32 = 1;
const COMBAT_PARAM_DISTANCEEFFECT: i32 = 2;
const COMBAT_PARAM_BLOCKSHIELD: i32 = 3;
const COMBAT_PARAM_BLOCKARMOR: i32 = 4;
const COMBAT_PARAM_CREATEITEM: i32 = 6; // enums.h:119
const COMBAT_PARAM_AGGRESSIVE: i32 = 7;
const COMBAT_PARAM_DISPEL: i32 = 8; // enums.h:121
const COMBAT_PARAM_NODAMAGE: i32 = 10; // enums.h:123

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
///
/// Domain: TFS `Combat::setCallback` / `CallBack::loadCallBack` (`combat.cpp` /
/// `baseevents.cpp`). At registration, TFS `getEvent` copies the named global into
/// a private registry id and **clears the global** so later scripts may reuse the
/// same name (`onGetFormulaValues`). We snapshot [`mlua::Function`] the same way.
#[derive(Clone, Default)]
pub struct CombatCallback {
    pub param: i32,
    pub function_name: String,
    /// Function captured at `setCallback` time (not re-looked-up at execute).
    pub function: Option<mlua::Function>,
}

impl std::fmt::Debug for CombatCallback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CombatCallback")
            .field("param", &self.param)
            .field("function_name", &self.function_name)
            .field("has_function", &self.function.is_some())
            .finish()
    }
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
    /// Conditions added via `combat:addCondition(condition)`.
    /// C++ `Combat::conditionList` — `combat.h`.
    pub conditions: Vec<crate::userdata::condition::ConditionBuilder>,
    /// `COMBAT_PARAM_DISPEL` → 772 bit-flag condition type to remove on hit.
    /// C++ `CombatParams::dispelType` — `combat.h:52`. `0` = unset.
    pub dispel_type: i32,
    /// `COMBAT_PARAM_CREATEITEM` → item id to create on hit tiles.
    /// C++ `Combat::createItem` — `combat.h`.
    pub create_item: i32,
    /// `COMBAT_PARAM_NODAMAGE` → whether combat applies no damage (e.g. soulfire).
    /// C++ `Combat::noDamage` — `combat.h`.
    pub no_damage: bool,
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
            k if k == COMBAT_PARAM_CREATEITEM => self.create_item = value,
            k if k == COMBAT_PARAM_AGGRESSIVE => self.aggressive = value != 0,
            k if k == COMBAT_PARAM_DISPEL => self.dispel_type = value,
            k if k == COMBAT_PARAM_NODAMAGE => self.no_damage = value != 0,
            _ => {} // Unknown params silently ignored (C++ `default: break`).
        }
    }

    /// `Combat::setCallback` / `CallBack::loadCallBack` — snapshot the Lua function.
    pub fn set_callback(
        &mut self,
        param: i32,
        function_name: String,
        function: Option<mlua::Function>,
    ) {
        self.callbacks.insert(
            param,
            CombatCallback {
                param,
                function_name,
                function,
            },
        );
    }

    /// `Combat::setFormula` — `combat.cpp`.
    pub fn set_formula(
        &mut self,
        formula_type: i32,
        min_a: f64,
        min_b: f64,
        max_a: f64,
        max_b: f64,
    ) {
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
    // `Combat` is a class table (extensible by `function Combat:getPositions(...)`
    // in `data/lib/core/combat.lua`) with a `__call` ctor. Gap 7a.
    let combat_new =
        lua.create_function(|_, ()| Ok(CombatRef(Rc::new(RefCell::new(CombatDef::new())))))?;
    crate::class_registry::register_class(lua, "Combat", Some(combat_new))?;

    // `createCombatArea(areaMatrix[, extArea])` — C++ `luaCreateCombatArea`
    // (`luascript.cpp:3545-3575`). Returns an `AreaRef` userdata. Optional
    // `extArea` is the diagonal overlay (`AREADIAGONAL_*`) — Phase 7.
    let create_area =
        lua.create_function(|_, (area, ext): (mlua::Value, Option<mlua::Value>)| {
            let matrix = parse_area_matrix(&area)?;
            let mut area_combat = AreaCombat::from_matrix(matrix);
            if let Some(ext_val) = ext {
                let ext_matrix = parse_area_matrix(&ext_val)?;
                area_combat = area_combat.with_ext_area(ext_matrix);
            }
            Ok(AreaRef(Rc::new(RefCell::new(area_combat))))
        })?;
    lua.globals().set("createCombatArea", create_area)?;

    Ok(())
}

/// Parse a Lua 2D table (table of tables of numbers) into a `Vec<Vec<u8>>`.
/// C++ `LuaScriptInterface::getArea` — `luascript.cpp`.
fn parse_area_matrix(value: &mlua::Value) -> Result<Vec<Vec<u8>>, mlua::Error> {
    let table = value
        .as_table()
        .ok_or_else(|| mlua::Error::runtime("createCombatArea: area argument must be a table"))?;
    let mut matrix = Vec::new();
    for pair in table.pairs::<i64, mlua::Value>() {
        let (_, row_value) = pair?;
        let row_table = row_value
            .as_table()
            .ok_or_else(|| mlua::Error::runtime("createCombatArea: each row must be a table"))?;
        let mut row = Vec::new();
        for cell_pair in row_table.pairs::<i64, mlua::Value>() {
            let (_, cell_value) = cell_pair?;
            let cell: i64 = cell_value.as_integer().ok_or_else(|| {
                mlua::Error::runtime("createCombatArea: cell values must be integers")
            })?;
            row.push(cell as u8);
        }
        matrix.push(row);
    }
    Ok(matrix)
}

impl UserData for CombatRef {
    fn register(registry: &mut mlua::UserDataRegistry<Self>) {
        crate::class_registry::register_with_recording(registry, "Combat");
    }

    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        // `combat:setParameter(key, value)` — `luascript.cpp:13032-13052`.
        // Accepts boolean or integer value (C++ coerces `true` → 1, `false` → 0).
        methods.add_method_mut(
            "setParameter",
            |_, this, (key, value): (i32, mlua::Value)| {
                let v = match value {
                    mlua::Value::Boolean(b) => {
                        if b {
                            1
                        } else {
                            0
                        }
                    }
                    mlua::Value::Integer(n) => n as i32,
                    _ => {
                        return Err(mlua::Error::runtime(
                            "setParameter: value must be boolean or integer",
                        ));
                    }
                };
                this.0.borrow_mut().set_parameter(key, v);
                Ok(true)
            },
        );

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
        methods.add_method_mut(
            "setFormula",
            |_, this, (ft, mina, minb, maxa, maxb): (i32, f64, f64, f64, f64)| {
                this.0.borrow_mut().set_formula(ft, mina, minb, maxa, maxb);
                Ok(true)
            },
        );

        // `combat:setCallback(param, functionName)` — `luascript.cpp:13157`.
        // Snapshot the global now (TFS `getEvent`) and clear it so later spell
        // scripts may redefine the same name without clobbering this combat.
        methods.add_method_mut("setCallback", |lua, this, (param, name): (i32, String)| {
            let func: mlua::Function = lua.globals().get(name.as_str()).map_err(|_| {
                mlua::Error::runtime(format!(
                    "combat:setCallback: global function '{name}' not found"
                ))
            })?;
            // TFS `LuaScriptInterface::getEvent` — `luascript.cpp:355-357`.
            lua.globals().set(name.as_str(), Value::Nil)?;
            this.0.borrow_mut().set_callback(param, name, Some(func));
            Ok(true)
        });

        // `combat:addCondition(condition)` — C++ `luaCombatAddCondition`
        // (`luascript.cpp:11812`). Clones the `ConditionBuilder` from the Lua
        // userdata and stores it in the combat's condition list.
        methods.add_method_mut("addCondition", |_, this, condition: mlua::AnyUserData| {
            let cond = condition
                .borrow::<crate::userdata::condition::ConditionBuilder>()?
                .clone();
            this.0.borrow_mut().conditions.push(cond);
            Ok(true)
        });

        // `combat:execute(creature, variant)` — `luascript.cpp:13198+`.
        // PC-3a: resolves the variant (NUMBER → target position, POSITION →
        // area at position), builds a `CombatExecuteRequest` with area offsets
        // from the combat's `AreaCombat` matrix (or empty for single-target),
        // and dispatches to the core via `call_combat_execute`.
        //
        // PC-3a Phase 1: when `formula` is `None` but a value callback is
        // registered (`CALLBACK_PARAM_LEVELMAGICVALUE` / `CALLBACK_PARAM_SKILLVALUE`),
        // invoke the Lua global function and use its `(min, max)` return as the
        // damage range. C++ `Combat::getCombatDamage` — `combat.cpp:100` →
        // `ValueCallback::getMinMaxValues` — `combat.cpp:1111-1170`.
        methods.add_method(
            "execute",
            |lua, this, (creature, variant): (Value, Value)| {
                let combat = this.0.borrow();
                let caster_id = resolve_creature_id(&creature)?;
                let (center_x, center_y, center_z) = resolve_variant_center(&variant, caster_id)?;

                // Resolve the caster's position — used for both area rotation
                // (direction from caster → center) and LoS checks.
                // 772 `AngleShapeSpell` uses `ThrowPossible(ActorX, ActorY, ...)`
                // (from caster), while `ExecuteCircleSpell` uses
                // `ThrowPossible(DestX, DestY, ...)` (from center). When center
                // == caster (non-directional), these are identical.
                let (caster_x, caster_y, caster_z) =
                    resolve_caster_position(caster_id).unwrap_or((center_x, center_y, center_z));

                // Resolve area offsets from the combat's area matrix.
                // C++ `Combat::hasArea` — `combat.cpp:13227`. If no area is set,
                // the combat is single-target (empty offsets → only the center
                // tile is checked, but we add (0,0) so the center is included).
                //
                // For directional spells (needDirection), the area matrix is
                // defined in "north-facing" orientation and must be rotated
                // based on the direction from caster → variant center.
                // C++ `AreaCombat::getArea` (`combat.cpp:1316-1345`) computes
                // the direction from `centerPos → targetPos` and picks the
                // pre-rotated area (N=original, E=rotate90, S=rotate180,
                // W=rotate270).
                let area_offsets: Vec<(i32, i32)> = match &combat.area {
                    Some(area_rc) => {
                        let area = area_rc.borrow();
                        let dx = center_x as i32 - caster_x as i32;
                        let dy = center_y as i32 - caster_y as i32;
                        resolve_oriented_offsets(&area, dx, dy)
                    }
                    None => vec![(0, 0)],
                };

                // Resolve damage min/max.
                // C++ `Combat::getCombatDamage` — `combat.cpp:100`. Three paths:
                // 1. `setFormula` was called → use the formula (literal or
                //    level/magic polynomial).
                // 2. `setCallback(LEVELMAGICVALUE/SKILLVALUE, name)` was called
                //    → invoke the Lua global `name(player, …)` and parse
                //    `(min, max)`. This is the path all 22 value-callback spells
                //    use (no spell in this pack calls `:setFormula`).
                // 3. Neither → `(0, 0)` (e.g. condition-only combats).
                let (damage_min, damage_max) = match &combat.formula {
                    Some(f) => match f.formula_type {
                        FormulaType::Damage => (f.min_a as i32, f.max_a as i32),
                        // For level/magic formulas, resolve via the caster's
                        // level + magic level if available. C++ `getCombatDamage`
                        // (`combat.cpp:117-119`): `fma(levelFormula, mina, minb)`.
                        FormulaType::LevelMagic => {
                            // TFS `fma(levelFormula, …)` shape; coeffs from profile
                            // (`772.lua` `spell.levelMult` / `magicMult`), not hardcoded.
                            let (level, magic) = read_caster_level_magic(caster_id)?;
                            let (lm, mm) = CURRENT_CTX.with(|c| {
                                let ptr = (*c.borrow())
                                    .ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                                if ptr.is_null() {
                                    return Err(mlua::Error::runtime("LuaContext not set"));
                                }
                                let ctx = unsafe { &*ptr };
                                Ok(ctx.get_spell_coeff())
                            })?;
                            let lf = lm * level + mm * magic;
                            let lo = (lf as f64 * f.min_a + f.min_b) as i32;
                            let hi = (lf as f64 * f.max_a + f.max_b) as i32;
                            (lo, hi)
                        }
                        // `COMBAT_FORMULA_SKILL` — TFS API shape (`combat.cpp:120-129`);
                        // magnitude from era `MechanicsProfile` (772 ClassicProbe /
                        // `772.lua` damageTuning; 1098 TFS getMaxWeaponDamage).
                        FormulaType::Skill => {
                            let (mina, minb, maxa, maxb) = (f.min_a, f.min_b, f.max_a, f.max_b);
                            CURRENT_CTX.with(|c| {
                                let ptr = (*c.borrow())
                                    .ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                                if ptr.is_null() {
                                    return Err(mlua::Error::runtime("LuaContext not set"));
                                }
                                let ctx = unsafe { &*ptr };
                                Ok(ctx.get_formula_skill_damage_bounds(
                                    caster_id, mina, minb, maxa, maxb,
                                ))
                            })?
                        }
                        FormulaType::Undefined => (f.min_b as i32, f.max_b as i32),
                    },
                    None => invoke_value_callback(lua, &combat, caster_id)?,
                };

                let request = CombatExecuteRequest {
                    caster_id,
                    center_x,
                    center_y,
                    center_z,
                    caster_x,
                    caster_y,
                    caster_z,
                    combat_type: combat.combat_type,
                    effect: combat.effect,
                    aggressive: combat.aggressive,
                    block_armor: combat.block_armor,
                    block_shield: combat.block_shield,
                    area_offsets: area_offsets.clone(),
                    damage_min,
                    damage_max,
                    conditions: combat
                        .conditions
                        .iter()
                        .map(|c| c.to_apply_spec())
                        .collect(),
                    dispel_type: if combat.dispel_type != 0 {
                        Some(combat.dispel_type)
                    } else {
                        None
                    },
                    create_item: combat.create_item,
                    no_damage: combat.no_damage,
                    distance_effect: combat.distance_effect,
                };
                // TFS `luaCombatExecute` returns boolean; `canDoCombat` failure
                // is `false` (rune not consumed), not a Lua error.
                if call_combat_execute(request).is_err() {
                    return Ok(false);
                }
                // Phase 6: event callbacks after tile resolution / damage.
                // C++ `TargetCallback::onTargetCombat` / `TileCallback::onTileCombat`
                // — `combat.cpp:720,776`.
                invoke_event_callbacks(
                    lua,
                    &combat,
                    caster_id,
                    center_x,
                    center_y,
                    center_z,
                    &area_offsets,
                )?;
                Ok(true)
            },
        );

        // `combat:getTargets(creature, variant)` — C++ `luaCombatGetTargets`
        // (`luascript.cpp`). Returns a table of `CreatureRef` userdata on the
        // combat's area tiles. PC-3a Phase 3: `poison_storm.lua` iterates targets
        // to apply poison outside `combat:execute`.
        methods.add_method(
            "getTargets",
            |lua, this, (creature, variant): (Value, Value)| {
                let combat = this.0.borrow();
                let caster_id = resolve_creature_id(&creature)?;
                let (center_x, center_y, center_z) = resolve_variant_center(&variant, caster_id)?;
                let (caster_x, caster_y, _caster_z) =
                    resolve_caster_position(caster_id).unwrap_or((center_x, center_y, center_z));
                let area_offsets: Vec<(i32, i32)> = match &combat.area {
                    Some(area_rc) => {
                        let area = area_rc.borrow();
                        let dx = center_x as i32 - caster_x as i32;
                        let dy = center_y as i32 - caster_y as i32;
                        resolve_oriented_offsets(&area, dx, dy)
                    }
                    None => vec![(0, 0)],
                };
                let ids = CURRENT_CTX.with(|c| {
                    let ptr =
                        (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                    if ptr.is_null() {
                        return Err(mlua::Error::runtime("LuaContext not set"));
                    }
                    let ctx = unsafe { &*ptr };
                    Ok(ctx.get_creatures_on_area(center_x, center_y, center_z, &area_offsets))
                })?;
                let table = lua.create_table()?;
                for (i, id) in ids.into_iter().enumerate() {
                    let ud = lua.create_userdata(CreatureRef(id))?;
                    table.set(i + 1, ud)?;
                }
                Ok(table)
            },
        );

        // Gap 7b — `__index` fallback so `combat:getPositions(creature, variant)`
        // / `combat:getTargets(...)` resolve `function Combat:getPositions(...)`
        // from `data/lib/core/combat.lua`. Native methods above keep priority.
        // C++ `LuaScriptInterface::registerClass`.
        methods.add_meta_method(MetaMethod::Index, |lua, _this, key: mlua::LuaString| {
            crate::class_registry::class_index_lookup(
                lua,
                crate::class_registry::COMBAT_INDEX_CHAIN,
                key,
            )
        });
    }
}

/// Resolve a creature ID from a Lua value (userdata `CreatureRef` or nil).
/// C++ `luaCombatExecute` — `luascript.cpp:13216` `getCreature(L, 2)`.
fn resolve_creature_id(value: &Value) -> Result<u64, mlua::Error> {
    match value {
        Value::UserData(ud) => {
            let cref = ud.borrow::<CreatureRef>()?;
            Ok(cref.0)
        }
        Value::Nil => Ok(0),
        _ => Err(mlua::Error::runtime(
            "combat:execute: creature must be CreatureRef or nil",
        )),
    }
}

/// Resolve the spell center position from the variant.
/// C++ `luaCombatExecute` — `luascript.cpp:13218-13257` dispatches on variant type:
/// - `VARIANT_NUMBER` → target creature's position
/// - `VARIANT_POSITION` / `VARIANT_TARGETPOSITION` → variant.pos
/// - `VARIANT_STRING` → target player's position
///
/// For now we support POSITION (table with x/y/z) and NUMBER (resolve via ctx).
/// If the variant is nil, fall back to the caster's position (origin spell).
fn resolve_variant_center(variant: &Value, caster_id: u64) -> Result<(u16, u16, u8), mlua::Error> {
    match variant {
        Value::Table(t) => {
            // Variant table: { type = N, pos = {x=.., y=.., z=..}, number = .. }
            let vtype: i32 = t.get("type").unwrap_or(0);
            if vtype == 2 || vtype == 3 {
                // VARIANT_POSITION (2) or VARIANT_TARGETPOSITION (3)
                // `pos` is a table {x=.., y=.., z=..} — extract manually
                // since `Position` doesn't impl `FromLua`.
                let pos_table: mlua::Table = t
                    .get("pos")
                    .map_err(|_| mlua::Error::runtime("variant: missing pos field"))?;
                let x: i64 = pos_table
                    .get("x")
                    .map_err(|_| mlua::Error::runtime("variant: pos missing x"))?;
                let y: i64 = pos_table
                    .get("y")
                    .map_err(|_| mlua::Error::runtime("variant: pos missing y"))?;
                let z: i64 = pos_table
                    .get("z")
                    .map_err(|_| mlua::Error::runtime("variant: pos missing z"))?;
                Ok((x as u16, y as u16, z as u8))
            } else if vtype == 1 {
                // VARIANT_NUMBER — SlotMap creature key (`CreatureRef` / `as_ffi`), not
                // wire u32. Reading as `u32` fails for packed keys → false "missing".
                let number: u64 = t
                    .get("number")
                    .map_err(|_| mlua::Error::runtime("variant: missing number field"))?;
                let pos = CURRENT_CTX.with(|c| {
                    let ptr =
                        (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                    if ptr.is_null() {
                        return Err(mlua::Error::runtime("LuaContext not set"));
                    }
                    let ctx = unsafe { &*ptr };
                    Ok(ctx.get_player_position(number))
                })?;
                pos.map(|p| (p.x, p.y, p.z))
                    .ok_or_else(|| mlua::Error::runtime("variant: target creature not found"))
            } else if vtype == 4 {
                // VARIANT_STRING — resolve online player by name (`spells.cpp`).
                let name: String = t.get("string").unwrap_or_default();
                let pos = CURRENT_CTX.with(|c| {
                    let ptr =
                        (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                    if ptr.is_null() {
                        return Err(mlua::Error::runtime("LuaContext not set"));
                    }
                    let ctx = unsafe { &*ptr };
                    let id = ctx.get_player_by_name(&name);
                    Ok(id.and_then(|pid| ctx.get_player_position(pid)))
                })?;
                pos.map(|p| (p.x, p.y, p.z))
                    .ok_or_else(|| mlua::Error::runtime("variant: string player not found"))
            } else {
                // Unknown variant type — fall back to caster position
                resolve_caster_position(caster_id)
            }
        }
        Value::Nil => resolve_caster_position(caster_id),
        _ => Err(mlua::Error::runtime(
            "combat:execute: variant must be table or nil",
        )),
    }
}

/// Resolve the caster's position via the ScriptContext.
fn resolve_caster_position(caster_id: u64) -> Result<(u16, u16, u8), mlua::Error> {
    if caster_id == 0 {
        return Err(mlua::Error::runtime(
            "combat:execute: no caster and no variant position",
        ));
    }
    CURRENT_CTX.with(|c| {
        let ptr = (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
        if ptr.is_null() {
            return Err(mlua::Error::runtime("LuaContext not set"));
        }
        let ctx = unsafe { &*ptr };
        ctx.get_player_position(caster_id)
            .map(|p| (p.x, p.y, p.z))
            .ok_or_else(|| mlua::Error::runtime("combat:execute: caster position not found"))
    })
}

/// Pick cardinal or diagonal area offsets from caster→center direction.
///
/// C++ `AreaCombat::getArea` (`combat.cpp:1308-1340`): when `hasExtArea` and both
/// dx/dy are non-zero, use the diagonal overlay (NW=raw, NE=mirror, SW=flip,
/// SE=transpose). Otherwise rotate the primary matrix for N/E/S/W.
fn resolve_oriented_offsets(area: &AreaCombat, dx_dir: i32, dy_dir: i32) -> Vec<(i32, i32)> {
    if area.has_ext_area()
        && dx_dir != 0
        && dy_dir != 0
        && let Some(ext) = area.ext_affected_offsets()
    {
        return transform_diagonal_offsets(&ext, dx_dir, dy_dir);
    }
    rotate_area_offsets(&area.affected_offsets(), dx_dir, dy_dir)
}

/// Diagonal orientation transforms — C++ `setupExtArea` (`combat.cpp:1435-1438`):
/// NW = raw, NE = mirror, SW = flip, SE = transpose.
fn transform_diagonal_offsets(offsets: &[(i32, i32)], dx_dir: i32, dy_dir: i32) -> Vec<(i32, i32)> {
    // Offset transforms equivalent to MatrixArea mirror/flip/transpose on
    // relative (dx, dy) cells (center stays at origin).
    offsets
        .iter()
        .copied()
        .map(|(dx, dy)| {
            if dx_dir < 0 && dy_dir < 0 {
                // NW — raw
                (dx, dy)
            } else if dx_dir > 0 && dy_dir < 0 {
                // NE — mirror (reflect over vertical axis → negate x)
                (-dx, dy)
            } else if dx_dir < 0 && dy_dir > 0 {
                // SW — flip (reflect over horizontal axis → negate y)
                (dx, -dy)
            } else {
                // SE — transpose (swap axes)
                (dy, dx)
            }
        })
        .collect()
}

/// Rotate area offsets based on the direction from caster to center.
///
/// C++ `AreaCombat::getArea` (`combat.cpp:1316-1345`) determines the direction
/// from `centerPos → targetPos` and picks the pre-rotated area. The area matrix
/// is defined in "north-facing" orientation; rotations are clockwise:
///
/// - North (dy<0): no rotation — `(dx, dy) → (dx, dy)`
/// - East  (dx>0): rotate90   — `(dx, dy) → (-dy, dx)`
/// - South (else): rotate180  — `(dx, dy) → (-dx, -dy)`
/// - West  (dx<0): rotate270  — `(dx, dy) → (dy, -dx)`
///
/// `dx` is the x offset (negative=west, positive=east), `dy` is the y offset
/// (negative=north, positive=south). When dx=dy=0 (non-directional spell,
/// center == caster), direction defaults to South → rotate180, which is a
/// no-op for symmetric areas and matches C++ behavior.
fn rotate_area_offsets(offsets: &[(i32, i32)], dx_dir: i32, dy_dir: i32) -> Vec<(i32, i32)> {
    let rotate = |(dx, dy): (i32, i32)| -> (i32, i32) {
        if dx_dir < 0 {
            // West — rotate270
            (dy, -dx)
        } else if dx_dir > 0 {
            // East — rotate90
            (-dy, dx)
        } else if dy_dir < 0 {
            // North — no rotation
            (dx, dy)
        } else {
            // South (default) — rotate180
            (-dx, -dy)
        }
    };
    offsets.iter().copied().map(rotate).collect()
}

/// Read the caster's level and magic level via the ScriptContext.
/// C++ `Combat::getCombatDamage` — `combat.cpp:118` `player->getLevel() * 2 + player->getMagicLevel() * 3`.
fn read_caster_level_magic(caster_id: u64) -> Result<(i32, i32), mlua::Error> {
    if caster_id == 0 {
        return Ok((0, 0));
    }
    CURRENT_CTX.with(|c| {
        let ptr = (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
        if ptr.is_null() {
            return Err(mlua::Error::runtime("LuaContext not set"));
        }
        let ctx = unsafe { &*ptr };
        let level = ctx.get_player_level(caster_id).unwrap_or(0);
        let magic = ctx.get_player_magic_level(caster_id).unwrap_or(0);
        Ok((level, magic))
    })
}

/// Invoke a value callback registered via `combat:setCallback` and return the
/// `(min, max)` damage range. C++ `ValueCallback::getMinMaxValues` —
/// `combat.cpp:1111-1170`.
///
/// Called when `combat.formula` is `None`. Looks up the callback function name
/// in Lua globals and calls it with the appropriate arguments based on the
/// callback param type:
/// - `CALLBACK_PARAM_LEVELMAGICVALUE` (0) → `fn(player, level, magic) → (min, max)`
/// - `CALLBACK_PARAM_SKILLVALUE` (1) → `fn(player, skill, attack, factor) → (min, max)`
///
/// The callback body typically calls `player:computeDamage(...)` /
/// `player:computeHealing(...)` / `player:computeSkillDamage(...)` from
/// `data/scripts/functions.lua`, which read `self:getMagicLevel()` and
/// `self:getLevel()`.
///
/// Returns `(0, 0)` if no value callback is registered. Event callbacks
/// (`TARGETTILE` / `TARGETCREATURE`) are handled separately (Phase 6) and
/// do not produce damage values.
fn invoke_value_callback(
    lua: &Lua,
    combat: &CombatDef,
    caster_id: u64,
) -> Result<(i32, i32), mlua::Error> {
    // Try LEVELMAGIC first, then SKILL — a combat registers at most one.
    let callback = combat
        .callbacks
        .get(&CALLBACK_PARAM_LEVELMAGICVALUE)
        .or_else(|| combat.callbacks.get(&CALLBACK_PARAM_SKILLVALUE));
    let Some(callback) = callback else {
        return Ok((0, 0));
    };

    let func = match resolve_callback_function(lua, callback) {
        Some(f) => f,
        None => {
            return Err(mlua::Error::runtime(format!(
                "combat:execute: callback function '{}' not found",
                callback.function_name
            )));
        }
    };

    // Build the `player` CreatureRef userdata to pass as the first argument.
    // The callback body calls `player:computeDamage(...)` etc., which are
    // `Player:` methods bridged onto `CreatureRef` via the `__index` fallback.
    let player_ud = lua.create_userdata(CreatureRef(caster_id))?;

    let (min, max): (f64, f64) = with_lua_instruction_budget(lua, || {
        if callback.param == CALLBACK_PARAM_LEVELMAGICVALUE {
            // `fn(player, level, magic_level) → (min, max)`
            // C++ `ValueCallback::getMinMaxValues` — `combat.cpp:1134-1147`.
            let (level, magic) = read_caster_level_magic(caster_id)?;
            func.call::<(f64, f64)>((player_ud, level, magic))
        } else {
            // `fn(player, skill, attack, factor) → (min, max)`
            // C++ `ValueCallback::getMinMaxValues` — `combat.cpp:1155-1163`:
            // `player->getWeaponSkill()`, `player->getWeapon()->getAttack()`,
            // `player->getAttackFactor()`.
            let params = CURRENT_CTX.with(|c| {
                let ptr =
                    (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
                if ptr.is_null() {
                    return Err(mlua::Error::runtime("LuaContext not set"));
                }
                let ctx = unsafe { &*ptr };
                Ok(ctx.get_player_weapon_combat_params(caster_id))
            })?;
            func.call::<(f64, f64)>((player_ud, params.skill, params.attack, params.attack_factor))
        }
    })?;

    Ok((min as i32, max as i32))
}

/// Prefer the function snapshotted at `setCallback`; fall back to global lookup.
fn resolve_callback_function(lua: &Lua, cb: &CombatCallback) -> Option<mlua::Function> {
    if let Some(f) = &cb.function {
        return Some(f.clone());
    }
    lua.globals()
        .get::<mlua::Function>(cb.function_name.as_str())
        .ok()
}

/// Invoke `TARGETCREATURE` / `TARGETTILE` callbacks after combat execute.
/// C++ `TargetCallback::onTargetCombat` / `TileCallback::onTileCombat` —
/// `combat.cpp:1223,1193`.
fn invoke_event_callbacks(
    lua: &Lua,
    combat: &CombatDef,
    caster_id: u64,
    center_x: u16,
    center_y: u16,
    center_z: u8,
    area_offsets: &[(i32, i32)],
) -> Result<(), mlua::Error> {
    with_lua_instruction_budget(lua, || {
        invoke_event_callbacks_inner(
            lua,
            combat,
            caster_id,
            center_x,
            center_y,
            center_z,
            area_offsets,
        )
    })
}

fn invoke_event_callbacks_inner(
    lua: &Lua,
    combat: &CombatDef,
    caster_id: u64,
    center_x: u16,
    center_y: u16,
    center_z: u8,
    area_offsets: &[(i32, i32)],
) -> Result<(), mlua::Error> {
    if let Some(cb) = combat.callbacks.get(&CALLBACK_PARAM_TARGETCREATURE) {
        let func = match resolve_callback_function(lua, cb) {
            Some(f) => f,
            None => {
                tracing::warn!(
                    name = %cb.function_name,
                    "TARGETCREATURE callback not found"
                );
                return Ok(());
            }
        };
        let target_ids = CURRENT_CTX.with(|c| {
            let ptr = (*c.borrow()).ok_or_else(|| mlua::Error::runtime("LuaContext not set"))?;
            if ptr.is_null() {
                return Err(mlua::Error::runtime("LuaContext not set"));
            }
            let ctx = unsafe { &*ptr };
            Ok(ctx.get_creatures_on_area(center_x, center_y, center_z, area_offsets))
        })?;
        for tid in target_ids {
            let caster_ud = lua.create_userdata(CreatureRef(caster_id))?;
            let target_ud = lua.create_userdata(CreatureRef(tid))?;
            let _: mlua::Value = func.call((caster_ud, target_ud))?;
        }
    }

    if let Some(cb) = combat.callbacks.get(&CALLBACK_PARAM_TARGETTILE) {
        let func = match resolve_callback_function(lua, cb) {
            Some(f) => f,
            None => {
                tracing::warn!(
                    name = %cb.function_name,
                    "TARGETTILE callback not found"
                );
                return Ok(());
            }
        };
        for &(dx, dy) in area_offsets {
            let tx = center_x as i32 + dx;
            let ty = center_y as i32 + dy;
            if tx < 0 || ty < 0 {
                continue;
            }
            let caster_ud = lua.create_userdata(CreatureRef(caster_id))?;
            let pos_ud = lua.create_userdata(PositionRef {
                x: tx as u16,
                y: ty as u16,
                z: center_z,
            })?;
            let _: mlua::Value = func.call((caster_ud, pos_ud))?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn area_combat_parses_square1x1() {
        // AREA_SQUARE1X1 = { {1,1,1}, {1,3,1}, {1,1,1} }
        let matrix = vec![vec![1, 1, 1], vec![1, 3, 1], vec![1, 1, 1]];
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

        def.set_parameter(COMBAT_PARAM_DISPEL, 1); // CONDITION_POISON
        assert_eq!(def.dispel_type, 1);

        def.set_parameter(COMBAT_PARAM_CREATEITEM, 1487);
        assert_eq!(def.create_item, 1487);
        def.set_parameter(COMBAT_PARAM_NODAMAGE, 1);
        assert!(def.no_damage);
        def.set_parameter(COMBAT_PARAM_DISTANCEEFFECT, 4);
        assert_eq!(def.distance_effect, 4);
    }

    #[test]
    fn diagonal_ext_area_orientation_differs_from_cardinal() {
        // Wall field: horizontal line through center (simplified).
        let primary = vec![vec![1, 3, 1]];
        // Diagonal: vertical-ish line for NW raw.
        let ext = vec![vec![1, 0, 0], vec![0, 3, 0], vec![0, 0, 1]];
        let area = AreaCombat::from_matrix(primary).with_ext_area(ext);
        assert!(area.has_ext_area());

        let north = resolve_oriented_offsets(&area, 0, -1);
        let ne = resolve_oriented_offsets(&area, 1, -1);
        let nw = resolve_oriented_offsets(&area, -1, -1);
        // Cardinal uses primary; diagonal uses ext transforms — NE ≠ NW.
        assert_ne!(ne, nw);
        // North (cardinal) uses primary rotate — not the same as NW raw ext.
        assert_ne!(north, nw);
    }

    #[test]
    fn create_combat_area_stores_ext_area() {
        let lua = Lua::new();
        register_combat_metatable(&lua).expect("registration must succeed");
        let result: mlua::AnyUserData = lua
            .load(
                r#"
                return createCombatArea(
                    {{1, 3, 1}},
                    {{1, 0}, {0, 3}}
                )
            "#,
            )
            .eval()
            .expect("createCombatArea with ext");
        let area_ref = result.borrow::<AreaRef>().expect("AreaRef");
        let area = area_ref.0.borrow();
        assert!(area.has_ext_area());
        assert_eq!(area.ext_matrix.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn combat_def_set_callback_stores_function() {
        let mut def = CombatDef::new();
        def.set_callback(
            CALLBACK_PARAM_SKILLVALUE,
            "onGetFormulaValues".to_string(),
            None,
        );
        assert_eq!(
            def.callbacks
                .get(&CALLBACK_PARAM_SKILLVALUE)
                .unwrap()
                .function_name,
            "onGetFormulaValues"
        );
    }

    /// Regression: shared global name `onGetFormulaValues` must not clobber earlier
    /// combats — TFS `getEvent` snapshots + clears the global (`luascript.cpp:334-360`).
    #[test]
    fn set_callback_snapshots_function_so_later_global_overwrite_is_ignored() {
        let lua = Lua::new();
        crate::combat_enums::register_combat_enums(&lua).expect("enums");
        register_combat_metatable(&lua).expect("register");
        // Minimal Player table so computeDamage path isn't needed — callbacks return literals.
        lua.load(
            r#"
            function onGetFormulaValues(player, level, magicLevel)
                return -100, -200
            end
            combat_a = Combat()
            combat_a:setCallback(CALLBACK_PARAM_LEVELMAGICVALUE, "onGetFormulaValues")

            function onGetFormulaValues(player, level, magicLevel)
                return -900, -1000
            end
            combat_b = Combat()
            combat_b:setCallback(CALLBACK_PARAM_LEVELMAGICVALUE, "onGetFormulaValues")
            "#,
        )
        .exec()
        .expect("load two combats");

        let combat_a = lua
            .globals()
            .get::<mlua::AnyUserData>("combat_a")
            .expect("combat_a");
        let combat_b = lua
            .globals()
            .get::<mlua::AnyUserData>("combat_b")
            .expect("combat_b");
        let a = combat_a.borrow::<CombatRef>().expect("CombatRef a");
        let b = combat_b.borrow::<CombatRef>().expect("CombatRef b");

        let cb_a =
            a.0.borrow()
                .callbacks
                .get(&CALLBACK_PARAM_LEVELMAGICVALUE)
                .cloned()
                .expect("callback a");
        let cb_b =
            b.0.borrow()
                .callbacks
                .get(&CALLBACK_PARAM_LEVELMAGICVALUE)
                .cloned()
                .expect("callback b");

        assert!(cb_a.function.is_some());
        assert!(cb_b.function.is_some());
        // Global must be cleared after setCallback (TFS getEvent).
        let global: mlua::Value = lua.globals().get("onGetFormulaValues").expect("get");
        assert!(matches!(global, mlua::Value::Nil));

        // Invoke snapshotted functions with dummy args — returns must differ.
        let player = lua.create_userdata(CreatureRef(1)).expect("player ud");
        let (min_a, max_a): (f64, f64) = cb_a
            .function
            .as_ref()
            .unwrap()
            .call((player.clone(), 120i32, 52i32))
            .expect("call a");
        let (min_b, max_b): (f64, f64) = cb_b
            .function
            .as_ref()
            .unwrap()
            .call((player, 120i32, 52i32))
            .expect("call b");
        assert_eq!((min_a, max_a), (-100.0, -200.0));
        assert_eq!((min_b, max_b), (-900.0, -1000.0));
    }

    #[test]
    fn create_combat_area_from_lua() {
        let lua = Lua::new();
        register_combat_metatable(&lua).expect("registration must succeed");
        crate::combat_enums::register_combat_enums(&lua).expect("enum registration must succeed");

        // Test createCombatArea with AREA_SQUARE1X1 equivalent.
        let result: mlua::AnyUserData = lua
            .load(
                r#"
                local area = createCombatArea({
                    {1, 1, 1},
                    {1, 3, 1},
                    {1, 1, 1}
                })
                return area
            "#,
            )
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
            .load(
                r#"
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
            "#,
            )
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

    /// PC-3a Phase 1: value callback invocation end-to-end.
    ///
    /// Sets up a `Combat` with `CALLBACK_PARAM_LEVELMAGICVALUE` pointing to a
    /// Lua global that calls `player:computeDamage(...)` (from `functions.lua`),
    /// then invokes `combat:execute()` and verifies the callback was called with
    /// the correct `(level, magic_level)` args and produced non-zero damage.
    ///
    /// C++ reference: `ValueCallback::getMinMaxValues` — `combat.cpp:1111-1170`.
    #[test]
    fn value_callback_levelmagic_invokes_lua_global() {
        use crate::context::with_lua_context;
        use tfs_rust_common::{
            ScriptContext, ScriptCreatureData, ScriptCreatureId, ScriptItemRef, WeaponCombatParams,
        };

        const CID: ScriptCreatureId = 42;
        const LEVEL: i32 = 20;
        const MAGIC: i32 = 10;

        struct Ctx;
        impl ScriptContext for Ctx {
            fn get_creature(&self, id: ScriptCreatureId) -> Option<ScriptCreatureData> {
                (id == CID).then_some(ScriptCreatureData {
                    name: "Test".into(),
                    guid: 1,
                })
            }
            fn get_item(&self, _: ScriptCreatureId) -> Option<ScriptItemRef> {
                None
            }
            fn get_config_string(&self, _: &str) -> Option<String> {
                None
            }
            fn get_player_level(&self, id: ScriptCreatureId) -> Option<i32> {
                (id == CID).then_some(LEVEL)
            }
            fn get_player_magic_level(&self, id: ScriptCreatureId) -> Option<i32> {
                (id == CID).then_some(MAGIC)
            }
            fn get_player_position(
                &self,
                id: ScriptCreatureId,
            ) -> Option<tfs_rust_common::Position> {
                (id == CID).then_some(tfs_rust_common::Position {
                    x: 100,
                    y: 100,
                    z: 7,
                })
            }
            fn get_player_weapon_combat_params(&self, _: ScriptCreatureId) -> WeaponCombatParams {
                WeaponCombatParams {
                    skill: 30,
                    attack: 50,
                    attack_factor: 1.0,
                }
            }
        }

        let lua = Lua::new();
        register_combat_metatable(&lua).expect("combat metatable");
        crate::userdata::register_creature_metatable(&lua).expect("creature metatable");
        crate::combat_enums::register_combat_enums(&lua).expect("combat enums");
        crate::constants::register_constants(&lua).expect("constants");
        // Register the event script bootstrap so `Player` is a table and
        // `function Player:method(...)` definitions in `functions.lua` work.
        crate::runtime::register_event_script_bootstrap(&lua).expect("bootstrap");

        // Load `functions.lua` so `Player:computeDamage` is available.
        let functions_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/scripts/functions.lua");
        if !functions_path.exists() {
            eprintln!(" Skipping — functions.lua not found");
            return;
        }
        let src = std::fs::read_to_string(&functions_path).expect("read functions.lua");
        lua.load(&src)
            .set_name("functions.lua")
            .exec()
            .expect("functions.lua loads");

        // Register the mutation applier so `call_combat_execute` doesn't panic.
        // We use a no-op applier — the test only checks that the callback fires
        // and produces non-zero damage, not that damage is applied to a target.
        crate::lua_mutation::register_lua_mutation_applier(|_, _| Ok(()));

        let ctx = Ctx;
        with_lua_context(&ctx, || {
            // Re-register applier that asserts non-zero damage from the callback.
            crate::lua_mutation::register_lua_mutation_applier(|_, mutation| {
                if let crate::lua_mutation::LuaMutation::CombatExecute { request } = mutation {
                    assert!(
                        request.damage_min != 0 || request.damage_max != 0,
                        "value callback must produce non-zero damage: min={}, max={}",
                        request.damage_min,
                        request.damage_max
                    );
                }
                Ok(())
            });

            let caster_ud = lua
                .create_userdata(crate::context::CreatureRef(CID))
                .expect("create caster userdata");
            lua.globals().set("caster", caster_ud).expect("set caster");

            let _: () = lua
                .load(
                    r#"
                    combat = Combat()
                    combat:setParameter(COMBAT_PARAM_TYPE, COMBAT_ENERGYDAMAGE)
                    combat:setParameter(COMBAT_PARAM_AGGRESSIVE, true)

                    function onGetFormulaValues(player, level, magicLevel)
                        return player:computeDamage(45, 10)
                    end

                    combat:setCallback(CALLBACK_PARAM_LEVELMAGICVALUE, "onGetFormulaValues")
                "#,
                )
                .set_name("setup_combat")
                .exec()
                .expect("combat setup must succeed");

            // Execute inside a mutation scope so `call_combat_execute` can dispatch.
            // The dummy non-null pointer is fine — the test applier ignores it.
            let dummy_world = 0x1usize as *mut ();
            crate::lua_mutation::with_lua_mutation_scope(dummy_world, || {
                let _: () = lua
                    .load("combat:execute(caster, nil)")
                    .set_name("execute")
                    .exec()
                    .expect("execute with LEVELMAGIC callback must succeed");
            });
        });
    }

    /// PC-3a Phase 1: SKILL value callback invocation.
    ///
    /// Verifies the SKILL callback path (`CALLBACK_PARAM_SKILLVALUE`) passes
    /// `(skill, attack, factor)` from Rust to the Lua callback, mirroring
    /// `berserk.lua`'s pattern.
    #[test]
    fn value_callback_skill_passes_weapon_params() {
        use crate::context::with_lua_context;
        use tfs_rust_common::{
            ScriptContext, ScriptCreatureData, ScriptCreatureId, ScriptItemRef, WeaponCombatParams,
        };

        const CID: ScriptCreatureId = 99;
        struct Ctx;
        impl ScriptContext for Ctx {
            fn get_creature(&self, id: ScriptCreatureId) -> Option<ScriptCreatureData> {
                (id == CID).then_some(ScriptCreatureData {
                    name: "Knight".into(),
                    guid: 2,
                })
            }
            fn get_item(&self, _: ScriptCreatureId) -> Option<ScriptItemRef> {
                None
            }
            fn get_config_string(&self, _: &str) -> Option<String> {
                None
            }
            fn get_player_level(&self, id: ScriptCreatureId) -> Option<i32> {
                (id == CID).then_some(35)
            }
            fn get_player_magic_level(&self, id: ScriptCreatureId) -> Option<i32> {
                (id == CID).then_some(0)
            }
            fn get_player_position(
                &self,
                id: ScriptCreatureId,
            ) -> Option<tfs_rust_common::Position> {
                (id == CID).then_some(tfs_rust_common::Position {
                    x: 200,
                    y: 200,
                    z: 7,
                })
            }
            fn get_player_weapon_combat_params(&self, _: ScriptCreatureId) -> WeaponCombatParams {
                WeaponCombatParams {
                    skill: 60,
                    attack: 40,
                    attack_factor: 1.0,
                }
            }
        }

        let lua = Lua::new();
        register_combat_metatable(&lua).expect("combat metatable");
        crate::userdata::register_creature_metatable(&lua).expect("creature metatable");
        crate::combat_enums::register_combat_enums(&lua).expect("combat enums");
        crate::constants::register_constants(&lua).expect("constants");
        crate::runtime::register_event_script_bootstrap(&lua).expect("bootstrap");

        let functions_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/scripts/functions.lua");
        if !functions_path.exists() {
            eprintln!(" Skipping — functions.lua not found");
            return;
        }
        let src = std::fs::read_to_string(&functions_path).expect("read functions.lua");
        lua.load(&src)
            .set_name("functions.lua")
            .exec()
            .expect("functions.lua loads");

        crate::lua_mutation::register_lua_mutation_applier(|_, mutation| {
            if let crate::lua_mutation::LuaMutation::CombatExecute { request } = mutation {
                assert!(
                    request.damage_min != 0 || request.damage_max != 0,
                    "SKILL callback must produce non-zero damage: min={}, max={}",
                    request.damage_min,
                    request.damage_max
                );
            }
            Ok(())
        });

        let ctx = Ctx;
        with_lua_context(&ctx, || {
            let caster_ud = lua
                .create_userdata(crate::context::CreatureRef(CID))
                .expect("create caster userdata");
            lua.globals().set("caster", caster_ud).expect("set caster");

            let _: () = lua
                .load(
                    r#"
                    combat = Combat()
                    combat:setParameter(COMBAT_PARAM_TYPE, COMBAT_PHYSICALDAMAGE)
                    combat:setParameter(COMBAT_PARAM_AGGRESSIVE, true)

                    function onGetSkillValues(player, skill, attack, factor)
                        return player:computeSkillDamage(80, 20, skill, false, true)
                    end

                    combat:setCallback(CALLBACK_PARAM_SKILLVALUE, "onGetSkillValues")
                "#,
                )
                .set_name("skill_callback_setup")
                .exec()
                .expect("SKILL combat setup must succeed");

            // Execute inside a mutation scope so `call_combat_execute` can dispatch.
            let dummy_world = 0x1usize as *mut ();
            crate::lua_mutation::with_lua_mutation_scope(dummy_world, || {
                let _: () = lua
                    .load("combat:execute(caster, nil)")
                    .set_name("skill_execute")
                    .exec()
                    .expect("SKILL callback execute must succeed");
            });
        });
    }

    /// Default (no GameWorld) still exposes ClassicProbe *ceiling* for deterministic
    /// unit tests; live `GameWorld` rolls one ProbeValue (see `game_world_script`).
    /// `COMBAT_FORMULA_SKILL` resolves via ClassicProbe (772 primary),
    /// not TFS `getMaxWeaponDamage` 0.085. Shape still `setFormula(SKILL, …)`.
    #[test]
    fn formula_skill_resolves_weapon_damage_range() {
        use crate::context::with_lua_context;
        use tfs_rust_common::{
            ScriptContext, ScriptCreatureData, ScriptCreatureId, ScriptItemRef, WeaponCombatParams,
        };

        const CID: ScriptCreatureId = 77;
        struct Ctx;
        impl ScriptContext for Ctx {
            fn get_creature(&self, id: ScriptCreatureId) -> Option<ScriptCreatureData> {
                (id == CID).then_some(ScriptCreatureData {
                    name: "Paladin".into(),
                    guid: 3,
                })
            }
            fn get_item(&self, _: ScriptCreatureId) -> Option<ScriptItemRef> {
                None
            }
            fn get_config_string(&self, _: &str) -> Option<String> {
                None
            }
            fn get_player_level(&self, id: ScriptCreatureId) -> Option<i32> {
                (id == CID).then_some(50)
            }
            fn get_player_magic_level(&self, id: ScriptCreatureId) -> Option<i32> {
                (id == CID).then_some(0)
            }
            fn get_player_position(
                &self,
                id: ScriptCreatureId,
            ) -> Option<tfs_rust_common::Position> {
                (id == CID).then_some(tfs_rust_common::Position {
                    x: 100,
                    y: 100,
                    z: 7,
                })
            }
            fn get_player_weapon_combat_params(&self, _: ScriptCreatureId) -> WeaponCombatParams {
                WeaponCombatParams {
                    skill: 80,
                    attack: 40,
                    attack_factor: 1.0,
                }
            }
        }

        let lua = Lua::new();
        register_combat_metatable(&lua).expect("combat metatable");
        crate::userdata::register_creature_metatable(&lua).expect("creature metatable");
        crate::combat_enums::register_combat_enums(&lua).expect("combat enums");
        crate::constants::register_constants(&lua).expect("constants");
        crate::runtime::register_event_script_bootstrap(&lua).expect("bootstrap");
        crate::lua_mutation::register_lua_mutation_applier(|_, _| Ok(()));

        let ctx = Ctx;
        with_lua_context(&ctx, || {
            // Default ScriptContext: probe ceiling (99*40*(80*5+50))/10000 = 178
            // Live GameWorld rolls one ProbeValue instead (lo==hi).
            crate::lua_mutation::register_lua_mutation_applier(|_, mutation| {
                if let crate::lua_mutation::LuaMutation::CombatExecute { request } = mutation {
                    assert_eq!(request.damage_min, 0);
                    assert_eq!(
                        request.damage_max, 178,
                        "default FORMULA_SKILL ceiling, got {}",
                        request.damage_max
                    );
                }
                Ok(())
            });

            let caster_ud = lua
                .create_userdata(crate::context::CreatureRef(CID))
                .expect("create caster userdata");
            lua.globals().set("caster", caster_ud).expect("set caster");

            let _: () = lua
                .load(
                    r#"
                    combat = Combat()
                    combat:setParameter(COMBAT_PARAM_TYPE, COMBAT_PHYSICALDAMAGE)
                    combat:setFormula(COMBAT_FORMULA_SKILL, 0, 0, 1, 0)
                "#,
                )
                .set_name("skill_formula_setup")
                .exec()
                .expect("FORMULA_SKILL setup must succeed");

            let dummy_world = 0x1usize as *mut ();
            crate::lua_mutation::with_lua_mutation_scope(dummy_world, || {
                let _: () = lua
                    .load("combat:execute(caster, nil)")
                    .set_name("skill_formula_execute")
                    .exec()
                    .expect("FORMULA_SKILL execute must succeed");
            });
        });
    }

    /// PC-3a Phase 1: `Player:` method bridge via `__index` fallback.
    ///
    /// Verifies that `CreatureRef` userdata can call `Player:computeDamage`
    /// defined in `functions.lua` — the `__index` metamethod falls back to
    /// the `Player` global table when the method isn't a native Rust method.
    #[test]
    fn creature_ref_index_fallback_resolves_player_methods() {
        use crate::context::with_lua_context;
        use tfs_rust_common::{ScriptContext, ScriptCreatureData, ScriptCreatureId, ScriptItemRef};

        const CID: ScriptCreatureId = 7;
        struct Ctx;
        impl ScriptContext for Ctx {
            fn get_creature(&self, id: ScriptCreatureId) -> Option<ScriptCreatureData> {
                (id == CID).then_some(ScriptCreatureData {
                    name: "Test".into(),
                    guid: 1,
                })
            }
            fn get_item(&self, _: ScriptCreatureId) -> Option<ScriptItemRef> {
                None
            }
            fn get_config_string(&self, _: &str) -> Option<String> {
                None
            }
            fn get_player_level(&self, id: ScriptCreatureId) -> Option<i32> {
                (id == CID).then_some(20)
            }
            fn get_player_magic_level(&self, id: ScriptCreatureId) -> Option<i32> {
                (id == CID).then_some(10)
            }
        }

        let lua = Lua::new();
        crate::userdata::register_creature_metatable(&lua).expect("creature metatable");
        crate::runtime::register_event_script_bootstrap(&lua).expect("bootstrap");

        let functions_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/scripts/functions.lua");
        if !functions_path.exists() {
            eprintln!(" Skipping — functions.lua not found");
            return;
        }
        let src = std::fs::read_to_string(&functions_path).expect("read functions.lua");
        lua.load(&src)
            .set_name("functions.lua")
            .exec()
            .expect("functions.lua loads");

        let ctx = Ctx;
        with_lua_context(&ctx, || {
            let ud = lua
                .create_userdata(crate::context::CreatureRef(CID))
                .expect("create userdata");
            lua.globals().set("player", ud).expect("set player");

            // Native `computeDamage` (profile coeffs). level=20, magic=10 → formula=70
            // min = (45-10)*70/100 = 24 → -24; max = (45+10)*70/100 = 38 → -38
            let (min, max): (i32, i32) = lua
                .load("return player:computeDamage(45, 10)")
                .eval()
                .expect("computeDamage should resolve as native userdata method");

            assert_eq!(min, -24, "computeDamage min");
            assert_eq!(max, -38, "computeDamage max");
        });
    }
}
