//! Engine class global registry — single owner of TFS-style class tables.
//!
//! C++ reference: `luascript.cpp` `LuaScriptInterface::registerClass` — creates
//! a class table with a `__call` metamethod so the global is both callable
//! (`Tile(pos)`) and extensible (`function Tile.relocateTo(self, ...)`).
//!
//! Idempotent and order-independent: never replaces an existing table, so the
//! registration sequence in `LuaRuntime::new` no longer decides whether a class
//! ends up callable, extensible, both, or `nil`. See `tasks/tools-actions-gap.md`
//! Gap 7a.

use mlua::{Function, Lua, MultiValue, Table, Value};

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
    Ok(table)
}

/// Register the table-only engine classes — those the data pack extends via
/// `function <Class>:method(...)` but for which no Rust constructor is wired
/// here. Keeps them extensible (`function Party:onJoin(...)`) without depending
/// on a hardcoded bootstrap list.
///
/// Ctor-bearing classes (`Tile`, `Position`, `Combat`, `ItemType`, `Spell`,
/// `Weapon`, `Player`, `Creature`, `Game`) register themselves via their own
/// registrars and are intentionally NOT created here.
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
        let ctor = lua
            .create_function(|_, n: i32| Ok(n * 2))
            .expect("ctor");
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
            "Monster", "Npc", "Item", "Container", "Party", "Teleport", "Vocation",
        ] {
            let kind: String = lua
                .load(&format!("return type({name})"))
                .eval()
                .unwrap_or_else(|_| panic!("type({name})"));
            assert_eq!(kind, "table", "{name} should be a table");
        }
    }
}
