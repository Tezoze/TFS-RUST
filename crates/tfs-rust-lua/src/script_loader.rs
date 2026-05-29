//! XML-based script loading for creaturescripts.

use std::collections::HashMap;
use std::path::Path;

use crate::runtime::{CallbackRef, LuaRuntime, LuaError};

/// Creature event types from XML.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CreatureEventType {
    Login,
    Logout,
    Death,
    // Others deferred to Track 2
}

/// Script loading errors.
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("XML parse error: {0}")]
    XmlParse(#[from] quick_xml::de::DeError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Lua error: {0}")]
    Lua(#[from] LuaError),

    #[error("Script file not found: {0}")]
    ScriptNotFound(String),
}

/// XML-based script loader.
pub struct ScriptLoader<'a> {
    runtime: &'a mut LuaRuntime,
}

impl<'a> ScriptLoader<'a> {
    pub fn new(runtime: &'a mut LuaRuntime) -> Self {
        Self { runtime }
    }

    /// Load creaturescripts from XML.
    ///
    /// Parses `data/creaturescripts/creaturescripts.xml` and loads all
    /// referenced script files.
    pub fn load_creaturescripts(
        &mut self,
        data_dir: &Path,
    ) -> Result<HashMap<CreatureEventType, Vec<CallbackRef>>, LoadError> {
        let xml_path = data_dir.join("creaturescripts/creaturescripts.xml");

        if !xml_path.exists() {
            tracing::warn!("Creaturescripts XML not found: {}", xml_path.display());
            return Ok(HashMap::new());
        }

        let xml_content = std::fs::read_to_string(&xml_path)?;
        let events: CreaturescriptsXml = quick_xml::de::from_str(&xml_content)?;

        let mut result: HashMap<CreatureEventType, Vec<CallbackRef>> = HashMap::new();

        for event in events.events {
            let event_type = match event.event_type.as_str() {
                "login" => CreatureEventType::Login,
                "logout" => CreatureEventType::Logout,
                "death" => CreatureEventType::Death,
                _ => {
                    tracing::warn!("Unknown event type: {}", event.event_type);
                    continue;
                }
            };

            if !matches!(event_type, CreatureEventType::Login | CreatureEventType::Logout) {
                continue;
            }

            // Load lib file first (warn if missing)
            let lib_path = data_dir.join("creaturescripts/lib/creaturescripts.lua");
            if lib_path.exists() {
                let lib_path_string = lib_path.display().to_string();
                if let Err(e) = self.runtime.load_script(&lib_path_string) {
                    tracing::warn!("Failed to load creaturescripts lib {}: {}", lib_path.display(), e);
                }
            } else {
                tracing::warn!("Creaturescripts lib not found: {}", lib_path.display());
            }

            // Load script file (warn on failure, continue)
            let script_path = data_dir.join("creaturescripts/scripts/").join(&event.script);
            if !script_path.exists() {
                tracing::warn!("Script file not found: {}", script_path.display());
                continue;
            }

            let script_path_string = script_path.display().to_string();
            if let Err(e) = self.runtime.load_script(&script_path_string) {
                tracing::warn!("Failed to load script {}: {}", script_path.display(), e);
                continue;
            }

            let global_fn = match event_type {
                CreatureEventType::Login => "onLogin",
                CreatureEventType::Logout => "onLogout",
                CreatureEventType::Death => "onDeath",
            };

            match self
                .runtime
                .register_callback(format!("{}::{}", event.event_type, event.script), global_fn)
            {
                Ok(callback) => {
                    tracing::info!(
                        "Registered callback {} from {} ({:?})",
                        global_fn,
                        event.script,
                        event_type
                    );
                    result.entry(event_type).or_default().push(callback);
                }
                Err(e) => {
                    tracing::warn!(
                        "Callback {} missing/invalid in {}: {}",
                        global_fn,
                        event.script,
                        e
                    );
                }
            }
        }

        Ok(result)
    }
}

/// XML structure for creaturescripts.xml.
#[derive(Debug, serde::Deserialize)]
struct CreaturescriptsXml {
    #[serde(rename = "event")]
    events: Vec<EventXml>,
}

#[derive(Debug, serde::Deserialize)]
struct EventXml {
    #[serde(rename = "@type")]
    event_type: String,
    #[serde(rename = "@script", default)]
    script: String,
}
