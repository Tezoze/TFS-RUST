//! Decode client → game packets (opcode first byte). Matches `ProtocolGame::parsePacket`.
// C++ reference: 1098 repo-root `src/protocolgame.cpp`; 772 `gameserver/src/protocolgame.cpp`
// (incoming `case 0x..` switch ~L466–528). Opcode support is version-keyed via
// `protocol_opcodes::client::is_supported` (Phase A2).

use tfs_rust_common::enums::Direction;
use tfs_rust_common::error::{Result, TfsRustError};
use tfs_rust_common::game_packet::{
    BugReportPayload, GamePacket, RuleViolationPayload, SayPayload, SetOutfitPayload, ThrowPayload,
    UseItemExPayload, UseItemPayload,
};
use tfs_rust_common::protocol_opcodes::client as C;
use tfs_rust_common::ProtocolVersion;

use crate::NetworkMessage;

/// Read opcode byte and dispatch to structured `GamePacket` for the connection's `version`.
pub fn parse_game_packet(
    msg: &mut NetworkMessage,
    version: ProtocolVersion,
) -> Result<(u8, GamePacket)> {
    let opcode = msg.read_u8()?;
    let packet = parse_game_opcode(opcode, msg, version)?;
    Ok((opcode, packet))
}

/// Parse payload after the opcode byte. Rejects opcodes the active protocol version does not dispatch
/// (`protocol_opcodes::client::is_supported`) so a 772 client cannot drive 1098-only handlers (and
/// vice versa).
pub fn parse_game_opcode(
    opcode: u8,
    msg: &mut NetworkMessage,
    version: ProtocolVersion,
) -> Result<GamePacket> {
    if !C::is_supported(opcode, version) {
        return Err(TfsRustError::Protocol(format!(
            "client game opcode 0x{opcode:02x} not valid for protocol {version}"
        )));
    }
    match opcode {
        C::ENTER_GAME => Ok(GamePacket::EnterGame),
        C::LOGOUT => Ok(GamePacket::Logout),
        C::PING_BACK => Ok(GamePacket::PingBack),
        C::PING => Ok(GamePacket::Ping),
        C::EXTENDED_OPCODE => parse_extended_opcode(msg),
        C::AUTO_WALK => parse_auto_walk(msg, version),
        C::MOVE_NORTH => Ok(GamePacket::Move(Direction::North)),
        C::MOVE_EAST => Ok(GamePacket::Move(Direction::East)),
        C::MOVE_SOUTH => Ok(GamePacket::Move(Direction::South)),
        C::MOVE_WEST => Ok(GamePacket::Move(Direction::West)),
        C::STOP_AUTO_WALK => Ok(GamePacket::StopAutoWalk),
        C::MOVE_NORTHEAST => Ok(GamePacket::Move(Direction::NorthEast)),
        C::MOVE_SOUTHEAST => Ok(GamePacket::Move(Direction::SouthEast)),
        C::MOVE_SOUTHWEST => Ok(GamePacket::Move(Direction::SouthWest)),
        C::MOVE_NORTHWEST => Ok(GamePacket::Move(Direction::NorthWest)),
        C::TURN_NORTH => Ok(GamePacket::Turn(Direction::North)),
        C::TURN_EAST => Ok(GamePacket::Turn(Direction::East)),
        C::TURN_SOUTH => Ok(GamePacket::Turn(Direction::South)),
        C::TURN_WEST => Ok(GamePacket::Turn(Direction::West)),
        C::EQUIP_OBJECT => Ok(GamePacket::EquipObject {
            sprite_id: msg.read_u16()?,
        }),
        C::THROW => parse_throw(msg),
        C::LOOK_IN_SHOP => Ok(GamePacket::LookInShop {
            item_id: msg.read_u16()?,
            count: msg.read_u8()?,
        }),
        C::PURCHASE => Ok(GamePacket::PlayerPurchase {
            item_id: msg.read_u16()?,
            count: msg.read_u8()?,
            amount: msg.read_u8()?,
            ignore_cap: msg.read_u8()? != 0,
            in_backpacks: msg.read_u8()? != 0,
        }),
        C::SALE => Ok(GamePacket::PlayerSale {
            item_id: msg.read_u16()?,
            count: msg.read_u8()?,
            amount: msg.read_u8()?,
            ignore_equipped: msg.read_u8()? != 0,
        }),
        C::CLOSE_SHOP => Ok(GamePacket::CloseShop),
        C::REQUEST_TRADE => Ok(GamePacket::RequestTrade {
            pos: msg.read_position()?,
            sprite_id: msg.read_u16()?,
            stack_pos: msg.read_u8()?,
            player_id: msg.read_u32()?,
        }),
        C::LOOK_IN_TRADE => Ok(GamePacket::LookInTrade {
            counter_offer: msg.read_u8()? == 0x01,
            index: msg.read_u8()?,
        }),
        C::ACCEPT_TRADE => Ok(GamePacket::AcceptTrade),
        C::CLOSE_TRADE => Ok(GamePacket::CloseTrade),
        C::USE_ITEM => parse_use_item(msg),
        C::USE_ITEM_EX => parse_use_item_ex(msg),
        C::USE_WITH_CREATURE => Ok(GamePacket::UseWithCreature {
            from_pos: msg.read_position()?,
            sprite_id: msg.read_u16()?,
            from_stack_pos: msg.read_u8()?,
            creature_id: msg.read_u32()?,
        }),
        C::ROTATE_ITEM => Ok(GamePacket::RotateItem {
            pos: msg.read_position()?,
            sprite_id: msg.read_u16()?,
            stack_pos: msg.read_u8()?,
        }),
        C::CLOSE_CONTAINER => Ok(GamePacket::CloseContainer {
            cid: msg.read_u8()?,
        }),
        C::UP_ARROW_CONTAINER => Ok(GamePacket::UpArrowContainer {
            cid: msg.read_u8()?,
        }),
        C::TEXT_WINDOW => Ok(GamePacket::TextWindow {
            window_text_id: msg.read_u32()?,
            new_text: msg.read_string()?,
        }),
        C::HOUSE_WINDOW => Ok(GamePacket::HouseWindow {
            door_id: msg.read_u8()?,
            house_id: msg.read_u32()?,
            text: msg.read_string()?,
        }),
        C::WRAP_ITEM => Ok(GamePacket::WrapItem {
            pos: msg.read_position()?,
            sprite_id: msg.read_u16()?,
            stack_pos: msg.read_u8()?,
        }),
        C::LOOK_AT => {
            let pos = msg.read_position()?;
            msg.skip(2)?; // spriteId
            let stack_pos = msg.read_u8()?;
            Ok(GamePacket::LookAt { pos, stack_pos })
        }
        C::LOOK_IN_BATTLE_LIST => Ok(GamePacket::LookInBattleList {
            creature_id: msg.read_u32()?,
        }),
        C::JOIN_AGGRESSION => Ok(GamePacket::JoinAggression),
        C::SAY => parse_say(msg, version),
        C::REQUEST_CHANNELS => Ok(GamePacket::RequestChannels),
        C::OPEN_CHANNEL => Ok(GamePacket::OpenChannel {
            channel_id: msg.read_u16()?,
        }),
        C::CLOSE_CHANNEL => Ok(GamePacket::CloseChannel {
            channel_id: msg.read_u16()?,
        }),
        C::OPEN_PRIVATE_CHANNEL => Ok(GamePacket::OpenPrivateChannel {
            receiver: msg.read_string()?,
        }),
        C::CLOSE_NPC_CHANNEL => Ok(GamePacket::CloseNpcChannel),
        C::FIGHT_MODES => {
            let raw_fight_mode = msg.read_u8()?;
            let raw_chase_mode = msg.read_u8()?;
            let raw_secure_mode = msg.read_u8()?;
            // OTClient v8 may send PVP mode as 4th byte (`GamePVPMode`); official client sends 3.
            let raw_pvp_mode = if msg.unread_bytes() > 0 {
                msg.read_u8()?
            } else {
                0
            };
            Ok(GamePacket::FightModes {
                raw_fight_mode,
                raw_chase_mode,
                raw_secure_mode,
                raw_pvp_mode,
            })
        }
        // C++ reads one `uint32_t`; second read is commented out (`src/protocolgame.cpp` ~1034).
        C::ATTACK => Ok(GamePacket::Attack {
            creature_id: msg.read_u32()?,
        }),
        C::FOLLOW => Ok(GamePacket::Follow {
            creature_id: msg.read_u32()?,
        }),
        C::PARTY_INVITE => Ok(GamePacket::PartyInvite {
            target_id: msg.read_u32()?,
        }),
        C::PARTY_JOIN => Ok(GamePacket::PartyJoin {
            target_id: msg.read_u32()?,
        }),
        C::PARTY_REVOKE_INVITE => Ok(GamePacket::PartyRevokeInvite {
            target_id: msg.read_u32()?,
        }),
        C::PARTY_PASS_LEADERSHIP => Ok(GamePacket::PartyPassLeadership {
            target_id: msg.read_u32()?,
        }),
        C::PARTY_LEAVE => Ok(GamePacket::PartyLeave),
        C::PARTY_SHARE_EXPERIENCE => Ok(GamePacket::PartyShareExperience {
            active: msg.read_u8()? == 1,
        }),
        C::CREATE_PRIVATE_CHANNEL => Ok(GamePacket::CreatePrivateChannel),
        C::CHANNEL_INVITE => Ok(GamePacket::ChannelInvite {
            name: msg.read_string()?,
        }),
        C::CHANNEL_EXCLUDE => Ok(GamePacket::ChannelExclude {
            name: msg.read_string()?,
        }),
        C::CANCEL_ATTACK_AND_FOLLOW => Ok(GamePacket::CancelAttackAndFollow),
        C::UPDATE_TILE => Ok(GamePacket::UpdateTile),
        C::UPDATE_CONTAINER => Ok(GamePacket::UpdateContainer {
            cid: msg.read_u8()?,
        }),
        C::BROWSE_FIELD => Ok(GamePacket::BrowseField {
            pos: msg.read_position()?,
        }),
        C::SEEK_IN_CONTAINER => Ok(GamePacket::SeekInContainer {
            cid: msg.read_u8()?,
            index: msg.read_u16()?,
        }),
        C::REQUEST_OUTFIT => Ok(GamePacket::RequestOutfit),
        C::SET_OUTFIT => parse_set_outfit(msg, version),
        C::TOGGLE_MOUNT => Ok(GamePacket::ToggleMount {
            mount: msg.read_u8()? != 0,
        }),
        C::VIP_ADD => Ok(GamePacket::VipAdd {
            name: msg.read_string()?,
        }),
        C::VIP_REMOVE => Ok(GamePacket::VipRemove {
            guid: msg.read_u32()?,
        }),
        C::VIP_EDIT => Ok(GamePacket::VipEdit {
            guid: msg.read_u32()?,
            description: msg.read_string()?,
            icon: msg.read_u32()?,
            notify: msg.read_u8()? != 0,
        }),
        C::BUG_REPORT => parse_bug_report(msg),
        C::THANK_YOU => Ok(GamePacket::ThankYou),
        C::DEBUG_ASSERT => Ok(GamePacket::DebugAssert {
            assert_line: msg.read_string()?,
            date: msg.read_string()?,
            description: msg.read_string()?,
            comment: msg.read_string()?,
        }),
        C::QUEST_LOG => Ok(GamePacket::QuestLog),
        C::QUEST_LINE => Ok(GamePacket::QuestLine {
            quest_id: msg.read_u16()?,
        }),
        C::RULE_VIOLATION_REPORT => parse_rule_violation(msg),
        C::GET_OBJECT_INFO => Ok(GamePacket::GetObjectInfo),
        C::MARKET_LEAVE => Ok(GamePacket::MarketLeave),
        C::MARKET_BROWSE => Ok(GamePacket::MarketBrowse {
            browse_id: msg.read_u16()?,
        }),
        C::MARKET_CREATE_OFFER => Ok(GamePacket::MarketCreateOffer {
            offer_type: msg.read_u8()?,
            sprite_id: msg.read_u16()?,
            amount: msg.read_u16()?,
            price: msg.read_u32()?,
            anonymous: msg.read_u8()? != 0,
        }),
        C::MARKET_CANCEL_OFFER => Ok(GamePacket::MarketCancelOffer {
            timestamp: msg.read_u32()?,
            counter: msg.read_u16()?,
        }),
        C::MARKET_ACCEPT_OFFER => Ok(GamePacket::MarketAcceptOffer {
            timestamp: msg.read_u32()?,
            counter: msg.read_u16()?,
            amount: msg.read_u16()?,
        }),
        C::MODAL_WINDOW_ANSWER => Ok(GamePacket::ModalWindowAnswer {
            window_id: msg.read_u32()?,
            button: msg.read_u8()?,
            choice: msg.read_u8()?,
        }),
        _ => Err(TfsRustError::Protocol(format!(
            "unknown client game opcode 0x{opcode:02x}"
        ))),
    }
}

fn parse_extended_opcode(msg: &mut NetworkMessage) -> Result<GamePacket> {
    Ok(GamePacket::ExtendedOpcode {
        opcode: msg.read_u8()?,
        buffer: msg.read_string()?,
    })
}

fn parse_auto_walk(msg: &mut NetworkMessage, version: ProtocolVersion) -> Result<GamePacket> {
    let n = msg.read_u8()? as usize;
    let len_invalid = match version.raw() {
        772 => n == 0 || msg.unread_bytes() != n || n > 128,
        1098 => n == 0 || msg.unread_bytes() != n,
        other => unreachable!("unsupported protocol version {other}"),
    };
    if len_invalid {
        return Err(TfsRustError::Protocol("invalid auto-walk length".into()));
    }
    let mut raw_dirs = Vec::with_capacity(n);
    for _ in 0..n {
        raw_dirs.push(msg.read_u8()?);
    }
    tracing::debug!(
        version = version.raw(),
        numdirs = n,
        ?raw_dirs,
        "autowalk: parsed packet"
    );
    // C++ reads with `getPreviousByte()` — last queued step is executed first (`protocolgame.cpp` ~857–889).
    let mut path = Vec::with_capacity(n);
    for raw in raw_dirs.into_iter().rev() {
        path.push(raw_dir_to_direction(raw)?);
    }
    tracing::debug!(?path, "autowalk: execution-order path (rev of wire)");
    Ok(GamePacket::AutoWalk { path })
}

fn raw_dir_to_direction(raw: u8) -> Result<Direction> {
    match raw {
        1 => Ok(Direction::East),
        2 => Ok(Direction::NorthEast),
        3 => Ok(Direction::North),
        4 => Ok(Direction::NorthWest),
        5 => Ok(Direction::West),
        6 => Ok(Direction::SouthWest),
        7 => Ok(Direction::South),
        8 => Ok(Direction::SouthEast),
        _ => Err(TfsRustError::Protocol(format!(
            "invalid auto-walk direction {raw}"
        ))),
    }
}

fn parse_use_item(msg: &mut NetworkMessage) -> Result<GamePacket> {
    Ok(GamePacket::UseItem(UseItemPayload {
        pos: msg.read_position()?,
        sprite_id: msg.read_u16()?,
        stack_pos: msg.read_u8()?,
        index: msg.read_u8()?,
    }))
}

fn parse_use_item_ex(msg: &mut NetworkMessage) -> Result<GamePacket> {
    Ok(GamePacket::UseItemEx(UseItemExPayload {
        from_pos: msg.read_position()?,
        from_sprite_id: msg.read_u16()?,
        from_stack_pos: msg.read_u8()?,
        to_pos: msg.read_position()?,
        to_sprite_id: msg.read_u16()?,
        to_stack_pos: msg.read_u8()?,
    }))
}

fn parse_throw(msg: &mut NetworkMessage) -> Result<GamePacket> {
    Ok(GamePacket::Throw(ThrowPayload {
        from_pos: msg.read_position()?,
        sprite_id: msg.read_u16()?,
        from_stack_pos: msg.read_u8()?,
        to_pos: msg.read_position()?,
        count: msg.read_u8()?,
    }))
}

/// `ProtocolGame::parseSetOutfit`.
///
/// - **772:** `lookType` + head/body/legs/feet only (`gameserver/src/protocolgame.cpp` ~790).
/// - **1098:** + `lookAddons` + `lookMount` (`src/protocolgame.cpp`).
fn parse_set_outfit(msg: &mut NetworkMessage, version: ProtocolVersion) -> Result<GamePacket> {
    let look_type = msg.read_u16()?;
    let look_head = msg.read_u8()?;
    let look_body = msg.read_u8()?;
    let look_legs = msg.read_u8()?;
    let look_feet = msg.read_u8()?;
    let (look_addons, look_mount) = match version.raw() {
        772 => (0u8, 0u16),
        1098 => (msg.read_u8()?, msg.read_u16()?),
        other => unreachable!("unsupported protocol version {other}"),
    };
    Ok(GamePacket::SetOutfit(SetOutfitPayload {
        look_type,
        look_head,
        look_body,
        look_legs,
        look_feet,
        look_addons,
        look_mount,
    }))
}

/// Trailing field a `parseSay` packet carries after its speak-class byte.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SayField {
    /// Private message: a length-prefixed receiver name (`msg.getString()`).
    Receiver,
    /// Channel talk: a `uint16_t` channel id (`msg.get<uint16_t>()`).
    Channel,
    /// Local speech (say/whisper/yell/broadcast/unknown): no field before the text.
    None,
}

/// Which trailing field `ProtocolGame::parseSay` reads for `speak_class` under `version`.
///
/// **The `SpeakClasses` enum values differ by era**, so this MUST be version-keyed — the byte
/// that means "channel talk" in 772 (`5`) means "private message" in 1098, and reading the wrong
/// trailing field (string vs. `u16`) desyncs the stream (observed as `EOF reading u16`).
///
/// C++ reference:
/// - 772  `gameserver/src/protocolgame.cpp:919-943` (`parseSay` switch) + `const.h:61-77`:
///   `TALKTYPE_PRIVATE=4`, `TALKTYPE_PRIVATE_RED=11`, `TALKTYPE_RVR_ANSWER=7` → receiver;
///   `TALKTYPE_CHANNEL_Y=5`, `TALKTYPE_CHANNEL_R1=10`, `TALKTYPE_CHANNEL_R2=14` → channel id.
/// - 1098 repo-root `src/protocolgame.cpp:984-1005` + `const.h:163-178`:
///   `TALKTYPE_PRIVATE_TO=5`, `TALKTYPE_PRIVATE_RED_TO=16` → receiver;
///   `TALKTYPE_CHANNEL_Y=7`, `TALKTYPE_CHANNEL_R1=14` → channel id.
fn say_field(speak_class: u8, version: ProtocolVersion) -> SayField {
    match version.raw() {
        772 => match speak_class {
            4 | 11 | 7 => SayField::Receiver, // PRIVATE, PRIVATE_RED, RVR_ANSWER
            5 | 10 | 14 => SayField::Channel, // CHANNEL_Y, CHANNEL_R1, CHANNEL_R2
            _ => SayField::None,
        },
        1098 => match speak_class {
            5 | 16 => SayField::Receiver, // PRIVATE_TO, PRIVATE_RED_TO
            7 | 14 => SayField::Channel,  // CHANNEL_Y, CHANNEL_R1
            _ => SayField::None,
        },
        _ => SayField::None,
    }
}

fn parse_say(msg: &mut NetworkMessage, version: ProtocolVersion) -> Result<GamePacket> {
    let speak_class = msg.read_u8()?;
    let mut channel_id = 0u16;
    let mut receiver = String::new();

    // The trailing field depends on both the speak class AND the protocol era (see `say_field`).
    match say_field(speak_class, version) {
        SayField::Receiver => receiver = msg.read_string()?,
        SayField::Channel => channel_id = msg.read_u16()?,
        SayField::None => {}
    }

    let text = msg.read_string()?;
    // C++ `parseSay` drops texts longer than 255 bytes at the wire layer
    // (772 `gameserver/src/protocolgame.cpp:944-946`, 1098 `src/protocolgame.cpp:1007-1009`).
    // The packet is discarded but the connection stays open — returning `Err` here mirrors that:
    // the caller logs a warn and continues reading the next packet (`protocol_game.rs:130-133`).
    if text.len() > 255 {
        return Err(TfsRustError::Protocol(
            "parseSay: text exceeds 255-byte limit".into(),
        ));
    }
    // `speak_class` is passed through as the raw wire byte. The active target is 772, whose
    // `SpeakClasses` values are the canonical set `tfs-rust-core::game_world_chat` switches on,
    // so 772 is correct end to end. 1098 chat dispatch needs a wire→core speak-class translation
    // (both directions) — tracked as the CH-8 follow-up in `tasks/chat-system-plan.md`.
    Ok(GamePacket::Say(SayPayload {
        speak_class,
        channel_id,
        receiver,
        text,
    }))
}

fn parse_bug_report(msg: &mut NetworkMessage) -> Result<GamePacket> {
    const BUG_CATEGORY_MAP: u8 = 0;
    let category = msg.read_u8()?;
    let message = msg.read_string()?;
    let position = if category == BUG_CATEGORY_MAP {
        Some(msg.read_position()?)
    } else {
        None
    };
    Ok(GamePacket::BugReport(BugReportPayload {
        category,
        message,
        position,
    }))
}

fn parse_rule_violation(msg: &mut NetworkMessage) -> Result<GamePacket> {
    const REPORT_TYPE_NAME: u8 = 0;
    const REPORT_TYPE_STATEMENT: u8 = 1;

    let report_type = msg.read_u8()?;
    let report_reason = msg.read_u8()?;
    let target_name = msg.read_string()?;
    let comment = msg.read_string()?;
    let mut translation = String::new();
    let mut statement_id: Option<u32> = None;
    match report_type {
        REPORT_TYPE_NAME => {
            translation = msg.read_string()?;
        }
        REPORT_TYPE_STATEMENT => {
            translation = msg.read_string()?;
            statement_id = Some(msg.read_u32()?);
        }
        _ => {}
    }
    Ok(GamePacket::RuleViolationReport(RuleViolationPayload {
        report_type,
        report_reason,
        target_name,
        comment,
        translation,
        statement_id,
    }))
}
