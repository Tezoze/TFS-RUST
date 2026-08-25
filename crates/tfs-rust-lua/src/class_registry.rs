//! Engine class global registry — single owner of TFS-style class tables.
//!
//! C++ reference: `luascript.cpp` `LuaScriptInterface::registerClass` — creates
//! a class table with a `__call` metamethod so the global is both callable
//! (`Tile(pos)`) and extensible (`function Tile.relocateTo(self, ...)`).
//!
//! Idempotent and order-independent: never replaces an existing table, so the
//! registration sequence in `LuaRuntime::new` no longer decides whether a class
//! ends up callable, extensible, both, or `nil`.
//! See `tasks/tools-actions/gap7-class-globals.md` (Gap 7a / 7c).

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use mlua::{
    AnyUserData, FromLua, FromLuaMulti, Function, IntoLua, IntoLuaMulti, Lua, MaybeSend,
    MultiValue, Table, UserData, UserDataFields, UserDataMethods, UserDataRegistry, Value,
};

/// Lua-registry key for the name → callable map written by [`register_class`].
const REGISTERED_CLASSES_KEY: &str = "tfs_registered_classes";
/// Lua-registry key for class → `{ method = true }` written by [`RecordingRegistry`].
const REGISTERED_METHODS_KEY: &str = "tfs_registered_methods";
/// Lua-registry key for class → `{ field = true }` written by [`RecordingRegistry`].
const REGISTERED_FIELDS_KEY: &str = "tfs_registered_fields";

thread_local! {
    static RECORDED_METHODS: RefCell<BTreeMap<String, BTreeSet<String>>> =
        const { RefCell::new(BTreeMap::new()) };
    static RECORDED_FIELDS: RefCell<BTreeMap<String, BTreeSet<String>>> =
        const { RefCell::new(BTreeMap::new()) };
}

/// Engine class globals that must go through [`register_class`].
///
/// `all_class_globals_are_tables` asserts every name here is in the registry
/// (so a `globals.set(Name, ctor_fn)` bypass cannot hide) **and** enumerates
/// whatever else `register_class` recorded (so a new class is covered without
/// a new test row). Gap 7c.
#[cfg(test)]
pub(crate) const REQUIRED_CLASS_GLOBALS: &[&str] = &[
    // 7a — userdata / table-only
    "Combat",
    "Container",
    "Creature",
    "Game",
    "Item",
    "ItemType",
    "Monster",
    "Npc",
    "Party",
    "Player",
    "Position",
    "Spell",
    "Teleport",
    "Tile",
    "Vocation",
    "Weapon",
    // 7c — revscript constructors
    "Action",
    "Channel",
    "Condition",
    "CreatureEvent",
    "GlobalEvent",
    "House",
    "Town",
    "MonsterType",
    "MoveEvent",
    "TalkAction",
    "Variant",
];

/// Classes `data/scripts/lib/helper_constructors.lua` wraps via
/// `getmetatable(class).__call`. A table-only `register_class(_, None)` is
/// not enough — each needs a constructor.
#[cfg(test)]
pub(crate) const HELPER_CTOR_CLASSES: &[&str] = &[
    "Action",
    "CreatureEvent",
    "Spell",
    "TalkAction",
    "MoveEvent",
    "GlobalEvent",
    "Weapon",
];

/// Userdata `__index` fallback chains (Gap 7b).
///
/// First non-nil hit wins, **after** mlua's registered Rust methods / field
/// getters. mlua's generated `__index` checks field getters → methods → the
/// user-supplied `__index` function last (`mlua-0.12.0/src/userdata/util.rs`),
/// so a Lua override on the class table cannot silently shadow an engine
/// method — no manual priority to assert.
///
/// Only userdata whose class table is extended by the data pack are listed.
/// `Spell` / `Weapon` / `Condition` have no `function <Class>:method(...)`
/// consumers in `data/`, so they are intentionally absent (no speculative
/// fallbacks).
///
/// C++ reference: `luascript.cpp` `LuaScriptInterface::registerClass` — TFS
/// chains the class hierarchy so `self:method()` resolves through base
/// classes (`Player` → `Creature`, `Container` → `Item`).
pub(crate) const CREATURE_INDEX_CHAIN: &[&str] = &["Player", "Creature"];
pub(crate) const MONSTER_INDEX_CHAIN: &[&str] = &["Monster", "Creature"];
pub(crate) const TILE_INDEX_CHAIN: &[&str] = &["Tile"];
pub(crate) const HOUSE_INDEX_CHAIN: &[&str] = &["House"];
pub(crate) const ITEM_INDEX_CHAIN: &[&str] = &["Item"];
pub(crate) const CONTAINER_INDEX_CHAIN: &[&str] = &["Container", "Item"];
pub(crate) const ITEM_TYPE_INDEX_CHAIN: &[&str] = &["ItemType"];
pub(crate) const POSITION_INDEX_CHAIN: &[&str] = &["Position"];
pub(crate) const COMBAT_INDEX_CHAIN: &[&str] = &["Combat"];
pub(crate) const VOCATION_INDEX_CHAIN: &[&str] = &["Vocation"];

/// `__index` fallback for userdata — walk a declared class chain (first
/// non-nil hit wins) so `tile:relocateTo(pos)` resolves a method defined as
/// `function Tile.relocateTo(self, ...)` in `data/lib/core/tile.lua`.
///
/// Call from each userdata's `add_methods`:
///
/// ```ignore
/// methods.add_meta_method(MetaMethod::Index, |lua, _this, key: mlua::LuaString| {
///     crate::class_registry::class_index_lookup(lua, TILE_INDEX_CHAIN, key)
/// });
/// ```
///
/// Native Rust methods keep priority: mlua only invokes this after the
/// registered-method lookup misses. A class global that is absent or not a
/// table (e.g. not yet registered) is skipped silently.
///
/// C++ reference: `luascript.cpp` `LuaScriptInterface::registerClass` — TFS
/// chains the class hierarchy; this is the userdata-side mirror of
/// `register_class` (Gap 7a).
pub(crate) fn class_index_lookup(
    lua: &Lua,
    chain: &'static [&'static str],
    key: mlua::LuaString,
) -> Result<Value, mlua::Error> {
    let globals = lua.globals();
    for name in chain {
        // Skip a class global that is absent or not a table (e.g. a bare
        // function from a not-yet-migrated registrar) rather than erroring —
        // the chain is best-effort, missing classes yield `nil`.
        let Ok(table) = globals.get::<Table>(*name) else {
            continue;
        };
        let value: Value = table.get(key.clone())?;
        if !matches!(value, Value::Nil) {
            return Ok(value);
        }
    }
    Ok(Value::Nil)
}

/// Get-or-create the class table for `name`, optionally attaching a `__call`
/// constructor.
///
/// The constructor closure keeps its original `(args...)` signature (no leading
/// `self`); this helper wraps it so Lua's `__call(self, ...)` drops the class
/// table before forwarding. One ctor closure therefore works as a `__call`
/// metamethod without each call site rewriting its parameter list.
///
/// Idempotent: a second call with the same name returns the existing table and
/// only attaches `__call` if the table has no `__call` yet. Safe in any
/// registration order — the table-only classes (`Monster`, `Party`, …) may be
/// created here before a ctor-bearing registrar attaches its `__call` later.
///
/// # Errors
///
/// Returns an `mlua::Error` if table/metatable creation or the global set fails.
pub(crate) fn register_class(
    lua: &Lua,
    name: &str,
    ctor: Option<Function>,
) -> Result<Table, mlua::Error> {
    let globals = lua.globals();
    // Reuse an existing table; create one only when the global is absent or a
    // non-table (the Gap 7a migration path — a bare function global gets
    // replaced by a proper class table). Never replaces an existing table.
    let table = match globals.get::<Table>(name) {
        Ok(existing) => existing,
        Err(_) => {
            let t = lua.create_table()?;
            globals.set(name, t.clone())?;
            t
        }
    };
    if let Some(f) = ctor {
        let has_call = table
            .metatable()
            .map(|mt| mt.get::<Function>("__call").is_ok())
            .unwrap_or(false);
        if !has_call {
            // `__call` receives `(self, ...)`; drop `self` so the ctor closure
            // can keep its original `(args...)` signature.
            let wrapper = lua.create_function(move |_, args: MultiValue| {
                let mut iter = args.into_iter();
                let _ = iter.next();
                let rest: MultiValue = iter.collect();
                let result: Value = f.call(rest)?;
                Ok(result)
            })?;
            match table.metatable() {
                Some(mt) => {
                    mt.set("__call", wrapper)?;
                }
                None => {
                    let mt = lua.create_table()?;
                    mt.set("__call", wrapper)?;
                    table.set_metatable(Some(mt))?;
                }
            }
        }
    }
    let callable = table
        .metatable()
        .map(|mt| mt.get::<Function>("__call").is_ok())
        .unwrap_or(false);
    record_registered_class(lua, name, callable)?;
    Ok(table)
}

fn record_registered_class(lua: &Lua, name: &str, callable: bool) -> Result<(), mlua::Error> {
    let table = match lua.named_registry_value::<Table>(REGISTERED_CLASSES_KEY) {
        Ok(t) => t,
        Err(_) => {
            let t = lua.create_table()?;
            lua.set_named_registry_value(REGISTERED_CLASSES_KEY, t.clone())?;
            t
        }
    };
    table.set(name, callable)?;
    Ok(())
}

/// Names recorded by [`register_class`] on this VM, as `(name, has_call)`.
/// Sorted by name. Empty when nothing has been registered yet.
pub(crate) fn registered_class_entries(lua: &Lua) -> Result<Vec<(String, bool)>, mlua::Error> {
    let Ok(table) = lua.named_registry_value::<Table>(REGISTERED_CLASSES_KEY) else {
        return Ok(Vec::new());
    };
    let mut entries = Vec::new();
    for pair in table.pairs::<String, bool>() {
        let (name, callable) = pair?;
        entries.push((name, callable));
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(entries)
}

/// LuaLS class base for `name`, from the userdata `__index` chains (Gap 7b).
///
/// `Player` → `Creature`, `Container` → `Item`. Other registered classes have
/// no engine inheritance.
pub(crate) fn class_lua_base(name: &str) -> Option<&'static str> {
    match name {
        "Player" => Some("Creature"),
        "Monster" => Some("Creature"),
        "Container" => Some("Item"),
        _ => None,
    }
}

/// Native userdata methods recorded by [`RecordingRegistry`], as `(class, methods)`.
/// Sorted by class name; method names sorted. Flushes the thread-local recorder
/// into the Lua registry first so a dummy `create_userdata` is enough to populate.
pub(crate) fn registered_method_entries(
    lua: &Lua,
) -> Result<Vec<(String, Vec<String>)>, mlua::Error> {
    flush_recorded_members(lua)?;
    read_name_set_registry(lua, REGISTERED_METHODS_KEY)
}

/// Native userdata fields recorded by [`RecordingRegistry`], as `(class, fields)`.
pub(crate) fn registered_field_entries(
    lua: &Lua,
) -> Result<Vec<(String, Vec<String>)>, mlua::Error> {
    flush_recorded_members(lua)?;
    read_name_set_registry(lua, REGISTERED_FIELDS_KEY)
}

fn read_name_set_registry(lua: &Lua, key: &str) -> Result<Vec<(String, Vec<String>)>, mlua::Error> {
    let Ok(table) = lua.named_registry_value::<Table>(key) else {
        return Ok(Vec::new());
    };
    let mut entries = Vec::new();
    for pair in table.pairs::<String, Table>() {
        let (class, names_table) = pair?;
        let mut names = Vec::new();
        for name_pair in names_table.pairs::<String, bool>() {
            let (name, present) = name_pair?;
            if present {
                names.push(name);
            }
        }
        names.sort();
        entries.push((class, names));
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(entries)
}

fn flush_recorded_members(lua: &Lua) -> Result<(), mlua::Error> {
    let methods_root = registry_table(lua, REGISTERED_METHODS_KEY)?;
    RECORDED_METHODS.with(|cell| merge_name_sets(&methods_root, lua, &cell.borrow()))?;
    let fields_root = registry_table(lua, REGISTERED_FIELDS_KEY)?;
    RECORDED_FIELDS.with(|cell| merge_name_sets(&fields_root, lua, &cell.borrow()))?;
    Ok(())
}

fn registry_table(lua: &Lua, key: &str) -> Result<Table, mlua::Error> {
    match lua.named_registry_value::<Table>(key) {
        Ok(t) => Ok(t),
        Err(_) => {
            let t = lua.create_table()?;
            lua.set_named_registry_value(key, t.clone())?;
            Ok(t)
        }
    }
}

fn merge_name_sets(
    root: &Table,
    lua: &Lua,
    recorded: &BTreeMap<String, BTreeSet<String>>,
) -> Result<(), mlua::Error> {
    for (class, names) in recorded {
        let class_table = match root.get::<Table>(class.as_str()) {
            Ok(t) => t,
            Err(_) => {
                let t = lua.create_table()?;
                root.set(class.as_str(), t.clone())?;
                t
            }
        };
        for name in names {
            class_table.set(name.as_str(), true)?;
        }
    }
    Ok(())
}

/// Run `UserData::add_fields` / `add_methods` through [`RecordingRegistry`] so
/// native method/field names are enumerable for LuaLS generation (pillar 5).
///
/// Call from an overridden `UserData::register` — do not also call the default
/// `register` body, or methods would be added twice.
pub(crate) fn register_with_recording<T: UserData>(
    registry: &mut UserDataRegistry<T>,
    class: &'static str,
) {
    let mut rec = RecordingRegistry {
        inner: registry,
        class,
    };
    T::add_fields(&mut rec);
    T::add_methods(&mut rec);
}

/// Forwards to [`UserDataRegistry`] while recording method/field names for LuaLS.
///
/// Metamethods (`__index`, …) are not recorded — they are not Lua-callable
/// method names.
struct RecordingRegistry<'a, T> {
    inner: &'a mut UserDataRegistry<T>,
    class: &'static str,
}

impl<T> RecordingRegistry<'_, T> {
    fn record_method(&self, name: &str) {
        if name.starts_with("__") {
            return;
        }
        RECORDED_METHODS.with(|cell| {
            cell.borrow_mut()
                .entry(self.class.to_string())
                .or_default()
                .insert(name.to_string());
        });
    }

    fn record_field(&self, name: &str) {
        if name.starts_with("__") {
            return;
        }
        RECORDED_FIELDS.with(|cell| {
            cell.borrow_mut()
                .entry(self.class.to_string())
                .or_default()
                .insert(name.to_string());
        });
    }
}

impl<T> UserDataMethods<T> for RecordingRegistry<'_, T> {
    fn add_method<M, A, R>(&mut self, name: impl Into<String>, method: M)
    where
        M: Fn(&Lua, &T, A) -> mlua::Result<R> + MaybeSend + 'static,
        A: FromLuaMulti,
        R: IntoLuaMulti,
    {
        let name = name.into();
        self.record_method(&name);
        self.inner.add_method(name, method);
    }

    fn add_method_mut<M, A, R>(&mut self, name: impl Into<String>, method: M)
    where
        M: FnMut(&Lua, &mut T, A) -> mlua::Result<R> + MaybeSend + 'static,
        A: FromLuaMulti,
        R: IntoLuaMulti,
    {
        let name = name.into();
        self.record_method(&name);
        self.inner.add_method_mut(name, method);
    }

    fn add_function<F, A, R>(&mut self, name: impl Into<String>, function: F)
    where
        F: Fn(&Lua, A) -> mlua::Result<R> + MaybeSend + 'static,
        A: FromLuaMulti,
        R: IntoLuaMulti,
    {
        let name = name.into();
        self.record_method(&name);
        self.inner.add_function(name, function);
    }

    fn add_function_mut<F, A, R>(&mut self, name: impl Into<String>, function: F)
    where
        F: FnMut(&Lua, A) -> mlua::Result<R> + MaybeSend + 'static,
        A: FromLuaMulti,
        R: IntoLuaMulti,
    {
        let name = name.into();
        self.record_method(&name);
        self.inner.add_function_mut(name, function);
    }

    fn add_meta_method<M, A, R>(&mut self, name: impl Into<String>, method: M)
    where
        M: Fn(&Lua, &T, A) -> mlua::Result<R> + MaybeSend + 'static,
        A: FromLuaMulti,
        R: IntoLuaMulti,
    {
        let name = name.into();
        self.record_method(&name);
        self.inner.add_meta_method(name, method);
    }

    fn add_meta_method_mut<M, A, R>(&mut self, name: impl Into<String>, method: M)
    where
        M: FnMut(&Lua, &mut T, A) -> mlua::Result<R> + MaybeSend + 'static,
        A: FromLuaMulti,
        R: IntoLuaMulti,
    {
        let name = name.into();
        self.record_method(&name);
        self.inner.add_meta_method_mut(name, method);
    }

    fn add_meta_function<F, A, R>(&mut self, name: impl Into<String>, function: F)
    where
        F: Fn(&Lua, A) -> mlua::Result<R> + MaybeSend + 'static,
        A: FromLuaMulti,
        R: IntoLuaMulti,
    {
        let name = name.into();
        self.record_method(&name);
        self.inner.add_meta_function(name, function);
    }

    fn add_meta_function_mut<F, A, R>(&mut self, name: impl Into<String>, function: F)
    where
        F: FnMut(&Lua, A) -> mlua::Result<R> + MaybeSend + 'static,
        A: FromLuaMulti,
        R: IntoLuaMulti,
    {
        let name = name.into();
        self.record_method(&name);
        self.inner.add_meta_function_mut(name, function);
    }
}

impl<T> UserDataFields<T> for RecordingRegistry<'_, T> {
    fn add_field<V>(&mut self, name: impl Into<String>, value: V)
    where
        V: IntoLua + 'static,
    {
        let name = name.into();
        self.record_field(&name);
        self.inner.add_field(name, value);
    }

    fn add_field_method_get<M, R>(&mut self, name: impl Into<String>, method: M)
    where
        M: Fn(&Lua, &T) -> mlua::Result<R> + MaybeSend + 'static,
        R: IntoLua,
    {
        let name = name.into();
        self.record_field(&name);
        self.inner.add_field_method_get(name, method);
    }

    fn add_field_method_set<M, A>(&mut self, name: impl Into<String>, method: M)
    where
        M: FnMut(&Lua, &mut T, A) -> mlua::Result<()> + MaybeSend + 'static,
        A: FromLua,
    {
        let name = name.into();
        self.record_field(&name);
        self.inner.add_field_method_set(name, method);
    }

    fn add_field_function_get<F, R>(&mut self, name: impl Into<String>, function: F)
    where
        F: Fn(&Lua, AnyUserData) -> mlua::Result<R> + MaybeSend + 'static,
        R: IntoLua,
    {
        let name = name.into();
        self.record_field(&name);
        self.inner.add_field_function_get(name, function);
    }

    fn add_field_function_set<F, A>(&mut self, name: impl Into<String>, function: F)
    where
        F: FnMut(&Lua, AnyUserData, A) -> mlua::Result<()> + MaybeSend + 'static,
        A: FromLua,
    {
        let name = name.into();
        self.record_field(&name);
        self.inner.add_field_function_set(name, function);
    }

    fn add_meta_field<V>(&mut self, name: impl Into<String>, value: V)
    where
        V: IntoLua + 'static,
    {
        // Metatable fields (`__tostring`, …) are not LuaLS instance fields.
        self.inner.add_meta_field(name, value);
    }

    fn add_meta_field_with<F, R>(&mut self, name: impl Into<String>, f: F)
    where
        F: FnOnce(&Lua) -> mlua::Result<R> + 'static,
        R: IntoLua,
    {
        self.inner.add_meta_field_with(name, f);
    }
}

/// Register the table-only engine classes — those the data pack extends via
/// `function <Class>:method(...)` but for which no Rust constructor is wired
/// here. Keeps them extensible (`function Party:onJoin(...)`) without depending
/// on a hardcoded bootstrap list.
///
/// Ctor-bearing classes (`Tile`, `Position`, `Combat`, `ItemType`, `Spell`,
/// `Weapon`, `Player`, `Creature`, `Game`, plus Gap 7c revscript ctors)
/// register themselves via their own registrars and are intentionally NOT
/// created here.
pub(crate) fn register_engine_class_tables(lua: &Lua) -> Result<(), mlua::Error> {
    // `Monster` / `Npc` — event hooks (`function Monster:onDropLoot`, …). The
    // data pack also calls `Npc()` / `Monster()` as constructors; those are a
    // separate gap — the table must exist first for method definitions.
    register_class(lua, "Monster", None)?;
    register_class(lua, "Npc", None)?;
    // `Item` / `Container` — `data/lib/core/{item,container}.lua` method defs.
    register_class(lua, "Item", None)?;
    register_class(lua, "Container", None)?;
    // `Party` / `Teleport` / `Vocation` — previously unregistered (`nil`),
    // breaking `data/lib/core/{party,teleport,vocation}.lua` and
    // `data/events/scripts/party.lua` at load time.
    register_class(lua, "Party", None)?;
    register_class(lua, "Teleport", None)?;
    register_class(lua, "Vocation", None)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_class_creates_callable_extensible_table() {
        let lua = Lua::new();
        let ctor = lua.create_function(|_, n: i32| Ok(n * 2)).expect("ctor");
        let t = register_class(&lua, "Widget", Some(ctor)).expect("register");
        // Extensible: a Lua method can be attached.
        t.set("greet", lua.create_function(|_, _: ()| Ok("hi")).unwrap())
            .expect("set method");
        let (kind, call_result, greet): (String, i32, String) = lua
            .load(
                r#"
                function Widget.luaSide(self) return "lua" end
                return type(Widget), Widget(21), Widget:greet()
                "#,
            )
            .eval()
            .expect("eval");
        assert_eq!(kind, "table");
        assert_eq!(call_result, 42);
        assert_eq!(greet, "hi");
    }

    #[test]
    fn register_class_is_idempotent_and_order_independent() {
        let lua = Lua::new();
        // Table-only first, then ctor-bearing — both must coexist.
        register_class(&lua, "Gadget", None).expect("table-only");
        let t1 = register_class(&lua, "Gadget", None).expect("second table-only");
        let ctor = lua.create_function(|_, n: i32| Ok(n + 1)).expect("ctor");
        let t2 = register_class(&lua, "Gadget", Some(ctor)).expect("with ctor");
        // Same table identity (idempotent — never replaces).
        let same: bool = lua
            .load("return Gadget == Gadget")
            .eval()
            .expect("identity");
        assert!(same);
        // __call now attached.
        let result: i32 = lua.load("return Gadget(5)").eval().expect("call");
        assert_eq!(result, 6);
        // Re-registration with another ctor does NOT overwrite __call.
        let ctor2 = lua.create_function(|_, n: i32| Ok(n + 100)).expect("ctor2");
        register_class(&lua, "Gadget", Some(ctor2)).expect("second ctor");
        let result2: i32 = lua.load("return Gadget(5)").eval().expect("call2");
        assert_eq!(result2, 6, "first __call wins on repeat registration");
        let _ = (t1, t2);
    }

    #[test]
    fn register_class_table_only_is_not_callable() {
        let lua = Lua::new();
        register_class(&lua, "Plain", None).expect("register");
        let kind: String = lua.load("return type(Plain)").eval().expect("kind");
        assert_eq!(kind, "table");
        // Extensible but not callable (no __call).
        lua.load("function Plain.method(self) return 1 end")
            .exec()
            .expect("define method");
        let v: i32 = lua
            .load("return Plain:method()")
            .eval()
            .expect("call method");
        assert_eq!(v, 1);
        let call_err = lua.load("return Plain()").exec();
        assert!(call_err.is_err(), "table-only class must not be callable");
    }

    #[test]
    fn register_engine_class_tables_registers_all_seven() {
        let lua = Lua::new();
        register_engine_class_tables(&lua).expect("register");
        for name in [
            "Monster",
            "Npc",
            "Item",
            "Container",
            "Party",
            "Teleport",
            "Vocation",
        ] {
            let kind: String = lua
                .load(&format!("return type({name})"))
                .eval()
                .unwrap_or_else(|_| panic!("type({name})"));
            assert_eq!(kind, "table", "{name} should be a table");
        }
    }

    #[test]
    fn class_index_lookup_walks_chain_first_hit_wins() {
        let lua = Lua::new();
        // `Player` → `Creature` chain: place a value under the same key on
        // each table and verify first-hit-wins + fall-through to the second.
        // (String values make the chain walk trivially observable; the helper
        // returns whatever sits at the key — a method would be a Function.)
        register_class(&lua, "Player", None).expect("Player");
        register_class(&lua, "Creature", None).expect("Creature");
        lua.load("Player.shared = 'player'")
            .exec()
            .expect("Player.shared");
        lua.load("Creature.shared = 'creature'")
            .exec()
            .expect("Creature.shared");
        lua.load("Creature.creature_only = 'creature'")
            .exec()
            .expect("Creature.creature_only");

        let key = lua.create_string("shared").expect("key shared");
        let v: String = class_index_lookup(&lua, CREATURE_INDEX_CHAIN, key)
            .expect("lookup shared")
            .as_string()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert_eq!(v, "player", "first hit (Player) wins");

        let key = lua
            .create_string("creature_only")
            .expect("key creature_only");
        let v: String = class_index_lookup(&lua, CREATURE_INDEX_CHAIN, key)
            .expect("lookup creature_only")
            .as_string()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert_eq!(v, "creature", "falls through Player → Creature");
    }

    #[test]
    fn class_index_lookup_returns_nil_for_missing_or_non_table_global() {
        let lua = Lua::new();
        register_class(&lua, "Player", None).expect("Player");
        // `Creature` deliberately not registered → chain skips it silently.
        let key = lua.create_string("nope").expect("key nope");
        let v = class_index_lookup(&lua, CREATURE_INDEX_CHAIN, key).expect("lookup");
        assert!(matches!(v, Value::Nil), "missing key → Nil");

        // A bare-function global (the Gap 7a pre-migration state) is skipped,
        // not erroring — the chain is best-effort.
        lua.globals()
            .set("Player", lua.create_function(|_, _: ()| Ok(())).unwrap())
            .unwrap();
        let key = lua.create_string("anything").expect("key");
        let v = class_index_lookup(&lua, CREATURE_INDEX_CHAIN, key).expect("lookup");
        assert!(matches!(v, Value::Nil), "non-table global → Nil, no error");
    }

    /// Gap 7c — every name that went through `register_class` on a real
    /// `LuaRuntime` is a Lua `table`, and `__call` is present exactly where a
    /// ctor was attached. Table-driven via the registry so a new
    /// `register_class` call is covered without a new test row. The required
    /// list catches a `globals.set(Name, ctor_fn)` bypass of a known class.
    #[test]
    fn all_class_globals_are_tables() {
        let runtime = crate::runtime::LuaRuntime::new().expect("runtime init");
        let lua = &runtime.lua;
        let entries = registered_class_entries(lua).expect("registry");
        assert!(
            !entries.is_empty(),
            "register_class must record at least the engine classes"
        );

        let registered: std::collections::HashSet<&str> =
            entries.iter().map(|(n, _)| n.as_str()).collect();
        for name in REQUIRED_CLASS_GLOBALS {
            assert!(
                registered.contains(name),
                "{name} must go through register_class (Gap 7c bypass)"
            );
        }

        for (name, callable) in &entries {
            let kind: String = lua
                .load(&format!("return type({name})"))
                .eval()
                .unwrap_or_else(|e| panic!("type({name}): {e}"));
            assert_eq!(kind, "table", "{name} must be a class table, got {kind}");

            let has_call: bool = lua
                .load(&format!(
                    "local mt = getmetatable({name})
                     return mt ~= nil and type(mt.__call) == 'function'"
                ))
                .eval()
                .unwrap_or_else(|e| panic!("getmetatable({name}).__call: {e}"));
            assert_eq!(
                has_call, *callable,
                "{name}: registry callable={callable} but getmetatable.__call is {has_call}"
            );
        }

        for name in HELPER_CTOR_CLASSES {
            let has_call: bool = lua
                .load(&format!(
                    "local mt = getmetatable({name})
                     return mt ~= nil and type(mt.__call) == 'function'"
                ))
                .eval()
                .unwrap_or_else(|e| panic!("helper ctor {name}: {e}"));
            assert!(
                has_call,
                "{name} must have __call (helper_constructors.lua wraps it)"
            );
        }
    }
}
