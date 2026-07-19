//! Talkaction registry — `data/scripts/talkactions/*.lua` self-registering
//! commands (`/i`, `/a`, …).
//!
//! C++ reference: `talkaction.h` `TalkActions` / `talkaction.cpp`
//! `TalkActions::playerSaySpell`. Indexed by words for O(1) prefix matching
//! during `player_say` dispatch.

use std::collections::HashMap;
use std::sync::Arc;

use tfs_rust_lua::TalkActionDef;

/// One registered talkaction — words, separator, and the Lua `onSay` callback
/// key. The callback is stored as an `Arc<mlua::RegistryKey>` inside the
/// `LuaEventDispatcher`'s `LuaRuntime`; the registry key is a `!Send` opaque
/// handle that is only valid for the lifetime of that runtime. `Arc` allows
/// multi-word `;`-separated registrations to share the same callback.
#[derive(Debug)]
pub struct TalkActionEntry {
    pub words: String,
    pub separator: String,
    pub on_say: Arc<mlua::RegistryKey>,
}

/// Indexed talkaction registry — materialized once at startup, immutable on
/// the game thread. Keyed by the talkaction words (case-sensitive; the C++
/// `playerSaySpell` uses `strncasecmp` but the TFS data pack uses lowercase
/// words like `/i`, so case-insensitive matching is done at dispatch time).
#[derive(Debug, Default)]
pub struct TalkActionRegistry {
    /// `words → entry`. Multi-word `;`-separated registrations expand into
    /// multiple entries (C++ `TalkActions::registerEvent`).
    pub entries: HashMap<String, TalkActionEntry>,
}

impl TalkActionRegistry {
    /// Build the registry from `TalkActionDef`s loaded by `load_talkaction_scripts`.
    /// Each `TalkActionDef` with a `;`-separated `words` field expands into
    /// multiple entries (C++ `explodeString` in `configureEvent`).
    pub fn from_defs(defs: Vec<TalkActionDef>) -> Self {
        let mut entries = HashMap::new();
        for def in defs {
            let Some(on_say) = def.on_say else {
                tracing::warn!(
                    words = %def.words,
                    "talkaction has no onSay callback; skipping"
                );
                continue;
            };
            // C++ `configureEvent` splits words on `;` and registers each.
            for word in def.words.split(';') {
                let word = word.trim();
                if word.is_empty() {
                    continue;
                }
                entries.insert(
                    word.to_string(),
                    TalkActionEntry {
                        words: word.to_string(),
                        separator: def.separator.clone(),
                        on_say: Arc::clone(&on_say),
                    },
                );
            }
        }
        Self { entries }
    }

    /// Find a talkaction matching the given text (case-insensitive prefix
    /// match, mirroring C++ `strncasecmp`). Returns the entry and the param
    /// string (text after the words, with separator handling).
    ///
    /// C++ reference: `talkaction.cpp:84-134` `TalkActions::playerSaySpell`.
    pub fn find_match(&self, text: &str) -> Option<(&TalkActionEntry, String)> {
        let text_lower = text.to_ascii_lowercase();
        tracing::debug!(
            text,
            text_lower,
            registered_words = ?self.entries.keys().collect::<Vec<_>>(),
            "find_match: scanning"
        );
        for (words, entry) in &self.entries {
            let words_lower = words.to_ascii_lowercase();
            if text_lower.len() < words_lower.len() {
                tracing::debug!(words = %words, "find_match: text shorter than words, skip");
                continue;
            }
            if !text_lower.starts_with(&words_lower) {
                tracing::debug!(words = %words, "find_match: prefix mismatch, skip");
                continue;
            }

            // C++: if wordsLength != talkactionLength, the char after must be ' '.
            let param = if text.len() == words.len() {
                String::new()
            } else {
                let rest = &text[words.len()..];
                if !rest.starts_with(' ') {
                    continue; // Not a word boundary — skip.
                }
                let mut param = rest[1..].to_string();

                // C++: if separator != " ", check param starts with separator
                // and strip it.
                if entry.separator != " " {
                    if param.is_empty() {
                        // C++: empty param with non-space separator → continue.
                        continue;
                    }
                    if !param.starts_with(&entry.separator) {
                        continue;
                    }
                    param = param[entry.separator.len()..].to_string();
                }
                param
            };

            return Some((entry, param));
        }
        None
    }
}
