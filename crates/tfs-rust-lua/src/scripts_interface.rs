//! Scripts-interface loader: allowlisted EventCallback, CreatureEvent, and
//! GlobalEvent revscripts.
//!
//! Pack surface: TFS `LuaScriptInterface::loadScripts(..., isScriptsInterface)`.
//! `EventCallback:register` / `__newindex` in `data/scripts/lib/event_callbacks.lua`
//! require `isScriptsInterface()`. `CreatureEvent:register` is *not* gated — the
//! allowlist is the only control for which creaturescripts execute.
//!
//! Reload stance (a): each pass clears `EventCallback` data and replaces the
//! pending CreatureEvent / GlobalEvent tables. No `/reload` talkaction yet.

use std::path::{Path, PathBuf};

use crate::combat_scripts::collect_lua_files;
use crate::runtime::{LuaError, LuaRuntime};

/// Relative to `data/scripts/`. Fail closed: unlisted files are skipped.
const SCRIPTS_INTERFACE_ALLOWLIST: &[&str] = &[
    "creaturescripts/login.lua",
    "creaturescripts/firstlogin.lua",
    "creaturescripts/playerdeath.lua",
    "creaturescripts/extendedopcode.lua",
    "creaturescripts/killstatistics.lua",
    "eventcallbacks/player/default_onReportBug.lua",
    "eventcallbacks/player/moveitem.lua",
    "eventcallbacks/monster/rarity.lua", // Phase 4; missing file is not an error
    "globalevents/record.lua",
];

/// Load allowlisted `data/scripts/eventcallbacks/**`,
/// `data/scripts/globalevents/**`, and `data/scripts/creaturescripts/*.lua`
/// with `isScriptsInterface` true.
///
/// Call after [`crate::actions::load_data_lib`] so `EventCallback` is the real
/// table, not the bootstrap stub. Does not scan `data/creaturescripts/**`.
///
/// Allowlisted exec failures are aggregated (boot-blocking). Unlisted files
/// are skipped with a warning. Missing allowlisted files (`rarity.lua`) are
/// silent. The scripts-interface guard resets on `Drop` even if this returns
/// `Err`.
pub fn load_scripts_interface(runtime: &LuaRuntime, data_dir: &Path) -> Result<(), LuaError> {
    let _guard = runtime.enter_scripts_interface();
    runtime.reset_pending_script_event_tables()?;
    // `EventCallback:clear` is not gated on `isScriptsInterface`.
    runtime.exec_chunk("scripts_interface_clear", "EventCallback:clear()")?;

    let scripts_root = data_dir.join("scripts");
    let mut files: Vec<PathBuf> = Vec::new();

    let eventcallbacks_dir = scripts_root.join("eventcallbacks");
    if eventcallbacks_dir.exists() {
        collect_lua_files(&eventcallbacks_dir, &mut files);
    }

    let globalevents_dir = scripts_root.join("globalevents");
    if globalevents_dir.exists() {
        collect_lua_files(&globalevents_dir, &mut files);
    }

    let creaturescripts_dir = scripts_root.join("creaturescripts");
    if creaturescripts_dir.exists() {
        match std::fs::read_dir(&creaturescripts_dir) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().is_some_and(|ext| ext == "lua") {
                        files.push(path);
                    }
                }
            }
            Err(e) => {
                return Err(LuaError::ScriptIo(
                    creaturescripts_dir.display().to_string(),
                    e.to_string(),
                ));
            }
        }
    }

    files.sort();

    let mut failures: Vec<(String, String)> = Vec::new();
    for path in &files {
        let Some(rel) = relative_scripts_path(path, &scripts_root) else {
            tracing::warn!(
                file = %path.display(),
                "scripts-interface: skipped (path not under data/scripts)"
            );
            continue;
        };
        if !SCRIPTS_INTERFACE_ALLOWLIST
            .iter()
            .any(|allowed| *allowed == rel)
        {
            tracing::warn!(file = %rel, "scripts-interface: skipped (not on allowlist)");
            continue;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("scripts_interface");
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

    runtime.install_pending_creature_events()?;
    runtime.install_pending_global_events()?;
    runtime.sync_event_callbacks_from_lua()?;

    if failures.is_empty() {
        runtime.warn_undispatched_event_callbacks();
        Ok(())
    } else {
        Err(LuaError::LibStageFailures(failures))
    }
}

fn relative_scripts_path(path: &Path, scripts_root: &Path) -> Option<String> {
    path.strip_prefix(scripts_root).ok().map(|rel| {
        rel.components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::load_data_lib;
    use std::path::PathBuf;

    /// Lua `EVENT_CALLBACK_ONLOOK` (`event_callbacks.lua`).
    const EVENT_CALLBACK_ONLOOK: i32 = 9;
    const EVENT_CALLBACK_ONMOVEITEM: i32 = 16;
    const EVENT_CALLBACK_ONITEMMOVED: i32 = 17;
    const EVENT_CALLBACK_ONREPORTBUG: i32 = 19;
    const EVENT_CALLBACK_ONDROPLOOT: i32 = 24;
    const EVENT_CALLBACK_ONSPAWN: i32 = 25;

    fn workspace_data_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
    }

    struct TempDataPack(PathBuf);

    impl TempDataPack {
        fn new() -> Self {
            let dir = std::env::temp_dir().join(format!(
                "tfs-scripts-iface-{}-{}",
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

        fn install_event_callbacks(&self) {
            let src = workspace_data_root().join("scripts/lib/event_callbacks.lua");
            let dst = self.0.join("scripts/lib/event_callbacks.lua");
            std::fs::copy(&src, &dst).expect("copy event_callbacks.lua");
        }
    }

    impl Drop for TempDataPack {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn has_event_callback(runtime: &LuaRuntime, ty: i32) -> bool {
        runtime.has_event_callback(ty)
    }

    fn eval_bool(runtime: &LuaRuntime, chunk: &str) -> bool {
        runtime.lua.load(chunk).eval().expect("eval bool")
    }

    /// After lib load only: spawn bus is empty, `hasEventCallback` does not error.
    #[test]
    fn has_event_callback_false_after_lib_load_only() {
        let data_root = workspace_data_root();
        if !data_root.join("scripts/lib/event_callbacks.lua").exists() {
            eprintln!("data pack not present — skipping");
            return;
        }

        let runtime = LuaRuntime::new().expect("runtime init");
        load_data_lib(&runtime, &data_root).expect("data lib");
        assert!(
            !has_event_callback(&runtime, EVENT_CALLBACK_ONSPAWN),
            "no onSpawn registration before scripts-interface scan"
        );
    }

    /// Missing `scripts/eventcallbacks/` is not an error; bus stays empty after the scan.
    #[test]
    fn missing_eventcallbacks_dir_is_ok() {
        if !workspace_data_root()
            .join("scripts/lib/event_callbacks.lua")
            .exists()
        {
            eprintln!("data pack not present — skipping");
            return;
        }

        let pack = TempDataPack::new();
        pack.install_event_callbacks();
        assert!(
            !pack.0.join("scripts/eventcallbacks").exists(),
            "fixture must not create scripts/eventcallbacks/"
        );

        let runtime = LuaRuntime::new().expect("runtime init");
        load_data_lib(&runtime, &pack.0).expect("data lib");
        load_scripts_interface(&runtime, &pack.0).expect("scripts interface");
        assert!(
            !has_event_callback(&runtime, EVENT_CALLBACK_ONSPAWN),
            "ONSPAWN must stay unregistered without eventcallbacks/"
        );
        assert!(
            eval_bool(&runtime, "return isScriptsInterface() == false"),
            "isScriptsInterface must be false after the scan"
        );
    }

    /// Allowlisted one-liner registers `onSpawn`.
    #[test]
    fn allowlisted_callback_registers() {
        let pack = TempDataPack::new();
        pack.install_event_callbacks();
        pack.write(
            "scripts/eventcallbacks/monster/rarity.lua",
            "local ec = EventCallback\n\
             ec.onSpawn = function() return true end\n\
             ec:register()\n",
        );

        let runtime = LuaRuntime::new().expect("runtime init");
        load_data_lib(&runtime, &pack.0).expect("data lib");
        load_scripts_interface(&runtime, &pack.0).expect("scripts interface");
        assert!(
            has_event_callback(&runtime, EVENT_CALLBACK_ONSPAWN),
            "allowlisted rarity.lua must register onSpawn"
        );
    }

    /// Unlisted file is not executed (fail closed).
    #[test]
    fn unlisted_file_is_skipped() {
        let pack = TempDataPack::new();
        pack.install_event_callbacks();
        pack.write(
            "scripts/eventcallbacks/player/not_in_manifest.lua",
            "UNLISTED_LOADED = true\n\
             local ec = EventCallback\n\
             ec.onLook = function() return true end\n\
             ec:register()\n",
        );

        let runtime = LuaRuntime::new().expect("runtime init");
        load_data_lib(&runtime, &pack.0).expect("data lib");
        load_scripts_interface(&runtime, &pack.0).expect("scripts interface");
        assert!(
            !eval_bool(&runtime, "return UNLISTED_LOADED == true"),
            "unlisted file must not execute"
        );
        assert!(
            !has_event_callback(&runtime, EVENT_CALLBACK_ONLOOK),
            "unlisted onLook must not register"
        );
    }

    /// Unlisted file under `scripts/globalevents/` is not executed.
    #[test]
    fn unlisted_globalevent_is_skipped() {
        let pack = TempDataPack::new();
        pack.install_event_callbacks();
        pack.write(
            "scripts/globalevents/not_on_allowlist.lua",
            "NOPE_LOADED = true\n\
             local e = GlobalEvent(\"Nope\")\n\
             function e.onRecord(c, o) return true end\n\
             e:type(\"record\")\n\
             e:register()\n",
        );

        let runtime = LuaRuntime::new().expect("runtime init");
        load_data_lib(&runtime, &pack.0).expect("data lib");
        load_scripts_interface(&runtime, &pack.0).expect("scripts interface");
        assert!(
            !eval_bool(&runtime, "return NOPE_LOADED == true"),
            "unlisted globalevents file must not execute"
        );
        assert!(
            !runtime.has_global_event("Nope"),
            "unlisted GlobalEvent must not drain"
        );
    }

    /// After the scan, `ec:register()` is a no-op (flag reset), not a nil call.
    #[test]
    fn register_after_scan_is_noop() {
        let pack = TempDataPack::new();
        pack.install_event_callbacks();

        let runtime = LuaRuntime::new().expect("runtime init");
        load_data_lib(&runtime, &pack.0).expect("data lib");
        load_scripts_interface(&runtime, &pack.0).expect("scripts interface");

        runtime
            .exec_chunk(
                "post_scan_register",
                "local ec = EventCallback\n\
                 ec.onSpawn = function() return true end\n\
                 ec:register()\n",
            )
            .expect("post-scan register must not error");
        assert!(
            !has_event_callback(&runtime, EVENT_CALLBACK_ONSPAWN),
            "register outside the scan must be a no-op"
        );
        assert!(
            eval_bool(&runtime, "return isScriptsInterface() == false"),
            "isScriptsInterface must be false after the scan"
        );
    }

    /// Shipped pack: report-bug loads; drop-loot and spawn do not.
    #[test]
    fn real_pack_allowlist() {
        let data_root = workspace_data_root();
        if !data_root
            .join("scripts/eventcallbacks/player/default_onReportBug.lua")
            .exists()
        {
            eprintln!("data pack not present — skipping");
            return;
        }

        let runtime = LuaRuntime::new().expect("runtime init");
        load_data_lib(&runtime, &data_root).expect("data lib");
        load_scripts_interface(&runtime, &data_root).expect("scripts interface");
        assert!(
            has_event_callback(&runtime, EVENT_CALLBACK_ONREPORTBUG),
            "default_onReportBug.lua is allowlisted"
        );
        assert!(
            !has_event_callback(&runtime, EVENT_CALLBACK_ONMOVEITEM),
            "moveitem policy is native — moveitem.lua must not register"
        );
        assert!(
            !has_event_callback(&runtime, EVENT_CALLBACK_ONITEMMOVED),
            "onItemMoved policy is native"
        );
        assert!(
            runtime.undispatched_event_callbacks().is_empty(),
            "allowlisted callbacks must have call sites: {:?}",
            runtime.undispatched_event_callbacks()
        );
        assert!(
            !has_event_callback(&runtime, EVENT_CALLBACK_ONSPAWN),
            "rarity.lua is not shipped"
        );
        assert!(
            !has_event_callback(&runtime, EVENT_CALLBACK_ONDROPLOOT),
            "default_onDropLoot.lua must not load"
        );
        assert!(
            runtime.has_creature_event("PlayerLogin"),
            "allowlisted login.lua must drain CreatureEvent PlayerLogin"
        );
        assert!(
            runtime.has_global_event("PlayerRecord"),
            "allowlisted record.lua must drain GlobalEvent PlayerRecord"
        );
        assert!(
            !runtime.has_creature_event("DropLoot"),
            "DropLoot must not be in the CreatureEvent registry"
        );
    }

    /// Second `load_data_lib` must not wipe registrations (`load_spell_scripts` path).
    #[test]
    fn second_load_data_lib_does_not_clear_callbacks() {
        let data_root = workspace_data_root();
        if !data_root
            .join("scripts/eventcallbacks/player/default_onReportBug.lua")
            .exists()
        {
            eprintln!("data pack not present — skipping");
            return;
        }

        let runtime = LuaRuntime::new().expect("runtime init");
        load_data_lib(&runtime, &data_root).expect("data lib");
        load_scripts_interface(&runtime, &data_root).expect("scripts interface");
        assert!(has_event_callback(&runtime, EVENT_CALLBACK_ONREPORTBUG));
        load_data_lib(&runtime, &data_root).expect("second lib load is a no-op");
        assert!(
            has_event_callback(&runtime, EVENT_CALLBACK_ONREPORTBUG),
            "idempotent load_data_lib must not EventCallback:clear()"
        );
    }

    /// Guard resets the flag even when an allowlisted file fails to exec.
    #[test]
    fn guard_resets_on_exec_error() {
        let pack = TempDataPack::new();
        pack.install_event_callbacks();
        pack.write(
            "scripts/eventcallbacks/monster/rarity.lua",
            "this is not valid lua {",
        );

        let runtime = LuaRuntime::new().expect("runtime init");
        load_data_lib(&runtime, &pack.0).expect("data lib");
        let err = load_scripts_interface(&runtime, &pack.0).expect_err("broken allowlisted file");
        match err {
            LuaError::LibStageFailures(failures) => {
                assert!(!failures.is_empty(), "must name the broken file");
            }
            other => panic!("expected LibStageFailures, got {other}"),
        }
        assert!(
            eval_bool(&runtime, "return isScriptsInterface() == false"),
            "guard must reset after Err"
        );
    }

    const FORBIDDEN_GENERATORS: &[&str] = &[
        "scripts/creaturescripts/droploot.lua",
        "scripts/creaturescripts/regeneratestamina.lua",
        "scripts/eventcallbacks/monster/default_onDropLoot.lua",
        "scripts/eventcallbacks/player/default_onMoveItem.lua",
        "scripts/eventcallbacks/player/default_onLook.lua",
        "scripts/eventcallbacks/player/default_onLookInBattleList.lua",
    ];

    /// Phase 6.4: shipped pack must not contain death-generator / look-double files.
    #[test]
    fn real_pack_has_no_forbidden_generators() {
        let data_root = workspace_data_root();
        for rel in FORBIDDEN_GENERATORS {
            let path = data_root.join(rel);
            assert!(
                !path.exists(),
                "forbidden generator still on disk: {}",
                path.display()
            );
        }
        assert!(
            !data_root
                .join("scripts/creaturescripts/logout.lua")
                .exists(),
            "logout.lua must be deleted after stamina strip"
        );
    }

    /// Content-data XML prefixes. Script-registry XML is forbidden (Phase 7 removed globalevents.xml).
    const CONTENT_XML_PREFIXES: &[&str] = &[
        "items/",
        "XML/",
        "raids/",
        "world/",
        "monster/monsters/",
        "npc/archive/",
    ];

    fn collect_xml_rels(dir: &Path, rel_prefix: &str, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            let rel = if rel_prefix.is_empty() {
                name.clone()
            } else {
                format!("{rel_prefix}/{name}")
            };
            if path.is_dir() {
                collect_xml_rels(&path, &rel, out);
            } else if path.extension().is_some_and(|ext| ext == "xml") {
                out.push(rel);
            }
        }
    }

    /// Phase 8.8c: no script-registry XML (Phase 7 removed globalevents.xml).
    #[test]
    fn no_script_registry_xml_outside_content_allowlist() {
        let data_root = workspace_data_root();
        if !data_root.is_dir() {
            eprintln!("data pack not present — skipping");
            return;
        }
        let mut xmls = Vec::new();
        collect_xml_rels(&data_root, "", &mut xmls);
        xmls.sort();
        let mut unexpected = Vec::new();
        for rel in xmls {
            if !CONTENT_XML_PREFIXES.iter().any(|p| rel.starts_with(p)) {
                unexpected.push(rel);
            }
        }
        assert!(
            unexpected.is_empty(),
            "script-registry XML outside content allowlist: {unexpected:?}"
        );
    }
}
