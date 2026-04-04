//! `config.lua` loaded via mlua (TFS-style globals).
// C++ reference: `configmanager.cpp` `ConfigManager::load`.

use mlua::{Lua, Value};
use std::path::Path;
use tfs_rust_common::error::{Result, TfsRustError};
use tracing::info;

pub struct ConfigManager {
    lua: Lua,
}

impl ConfigManager {
    pub fn load(path: &Path) -> Result<Self> {
        let lua = Lua::new();
        let chunk = std::fs::read_to_string(path)
            .map_err(|e| TfsRustError::Config(format!("read {}: {e}", path.display())))?;
        lua.load(chunk)
            .set_name(path.display().to_string())
            .exec()
            .map_err(|e| TfsRustError::Lua(format!("execute {}: {e}", path.display())))?;
        info!(file = %path.display(), "loaded config.lua");
        Ok(Self { lua })
    }

    pub fn get_string(&self, key: &str) -> Result<String> {
        let v: Value = self
            .lua
            .globals()
            .get(key)
            .map_err(|e| TfsRustError::Config(format!("lua get {key}: {e}")))?;
        match v {
            Value::String(s) => Ok(s.to_string_lossy().into_owned()),
            Value::Integer(i) => Ok(i.to_string()),
            Value::Number(n) => Ok(n.to_string()),
            Value::Boolean(b) => Ok(b.to_string()),
            Value::Nil => Err(TfsRustError::Config(format!("missing string key `{key}`"))),
            _ => Err(TfsRustError::Config(format!(
                "key `{key}` is not convertible to string"
            ))),
        }
    }

    pub fn get_f64(&self, key: &str) -> Result<f64> {
        let v: Value = self
            .lua
            .globals()
            .get(key)
            .map_err(|e| TfsRustError::Config(format!("lua get {key}: {e}")))?;
        match v {
            Value::Number(n) => Ok(n),
            Value::Integer(i) => Ok(i as f64),
            Value::Nil => Err(TfsRustError::Config(format!("missing number key `{key}`"))),
            _ => Err(TfsRustError::Config(format!("key `{key}` is not a number"))),
        }
    }

    pub fn get_i64(&self, key: &str) -> Result<i64> {
        let v: Value = self
            .lua
            .globals()
            .get(key)
            .map_err(|e| TfsRustError::Config(format!("lua get {key}: {e}")))?;
        match v {
            Value::Integer(i) => Ok(i),
            Value::Number(n) => Ok(n as i64),
            Value::Nil => Err(TfsRustError::Config(format!("missing integer key `{key}`"))),
            _ => Err(TfsRustError::Config(format!(
                "key `{key}` is not an integer"
            ))),
        }
    }

    pub fn get_bool(&self, key: &str) -> Result<bool> {
        let v: Value = self
            .lua
            .globals()
            .get(key)
            .map_err(|e| TfsRustError::Config(format!("lua get {key}: {e}")))?;
        match v {
            Value::Boolean(b) => Ok(b),
            Value::Nil => Err(TfsRustError::Config(format!("missing bool key `{key}`"))),
            _ => Err(TfsRustError::Config(format!(
                "key `{key}` is not a boolean"
            ))),
        }
    }

    /// Fail fast at startup if any required global is absent.
    pub fn require_keys(&self, keys: &[&str]) -> Result<()> {
        for key in keys {
            let v: Value = self
                .lua
                .globals()
                .get(*key)
                .map_err(|e| TfsRustError::Config(format!("lua get {key}: {e}")))?;
            if v.is_nil() {
                return Err(TfsRustError::Config(format!(
                    "required config key `{key}` is missing"
                )));
            }
        }
        Ok(())
    }

    pub fn lua(&self) -> &Lua {
        &self.lua
    }
}
