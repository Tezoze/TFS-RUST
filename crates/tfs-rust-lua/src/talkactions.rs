//! Talkaction registry and Lua loader.
//!
//! C++ reference: `src/talkaction.cpp` `TalkActions::load` — talkaction
//! registration from XML (adapted to self-registering Lua convention, mirroring
//! the `Channel` / `Action` pattern).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::runtime::{LuaError, LuaRuntime, PendingTalkAction};

/// Talkaction definition loaded from Lua scripts.
///
/// C++ reference: `talkaction.h` `TalkAction` — words, separator, onSay callback.
#[derive(Debug)]
pub struct TalkActionDef {
    pub words: String,
    pub separator: String,
    pub on_say: Option<Arc<mlua::RegistryKey>>,
}

impl From<PendingTalkAction> for TalkActionDef {
    fn from(pending: PendingTalkAction) -> Self {
        Self {
            words: pending.words,
            separator: pending.separator,
            on_say: pending.on_say.map(Arc::new),
        }
    }
}

/// Load all talkaction scripts from `data/scripts/talkactions/<subdir>/*.lua`.
///
/// No manifest file — every `.lua` directly under the given subdirectory
/// self-registers via `TalkAction(words):register()`. This mirrors the
/// `data/scripts/chatchannels` and `data/scripts/actions` convention.
///
/// C++ reference: `talkaction.cpp` `TalkActions::load` — talkaction
/// registration (adapted from XML to self-registering Lua).
pub fn load_talkaction_scripts(
    runtime: &mut LuaRuntime,
    data_dir: &Path,
    subdir: &str,
) -> Result<Vec<TalkActionDef>, LuaError> {
    let dir = data_dir.join("scripts/talkactions").join(subdir);
    if !dir.exists() {
        tracing::warn!("Talkactions directory not found: {}", dir.display());
        return Ok(Vec::new());
    }

    let entries = std::fs::read_dir(&dir)
        .map_err(|e| LuaError::ScriptIo(dir.display().to_string(), e.to_string()))?;

    for entry in entries {
        let entry =
            entry.map_err(|e| LuaError::ScriptIo(dir.display().to_string(), e.to_string()))?;
        let path = entry.path();

        if path.extension().is_some_and(|e| e == "lua") {
            let path_string = path.display().to_string();
            if let Err(e) = runtime.load_talkaction_script(&path_string) {
                tracing::warn!("Failed to load talkaction script {}: {e}", path.display());
            }
        }
    }

    // Convert pending talkactions to talkaction definitions
    let pending = runtime.drain_pending_talkactions();
    let talkactions: Vec<TalkActionDef> = pending.into_iter().map(Into::into).collect();

    tracing::info!("Loaded {} talkactions from {}", talkactions.len(), subdir);
    Ok(talkactions)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace_data_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data")
    }

    /// CH-6 smoke test: `data/scripts/talkactions/god/create_item.lua` loads
    /// without errors and registers the `/i` talkaction via the
    /// `TalkAction("/i"):register()` self-registering API.
    #[test]
    fn create_item_talkaction_loads() {
        let data_root = workspace_data_root();
        let god_dir = data_root.join("scripts/talkactions/god");
        if !god_dir.exists() {
            eprintln!("god/ talkactions directory not found — skipping");
            return;
        }

        let mut runtime = LuaRuntime::new().expect("runtime init");
        let defs = load_talkaction_scripts(&mut runtime, &data_root, "god")
            .expect("talkactions should load");

        assert!(
            !defs.is_empty(),
            "expected at least one talkaction from god/"
        );

        let create_item = defs.iter().find(|d| d.words == "/i");
        assert!(
            create_item.is_some(),
            "expected /i talkaction from create_item.lua"
        );
        let ci = create_item.unwrap();
        assert_eq!(ci.separator, " ");
        assert!(ci.on_say.is_some(), "/i must have an onSay callback");
    }
}
