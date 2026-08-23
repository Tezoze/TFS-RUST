//! Sandboxed Lua-as-data loader + schema gate.
//!
//! Pilot infrastructure for `docs/DATA_FORMAT_MIGRATION.md` Phase 0. Data files
//! (`data/vocations.lua`, future `data/outfits.lua`, …) are pure data — they
//! return a table, no side effects. This module loads them in a fresh restricted
//! `Lua` with `io`/`os`/`package`/`require`/`dofile`/`loadfile` stripped so a
//! "config" file cannot do I/O or escape the sandbox, then hands the materialized
//! table to `serde` via `mlua`'s `LuaSerdeExt` (`lua.from_value`).
//!
//! Reuses the plain `Lua::new()` + table-load pattern already proven in
//! `tfs-rust-core/src/formulas.rs::load_mechanics`; this module adds sandboxing,
//! a `schema` version gate, and the `serde` deserialize seam.

use mlua::{Lua, Table, Value};
use tfs_rust_common::error::{Result, TfsRustError};

/// Build a fresh sandboxed `Lua` for loading pure-data files.
///
/// Strips `io`, `os`, `package`, `require`, `dofile`, `loadfile` from the global
/// environment so data files cannot perform I/O, load modules, or execute
/// arbitrary files. `string`/`table`/`math` remain (data files may use them for
/// derivation/era-conditionals per the migration doc).
pub fn sandboxed_data_lua() -> Result<Lua> {
    let lua = Lua::new();
    let globals = lua.globals();
    // Strip I/O / module / file-load globals.
    for key in ["io", "os", "package", "require", "dofile", "loadfile"] {
        let _ = globals.set(key, Value::Nil);
    }
    Ok(lua)
}

/// Verify the `schema` field on a loaded root table matches `expected`.
///
/// Each data file declares `schema = N` at the top so the format can evolve
/// safely. Fails fast at startup with a clear message if the version is wrong
/// or missing — never a mid-game panic.
pub fn require_schema(root: &Table, expected: u32) -> Result<()> {
    match root.get::<Value>("schema") {
        Ok(Value::Integer(i)) => {
            if i as u32 != expected {
                return Err(TfsRustError::Content {
                    file: "(data-lua)".into(),
                    message: format!("schema version mismatch: expected {expected}, got {i}"),
                });
            }
        }
        Ok(Value::Number(f)) => {
            if f as u32 != expected {
                return Err(TfsRustError::Content {
                    file: "(data-lua)".into(),
                    message: format!("schema version mismatch: expected {expected}, got {f}"),
                });
            }
        }
        Ok(_) => {
            return Err(TfsRustError::Content {
                file: "(data-lua)".into(),
                message: format!("missing or non-numeric 'schema' field; expected {expected}"),
            });
        }
        Err(_) => {
            return Err(TfsRustError::Content {
                file: "(data-lua)".into(),
                message: format!("missing 'schema' field; expected {expected}"),
            });
        }
    }
    Ok(())
}

/// Eval a data-Lua source string as a table (`return { schema = N, ... }`).
///
/// `name` is the chunk name used in Lua error messages (file path or test label).
pub fn load_data_table_str(lua: &Lua, src: &str, name: &str) -> Result<Table> {
    let value: Value = lua
        .load(src)
        .set_name(name.to_string())
        .eval()
        .map_err(|e| TfsRustError::Content {
            file: name.to_string(),
            message: format!("lua eval failed: {e}"),
        })?;
    match value {
        Value::Table(t) => Ok(t),
        other => Err(TfsRustError::Content {
            file: name.to_string(),
            message: format!("data file must return a table; got {}", other.type_name()),
        }),
    }
}

/// Load a data-Lua file, eval it as a table, and return the root table.
///
/// The file must `return { schema = N, ... }`. The caller is responsible for
/// `require_schema` + `lua.from_value` into the target `serde` type.
pub fn load_data_table(lua: &Lua, path: &std::path::Path) -> Result<Table> {
    let src = std::fs::read_to_string(path).map_err(|e| TfsRustError::Content {
        file: path.to_string_lossy().into_owned(),
        message: e.to_string(),
    })?;
    load_data_table_str(lua, &src, &path.display().to_string())
}
