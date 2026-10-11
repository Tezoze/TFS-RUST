//! Channel script hooks (`canJoin`, `onJoin`, `onLeave`, `onSpeak`).
//!
//! Pack surface: TFS `chat.cpp` `ChatChannel::executeCanJoinEvent` /
//! `executeOnJoinEvent` / `executeOnLeaveEvent` / `executeOnSpeakEvent`.
//! The keys stay on the game thread with the Lua runtime. Native join and
//! mute checks run first; these hooks mutate after.

use std::collections::HashMap;
use std::sync::Arc;

use tfs_rust_lua::ChatChannelDef;

/// One channel's optional script callbacks.
#[derive(Debug, Clone, Default)]
pub struct ChannelHookSet {
    pub can_join: Option<Arc<mlua::RegistryKey>>,
    pub on_join: Option<Arc<mlua::RegistryKey>>,
    pub on_leave: Option<Arc<mlua::RegistryKey>>,
    pub on_speak: Option<Arc<mlua::RegistryKey>>,
}

/// Indexed by channel id. Missing id means the pack registered no hook.
#[derive(Debug, Default)]
pub struct ChatChannelHookRegistry {
    by_id: HashMap<u16, ChannelHookSet>,
}

impl ChatChannelHookRegistry {
    pub fn from_defs(defs: &[ChatChannelDef]) -> Self {
        let mut by_id = HashMap::new();
        for def in defs {
            let set = ChannelHookSet {
                can_join: def.can_join.clone(),
                on_join: def.on_join.clone(),
                on_leave: def.on_leave.clone(),
                on_speak: def.on_speak.clone(),
            };
            if set.can_join.is_none()
                && set.on_join.is_none()
                && set.on_leave.is_none()
                && set.on_speak.is_none()
            {
                continue;
            }
            by_id.insert(def.id, set);
        }
        Self { by_id }
    }

    pub fn get(&self, id: u16) -> Option<&ChannelHookSet> {
        self.by_id.get(&id)
    }
}

/// `onSpeak` result. `false` drops the line; a number replaces the speak type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChannelSpeakResult {
    Keep,
    Cancel,
    Retype(u8),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_defs_register_nothing() {
        let reg = ChatChannelHookRegistry::from_defs(&[]);
        assert!(reg.get(7).is_none());
    }
}
