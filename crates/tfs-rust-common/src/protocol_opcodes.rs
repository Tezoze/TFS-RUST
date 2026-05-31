//! Client → game and game → client opcode bytes (game protocol).
//!
//! Per-version opcode tables (Phase A2 — `docs/PROTOCOL_VERSIONING.md` §2.7, §4.3).
//! C++ reference:
//! - **1098** (canonical here): repo-root `src/protocolgame.cpp` — `parsePacket` (incoming),
//!   `ProtocolGame::send*` (outgoing).
//! - **772**: `gameserver/src/protocolgame.cpp` `parsePacket` (incoming switch ~L466–528) — sole
//!   wire authority for 7.72. Byte values for *shared* commands are identical across both eras; only
//!   a handful diverge (rule-violation block) or are era-exclusive (see grouping below).

use crate::protocol_version::ProtocolVersion;

/// First byte of an **incoming** game packet (client → server).
///
/// Byte values are shared between 772 and 1098 **unless** marked otherwise. Era-exclusive opcodes are
/// grouped at the end. Use [`client::is_supported`] to gate dispatch by the connection's version.
pub mod client {
    /// OTClient `ClientEnterGame` / `sendEnterGame` (e.g. `protocolgamesend.cpp`). Single-byte, no payload.
    /// TFS 1.4.2 `ProtocolGame::parsePacket`: no `case 0x0F` when `player` is set — falls through to `default` (no-op).
    pub const ENTER_GAME: u8 = 0x0F;
    pub const LOGOUT: u8 = 0x14;
    pub const PING_BACK: u8 = 0x1D;
    pub const PING: u8 = 0x1E;
    pub const EXTENDED_OPCODE: u8 = 0x32;
    pub const AUTO_WALK: u8 = 0x64;
    pub const MOVE_NORTH: u8 = 0x65;
    pub const MOVE_EAST: u8 = 0x66;
    pub const MOVE_SOUTH: u8 = 0x67;
    pub const MOVE_WEST: u8 = 0x68;
    pub const STOP_AUTO_WALK: u8 = 0x69;
    pub const MOVE_NORTHEAST: u8 = 0x6A;
    pub const MOVE_SOUTHEAST: u8 = 0x6B;
    pub const MOVE_SOUTHWEST: u8 = 0x6C;
    pub const MOVE_NORTHWEST: u8 = 0x6D;
    pub const TURN_NORTH: u8 = 0x6F;
    pub const TURN_EAST: u8 = 0x70;
    pub const TURN_SOUTH: u8 = 0x71;
    pub const TURN_WEST: u8 = 0x72;
    pub const EQUIP_OBJECT: u8 = 0x77;
    pub const THROW: u8 = 0x78;
    pub const LOOK_IN_SHOP: u8 = 0x79;
    pub const PURCHASE: u8 = 0x7A;
    pub const SALE: u8 = 0x7B;
    pub const CLOSE_SHOP: u8 = 0x7C;
    pub const REQUEST_TRADE: u8 = 0x7D;
    pub const LOOK_IN_TRADE: u8 = 0x7E;
    pub const ACCEPT_TRADE: u8 = 0x7F;
    pub const CLOSE_TRADE: u8 = 0x80;
    pub const USE_ITEM: u8 = 0x82;
    pub const USE_ITEM_EX: u8 = 0x83;
    pub const USE_WITH_CREATURE: u8 = 0x84;
    pub const ROTATE_ITEM: u8 = 0x85;
    pub const CLOSE_CONTAINER: u8 = 0x87;
    pub const UP_ARROW_CONTAINER: u8 = 0x88;
    pub const TEXT_WINDOW: u8 = 0x89;
    pub const HOUSE_WINDOW: u8 = 0x8A;
    pub const WRAP_ITEM: u8 = 0x8B;
    pub const LOOK_AT: u8 = 0x8C;
    pub const LOOK_IN_BATTLE_LIST: u8 = 0x8D;
    pub const JOIN_AGGRESSION: u8 = 0x8E;
    pub const SAY: u8 = 0x96;
    pub const REQUEST_CHANNELS: u8 = 0x97;
    pub const OPEN_CHANNEL: u8 = 0x98;
    pub const CLOSE_CHANNEL: u8 = 0x99;
    pub const OPEN_PRIVATE_CHANNEL: u8 = 0x9A;
    pub const CLOSE_NPC_CHANNEL: u8 = 0x9E;
    pub const FIGHT_MODES: u8 = 0xA0;
    pub const ATTACK: u8 = 0xA1;
    pub const FOLLOW: u8 = 0xA2;
    pub const PARTY_INVITE: u8 = 0xA3;
    pub const PARTY_JOIN: u8 = 0xA4;
    pub const PARTY_REVOKE_INVITE: u8 = 0xA5;
    pub const PARTY_PASS_LEADERSHIP: u8 = 0xA6;
    pub const PARTY_LEAVE: u8 = 0xA7;
    pub const PARTY_SHARE_EXPERIENCE: u8 = 0xA8;
    pub const CREATE_PRIVATE_CHANNEL: u8 = 0xAA;
    pub const CHANNEL_INVITE: u8 = 0xAB;
    pub const CHANNEL_EXCLUDE: u8 = 0xAC;
    pub const CANCEL_ATTACK_AND_FOLLOW: u8 = 0xBE;
    pub const UPDATE_TILE: u8 = 0xC9;
    pub const UPDATE_CONTAINER: u8 = 0xCA;
    pub const BROWSE_FIELD: u8 = 0xCB;
    pub const SEEK_IN_CONTAINER: u8 = 0xCC;
    pub const REQUEST_OUTFIT: u8 = 0xD2;
    pub const SET_OUTFIT: u8 = 0xD3;
    pub const TOGGLE_MOUNT: u8 = 0xD4;
    pub const VIP_ADD: u8 = 0xDC;
    pub const VIP_REMOVE: u8 = 0xDD;
    pub const VIP_EDIT: u8 = 0xDE;
    pub const BUG_REPORT: u8 = 0xE6;
    pub const THANK_YOU: u8 = 0xE7;
    pub const DEBUG_ASSERT: u8 = 0xE8;
    pub const QUEST_LOG: u8 = 0xF0;
    pub const QUEST_LINE: u8 = 0xF1;
    pub const RULE_VIOLATION_REPORT: u8 = 0xF2;
    pub const GET_OBJECT_INFO: u8 = 0xF3;
    pub const MARKET_LEAVE: u8 = 0xF4;
    pub const MARKET_BROWSE: u8 = 0xF5;
    pub const MARKET_CREATE_OFFER: u8 = 0xF6;
    pub const MARKET_CANCEL_OFFER: u8 = 0xF7;
    pub const MARKET_ACCEPT_OFFER: u8 = 0xF8;
    pub const MODAL_WINDOW_ANSWER: u8 = 0xF9;

    /// 7.72-only incoming opcodes (`gameserver/src/protocolgame.cpp` `parsePacket` ~L504–506).
    /// 10.98 collapses the rule-violation flow into a single [`RULE_VIOLATION_REPORT`] (`0xF2`).
    pub mod v772 {
        pub const PROCESS_RULE_VIOLATION_REPORT: u8 = 0x9B;
        pub const CLOSE_RULE_VIOLATION_REPORT: u8 = 0x9C;
        pub const CANCEL_RULE_VIOLATION_REPORT: u8 = 0x9D;
    }

    /// Every incoming opcode the 10.98 client protocol dispatches (repo-root `src/protocolgame.cpp`
    /// `parsePacket`). This is the canonical 10.98 set; `0x0F` (ENTER_GAME) falls through to the
    /// default no-op in C++ but is parsed here as a semantic `EnterGame`.
    const SUPPORTED_V1098: &[u8] = &[
        ENTER_GAME,
        LOGOUT,
        PING_BACK,
        PING,
        EXTENDED_OPCODE,
        AUTO_WALK,
        MOVE_NORTH,
        MOVE_EAST,
        MOVE_SOUTH,
        MOVE_WEST,
        STOP_AUTO_WALK,
        MOVE_NORTHEAST,
        MOVE_SOUTHEAST,
        MOVE_SOUTHWEST,
        MOVE_NORTHWEST,
        TURN_NORTH,
        TURN_EAST,
        TURN_SOUTH,
        TURN_WEST,
        EQUIP_OBJECT,
        THROW,
        LOOK_IN_SHOP,
        PURCHASE,
        SALE,
        CLOSE_SHOP,
        REQUEST_TRADE,
        LOOK_IN_TRADE,
        ACCEPT_TRADE,
        CLOSE_TRADE,
        USE_ITEM,
        USE_ITEM_EX,
        USE_WITH_CREATURE,
        ROTATE_ITEM,
        CLOSE_CONTAINER,
        UP_ARROW_CONTAINER,
        TEXT_WINDOW,
        HOUSE_WINDOW,
        WRAP_ITEM,
        LOOK_AT,
        LOOK_IN_BATTLE_LIST,
        JOIN_AGGRESSION,
        SAY,
        REQUEST_CHANNELS,
        OPEN_CHANNEL,
        CLOSE_CHANNEL,
        OPEN_PRIVATE_CHANNEL,
        CLOSE_NPC_CHANNEL,
        FIGHT_MODES,
        ATTACK,
        FOLLOW,
        PARTY_INVITE,
        PARTY_JOIN,
        PARTY_REVOKE_INVITE,
        PARTY_PASS_LEADERSHIP,
        PARTY_LEAVE,
        PARTY_SHARE_EXPERIENCE,
        CREATE_PRIVATE_CHANNEL,
        CHANNEL_INVITE,
        CHANNEL_EXCLUDE,
        CANCEL_ATTACK_AND_FOLLOW,
        UPDATE_TILE,
        UPDATE_CONTAINER,
        BROWSE_FIELD,
        SEEK_IN_CONTAINER,
        REQUEST_OUTFIT,
        SET_OUTFIT,
        TOGGLE_MOUNT,
        VIP_ADD,
        VIP_REMOVE,
        VIP_EDIT,
        BUG_REPORT,
        THANK_YOU,
        DEBUG_ASSERT,
        QUEST_LOG,
        QUEST_LINE,
        RULE_VIOLATION_REPORT,
        GET_OBJECT_INFO,
        MARKET_LEAVE,
        MARKET_BROWSE,
        MARKET_CREATE_OFFER,
        MARKET_CANCEL_OFFER,
        MARKET_ACCEPT_OFFER,
        MODAL_WINDOW_ANSWER,
    ];

    /// 10.98 opcodes with **no** 7.72 equivalent (`docs/PROTOCOL_VERSIONING.md` §2.7): modern shop,
    /// market, quest-log, mount, equip, wrap, browse/seek-container, VIP-edit, and the `0xF2`
    /// rule-violation report (7.72 uses the [`v772`] trio instead).
    const V772_REMOVED: &[u8] = &[
        EQUIP_OBJECT,
        LOOK_IN_SHOP,
        PURCHASE,
        SALE,
        CLOSE_SHOP,
        WRAP_ITEM,
        JOIN_AGGRESSION,
        CLOSE_NPC_CHANNEL,
        BROWSE_FIELD,
        SEEK_IN_CONTAINER,
        TOGGLE_MOUNT,
        VIP_EDIT,
        THANK_YOU,
        QUEST_LOG,
        QUEST_LINE,
        RULE_VIOLATION_REPORT,
        GET_OBJECT_INFO,
        MARKET_LEAVE,
        MARKET_BROWSE,
        MARKET_CREATE_OFFER,
        MARKET_CANCEL_OFFER,
        MARKET_ACCEPT_OFFER,
        MODAL_WINDOW_ANSWER,
    ];

    /// Incoming opcodes unique to 7.72 (the [`v772`] rule-violation trio).
    const V772_ADDED: &[u8] = &[
        v772::PROCESS_RULE_VIOLATION_REPORT,
        v772::CLOSE_RULE_VIOLATION_REPORT,
        v772::CANCEL_RULE_VIOLATION_REPORT,
    ];

    /// Does the 10.98 client protocol dispatch `op`?
    fn is_supported_v1098(op: u8) -> bool {
        SUPPORTED_V1098.contains(&op)
    }

    /// Does the 7.72 client protocol dispatch `op`? Derived from the 10.98 set minus
    /// [`V772_REMOVED`] plus [`V772_ADDED`], matching `gameserver/src/protocolgame.cpp` `parsePacket`.
    fn is_supported_v772(op: u8) -> bool {
        (is_supported_v1098(op) && !V772_REMOVED.contains(&op)) || V772_ADDED.contains(&op)
    }

    /// Is `op` a valid incoming opcode for `version`? Gates dispatch so each era only accepts its own
    /// client packets (`docs/PROTOCOL_VERSIONING.md` §2.7, §4.3). Unknown / unsupported opcodes are
    /// rejected by the caller (`game_parse`).
    pub fn is_supported(op: u8, version: super::ProtocolVersion) -> bool {
        match version.raw() {
            772 => is_supported_v772(op),
            1098 => is_supported_v1098(op),
            _ => false,
        }
    }
}

/// First payload byte of an **outgoing** game packet (server → client), after encryption/checksum framing.
///
/// Most server→client opcode **byte values** are shared between 772 and 1098; the layouts differ
/// (handled per-codec), not the opcode. The exception is the self-appear opcode, which is genuinely
/// version-keyed (`0x0A` in 772 vs `0x17` in 1098) — use [`server::self_appear`].
pub mod server {
    pub const MAP_DESCRIPTION: u8 = 0x64;
    pub const MAGIC_EFFECT: u8 = 0x83;
    pub const CREATURE_HEALTH: u8 = 0x8C;
    pub const SEND_PING: u8 = 0x1D;
    pub const SEND_PING_BACK: u8 = 0x1E;
    pub const TEXT_MESSAGE: u8 = 0xB4;
    pub const EXTENDED_OPCODE: u8 = 0x32;
    /// OTCv8 feature list (`ProtocolGame::sendFeatures` / `sendOTCFeatures` — opcode 0x43).
    pub const OTCV8_FEATURES: u8 = 0x43;

    /// Self-appear opcode (`ProtocolGame::sendAddCreature` self branch). Version-keyed:
    /// 772 = `0x0A` (`gameserver/src/protocolgame.cpp`), 1098 = `0x17` (repo-root `src/protocolgame.cpp`).
    /// Sourced from [`ProtocolCaps::self_appear_opcode`](crate::ProtocolCaps) — single source of truth.
    #[inline]
    pub fn self_appear(version: super::ProtocolVersion) -> u8 {
        version.caps().self_appear_opcode
    }
}
