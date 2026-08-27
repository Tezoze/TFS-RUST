//! TFS Lua pack `db` / `result` globals (pack SQL, not mechanics).
//!
//! C++ reference: `luascript.cpp` `luaDatabaseStoreQuery` / `luaDatabaseExecute` /
//! `luaDatabaseAsyncExecute` / `luaDatabaseEscapeString` / `luaResultGetNumber` /
//! `luaResultGetString` / `luaResultNext` / `luaResultFree`; `database.cpp`
//! `storeQuery` / `executeQuery` / `escapeBlob` / `DBResult::next`.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::OnceLock;

use mlua::{Lua, Value};

/// Pack SQL request from Lua `db.query` / `db.storeQuery`.
#[derive(Debug, Clone)]
pub enum LuaDbRequest {
    Execute(String),
    StoreQuery(String),
}

/// Outcome of a pack SQL request (TFS does not throw SQL errors to Lua).
#[derive(Debug, Clone)]
pub enum LuaDbOutcome {
    Execute(bool),
    Rows(Vec<HashMap<String, Option<String>>>),
}

/// Game-thread SQL bridge (IO happens on the sqlx side).
pub type LuaDbBridge = fn(LuaDbRequest) -> LuaDbOutcome;

static DB_BRIDGE: OnceLock<LuaDbBridge> = OnceLock::new();

#[cfg(test)]
thread_local! {
    static TEST_BRIDGE: Cell<Option<LuaDbBridge>> = const { Cell::new(None) };
}

struct StoredResult {
    rows: Vec<HashMap<String, Option<String>>>,
    index: usize,
}

thread_local! {
    static RESULT_STORE: RefCell<HashMap<u32, StoredResult>> = RefCell::new(HashMap::new());
    static NEXT_RESULT_ID: Cell<u32> = const { Cell::new(1) };
}

/// Register the core handler that runs pack SQL (called once at startup).
pub fn register_lua_db_bridge(bridge: LuaDbBridge) {
    #[cfg(test)]
    TEST_BRIDGE.with(|c| c.set(Some(bridge)));
    let _ = DB_BRIDGE.set(bridge);
}

/// Register globals `db` and `result` — C++ `luaDatabase*` / `luaResult*`.
pub fn register_lua_database(lua: &Lua) -> Result<(), mlua::Error> {
    let globals = lua.globals();

    let db = lua.create_table()?;
    db.set(
        "escapeString",
        lua.create_function(|_, s: String| Ok(escape_mysql_quoted(&s)))?,
    )?;
    db.set(
        "query",
        lua.create_function(|_, sql: String| Ok(db_execute(&sql)))?,
    )?;
    db.set(
        "asyncQuery",
        lua.create_function(|_, sql: String| Ok(db_execute(&sql)))?,
    )?;
    db.set(
        "storeQuery",
        lua.create_function(|_, sql: String| Ok(db_store_query(&sql)))?,
    )?;
    globals.set("db", db)?;

    let result = lua.create_table()?;
    result.set(
        "getNumber",
        lua.create_function(|_, (id, col): (u32, String)| Ok(result_get_number(id, &col)))?,
    )?;
    result.set(
        "getString",
        lua.create_function(|lua, (id, col): (u32, String)| result_get_string(lua, id, &col))?,
    )?;
    result.set(
        "next",
        lua.create_function(|_, id: u32| Ok(result_next(id)))?,
    )?;
    result.set(
        "free",
        lua.create_function(|_, id: u32| Ok(result_free(id)))?,
    )?;
    globals.set("result", result)?;
    Ok(())
}

/// `Database::escapeString` → `escapeBlob`: quote and MySQL-escape.
fn escape_mysql_quoted(s: &str) -> String {
    let mut out = Vec::with_capacity(s.len().saturating_mul(2).saturating_add(2));
    out.push(b'\'');
    for &b in s.as_bytes() {
        match b {
            0 => out.extend_from_slice(b"\\0"),
            b'\n' => out.extend_from_slice(b"\\n"),
            b'\r' => out.extend_from_slice(b"\\r"),
            b'\\' => out.extend_from_slice(b"\\\\"),
            b'\'' => out.extend_from_slice(b"\\'"),
            b'"' => out.extend_from_slice(b"\\\""),
            0x1a => out.extend_from_slice(b"\\Z"),
            _ => out.push(b),
        }
    }
    out.push(b'\'');
    String::from_utf8(out).unwrap_or_else(|e| String::from_utf8_lossy(&e.into_bytes()).into_owned())
}

fn call_db_bridge(req: LuaDbRequest) -> LuaDbOutcome {
    #[cfg(test)]
    if let Some(bridge) = TEST_BRIDGE.with(|c| c.get()) {
        return bridge(req);
    }
    match DB_BRIDGE.get() {
        Some(bridge) => bridge(req),
        None => match req {
            LuaDbRequest::Execute(_) => LuaDbOutcome::Execute(false),
            LuaDbRequest::StoreQuery(_) => LuaDbOutcome::Rows(Vec::new()),
        },
    }
}

fn db_execute(sql: &str) -> bool {
    match call_db_bridge(LuaDbRequest::Execute(sql.to_string())) {
        LuaDbOutcome::Execute(ok) => ok,
        LuaDbOutcome::Rows(_) => false,
    }
}

fn db_store_query(sql: &str) -> Value {
    let rows = match call_db_bridge(LuaDbRequest::StoreQuery(sql.to_string())) {
        LuaDbOutcome::Rows(rows) => rows,
        LuaDbOutcome::Execute(_) => Vec::new(),
    };
    if rows.is_empty() {
        return Value::Boolean(false);
    }
    let id = NEXT_RESULT_ID.get();
    let next = id.wrapping_add(1);
    NEXT_RESULT_ID.set(if next == 0 { 1 } else { next });
    RESULT_STORE.with(|store| {
        store
            .borrow_mut()
            .insert(id, StoredResult { rows, index: 0 });
    });
    Value::Integer(i64::from(id))
}

fn current_cell<'a>(res: &'a StoredResult, col: &str) -> Option<&'a Option<String>> {
    res.rows.get(res.index).and_then(|row| row.get(col))
}

fn cell_to_number(cell: Option<&Option<String>>) -> i64 {
    let Some(Some(s)) = cell else {
        return 0;
    };
    let t = s.trim();
    if let Ok(n) = t.parse::<i64>() {
        return n;
    }
    if let Ok(n) = t.parse::<f64>() {
        return n as i64;
    }
    0
}

fn result_get_number(id: u32, col: &str) -> Value {
    RESULT_STORE.with(|store| {
        let store = store.borrow();
        let Some(res) = store.get(&id) else {
            return Value::Boolean(false);
        };
        Value::Integer(cell_to_number(current_cell(res, col)))
    })
}

fn result_get_string(lua: &Lua, id: u32, col: &str) -> Result<Value, mlua::Error> {
    let text = RESULT_STORE.with(|store| {
        let store = store.borrow();
        store.get(&id).map(|res| match current_cell(res, col) {
            Some(Some(v)) => v.clone(),
            Some(None) | None => String::new(),
        })
    });
    match text {
        None => Ok(Value::Boolean(false)),
        Some(s) => Ok(Value::String(lua.create_string(s)?)),
    }
}

fn result_next(id: u32) -> bool {
    RESULT_STORE.with(|store| {
        let mut store = store.borrow_mut();
        let Some(res) = store.get_mut(&id) else {
            return false;
        };
        // C++ `DBResult::next` always `mysql_fetch_row`; past-end current row is null.
        res.index = res.index.saturating_add(1);
        res.index < res.rows.len()
    })
}

fn result_free(id: u32) -> bool {
    RESULT_STORE.with(|store| store.borrow_mut().remove(&id).is_some())
}

#[cfg(test)]
fn reset_result_store() {
    RESULT_STORE.with(|s| s.borrow_mut().clear());
    NEXT_RESULT_ID.with(|c| c.set(1));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::LuaRuntime;

    fn eval<T: mlua::FromLua>(runtime: &LuaRuntime, chunk: &str) -> T {
        runtime.lua.load(chunk).eval().expect("eval")
    }

    fn row(pairs: &[(&str, Option<&str>)]) -> HashMap<String, Option<String>> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), v.map(str::to_string)))
            .collect()
    }

    fn mock_ok(req: LuaDbRequest) -> LuaDbOutcome {
        match req {
            LuaDbRequest::Execute(_) => LuaDbOutcome::Execute(true),
            LuaDbRequest::StoreQuery(sql) => {
                if sql.contains("empty") {
                    LuaDbOutcome::Rows(Vec::new())
                } else if sql.contains("two") {
                    LuaDbOutcome::Rows(vec![
                        row(&[("id", Some("1")), ("name", Some("a"))]),
                        row(&[("id", Some("2")), ("name", Some("b"))]),
                    ])
                } else {
                    LuaDbOutcome::Rows(vec![row(&[("id", Some("42")), ("name", Some("Hero"))])])
                }
            }
        }
    }

    #[test]
    fn escape_string_quotes_and_escapes() {
        reset_result_store();
        let runtime = LuaRuntime::new().expect("runtime");
        let got: String = eval(&runtime, r#"return db.escapeString("O'Brien\\path")"#);
        assert_eq!(got, "'O\\'Brien\\\\path'");
        let quote: String = eval(&runtime, r#"return db.escapeString("a'b")"#);
        assert_eq!(quote, "'a\\'b'");
        let slash: String = eval(&runtime, r#"return db.escapeString("a\\b")"#);
        assert_eq!(slash, "'a\\\\b'");
    }

    #[test]
    fn store_query_empty_returns_false() {
        reset_result_store();
        register_lua_db_bridge(mock_ok);
        let runtime = LuaRuntime::new().expect("runtime");
        let is_false: bool = eval(&runtime, r#"return db.storeQuery("empty") == false"#);
        assert!(is_false);
    }

    #[test]
    fn store_query_rows_get_next_free() {
        reset_result_store();
        register_lua_db_bridge(mock_ok);
        let runtime = LuaRuntime::new().expect("runtime");
        let chunk = r#"
            local id = db.storeQuery("two")
            if id == false then return "no-id" end
            local a = result.getString(id, "name") .. tostring(result.getNumber(id, "id"))
            local n1 = result.next(id)
            local b = result.getString(id, "name") .. tostring(result.getNumber(id, "id"))
            local n2 = result.next(id)
            local freed = result.free(id)
            local after = result.getNumber(id, "id")
            return a .. "," .. tostring(n1) .. "," .. b .. "," .. tostring(n2) .. "," .. tostring(freed) .. "," .. type(after)
        "#;
        let got: String = eval(&runtime, chunk);
        assert_eq!(got, "a1,true,b2,false,true,boolean");
    }

    #[test]
    fn query_returns_mock_bool() {
        reset_result_store();
        register_lua_db_bridge(|req| match req {
            LuaDbRequest::Execute(sql) if sql.contains("ok") => LuaDbOutcome::Execute(true),
            LuaDbRequest::Execute(_) => LuaDbOutcome::Execute(false),
            LuaDbRequest::StoreQuery(_) => LuaDbOutcome::Rows(Vec::new()),
        });
        let runtime = LuaRuntime::new().expect("runtime");
        let ok: bool = eval(&runtime, r#"return db.query("ok")"#);
        let bad: bool = eval(&runtime, r#"return db.query("fail")"#);
        let async_ok: bool = eval(&runtime, r#"return db.asyncQuery("ok")"#);
        assert!(ok);
        assert!(!bad);
        assert!(async_ok);
    }

    #[test]
    fn deathlist_shaped_result_id_check() {
        reset_result_store();
        register_lua_db_bridge(mock_ok);
        let runtime = LuaRuntime::new().expect("runtime");
        let n: i64 = eval(
            &runtime,
            r#"
            local resultId = db.storeQuery("select")
            local n = 0
            if resultId ~= false then
                n = result.getNumber(resultId, "id")
                result.free(resultId)
            end
            return n
            "#,
        );
        assert_eq!(n, 42);
    }
}
