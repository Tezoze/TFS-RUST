//! `Channel` — self-registering chat-channel definition (revscriptsys-style pilot; no
//! CipSoft/TFS precedent for the *registration mechanism* itself — TFS 1.4.2 still loads
//! chatchannels.xml even in the revscriptsys era, confirmed against repo-root `src/chat.cpp`
//! `Chat::load`). Hook *contracts* (`onSpeak`/`canJoin`/`onJoin`/`onLeave`) mirror
//! `chat.cpp` `executeOnSpeakEvent`/`executeCanJoinEvent`/`executeOnJoinEvent`/`executeOnLeaveEvent`.

use mlua::{UserData, UserDataMethods};
use std::cell::Cell;

/// Channel userdata for self-registering chat channels.
///
/// C++ reference: `chat.h` `ChatChannel(channelId, channelName)` — channel identity storage.
pub struct ChannelHandle {
    pub id: u16,
    pub name: String,
    pub public: Cell<bool>,
}

impl ChannelHandle {
    pub fn new(id: u16, name: String) -> Self {
        Self {
            id,
            name,
            public: Cell::new(false),
        }
    }
}

impl UserData for ChannelHandle {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("public", |_, this, is_public: bool| {
            this.public.set(is_public);
            Ok(())
        });

        // Register the channel — reads `onSpeak`/`canJoin`/`onJoin`/`onLeave` hooks from the
        // channel's Lua table and pushes a `ChatChannelDef` into the loader's pending-channel buffer.
        //
        // C++ reference: `chat.cpp` `Chat::load` — channel registration from XML (adapted to
        // self-registering Lua convention).
        methods.add_method("register", |lua, this, ()| {
            // Try to read hooks from the channel's table (table field style)
            let channel_table = lua.create_table()?;
            channel_table.set("id", this.id)?;
            channel_table.set("name", this.name.clone())?;
            channel_table.set("public", this.public.get())?;

            // Try to get the channel table from the global scope (if using table field style)
            let global_channel: Option<mlua::Table> = lua.globals().get("channel").ok();

            // Read hooks - try table field style first, then global scope
            let on_speak = global_channel
                .as_ref()
                .and_then(|ch| ch.get::<Option<mlua::Function>>("onSpeak").ok().flatten())
                .or_else(|| lua.globals().get::<Option<mlua::Function>>("onSpeak").ok().flatten());

            let can_join = global_channel
                .as_ref()
                .and_then(|ch| ch.get::<Option<mlua::Function>>("canJoin").ok().flatten())
                .or_else(|| lua.globals().get::<Option<mlua::Function>>("canJoin").ok().flatten());

            let on_join = global_channel
                .as_ref()
                .and_then(|ch| ch.get::<Option<mlua::Function>>("onJoin").ok().flatten())
                .or_else(|| lua.globals().get::<Option<mlua::Function>>("onJoin").ok().flatten());

            let on_leave = global_channel
                .as_ref()
                .and_then(|ch| ch.get::<Option<mlua::Function>>("onLeave").ok().flatten())
                .or_else(|| lua.globals().get::<Option<mlua::Function>>("onLeave").ok().flatten());

            if let Some(func) = on_speak {
                channel_table.set("onSpeak", func)?;
            }
            if let Some(func) = can_join {
                channel_table.set("canJoin", func)?;
            }
            if let Some(func) = on_join {
                channel_table.set("onJoin", func)?;
            }
            if let Some(func) = on_leave {
                channel_table.set("onLeave", func)?;
            }

            // Push to pending buffer
            let pending: mlua::Table = lua.globals().get("_pending_channels")?;
            let len = pending.len()?;
            pending.set(len + 1, channel_table)?;

            Ok(())
        });
    }
}

/// Register the Channel metatable in the Lua runtime.
pub fn register_channel_metatable(lua: &mlua::Lua) -> Result<(), mlua::Error> {
    lua.register_userdata_type::<ChannelHandle>(|_registry| {})
}
