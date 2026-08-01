//! Action registry and Lua loader (`data/scripts/actions/**`).
//!
//! C++ reference: `src/actions.cpp` `Actions::registerLuaEvent` / `Action::executeUse`.
//! Self-registering Lua mirrors TalkAction / Channel (plain table + `:register()`).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::combat_scripts::collect_lua_files;
use crate::runtime::{LuaError, LuaRuntime, PendingAction};

/// Action definition loaded from a revscript (`Action():id(…):register()`).
///
/// C++ reference: `actions.h` `Action` — item id / action id maps + `onUse` callback.
#[derive(Debug)]
pub struct ActionDef {
    pub item_ids: Vec<u16>,
    pub action_ids: Vec<u16>,
    pub on_use: Option<Arc<mlua::RegistryKey>>,
}

impl From<PendingAction> for ActionDef {
    fn from(pending: PendingAction) -> Self {
        Self {
            item_ids: pending.item_ids,
            action_ids: pending.action_ids,
            on_use: pending.on_use.map(Arc::new),
        }
    }
}

/// Inject door ID tables + `table.contains` from `data/global.lua` without loading
/// `lib.lua` / compat (bootstrap deliberately skips full `global.lua`).
///
/// Enables [`doors.lua`](data/scripts/actions/other/doors.lua) to register at load time.
pub fn inject_door_tables_from_global(
    runtime: &LuaRuntime,
    data_dir: &Path,
) -> Result<(), LuaError> {
    let path = data_dir.join("global.lua");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| LuaError::ScriptIo(path.display().to_string(), e.to_string()))?;

    let start = text.find("keys = {").ok_or_else(|| {
        LuaError::ScriptIo(
            path.display().to_string(),
            "missing keys = { in global.lua".into(),
        )
    })?;
    let end = text.find("function getDistanceBetween").ok_or_else(|| {
        LuaError::ScriptIo(
            path.display().to_string(),
            "missing getDistanceBetween marker in global.lua".into(),
        )
    })?;
    let tables = text[start..end].trim();

    let contains_start = text.find("table.contains = function").ok_or_else(|| {
        LuaError::ScriptIo(
            path.display().to_string(),
            "missing table.contains in global.lua".into(),
        )
    })?;
    let contains_rest = &text[contains_start..];
    let contains_end = contains_rest.find("\nend\n").ok_or_else(|| {
        LuaError::ScriptIo(
            path.display().to_string(),
            "unterminated table.contains in global.lua".into(),
        )
    })?;
    let contains = &contains_rest[..=contains_end + 3];

    let chunk = format!("{tables}\n\n{contains}\n");
    runtime.exec_chunk("door_tables_from_global", &chunk)
}

/// Load all action scripts from `data/scripts/actions/**/*.lua`.
///
/// C++ reference: `actions.cpp` `Actions::loadFromXml` (adapted to revscript scan).
pub fn load_action_scripts(
    runtime: &mut LuaRuntime,
    data_dir: &Path,
) -> Result<Vec<ActionDef>, LuaError> {
    let dir = data_dir.join("scripts/actions");
    if !dir.exists() {
        tracing::warn!("Actions directory not found: {}", dir.display());
        return Ok(Vec::new());
    }

    let mut lua_files: Vec<PathBuf> = Vec::new();
    collect_lua_files(&dir, &mut lua_files);
    lua_files.sort();

    for path in &lua_files {
        let path_string = path.display().to_string();
        if let Err(e) = runtime.load_action_script(&path_string) {
            tracing::warn!("Failed to load action script {path_string}: {e}");
        }
    }

    let pending = runtime.drain_pending_actions();
    let actions: Vec<ActionDef> = pending.into_iter().map(Into::into).collect();
    tracing::info!(
        count = actions.len(),
        files = lua_files.len(),
        "Loaded action scripts"
    );
    Ok(actions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn workspace_data_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
    }

    #[test]
    fn food_action_loads_and_registers_meat() {
        let data_root = workspace_data_root();
        let food = data_root.join("scripts/actions/other/food.lua");
        if !food.exists() {
            eprintln!("food.lua not found — skipping");
            return;
        }

        let mut runtime = LuaRuntime::new().expect("runtime init");
        inject_door_tables_from_global(&runtime, &data_root).expect("door tables");
        let defs = load_action_scripts(&mut runtime, &data_root).expect("actions load");

        let food_def = defs.iter().find(|d| d.item_ids.contains(&2666));
        assert!(
            food_def.is_some(),
            "expected food.lua to register meat id 2666; got {} defs",
            defs.len()
        );
        assert!(
            food_def.unwrap().on_use.is_some(),
            "food action must have onUse"
        );
    }

    #[test]
    fn door_tables_inject_enables_doors_register() {
        let data_root = workspace_data_root();
        let doors = data_root.join("scripts/actions/other/doors.lua");
        if !doors.exists() {
            eprintln!("doors.lua not found — skipping");
            return;
        }

        let mut runtime = LuaRuntime::new().expect("runtime init");
        inject_door_tables_from_global(&runtime, &data_root).expect("door tables");
        // Load only doors.lua
        runtime
            .load_action_script(doors.to_str().expect("utf8 path"))
            .expect("doors.lua should load with door tables");
        let pending = runtime.drain_pending_actions();
        assert_eq!(pending.len(), 1);
        assert!(
            pending[0].item_ids.contains(&1210),
            "doors should register closed door 1210"
        );
        assert!(pending[0].on_use.is_some());
    }
}
