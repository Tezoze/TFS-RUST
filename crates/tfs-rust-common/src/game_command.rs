//! Game-thread commands: client protocol, scheduler, shutdown, async Lua results.
// C++ reference (this repo): `src/connection.cpp`, `src/game.cpp`, `src/tasks.cpp`.

use crate::conn_id::ConnId;
use crate::game_packet::GamePacket;
use crate::owned_player_load::OwnedPlayerLoad;

#[derive(Debug)]
pub enum GameCommand {
    /// Stop the game loop.
    Shutdown,
    /// `addEvent` / scheduler wake (Phase 4 `Scheduler`).
    LuaCallback { event_id: u64 },
    /// Result of `db.asyncQuery` / async work delivered on the next tick (OTCv8 / extended flows).
    LuaAsyncResult {
        conn_id: ConnId,
        request_id: u64,
        /// Opaque success payload (Lua or JSON); empty on failure.
        payload: Vec<u8>,
        success: bool,
    },
    /// Character selected — enter world (may originate outside game opcode stream).
    /// Game thread must **not** await DB I/O: spawn load and wait for [`Self::PlayerLoaded`].
    PlayerLogin {
        conn_id: ConnId,
        name: String,
        /// `OperatingSystem_t` from first game TCP message (`protocolgame.cpp` `onRecvFirstMessage`).
        operating_system: u16,
        /// `0` = not detected; else OTCv8 build (253, 260, …) after `"OTCv8"` probe.
        otclient_v8: u16,
        /// TCP peer IPv4 packed for `luaPlayerGetIp` (`0` if unknown / non-v4).
        peer_ip: u32,
    },
    /// Async character load finished — apply on the game thread only if `conn_id` is still current.
    PlayerLoaded {
        conn_id: ConnId,
        name: String,
        operating_system: u16,
        otclient_v8: u16,
        /// TCP peer IPv4 packed for `luaPlayerGetIp`.
        peer_ip: u32,
        data: OwnedPlayerLoad,
    },
    /// Async character load failed (not found / DB error / overload reject).
    PlayerLoadFailed {
        conn_id: ConnId,
        name: String,
        reason: String,
    },
    /// Close connection and clean up player session (logout / kick).
    // C++ reference: `ProtocolGame::disconnect()` (`src/protocolgame.cpp`);
    // 772 `Connection::Logout` StopFight (`connections.cc:37`).
    PlayerDisconnect {
        conn_id: ConnId,
        /// Send logout effect (poff) before closing.
        display_effect: bool,
        /// 772 `StartLogout` StopFight. `true` only for `CL_CMD_LOGOUT` / idle kick;
        /// socket drop is `false` (`connections.cc:37`).
        stop_fight: bool,
    },
    /// I/O thread registered a bounded outbound writer — mirror into game-thread sink map (GL-3).
    RegisterOutputSink { conn_id: ConnId },
    /// I/O thread removed outbound writer (TCP closed / writer task ended).
    UnregisterOutputSink { conn_id: ConnId },
    /// One decoded client game packet.
    Game { conn_id: ConnId, packet: GamePacket },
    /// Access-list names resolved off the game thread (`guid_by_name`).
    /// Re-apply the list text with the new GUID map.
    HouseNamesResolved {
        house_id: u32,
        list_id: u32,
        text: String,
        resolved: Vec<(String, u32)>,
    },
    /// Offline VIP add — `IOLoginData::getGuidByNameEx` finished off the game thread.
    VipLookupFinished {
        requester_guid: u32,
        /// `None` when no living character matches the typed name.
        target_guid: Option<u32>,
        target_name: String,
    },
    /// Offline `GetCharacterID` for mailbox `SendMail` (`moveuse.cc:764-770`).
    MailLookupFinished {
        /// SlotMap ffi of the letter/parcel still on the mailbox (or already gone).
        item_id: u64,
        town_id: u32,
        /// `None` when no living character matches the addressee.
        guid: Option<u32>,
    },
    /// Offline mail DB append finished (`mail_delivery.rs`).
    MailDeliveryFinished {
        guid: u32,
        ok: bool,
        /// `(pid, sid, itemtype)` rows written this append; empty on failure.
        /// Used to detect a stale `PlayerLoaded` that raced the persist.
        appended: Vec<(i32, i32, u16)>,
    },
    /// House policy eviction candidates from async SQL (`EvictFreeAccounts` /
    /// `EvictDeletedCharacters` / `EvictExGuildLeaders`).
    HousePolicyScanFinished {
        /// `(house_id, owner_guid)` rows still owned by that guid at query time.
        evict: Vec<(u32, u32)>,
    },
}
