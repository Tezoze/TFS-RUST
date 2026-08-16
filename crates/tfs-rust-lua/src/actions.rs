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
/// C++ reference: `actions.h` `Action` — item id / action id maps + `onUse` + `allowFarUse`.
#[derive(Debug)]
pub struct ActionDef {
    pub item_ids: Vec<u16>,
    pub action_ids: Vec<u16>,
    pub on_use: Option<Arc<mlua::RegistryKey>>,
    /// C++ `Action::allowFarUse` — `actions.h`. Default `false`.
    pub allow_far_use: bool,
}

impl From<PendingAction> for ActionDef {
    fn from(pending: PendingAction) -> Self {
        Self {
            item_ids: pending.item_ids,
            action_ids: pending.action_ids,
            on_use: pending.on_use.map(Arc::new),
            allow_far_use: pending.allow_far_use,
        }
    }
}

/// Inject `actionIds` + door ID tables + `table.contains` + `getFormattedWorldTime`
/// and `getPlayerFlagValue` from `data/global.lua` / a compat one-liner without
/// loading `lib.lua` or `compat.lua` (bootstrap skips full `global.lua`).
/// TVP defines `actionIds` in `global.lua`, not `actionids.lua`.
///
/// Enables [`doors.lua`](data/scripts/actions/other/doors.lua), tools scripts
/// that read `actionIds.*`, and [`watch.lua`](data/scripts/actions/other/watch.lua).
pub fn inject_door_tables_from_global(
    runtime: &LuaRuntime,
    data_dir: &Path,
) -> Result<(), LuaError> {
    let path = data_dir.join("global.lua");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| LuaError::ScriptIo(path.display().to_string(), e.to_string()))?;

    // Prefer `actionIds` (TVP `global.lua`); fall back to `keys` for packs that
    // still start their table block later.
    let start = text
        .find("actionIds = {")
        .or_else(|| text.find("keys = {"))
        .ok_or_else(|| {
            LuaError::ScriptIo(
                path.display().to_string(),
                "missing actionIds = { or keys = { in global.lua".into(),
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

    // E9: `getFormattedWorldTime` only — do not `dofile` `global.lua` (it loads `lib.lua`).
    let time_start = text.find("function getFormattedWorldTime").ok_or_else(|| {
        LuaError::ScriptIo(
            path.display().to_string(),
            "missing getFormattedWorldTime in global.lua".into(),
        )
    })?;
    let time_rest = &text[time_start..];
    let time_end = time_rest.find("\nend\n").ok_or_else(|| {
        LuaError::ScriptIo(
            path.display().to_string(),
            "unterminated getFormattedWorldTime in global.lua".into(),
        )
    })?;
    let formatted_time = &time_rest[..=time_end + 3];

    // R4: compat one-liner only — do not `dofile` `compat.lua`.
    let flag_helper = "function getPlayerFlagValue(cid, flag) local p = Player(cid) return p ~= nil and p:hasFlag(flag) or false end";

    let chunk = format!("{tables}\n\n{contains}\n\n{formatted_time}\n\n{flag_helper}\n");
    runtime.exec_chunk("door_tables_from_global", &chunk)
}

/// Load `data/formulas/<clientVersion>.lua` into the **game** Lua VM so action
/// scripts can read era knobs (`formulas.fishing`, `formulas.destroyableStone`).
///
/// Same source file the core `load_mechanics` parser uses (separate VM for
/// Tier-1/Tier-2 hooks). Missing file is an error — tools scripts must not
/// fall back to hardcoded parity numbers.
pub fn inject_era_formulas(
    runtime: &LuaRuntime,
    data_dir: &Path,
    client_version: u16,
) -> Result<(), LuaError> {
    let path = data_dir
        .join("formulas")
        .join(format!("{client_version}.lua"));
    let text = std::fs::read_to_string(&path)
        .map_err(|e| LuaError::ScriptIo(path.display().to_string(), e.to_string()))?;
    runtime.exec_chunk(&path.display().to_string(), &text)
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
/// No file names are hardcoded — the scan picks up whatever the data pack
/// contains. `data/lib/compat/` and `data/lib/debugging/` are skipped (only
/// `data/lib/core/` is scanned; minimal blast radius per
/// `tasks/tools-actions/decisions.md` resolved decision #3). `core.lua` and
/// `lib.lua` are skipped as `dofile` dispatchers: they double-load every core
/// file under a recursive scan, and their CWD-relative `dofile` fails outside
/// the repo root. (Step 11 may replace this scan with the Lua `dofile` chain.)
///
/// Load order is alphabetical (sorted `PathBuf`), matching TVP's `sort(v.begin(),
/// v.end())`. No `data/lib/core/*.lua` file references another at load time
/// (the `storages.lua`-first convention in `core.lua` is for script consumers,
/// not core-file cross-deps), so alphabetical order is safe. The Gap 5
/// assertion ([`assert_required_data_globals`]) is a cheap extra guard on the
/// tools contract; the load itself is the primary defense.
///
/// Lib-stage errors are **fatal and aggregated** (Gap 5a): every IO/exec
/// failure is collected and returned as [`LuaError::LibStageFailures`]. A
/// broken lib file is a boot-blocking defect — the data pack ships with this
/// repo. Content-stage loaders (`load_action_scripts`, spell/weapon scans)
/// stay warn-and-continue so a broken shard script cannot brick the server.
pub fn load_data_lib(runtime: &LuaRuntime, data_dir: &Path) -> Result<(), LuaError> {
    let mut failures: Vec<(String, String)> = Vec::new();

    // `data/lib/core/**/*.lua` — replicates `data/lib/lib.lua` → `core.lua`
    // dofile chain. Recursive scan, sorted (matches TVP's `sort`).
    let core_dir = data_dir.join("lib/core");
    if core_dir.exists() {
        let mut files: Vec<PathBuf> = Vec::new();
        collect_lua_files(&core_dir, &mut files);
        files.retain(|p| !is_dofile_dispatcher(p));
        files.sort();
        exec_lib_stage_files(runtime, &files, "lib_core", &mut failures);
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
        exec_lib_stage_files(runtime, &files, "scripts_lib", &mut failures);
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
        match std::fs::read_dir(&scripts_dir) {
            Ok(entries) => {
                let mut top_level: Vec<PathBuf> = entries
                    .filter_map(std::result::Result::ok)
                    .map(|e| e.path())
                    .filter(|p| p.extension().is_some_and(|ext| ext == "lua"))
                    .collect();
                top_level.sort();
                exec_lib_stage_files(runtime, &top_level, "data_scripts_top", &mut failures);
            }
            Err(e) => {
                failures.push((scripts_dir.display().to_string(), e.to_string()));
            }
        }
    }

    if failures.is_empty() {
        Ok(())
    } else {
        Err(LuaError::LibStageFailures(failures))
    }
}

/// `core.lua` / `lib.lua` are `dofile` dispatchers, redundant under a recursive
/// scan. Skip by filename so a copy in `data/lib/core` cannot sneak back in.
fn is_dofile_dispatcher(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|n| n.to_str()),
        Some("core.lua") | Some("lib.lua")
    )
}

/// Execute each lib-stage file, collecting IO and exec errors instead of
/// aborting at the first. Used by [`load_data_lib`] so boot lists every
/// broken file (Gap 5a). Content-stage loaders must **not** call this — they
/// warn-and-continue per file.
fn exec_lib_stage_files(
    runtime: &LuaRuntime,
    files: &[PathBuf],
    fallback_chunk_name: &str,
    failures: &mut Vec<(String, String)>,
) {
    for path in files {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(fallback_chunk_name);
        match std::fs::read_to_string(path) {
            Ok(src) => {
                if let Err(e) = runtime.exec_chunk(name, &src) {
                    failures.push((path.display().to_string(), e.to_string()));
                }
            }
            Err(e) => {
                failures.push((path.display().to_string(), e.to_string()));
            }
        }
    }
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
    ("formulas", GlobalKind::Table),
    ("formulas.fishingSuccess", GlobalKind::Function),
    ("formulas.destroyableStone", GlobalKind::Table),
];

/// Assert that every global in [`REQUIRED_DATA_GLOBALS`] is present and of the
/// right kind after [`load_data_lib`]. Returns `Err(MissingGlobals)` listing the
/// missing names so a regressed load contract fails fast at boot.
///
/// Cheap extra guard on the tools contract; Gap 5a made the load itself the
/// primary defense (`LuaError::LibStageFailures`). Call immediately after
/// `load_data_lib` (and `inject_door_tables_from_global`, which supplies
/// `table.contains`) — before any action/spell script load that depends on
/// these globals.
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
            globals.get::<mlua::Value>(name).unwrap_or(mlua::Value::Nil)
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

    thread_local! {
        static CAPTURED_TEXT_DIALOG: std::cell::RefCell<Option<(u16, String)>> =
            const { std::cell::RefCell::new(None) };
    }

    fn capture_text_dialog_applier(
        _: *mut (),
        mutation: crate::lua_mutation::LuaMutation,
    ) -> Result<(), String> {
        if let crate::lua_mutation::LuaMutation::PlayerShowTextDialog {
            item_type, text, ..
        } = mutation
        {
            CAPTURED_TEXT_DIALOG.with(|c| *c.borrow_mut() = Some((item_type, text)));
        }
        Ok(())
    }

    /// Scratch data pack for Gap 5a policy tests. Removed on drop so a panic
    /// still cleans up.
    struct TempDataPack(PathBuf);

    impl TempDataPack {
        fn new() -> Self {
            let dir = std::env::temp_dir().join(format!(
                "tfs-gap5a-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0)
            ));
            std::fs::create_dir_all(dir.join("lib/core")).expect("temp lib/core");
            std::fs::create_dir_all(dir.join("scripts/lib")).expect("temp scripts/lib");
            std::fs::create_dir_all(dir.join("scripts")).expect("temp scripts");
            Self(dir)
        }

        fn write(&self, rel: &str, contents: &str) {
            let path = self.0.join(rel);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("temp parent");
            }
            std::fs::write(&path, contents).expect("write temp lua");
        }
    }

    impl Drop for TempDataPack {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
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
        inject_era_formulas(&runtime, &data_root, 772).expect("era formulas");

        assert_required_data_globals(&runtime)
            .expect("required data globals present after lib load");
    }

    /// Gap 4: `SKILL_*` engine constants survive lib load, and TVP
    /// `actionIds` from `global.lua` is not replaced by `actionids.lua`.
    #[test]
    fn gap4_skill_and_destroyable_stone_present_after_lib_load() {
        let data_root = workspace_data_root();
        if !data_root.join("scripts/functions.lua").exists() {
            eprintln!("data pack not present — skipping");
            return;
        }

        let runtime = LuaRuntime::new().expect("runtime init");
        inject_door_tables_from_global(&runtime, &data_root).expect("door tables");
        load_data_lib(&runtime, &data_root).expect("data lib");
        inject_era_formulas(&runtime, &data_root, 772).expect("era formulas");

        let globals = runtime.lua.globals();
        let fishing: i32 = globals.get("SKILL_FISHING").expect("SKILL_FISHING");
        assert_eq!(fishing, 6, "enums.h skills_t SKILL_FISHING");

        let action_ids: mlua::Table = globals.get("actionIds").expect("actionIds table");
        // TVP `data/global.lua` `actionIds` — 4000–4005.
        let stone: i32 = action_ids
            .get("destroyableStone")
            .expect("actionIds.destroyableStone");
        assert_eq!(stone, 4004, "TVP destroyableStone");
        assert_eq!(action_ids.get::<i32>("sandHole").expect("sandHole"), 4002);
        assert_eq!(action_ids.get::<i32>("pickHole").expect("pickHole"), 4003);
        assert_eq!(
            action_ids.get::<i32>("levelDoor").expect("levelDoor"),
            1000,
            "TFS extras merged by actionids.lua must survive"
        );
    }

    /// Gap 6: era tool numbers come from `data/formulas/<v>.lua`, not the scripts.
    #[test]
    fn gap6_era_formulas_supply_pick_and_fishing_numbers() {
        let data_root = workspace_data_root();
        if !data_root.join("formulas/772.lua").exists() {
            eprintln!("formulas not present — skipping");
            return;
        }

        let runtime = LuaRuntime::new().expect("runtime init");
        inject_era_formulas(&runtime, &data_root, 772).expect("772 formulas");
        let globals = runtime.lua.globals();
        let formulas: mlua::Table = globals.get("formulas").expect("formulas");
        let fishing: mlua::Table = formulas.get("fishing").expect("formulas.fishing");
        let model: String = fishing.get("model").expect("fishing.model");
        assert_eq!(model, "probe", "772 moveuse.dat TestSkill Probe");
        let diff: i32 = fishing.get("diff").expect("fishing.diff");
        let prob: i32 = fishing.get("prob").expect("fishing.prob");
        assert_eq!((diff, prob), (80, 50), "TestSkill (User,Fishing,80,50)");
        let stone: mlua::Table = formulas
            .get("destroyableStone")
            .expect("formulas.destroyableStone");
        assert_eq!(stone.get::<i32>("chance").expect("chance"), 40);
        assert_eq!(stone.get::<i32>("selfDamage").expect("selfDamage"), -50);
        let other: mlua::Table = formulas.get("otherActions").expect("formulas.otherActions");
        assert!(
            !other.get::<bool>("changeGold").expect("changeGold"),
            "772 has no coin-exchange on use"
        );
        let success: mlua::Function = formulas
            .get("fishingSuccess")
            .expect("formulas.fishingSuccess");
        let _: bool = success.call(10).expect("fishingSuccess(10)");

        inject_era_formulas(&runtime, &data_root, 1098).expect("1098 formulas");
        let formulas: mlua::Table = runtime.lua.globals().get("formulas").expect("formulas");
        let fishing: mlua::Table = formulas.get("fishing").expect("formulas.fishing");
        let model: String = fishing.get("model").expect("fishing.model");
        assert_eq!(model, "linear");
        let coeff: f64 = fishing.get("skillCoeff").expect("skillCoeff");
        assert!(
            (coeff - 0.597).abs() < 1e-9,
            "TFS fishing skillCoeff, got {coeff}"
        );
        let other: mlua::Table = formulas.get("otherActions").expect("formulas.otherActions");
        assert!(
            other.get::<bool>("changeGold").expect("changeGold"),
            "1098 TFS change_gold is enabled"
        );
    }

    /// Gap 7a regression guard: every `data/lib/core/*.lua` file must load
    /// without error once engine class globals are registered via
    /// `register_class`. Before Gap 7a, nine core lib files failed (e.g.
    /// `function Tile.relocateTo(...` raised "attempt to index global 'Tile'
    /// (a function value)"; `Party`/`Teleport`/`Vocation` were `nil`).
    /// Gap 5a makes `load_data_lib` fatal for the whole lib stage (including
    /// `scripts/lib` and top-level `scripts/*.lua`); this test remains the
    /// per-file 7a guard for `data/lib/core` itself, including `core.lua`
    /// under a workspace-root CWD.
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

    /// Gap 5a primary guard: Phase 2 (`load_data_lib`) must return `Ok` against
    /// the shipped data pack. Replaces the 10-name allowlist as the load-stage
    /// check; `required_data_globals_present_after_lib_load` stays as a cheap
    /// extra assertion on the tools contract.
    #[test]
    fn lib_stage_loads_with_zero_failures() {
        let data_root = workspace_data_root();
        if !data_root.join("lib/core").exists() {
            eprintln!("data/lib/core not present — skipping");
            return;
        }

        let runtime = LuaRuntime::new().expect("runtime init");
        inject_door_tables_from_global(&runtime, &data_root).expect("door tables");
        load_data_lib(&runtime, &data_root).expect("Phase 2 lib stage must return Ok (Gap 5a)");
    }

    /// Gap 5a policy: lib-stage exec errors are collected into one
    /// `LibStageFailures` (not warn-and-continue, not first-error abort), and
    /// `core.lua` / `lib.lua` dispatchers are skipped so a CWD-relative dofile
    /// cannot brick boot under the recursive scan.
    #[test]
    fn lib_stage_failures_are_fatal_and_aggregated() {
        let pack = TempDataPack::new();
        pack.write("lib/core/core.lua", "error('dispatcher should not run')");
        pack.write(
            "lib/core/lib.lua",
            "error('lib.lua dispatcher should not run')",
        );
        pack.write("lib/core/ok.lua", "-- fine");
        pack.write("lib/core/broken_a.lua", "error('boom-a')");
        pack.write("lib/core/broken_b.lua", "error('boom-b')");
        pack.write("scripts/lib/ok.lua", "-- fine");
        pack.write("scripts/scarab_ok.lua", "-- fine");

        let runtime = LuaRuntime::new().expect("runtime init");
        let err =
            load_data_lib(&runtime, &pack.0).expect_err("broken lib files must fail the lib stage");
        let display = format!("{err}");
        let LuaError::LibStageFailures(failures) = err else {
            panic!("expected LibStageFailures, got {err:?}");
        };

        let names: Vec<&str> = failures
            .iter()
            .filter_map(|(p, _)| Path::new(p).file_name()?.to_str())
            .collect();
        assert!(
            names.contains(&"broken_a.lua") && names.contains(&"broken_b.lua"),
            "aggregated failures must list both broken files, got {failures:?}"
        );
        assert_eq!(
            failures.len(),
            2,
            "dispatchers and ok files must not be reported, got {failures:?}"
        );
        assert!(
            !names.iter().any(|n| *n == "core.lua" || *n == "lib.lua"),
            "dofile dispatchers must be skipped, got {failures:?}"
        );

        assert!(
            display.contains("boom-a") && display.contains("boom-b"),
            "Display must list every error, got {display}"
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
    /// item ids, including `fishing_rod.lua` (`Action:allowFarUse`).
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

        let mut errors = Vec::new();
        for path in &lua_files {
            let path_string = path.display().to_string();
            if let Err(e) = runtime.load_action_script(&path_string) {
                errors.push((path_string, e.to_string()));
            }
        }

        let pending = runtime.drain_pending_actions();
        assert!(errors.is_empty(), "tool script load errors: {errors:?}");

        assert_eq!(
            pending.len(),
            9,
            "expected 9 tool actions, got {}: {:?}",
            pending.len(),
            pending.iter().map(|p| &p.item_ids).collect::<Vec<_>>()
        );

        // Verify key item ids are registered (machete registers two: 2420, 2442).
        let all_ids: Vec<u16> = pending
            .iter()
            .flat_map(|p| p.item_ids.iter().copied())
            .collect();
        for expected in [
            2416u16, 2580, 2342, 2566, 2420, 2442, 2553, 2120, 2550, 2554,
        ] {
            assert!(
                all_ids.contains(&expected),
                "expected item id {expected} in tool action registrations, got {all_ids:?}"
            );
        }

        let fishing = pending.iter().find(|p| p.item_ids.contains(&2580));
        assert!(
            fishing.is_some_and(|p| p.allow_far_use),
            "fishing rod (2580) must register with allowFarUse"
        );
    }

    /// E1: `FLUID_*` / `TALKTYPE_SAY` / `CONST_ME_SOUND_*` / `ITEM_*_COIN` /
    /// `CONDITION_PARAM_DRUNKENNESS` so `fluids.lua` and `change_gold.lua`
    /// load (they previously failed on nil globals at table-build time).
    #[test]
    fn e1_other_action_constants_unblock_fluids_and_change_gold_load() {
        let data_root = workspace_data_root();
        let other_dir = data_root.join("scripts/actions/other");
        if !other_dir.exists() {
            eprintln!("other actions dir not found — skipping");
            return;
        }

        let mut runtime = LuaRuntime::new().expect("runtime init");
        inject_door_tables_from_global(&runtime, &data_root).expect("door tables");
        load_data_lib(&runtime, &data_root).expect("data lib");
        inject_era_formulas(&runtime, &data_root, 772).expect("era formulas");

        let globals = runtime.lua.globals();
        let get = |name: &str| {
            globals
                .get::<i32>(name)
                .unwrap_or_else(|_| panic!("{name}"))
        };
        assert_eq!(get("FLUID_WATER"), 1);
        assert_eq!(get("TALKTYPE_SAY"), 1);
        assert_eq!(get("CONST_ME_SOUND_YELLOW"), 22);
        assert_eq!(get("ITEM_GOLD_COIN"), 2148);
        assert_eq!(get("CONDITION_PARAM_DRUNKENNESS"), 55);
        assert_eq!(get("TALKTYPE_MONSTER_SAY"), 0x11);

        let mut errors = Vec::new();
        for name in [
            "fluids.lua",
            "change_gold.lua",
            "music.lua",
            "birdcage.lua",
            "used_lamp.lua",
            "create_bread.lua",
        ] {
            let path = other_dir.join(name);
            let path_string = path.display().to_string();
            if let Err(e) = runtime.load_action_script(&path_string) {
                errors.push((path_string, e.to_string()));
            }
        }
        assert!(
            errors.is_empty(),
            "E1 must unblock other-action script load: {errors:?}"
        );
    }

    /// 772 has no coin-exchange on use. 1098 TFS `change_gold.lua` registers coins.
    #[test]
    fn change_gold_registers_only_when_formulas_enable_it() {
        let data_root = workspace_data_root();
        let path = data_root.join("scripts/actions/other/change_gold.lua");
        if !path.exists() {
            eprintln!("change_gold.lua not found — skipping");
            return;
        }
        let path_string = path.display().to_string();

        let mut runtime = LuaRuntime::new().expect("runtime");
        inject_era_formulas(&runtime, &data_root, 772).expect("772 formulas");
        runtime
            .load_action_script(&path_string)
            .expect("772 load change_gold");
        let pending = runtime.drain_pending_actions();
        assert!(
            !pending.iter().any(|p| p.item_ids.contains(&2148)
                || p.item_ids.contains(&2152)
                || p.item_ids.contains(&2160)),
            "772 must not register gold/platinum/crystal: {:?}",
            pending.iter().map(|p| &p.item_ids).collect::<Vec<_>>()
        );

        let mut runtime = LuaRuntime::new().expect("runtime");
        inject_era_formulas(&runtime, &data_root, 1098).expect("1098 formulas");
        runtime
            .load_action_script(&path_string)
            .expect("1098 load change_gold");
        let pending = runtime.drain_pending_actions();
        let ids: Vec<u16> = pending
            .iter()
            .flat_map(|p| p.item_ids.iter().copied())
            .collect();
        for expected in [2148u16, 2152, 2160] {
            assert!(
                ids.contains(&expected),
                "1098 change_gold must register {expected}, got {ids:?}"
            );
        }
    }

    /// E3: 772 `UseWeapon` (`moveuse.cc`) is `random(1,3)==1` then `Change` in place,
    /// not TFS `math.random(7)` + `Game.createItem` + `remove`.
    #[test]
    fn e3_destroy_item_uses_one_in_three_transform() {
        let src = std::fs::read_to_string(workspace_data_root().join("scripts/functions.lua"))
            .expect("functions.lua");
        let start = src
            .find("function destroyItem")
            .expect("destroyItem helper");
        let rest = &src[start..];
        let end = rest.find("\nfunction ").unwrap_or(rest.len());
        let body = &rest[..end];
        assert!(
            body.contains("math.random(1, 3)"),
            "772 UseWeapon random(1,3): {body}"
        );
        assert!(
            body.contains("target:transform(destroyId)"),
            "772 Change via transform: {body}"
        );
        assert!(
            !body.contains("math.random(7)"),
            "TFS 1/7 must not remain: {body}"
        );
        assert!(
            !body.contains("Game.createItem(destroyId"),
            "TFS create+remove must not remain: {body}"
        );
    }

    /// E2: no-target is TFS `pushThing(nullptr)` (`uid/itemid/actionid/type = 0`),
    /// not nil; `isHotkey` is the 6th boolean (`Action::executeUse` `callFunction(6)`).
    #[test]
    fn e2_no_target_is_zero_thing_table_and_is_hotkey_boolean() {
        let runtime = LuaRuntime::new().expect("runtime init");
        let probe: mlua::Function = runtime
            .lua
            .load(
                r#"
                function(player, item, fromPosition, target, toPosition, isHotkey)
                    return type(target) == "table"
                        and target.uid == 0
                        and target.itemid == 0
                        and target.actionid == 0
                        and target.type == 0
                        and type(isHotkey) == "boolean"
                        and isHotkey == true
                end
                "#,
            )
            .eval()
            .expect("probe");
        let key = runtime.lua.create_registry_value(probe).expect("registry");
        let ok = runtime
            .call_action_on_use(&key, 1, 1, (100, 100, 7), None, None, (100, 100, 7), true)
            .expect("call");
        assert!(ok, "zero-thing table + isHotkey true");

        let probe_false: mlua::Function = runtime
            .lua
            .load(
                r#"
                function(player, item, fromPosition, target, toPosition, isHotkey)
                    return target.itemid == 0 and isHotkey == false
                end
                "#,
            )
            .eval()
            .expect("probe false");
        let key_false = runtime
            .lua
            .create_registry_value(probe_false)
            .expect("registry");
        let ok = runtime
            .call_action_on_use(
                &key_false,
                1,
                1,
                (100, 100, 7),
                None,
                None,
                (100, 100, 7),
                false,
            )
            .expect("call false");
        assert!(
            ok,
            "isHotkey false + target.itemid field access must not error"
        );

        let probe_item: mlua::Function = runtime
            .lua
            .load(
                r#"
                function(player, item, fromPosition, target, toPosition, isHotkey)
                    return type(target) == "userdata" and type(isHotkey) == "boolean"
                end
                "#,
            )
            .eval()
            .expect("probe item");
        let key_item = runtime
            .lua
            .create_registry_value(probe_item)
            .expect("registry");
        let ok = runtime
            .call_action_on_use(
                &key_item,
                1,
                1,
                (100, 100, 7),
                Some(1),
                None,
                (100, 100, 7),
                false,
            )
            .expect("call item");
        assert!(ok, "item target stays userdata");
    }

    /// Gap 3: engine verbs used by tools scripts are registered on the shipped VM.
    #[test]
    fn gap3_tool_lua_verbs_are_registered() {
        use crate::context::CreatureRef;

        let runtime = LuaRuntime::new().expect("runtime init");
        let lua = &runtime.lua;
        let create: String = lua
            .load("return type(Game.createItem)")
            .eval()
            .expect("Game.createItem");
        assert_eq!(create, "function");
        let health: String = lua
            .load("return type(doTargetCombatHealth)")
            .eval()
            .expect("doTargetCombatHealth");
        assert_eq!(health, "function");
        let combat: String = lua
            .load("return type(doTargetCombat)")
            .eval()
            .expect("doTargetCombat");
        assert_eq!(combat, "function");

        let p = lua.create_userdata(CreatureRef(1)).expect("player");
        lua.globals().set("p", p).unwrap();
        let tries: String = lua
            .load("return type(p.addSkillTries)")
            .eval()
            .expect("addSkillTries");
        assert_eq!(tries, "function");
        let tile = lua
            .create_userdata(crate::userdata::tile::TileRef { x: 1, y: 1, z: 7 })
            .expect("tile");
        lua.globals().set("t", tile).unwrap();
        let add: String = lua.load("return type(t.addItem)").eval().expect("addItem");
        assert_eq!(add, "function");
        let add_ex: String = lua
            .load("return type(t.addItemEx)")
            .eval()
            .expect("addItemEx");
        assert_eq!(add_ex, "function");
        let create_tile: String = lua
            .load("return type(Game.createTile)")
            .eval()
            .expect("Game.createTile");
        assert_eq!(create_tile, "function");
        let bottom: String = lua
            .load("return type(t.getBottomCreature)")
            .eval()
            .expect("getBottomCreature");
        assert_eq!(bottom, "function");
    }

    /// Gap 1: `Action:allowFarUse(bool)` stores `_allow_far_use` and drains onto
    /// `PendingAction` / `ActionDef`. Default is `false` (C++ `Action` ctor).
    #[test]
    fn action_allow_far_use_drains_onto_pending() {
        let dir = std::env::temp_dir().join(format!(
            "tfs-gap1-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        struct Cleanup(PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
        let _cleanup = Cleanup(dir.clone());
        let path = dir.join("allow_far_use.lua");
        std::fs::write(
            &path,
            r#"
            local far = Action()
            function far.onUse() return true end
            far:id(2580)
            far:allowFarUse(true)
            far:register()

            local near = Action()
            function near.onUse() return true end
            near:id(2553)
            near:register()
            "#,
        )
        .expect("write probe");

        let mut runtime = LuaRuntime::new().expect("runtime init");
        runtime
            .load_action_script(&path.display().to_string())
            .expect("probe load");

        let pending = runtime.drain_pending_actions();
        assert_eq!(pending.len(), 2, "both actions must register");

        let far = pending.iter().find(|p| p.item_ids.contains(&2580)).unwrap();
        let near = pending.iter().find(|p| p.item_ids.contains(&2553)).unwrap();
        assert!(far.allow_far_use, "allowFarUse(true) must drain");
        assert!(!near.allow_far_use, "default allowFarUse is false");

        let defs: Vec<ActionDef> = pending.into_iter().map(Into::into).collect();
        assert!(
            defs.iter()
                .find(|d| d.item_ids.contains(&2580))
                .is_some_and(|d| d.allow_far_use)
        );
        assert!(
            defs.iter()
                .find(|d| d.item_ids.contains(&2553))
                .is_some_and(|d| !d.allow_far_use)
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
        let key: String = globals.get("ITEM_ATTRIBUTE_KEYNUMBER").expect("KEYNUMBER");
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
        let level: String = globals.get("ITEM_ATTRIBUTE_DOORLEVEL").expect("DOORLEVEL");
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
            .load(
                "function Tile.__gap7b_probe(self) return 'Tile' end return _probe:__gap7b_probe()",
            )
            .eval()
            .expect("Tile __index fallback");

        let pos = lua
            .create_userdata(PositionRef { x: 1, y: 2, z: 3 })
            .unwrap();
        globals.set("_probe", pos).unwrap();
        let _: String = lua
            .load("function Position.__gap7b_probe(self) return 'Position' end return _probe:__gap7b_probe()")
            .eval()
            .expect("Position __index fallback");

        let item = lua.create_userdata(ItemRef(1)).unwrap();
        globals.set("_probe", item).unwrap();
        let _: String = lua
            .load(
                "function Item.__gap7b_probe(self) return 'Item' end return _probe:__gap7b_probe()",
            )
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

    /// E7: construction kits drop floor/house messages; house → transform + poff.
    #[test]
    fn e7_construction_kits_house_poff_else_blockhit_no_text() {
        let src = std::fs::read_to_string(
            workspace_data_root().join("scripts/actions/other/construction_kits.lua"),
        )
        .expect("construction_kits.lua");
        assert!(src.contains("tile:getHouse()"), "E7 getHouse");
        assert!(src.contains("CONST_ME_POFF"), "house effect 3");
        assert!(src.contains("CONST_ME_BLOCKHIT"), "else effect 4");
        assert!(
            !src.contains("You may construct"),
            "772 has no house message"
        );
        assert!(
            !src.contains("Put the construction kit"),
            "772 has no floor message"
        );
    }

    /// E9: inject `getFormattedWorldTime` from `global.lua`; watch ids include cuckoo, drop sundial.
    #[test]
    fn e9_get_formatted_world_time_and_watch_ids() {
        let data_root = workspace_data_root();
        let runtime = LuaRuntime::new().expect("runtime init");
        inject_door_tables_from_global(&runtime, &data_root).expect("door tables");

        let lua = &runtime.lua;
        lua.globals()
            .set(
                "getWorldTime",
                lua.create_function(|_, ()| Ok(65i32)).expect("stub"),
            )
            .expect("override getWorldTime");
        let formatted: String = lua
            .load("return getFormattedWorldTime()")
            .eval()
            .expect("getFormattedWorldTime");
        assert_eq!(formatted, "1:05");

        lua.globals()
            .set(
                "getWorldTime",
                lua.create_function(|_, ()| Ok(9i32)).expect("stub"),
            )
            .expect("override getWorldTime");
        let padded: String = lua
            .load("return getFormattedWorldTime()")
            .eval()
            .expect("padded minutes");
        assert_eq!(padded, "0:09");

        let src = std::fs::read_to_string(data_root.join("scripts/actions/other/watch.lua"))
            .expect("watch.lua");
        assert!(src.contains("getFormattedWorldTime()"), "E9 time helper");
        assert!(src.contains("1877"), "cuckoo 1877");
        assert!(src.contains("1881"), "cuckoo 1881");
        assert!(!src.contains("3900"), "no sundial 3900");
    }

    /// R4: inject `getPlayerFlagValue` without loading `compat.lua`.
    #[test]
    fn r4_get_player_flag_value_uses_has_flag() {
        use crate::context::{CreatureRef, with_lua_context};
        use tfs_rust_common::{
            ScriptContext, ScriptCreatureData, ScriptCreatureId, ScriptItemId, ScriptItemRef,
        };

        const INFINITE_CAP: u64 = 1 << 20;

        struct FlagCtx;
        impl ScriptContext for FlagCtx {
            fn get_creature(&self, id: ScriptCreatureId) -> Option<ScriptCreatureData> {
                (id == 1).then_some(ScriptCreatureData {
                    name: "Quest".into(),
                    guid: 1,
                })
            }
            fn get_item(&self, _: ScriptItemId) -> Option<ScriptItemRef> {
                None
            }
            fn get_config_string(&self, _: &str) -> Option<String> {
                None
            }
            fn get_player_level(&self, id: ScriptCreatureId) -> Option<i32> {
                (id == 1).then_some(8)
            }
            fn player_has_flag(&self, id: ScriptCreatureId, flag: u64) -> bool {
                id == 1 && flag == INFINITE_CAP
            }
        }

        let data_root = workspace_data_root();
        let runtime = LuaRuntime::new().expect("runtime init");
        inject_door_tables_from_global(&runtime, &data_root).expect("door tables");
        let lua = &runtime.lua;
        let p = lua.create_userdata(CreatureRef(1)).expect("player");
        lua.globals().set("p", p).unwrap();
        with_lua_context(&FlagCtx, || {
            let via_ud: bool = lua
                .load("return getPlayerFlagValue(p, PlayerFlag_HasInfiniteCapacity)")
                .eval()
                .expect("flag via userdata");
            assert!(
                via_ud,
                "Player(userdata) must resolve for the compat wrapper"
            );
            let missing: bool = lua
                .load("return getPlayerFlagValue(p, 1)")
                .eval()
                .expect("unset flag");
            assert!(!missing);
            let via_id: bool = lua
                .load("return getPlayerFlagValue(1, PlayerFlag_HasInfiniteCapacity)")
                .eval()
                .expect("flag via id");
            assert!(via_id);
        });
    }

    /// E8: fluids.lua drink numbers/messages; engine stack is `addCondition` only.
    #[test]
    fn e8_fluids_drink_numbers_and_messages() {
        let src =
            std::fs::read_to_string(workspace_data_root().join("scripts/actions/other/fluids.lua"))
                .expect("fluids.lua");
        assert!(src.contains("math.random(50, 150)"), "772 mana 50..=150");
        assert!(src.contains("math.random(25, 75)"), "772 life 25..=75");
        assert!(src.contains("\"Mmmh.\""), "lemonade");
        assert!(src.contains("\"Gulp.\""), "milk/default");
        assert!(src.contains("\"Aah...\""), "beer/wine");
        assert!(
            src.contains("player:addCondition(drunk)"),
            "E8 stack via addCondition"
        );
        assert!(!src.contains("setEarliestSpellTime"), "no potion exhaust");
        assert!(!src.contains("CONST_ME_MAGIC_BLUE"), "no magic-blue");
        assert!(
            !src.contains("CONDITION_PARAM_DRUNKENNESS"),
            "E8 ignores drunkenness param"
        );
        assert!(!src.contains("queryAdd"), "no TFS queryAdd");
        assert!(src.contains("Game.createItem(2016"), "spill 2016");
    }

    /// E6: `GetSpellbook` format — vocation/learned Light Healing, Berserk `4*Level`, no ML groups.
    #[test]
    fn e6_spellbook_learned_filter_and_getspellbook_format() {
        use tfs_rust_common::{
            ScriptContext, ScriptCreatureData, ScriptCreatureId, ScriptInstantSpell,
            ScriptItemData, ScriptItemId, ScriptItemRef,
        };

        struct SpellbookCtx;
        impl ScriptContext for SpellbookCtx {
            fn get_creature(&self, id: ScriptCreatureId) -> Option<ScriptCreatureData> {
                (id == 1).then_some(ScriptCreatureData {
                    name: "Mage".into(),
                    guid: 1,
                })
            }
            fn get_item(&self, id: ScriptItemId) -> Option<ScriptItemRef> {
                Some(ScriptItemRef(id))
            }
            fn get_config_string(&self, _: &str) -> Option<String> {
                None
            }
            fn get_item_data(&self, id: ScriptItemId) -> Option<ScriptItemData> {
                (id == 1).then_some(ScriptItemData {
                    item_type: 2175,
                    count: 1,
                    weight: 0,
                    name: "spellbook".into(),
                    action_id: 0,
                    unique_id: 0,
                    is_store_item: false,
                    fluid_type: 0,
                })
            }
            fn player_has_learned_spell(&self, id: ScriptCreatureId, name: &str) -> bool {
                id == 1
                    && (name.eq_ignore_ascii_case("Light Healing")
                        || name.eq_ignore_ascii_case("Berserk"))
            }
            fn list_instant_spells(&self) -> Vec<ScriptInstantSpell> {
                vec![
                    ScriptInstantSpell {
                        name: "Light Healing".into(),
                        words: "ex,ura".into(),
                        level: 9,
                        magic_level: 0,
                        mana: 25,
                        mana_percent: 0,
                    },
                    ScriptInstantSpell {
                        name: "Berserk".into(),
                        words: "ex,ori".into(),
                        level: 35,
                        magic_level: 0,
                        mana: 0,
                        mana_percent: 80,
                    },
                    ScriptInstantSpell {
                        name: "Intense Healing".into(),
                        words: "ex,ura, gran".into(),
                        level: 11,
                        magic_level: 0,
                        mana: 40,
                        mana_percent: 0,
                    },
                    ScriptInstantSpell {
                        name: "Invite Guests".into(),
                        words: "aleta sio".into(),
                        level: 0,
                        magic_level: 0,
                        mana: 0,
                        mana_percent: 0,
                    },
                ]
            }
            fn list_player_instant_spells(&self, id: ScriptCreatureId) -> Vec<ScriptInstantSpell> {
                if id != 1 {
                    return Vec::new();
                }
                self.list_instant_spells()
                    .into_iter()
                    .filter(|s| {
                        s.name.eq_ignore_ascii_case("Light Healing")
                            || s.name.eq_ignore_ascii_case("Berserk")
                    })
                    .collect()
            }
        }

        CAPTURED_TEXT_DIALOG.with(|c| *c.borrow_mut() = None);
        crate::lua_mutation::register_lua_mutation_applier(capture_text_dialog_applier);

        let path = workspace_data_root().join("scripts/actions/other/spellbook.lua");
        if !path.exists() {
            eprintln!("spellbook.lua not found — skipping");
            return;
        }

        let mut runtime = LuaRuntime::new().expect("runtime");
        runtime
            .load_action_script(&path.display().to_string())
            .expect("load spellbook");
        let pending = runtime.drain_pending_actions();
        let action = pending
            .iter()
            .find(|p| p.item_ids.contains(&2175))
            .expect("2175 registered");
        assert!(
            !pending.iter().any(|p| p.item_ids.contains(&2217)),
            "2217 is not GetSpellbook"
        );
        let on_use = action.on_use.as_ref().expect("onUse");

        crate::lua_mutation::with_lua_mutation_scope(std::ptr::without_provenance_mut(1), || {
            crate::context::with_lua_context(&SpellbookCtx, || {
                runtime
                    .call_action_on_use(
                        on_use,
                        1,
                        1,
                        (100, 100, 7),
                        None,
                        None,
                        (100, 100, 7),
                        false,
                    )
                    .expect("onUse");
            });
        });

        let (item_type, text) = CAPTURED_TEXT_DIALOG
            .with(|c| c.borrow().clone())
            .expect("showTextDialog");
        assert_eq!(item_type, 2175);
        assert!(
            text.contains("Spells for Level 9\n  exura - Light Healing: 25\n"),
            "Light Healing line: {text:?}"
        );
        assert!(
            text.contains("Spells for Level 35\n  exori - Berserk: 4*Level\n"),
            "Berserk 4*Level: {text:?}"
        );
        assert!(
            !text.contains("Intense Healing"),
            "unlearned absent: {text:?}"
        );
        assert!(!text.contains("Invite Guests"), "level-0 skipped: {text:?}");
        assert!(
            !text.contains("Spells for Magic Level"),
            "no ML groups: {text:?}"
        );
        assert!(
            !text.contains("80%"),
            "Berserk is not manapercent: {text:?}"
        );
    }
}
