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

/// Load `data/lib/core/*.lua` (in `core.lua` dependency order) then
/// `data/scripts/functions.lua` into the runtime.
///
/// C++ reference: `data/lib/lib.lua` / `data/lib/core/core.lua` `dofile` chain.
/// `dofile` is not wired in our Lua VM, so we load the files explicitly from Rust
/// in the same order `core.lua` prescribes (`storages.lua` first, then the rest).
/// `compat.lua` and `debugging/` are deliberately skipped (minimal blast radius —
/// see `tasks/tools-actions-gap.md` open question #2).
///
/// Each file is warn-and-continue: a missing or erroring lib file logs a warning
/// but does not abort the load, mirroring `combat_scripts.rs` spell-path behavior.
pub fn load_data_lib(runtime: &LuaRuntime, data_dir: &Path) -> Result<(), LuaError> {
    // Order mirrors `data/lib/core/core.lua` dofile sequence.
    const CORE_FILES: &[&str] = &[
        "storages.lua",
        "achievements.lua",
        "combat.lua",
        "constants.lua",
        "container.lua",
        "creature.lua",
        "game.lua",
        "item.lua",
        "itemtype.lua",
        "party.lua",
        "player.lua",
        "position.lua",
        "teleport.lua",
        "tile.lua",
        "vocation.lua",
    ];

    let core_dir = data_dir.join("lib/core");
    for name in CORE_FILES {
        let path = core_dir.join(name);
        if !path.exists() {
            tracing::warn!("data lib core file not found: {}", path.display());
            continue;
        }
        let src = std::fs::read_to_string(&path)
            .map_err(|e| LuaError::ScriptIo(path.display().to_string(), e.to_string()))?;
        if let Err(e) = runtime.exec_chunk(name, &src) {
            tracing::warn!("Failed to load {}: {}", path.display(), e);
        }
    }

    // `data/scripts/functions.lua` — defines `onUseRope` / `onUseShovel` / `destroyItem`
    // / `Player:computeDamage` / `Player:conjureItem` etc. Required by action scripts
    // (tools, food, levers) and spell scripts alike.
    let functions_path = data_dir.join("scripts/functions.lua");
    if functions_path.exists() {
        let src = std::fs::read_to_string(&functions_path)
            .map_err(|e| LuaError::ScriptIo(functions_path.display().to_string(), e.to_string()))?;
        if let Err(e) = runtime.exec_chunk("functions.lua", &src) {
            tracing::warn!("Failed to load functions.lua: {}", e);
        }
    } else {
        tracing::warn!("functions.lua not found: {}", functions_path.display());
    }

    Ok(())
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

    /// All 9 `data/scripts/actions/tools/*.lua` must load and register their
    /// item ids. `fishing_rod.lua` currently fails (Gap 1: missing
    /// `Action:allowFarUse`) — it's excluded until that gap is closed.
    #[test]
    fn tools_scripts_load_and_register() {
        let data_root = workspace_data_root();
        let tools_dir = data_root.join("scripts/actions/tools");
        if !tools_dir.exists() {
            eprintln!("tools dir not found — skipping");
            return;
        }

        let mut runtime = LuaRuntime::new().expect("runtime init");
        inject_door_tables_from_global(&runtime, &data_root).expect("door tables");
        load_data_lib(&runtime, &data_root).expect("data lib");

        let mut lua_files: Vec<PathBuf> = Vec::new();
        collect_lua_files(&tools_dir, &mut lua_files);
        lua_files.sort();

        // fishing_rod.lua requires Action:allowFarUse (Gap 1 — not yet implemented).
        lua_files.retain(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n != "fishing_rod.lua")
        });

        let mut errors = Vec::new();
        for path in &lua_files {
            let path_string = path.display().to_string();
            if let Err(e) = runtime.load_action_script(&path_string) {
                errors.push((path_string, e.to_string()));
            }
        }

        let pending = runtime.drain_pending_actions();
        assert!(
            errors.is_empty(),
            "tool script load errors (excluding fishing_rod): {errors:?}"
        );

        // 8 scripts (all except fishing_rod) should each register at least one id.
        assert_eq!(
            pending.len(),
            8,
            "expected 8 tool actions (excluding fishing_rod), got {}: {:?}",
            pending.len(),
            pending.iter().map(|p| &p.item_ids).collect::<Vec<_>>()
        );

        // Verify key item ids are registered.
        let all_ids: Vec<u16> = pending.iter().flat_map(|p| p.item_ids.iter().copied()).collect();
        for expected in [2416u16, 2342, 2566, 2420, 2442, 2553, 2120, 2550, 2554] {
            assert!(
                all_ids.contains(&expected),
                "expected item id {expected} in tool action registrations, got {all_ids:?}"
            );
        }
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
        assert!(
            pending[0].item_ids.contains(&1211),
            "doors should register open door 1211"
        );
        assert!(
            pending[0].item_ids.contains(&1209),
            "doors should register locked door 1209"
        );
        assert!(
            pending[0].item_ids.contains(&2088),
            "doors should register key id 2088 for use-with"
        );
        assert!(
            pending[0].item_ids.contains(&1223),
            "doors should register closed quest door 1223"
        );
        assert!(
            pending[0].item_ids.contains(&1227),
            "doors should register closed level door 1227"
        );
        assert!(pending[0].on_use.is_some());
    }

    #[test]
    fn remere_key_attr_constants_are_string_aliases() {
        let runtime = LuaRuntime::new().expect("runtime init");
        let globals = runtime.lua.globals();
        let key: String = globals
            .get("ITEM_ATTRIBUTE_KEYNUMBER")
            .expect("KEYNUMBER");
        let hole: String = globals
            .get("ITEM_ATTRIBUTE_KEYHOLENUMBER")
            .expect("KEYHOLE");
        assert_eq!(key, "keynumber");
        assert_eq!(hole, "keyholenumber");
        let quest_n: String = globals
            .get("ITEM_ATTRIBUTE_DOORQUESTNUMBER")
            .expect("DOORQUESTNUMBER");
        let quest_v: String = globals
            .get("ITEM_ATTRIBUTE_DOORQUESTVALUE")
            .expect("DOORQUESTVALUE");
        let level: String = globals
            .get("ITEM_ATTRIBUTE_DOORLEVEL")
            .expect("DOORLEVEL");
        assert_eq!(quest_n, "doorquestnumber");
        assert_eq!(quest_v, "doorquestvalue");
        assert_eq!(level, "doorlevel");
    }
}
