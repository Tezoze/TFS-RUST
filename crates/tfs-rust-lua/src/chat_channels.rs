//! Chat channel registry and Lua loader.
//!
//! C++ reference: `src/chat.cpp` `Chat::load` — channel registration from XML (adapted to
//! self-registering Lua convention).

use std::path::Path;

use crate::runtime::{LuaError, LuaRuntime, PendingChatChannel};

/// Chat channel definition loaded from Lua scripts.
///
/// C++ reference: `chat.h` `ChatChannel` — channel identity + hook storage.
#[derive(Debug)]
pub struct ChatChannelDef {
    pub id: u16,
    pub name: String,
    pub public: bool,
    pub on_speak: Option<mlua::RegistryKey>,
    pub can_join: Option<mlua::RegistryKey>,
    pub on_join: Option<mlua::RegistryKey>,
    pub on_leave: Option<mlua::RegistryKey>,
}

impl From<PendingChatChannel> for ChatChannelDef {
    fn from(pending: PendingChatChannel) -> Self {
        Self {
            id: pending.id,
            name: pending.name,
            public: pending.public,
            on_speak: pending.on_speak,
            can_join: pending.can_join,
            on_join: pending.on_join,
            on_leave: pending.on_leave,
        }
    }
}

/// Load all chat channel scripts from `data/scripts/chatchannels/*.lua`.
///
/// No manifest file — every `.lua` directly under `data/scripts/chatchannels/` self-registers
/// via `Channel(id, name):register()`. This mirrors the `data/scripts/actions` and
/// `data/scripts/talkactions` convention.
///
/// C++ reference: `chat.cpp` `Chat::load` — channel registration (adapted from XML to
/// self-registering Lua).
pub fn load_chat_channel_scripts(
    runtime: &mut LuaRuntime,
    data_dir: &Path,
) -> Result<Vec<ChatChannelDef>, LuaError> {
    let dir = data_dir.join("scripts/chatchannels");
    if !dir.exists() {
        tracing::warn!("Chat channels directory not found: {}", dir.display());
        return Ok(Vec::new());
    }

    let entries = std::fs::read_dir(&dir)
        .map_err(|e| LuaError::ScriptIo(dir.display().to_string(), e.to_string()))?;

    for entry in entries {
        let entry = entry.map_err(|e| LuaError::ScriptIo(dir.display().to_string(), e.to_string()))?;
        let path = entry.path();

        // Skip non-Lua files and ruleviolations.lua (RVR non-goal per chat-system-plan.md §1)
        if path.extension().is_some_and(|e| e == "lua") {
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if file_name == "ruleviolations.lua" {
                tracing::info!("Skipping ruleviolations.lua (RVR non-goal)");
                continue;
            }

            let path_string = path.display().to_string();
            if let Err(e) = runtime.load_channel_script(&path_string) {
                tracing::warn!("Failed to load channel script {}: {e}", path.display());
            }
        }
    }

    // Convert pending channels to channel definitions
    let pending = runtime.drain_pending_chat_channels();
    let channels: Vec<ChatChannelDef> = pending.into_iter().map(Into::into).collect();

    tracing::info!("Loaded {} chat channels", channels.len());
    Ok(channels)
}
