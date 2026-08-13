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

/// Load the data-pack lib stage into the runtime: `data/lib/core/**/*.lua`
/// (replicating TVP's `data/global.lua` → `dofile('data/lib/lib.lua')` →
/// `core.lua` dofile chain), then `data/scripts/lib/**/*.lua` (matching TVP's
/// `loadScripts("scripts/lib", true, false)`), then top-level
/// `data/scripts/*.lua` (part of TVP's `loadScripts("scripts", false, false)`
/// recursive scan — the per-subsystem loaders cover the subdirectories).
///
/// C++ reference: `scriptmanager.cpp` `ScriptingManager::loadScriptSystems`
/// (lib stage) + `script.cpp` `Scripts::loadScripts` (recursive sorted scan).
///
/// `dofile` is not wired in our Lua VM, so we scan the directories recursively
/// from Rust instead. No file names are hardcoded — the scan picks up whatever
/// the data pack contains. `data/lib/compat/` and `data/lib/debugging/` are
/// skipped (only `data/lib/core/` is scanned; minimal blast radius per
/// `tasks/tools-actions/decisions.md` resolved decision #3).
///
/// Load order is alphabetical (sorted `PathBuf`), matching TVP's `sort(v.begin(),
/// v.end())`. No `data/lib/core/*.lua` file references another at load time
/// (the `storages.lua`-first convention in `core.lua` is for script consumers,
/// not core-file cross-deps), so alphabetical order is safe. The Gap 5
/// assertion ([`assert_required_data_globals`]) catches any missing global
/// regardless of which file defines it.
///
/// Each file is warn-and-continue: a missing or erroring lib file logs a warning
/// but does not abort the load, mirroring `combat_scripts.rs` spell-path behavior.
pub fn load_data_lib(runtime: &LuaRuntime, data_dir: &Path) -> Result<(), LuaError> {
    // `data/lib/core/**/*.lua` — replicates `data/lib/lib.lua` → `core.lua`
    // dofile chain. Recursive scan, sorted (matches TVP's `sort`).
    let core_dir = data_dir.join("lib/core");
    if core_dir.exists() {
        let mut files: Vec<PathBuf> = Vec::new();
        collect_lua_files(&core_dir, &mut files);
        files.sort();
        for path in &files {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("lib_core");
            let src = std::fs::read_to_string(path)
                .map_err(|e| LuaError::ScriptIo(path.display().to_string(), e.to_string()))?;
            if let Err(e) = runtime.exec_chunk(name, &src) {
                tracing::warn!("Failed to load {}: {}", path.display(), e);
            }
        }
    } else {
        tracing::warn!("data/lib/core not found: {}", core_dir.display());
    }

    // `data/scripts/lib/**/*.lua` — TVP's `loadScripts("scripts/lib", true,
    // false)`: revscript lib files (`create_functions.lua`,
    // `defaults_move_event.lua`, `event_callbacks.lua`,
    // `helper_constructors.lua`, `register_monster_type.lua`). Recursive scan,
    // sorted. No cross-file load-time deps.
    let scripts_lib_dir = data_dir.join("scripts/lib");
    if scripts_lib_dir.exists() {
        let mut files: Vec<PathBuf> = Vec::new();
        collect_lua_files(&scripts_lib_dir, &mut files);
        files.sort();
        for path in &files {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("scripts_lib");
            let src = std::fs::read_to_string(path)
                .map_err(|e| LuaError::ScriptIo(path.display().to_string(), e.to_string()))?;
            if let Err(e) = runtime.exec_chunk(name, &src) {
                tracing::warn!("Failed to load {}: {}", path.display(), e);
            }
        }
    } else {
        tracing::warn!("data/scripts/lib not found: {}", scripts_lib_dir.display());
    }

    // Top-level `data/scripts/*.lua` (non-recursive) — part of TVP's
    // `loadScripts("scripts", false, false)` recursive scan. The per-subsystem
    // loaders (`load_action_scripts`, `load_spell_scripts`, etc.) cover the
    // subdirectories; this picks up the top-level files (`functions.lua`,
    // `scarab_tiles.lua`) that no subsystem loader scans. Cross-file calls are
    // inside `onUse*` bodies (deferred to use-time), so alphabetical order is
    // safe.
    let scripts_dir = data_dir.join("scripts");
    if scripts_dir.exists() {
        let mut top_level: Vec<PathBuf> = std::fs::read_dir(&scripts_dir)
            .map_err(|e| LuaError::ScriptIo(scripts_dir.display().to_string(), e.to_string()))?
            .filter_map(std::result::Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|ext| ext == "lua"))
            .collect();
        top_level.sort();
        for path in &top_level {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("data_scripts_top");
            let src = std::fs::read_to_string(path)
                .map_err(|e| LuaError::ScriptIo(path.display().to_string(), e.to_string()))?;
            if let Err(e) = runtime.exec_chunk(name, &src) {
                tracing::warn!("Failed to load {}: {}", path.display(), e);
            }
        }
    }

    Ok(())
}

/// Expected kind of a required data-pack global, for [`assert_required_data_globals`].
#[derive(Copy, Clone)]
enum GlobalKind {
    Function,
    Table,
}

/// Globals that must exist after [`load_data_lib`] for the tools action scripts
/// (and `functions.lua`) to work at runtime. Asserted at boot so a regressed load
/// order or a missing data file aborts startup with a named list instead of
/// surfacing as a `nil` global inside a rope/shovel click hours later.
///
/// See `tasks/tools-actions/gaps-load.md` Gap 5. `table.contains` is dotted — it lives
/// on the standard `table` library, injected by `inject_door_tables_from_global`.
const REQUIRED_DATA_GLOBALS: &[(&str, GlobalKind)] = &[
    ("onUseRope", GlobalKind::Function),
    ("onUsePick", GlobalKind::Function),
    ("onUseShovel", GlobalKind::Function),
    ("onUseScythe", GlobalKind::Function),
    ("onUseMachete", GlobalKind::Function),
    ("onUseKnife", GlobalKind::Function),
    ("destroyItem", GlobalKind::Function),
    ("checkScarabTile", GlobalKind::Function),
    ("table.contains", GlobalKind::Function),
    ("actionIds", GlobalKind::Table),
];

/// Assert that every global in [`REQUIRED_DATA_GLOBALS`] is present and of the
/// right kind after [`load_data_lib`]. Returns `Err(MissingGlobals)` listing the
/// missing names so a regressed load contract fails fast at boot.
///
/// Call this immediately after `load_data_lib` (and
/// `inject_door_tables_from_global`, which supplies `table.contains`) — before
/// any action/spell script load that depends on these globals.
pub fn assert_required_data_globals(runtime: &LuaRuntime) -> Result<(), LuaError> {
    let globals = runtime.lua.globals();
    let mut missing: Vec<String> = Vec::new();

    for &(name, kind) in REQUIRED_DATA_GLOBALS {
        // Dotted names (e.g. `table.contains`) resolve through the parent table.
        let value = if let Some((tbl, field)) = name.split_once('.') {
            globals
                .get::<mlua::Table>(tbl)
                .ok()
                .and_then(|t| t.get::<mlua::Value>(field).ok())
                .unwrap_or(mlua::Value::Nil)
        } else {
            globals
                .get::<mlua::Value>(name)
                .unwrap_or(mlua::Value::Nil)
        };

        let ok = matches!(
            (kind, value),
            (GlobalKind::Function, mlua::Value::Function(_))
                | (GlobalKind::Table, mlua::Value::Table(_))
        );
        if !ok {
            missing.push(name.to_string());
        }
    }

    if missing.is_empty() {
        Ok(())
    } else {
        Err(LuaError::MissingGlobals(missing))
    }
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

    /// Gap 5 regression guard: after the full lib load stage
    /// (`inject_door_tables_from_global` + `load_data_lib`), every declared
    /// required global must resolve to the right kind. If this fails, the
    /// data-pack load order has regressed and a tools script will hit a `nil`
    /// at use-time. Fail the test (and thus boot) with the named list instead.
    #[test]
    fn required_data_globals_present_after_lib_load() {
        let data_root = workspace_data_root();
        if !data_root.join("scripts/functions.lua").exists() {
            eprintln!("data pack not present — skipping");
            return;
        }

        let runtime = LuaRuntime::new().expect("runtime init");
        inject_door_tables_from_global(&runtime, &data_root).expect("door tables");
        load_data_lib(&runtime, &data_root).expect("data lib");

        assert_required_data_globals(&runtime)
            .expect("required data globals present after lib load");
    }

    /// Gap 7a regression guard: every `data/lib/core/*.lua` file must load
    /// without error once engine class globals are registered via
    /// `register_class`. Before Gap 7a, nine core lib files failed (e.g.
    /// `function Tile.relocateTo(...` raised "attempt to index global 'Tile'
    /// (a function value)"; `Party`/`Teleport`/`Vocation` were `nil`).
    /// `load_data_lib` is warn-and-continue, so this test re-runs the same
    /// scan and surfaces every error — the assertion `load_data_lib` cannot
    /// make until Gap 5a makes lib-stage failures fatal.
    #[test]
    fn lib_core_files_load_with_zero_errors() {
        let data_root = workspace_data_root();
        let core_dir = data_root.join("lib/core");
        if !core_dir.exists() {
            eprintln!("data/lib/core not present — skipping");
            return;
        }

        // `core.lua` calls `dofile("data/lib/core/storages.lua")` with a
        // path relative to the process CWD — needs CWD = workspace root.
        let workspace_root = data_root.parent().expect("data/ has a parent");
        let prev_cwd = std::env::current_dir().ok();
        std::env::set_current_dir(workspace_root).expect("chdir to workspace root");

        let runtime = LuaRuntime::new().expect("runtime init");
        inject_door_tables_from_global(&runtime, &data_root).expect("door tables");

        let mut files: Vec<PathBuf> = Vec::new();
        collect_lua_files(&core_dir, &mut files);
        files.sort();

        let mut errors = Vec::new();
        for path in &files {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("lib_core");
            let src = std::fs::read_to_string(path).expect("read lib file");
            if let Err(e) = runtime.exec_chunk(name, &src) {
                errors.push((path.display().to_string(), e.to_string()));
            }
        }

        if let Some(prev) = prev_cwd {
            let _ = std::env::set_current_dir(prev);
        }

        assert!(
            errors.is_empty(),
            "data/lib/core load failures (Gap 7a regression): {errors:?}"
        );
    }

    /// Gap 7c regression guard: every `data/scripts/lib/**/*.lua` file must
    /// load without error once revscript ctor globals go through
    /// `register_class` and `createFunctions` is defined in `data/lib/core`.
    /// Before Gap 7c, 3 of 5 failed (`create_functions.lua`,
    /// `helper_constructors.lua`, `register_monster_type.lua`).
    #[test]
    fn scripts_lib_files_load_with_zero_failures() {
        let data_root = workspace_data_root();
        let scripts_lib_dir = data_root.join("scripts/lib");
        if !scripts_lib_dir.exists() {
            eprintln!("data/scripts/lib not present — skipping");
            return;
        }

        let workspace_root = data_root.parent().expect("data/ has a parent");
        let prev_cwd = std::env::current_dir().ok();
        std::env::set_current_dir(workspace_root).expect("chdir to workspace root");

        let runtime = LuaRuntime::new().expect("runtime init");
        inject_door_tables_from_global(&runtime, &data_root).expect("door tables");

        // `createFunctions` lives in `data/lib/core` (Gap 7c port; not compat.lua).
        let core_dir = data_root.join("lib/core");
        if core_dir.exists() {
            let mut core_files: Vec<PathBuf> = Vec::new();
            collect_lua_files(&core_dir, &mut core_files);
            core_files.sort();
            for path in &core_files {
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("lib_core");
                let src = std::fs::read_to_string(path).expect("read core lib file");
                let _ = runtime.exec_chunk(name, &src);
            }
        }

        let mut files: Vec<PathBuf> = Vec::new();
        collect_lua_files(&scripts_lib_dir, &mut files);
        files.sort();

        let mut errors = Vec::new();
        for path in &files {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("scripts_lib");
            let src = std::fs::read_to_string(path).expect("read scripts/lib file");
            if let Err(e) = runtime.exec_chunk(name, &src) {
                errors.push((path.display().to_string(), e.to_string()));
            }
        }

        if let Some(prev) = prev_cwd {
            let _ = std::env::set_current_dir(prev);
        }

        assert!(
            errors.is_empty(),
            "data/scripts/lib load failures (Gap 7c regression): {errors:?}"
        );
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

    /// Gap 7b — for each userdata whose class table is extended by the data
    /// pack, a Lua-defined method on that class table must be **callable
    /// through a live userdata instance**. This is the check that would have
    /// caught the 7a-only plan (class table alone fixes *load* but not *call*).
    ///
    /// Each row: (class global name, a live userdata value to set as `_probe`,
    /// the expected return of `_probe:__gap7b_probe()`). The Lua method is
    /// defined as `function <Class>.__gap7b_probe(self) return "<Class>" end`.
    #[test]
    fn gap7b_lua_class_method_callable_via_userdata() {
        use crate::context::{CreatureRef, ItemRef};
        use crate::userdata::combat::{CombatDef, CombatRef};
        use crate::userdata::container::ContainerRef;
        use crate::userdata::item_type::ItemTypeRef;
        use crate::userdata::position::PositionRef;
        use crate::userdata::tile::TileRef;
        use crate::userdata::vocation::VocationRef;
        use std::cell::RefCell;
        use std::rc::Rc;

        let runtime = LuaRuntime::new().expect("runtime init");
        let lua = &runtime.lua;
        let globals = lua.globals();

        // Build a live userdata of each type and expose it as `_probe`.
        // (`Tile`/`Position` constructors would also work from Lua, but going
        // through `create_userdata` avoids any ScriptContext dependency and
        // uniformly covers table-only classes like `Vocation` that have no
        // Lua constructor.)
        let tile = lua.create_userdata(TileRef { x: 1, y: 2, z: 3 }).unwrap();
        globals.set("_probe", tile).unwrap();
        let _: String = lua
            .load("function Tile.__gap7b_probe(self) return 'Tile' end return _probe:__gap7b_probe()")
            .eval()
            .expect("Tile __index fallback");

        let pos = lua.create_userdata(PositionRef { x: 1, y: 2, z: 3 }).unwrap();
        globals.set("_probe", pos).unwrap();
        let _: String = lua
            .load("function Position.__gap7b_probe(self) return 'Position' end return _probe:__gap7b_probe()")
            .eval()
            .expect("Position __index fallback");

        let item = lua.create_userdata(ItemRef(1)).unwrap();
        globals.set("_probe", item).unwrap();
        let _: String = lua
            .load("function Item.__gap7b_probe(self) return 'Item' end return _probe:__gap7b_probe()")
            .eval()
            .expect("Item __index fallback");

        let cont = lua.create_userdata(ContainerRef(1)).unwrap();
        globals.set("_probe", cont).unwrap();
        let _: String = lua
            .load("function Container.__gap7b_probe(self) return 'Container' end return _probe:__gap7b_probe()")
            .eval()
            .expect("Container __index fallback");

        let it = lua.create_userdata(ItemTypeRef(42)).unwrap();
        globals.set("_probe", it).unwrap();
        let _: String = lua
            .load("function ItemType.__gap7b_probe(self) return 'ItemType' end return _probe:__gap7b_probe()")
            .eval()
            .expect("ItemType __index fallback");

        let combat = lua
            .create_userdata(CombatRef(Rc::new(RefCell::new(CombatDef::new()))))
            .unwrap();
        globals.set("_probe", combat).unwrap();
        let _: String = lua
            .load("function Combat.__gap7b_probe(self) return 'Combat' end return _probe:__gap7b_probe()")
            .eval()
            .expect("Combat __index fallback");

        let voc = lua.create_userdata(VocationRef(5)).unwrap();
        globals.set("_probe", voc).unwrap();
        let _: String = lua
            .load("function Vocation.__gap7b_probe(self) return 'Vocation' end return _probe:__gap7b_probe()")
            .eval()
            .expect("Vocation __index fallback");

        let creature = lua.create_userdata(CreatureRef(1)).unwrap();
        globals.set("_probe", creature).unwrap();
        let _: String = lua
            .load("function Player.__gap7b_probe(self) return 'Player' end return _probe:__gap7b_probe()")
            .eval()
            .expect("CreatureRef → Player __index fallback");
    }

    /// Gap 7b — `CreatureRef` must reach **both** `Player` and `Creature`
    /// class tables (chain `Player` → `Creature`). The previous hardcoded
    /// `"Player"` fallback silently missed all 15 methods in
    /// `data/lib/core/creature.lua` — this is the latent-bug regression guard.
    #[test]
    fn gap7b_creature_ref_reaches_creature_table() {
        use crate::context::CreatureRef;

        let runtime = LuaRuntime::new().expect("runtime init");
        let lua = &runtime.lua;
        let globals = lua.globals();

        let creature = lua.create_userdata(CreatureRef(1)).unwrap();
        globals.set("_probe", creature).unwrap();
        // Define a method ONLY on `Creature` (not `Player`); the chain must
        // fall through `Player` (nil) and find it on `Creature`.
        let got: String = lua
            .load(
                "function Creature.__gap7b_creature_only(self) return 'creature' end \
                 return _probe:__gap7b_creature_only()",
            )
            .eval()
            .expect("CreatureRef → Creature fallback");
        assert_eq!(got, "creature");
    }

    /// Gap 7b — a native Rust method must still win over a same-named Lua
    /// method on the class table. mlua's generated `__index` checks
    /// registered methods before the user `__index` fallback, so a Lua
    /// override cannot silently shadow an engine method.
    #[test]
    fn gap7b_native_method_wins_over_lua_override() {
        use crate::userdata::item_type::ItemTypeRef;
        use crate::userdata::vocation::VocationRef;

        let runtime = LuaRuntime::new().expect("runtime init");
        let lua = &runtime.lua;
        let globals = lua.globals();

        // `ItemType:getId()` is a native method returning `this.0` (42).
        // A Lua override on the `ItemType` class table must NOT take effect.
        let it = lua.create_userdata(ItemTypeRef(42)).unwrap();
        globals.set("_probe", it).unwrap();
        lua.load("function ItemType.getId(self) return 999 end")
            .exec()
            .expect("define override");
        let got: i64 = lua
            .load("return _probe:getId()")
            .eval()
            .expect("getId call");
        assert_eq!(got, 42, "native ItemType:getId must win over Lua override");

        // Same check with `Vocation:getId()` (native returns `this.0` = 5).
        let voc = lua.create_userdata(VocationRef(5)).unwrap();
        globals.set("_probe", voc).unwrap();
        let got: i64 = lua
            .load("function Vocation.getId(self) return 999 end return _probe:getId()")
            .eval()
            .expect("Vocation getId call");
        assert_eq!(got, 5, "native Vocation:getId must win over Lua override");
    }
}
