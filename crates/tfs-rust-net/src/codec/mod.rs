//! Protocol version codec seam (Track A — `docs/PROTOCOL_VERSIONING.md` §4.2).
//!
//! C++ reference: 10.98 `src/protocolgame.cpp`; 7.72 `gameserver/src/protocolgame.cpp` (Phase A5).

mod v1098;
mod v772;
pub mod wire;

pub use v772::Codec772;
pub use v1098::Codec1098;
pub use wire::{
    AddCreatureWire, AnimatedTextWire, ChannelOpenWire, ChannelsDialogWire, CombatDamageNotifyWire,
    ContainerOpenWire, CreatePrivateChannelWire, CreatureHealthWire, CreatureSpeedWire,
    CreatureSquareWire, DistanceShootWire, ItemStack, ItemTemplateArgs, ItemWire, MagicEffectWire,
    OutfitWire, PlayerSkillsWire, PlayerStatsWire, TextWindowWire,
};

use tfs_rust_common::{Position, ProtocolCaps, ProtocolVersion};

use crate::NetworkMessage;
use crate::creature_encode::AddCreatureWire as CreatureWire;
use crate::creature_encode::OutfitWire as CreatureOutfitWire;

/// Outgoing wire encoder — one impl per protocol family (A1: 1098 only).
pub trait ProtocolCodec {
    fn caps(&self) -> ProtocolCaps;

    /// Per-tile `GetTileDescription` prefix. 10.98 writes a `u16` "environmental effects" field
    /// (`src/protocolgame.cpp`); 7.72 (`gameserver/src/protocolgame.cpp`) writes nothing.
    fn write_tile_environment_prefix(&self, msg: &mut NetworkMessage);

    /// Byte length of [`Self::write_tile_environment_prefix`] (2 for 1098, 0 for 772).
    fn tile_environment_prefix_len(&self) -> usize;

    /// Whether `GetTileDescription` caps the creature loop at the 10-thing stack limit.
    ///
    /// 7.72 (`gameserver/src/protocolgame.cpp:572-574`) returns early once `count` hits 10 inside
    /// the creature loop; 10.98 (`src/protocolgame.cpp:669-682`) increments `count` but never
    /// checks it during creature emission (the cap is only enforced in the top-items `break` and
    /// the down-items `return`).
    fn tile_description_caps_creatures(&self) -> bool;

    /// Mirrors C++ `NetworkMessage::addItem` template field list (parity); higher-level call sites
    /// use the `ItemTemplateArgs` struct form.
    #[allow(clippy::too_many_arguments)]
    fn write_item_template(
        &self,
        msg: &mut NetworkMessage,
        client_id: u16,
        count: u8,
        stackable: bool,
        is_splash_or_fluid: bool,
        is_animation: bool,
        with_description: bool,
    );

    fn item_template_wire_len(
        &self,
        client_id: u16,
        count: u8,
        stackable: bool,
        is_splash_or_fluid: bool,
        is_animation: bool,
        with_description: bool,
    ) -> usize;

    fn write_add_creature(&self, msg: &mut NetworkMessage, c: &CreatureWire);

    fn add_creature_wire_len(&self, c: &CreatureWire) -> usize;

    fn write_outfit(&self, msg: &mut NetworkMessage, o: &CreatureOutfitWire);

    fn encode_player_stats(&self, s: &PlayerStatsWire) -> NetworkMessage;

    fn encode_player_skills(&self, s: &PlayerSkillsWire) -> NetworkMessage;

    fn encode_basic_data(
        &self,
        is_premium: bool,
        premium_ends_at: u32,
        vocation_client_id: u8,
    ) -> NetworkMessage;

    fn encode_self_appear_login(&self, player_id: u32, server_beat: u16) -> NetworkMessage;

    /// Standalone `0x6A` tile item. 10.98 always includes `stackpos`; 7.72 omits it (OTCv8
    /// `GameTileAddThingWithStackpos` is 8.41+ only — `otclient_stackpos` is ignored on 772).
    fn encode_add_tile_item(
        &self,
        pos: Position,
        stack_pos: u8,
        args: ItemTemplateArgs,
        otclient_stackpos: bool,
    ) -> NetworkMessage;

    fn encode_update_tile_item(
        &self,
        pos: Position,
        stack_pos: u8,
        args: ItemTemplateArgs,
    ) -> NetworkMessage;

    fn encode_inventory_item(&self, slot: u8, args: ItemTemplateArgs) -> NetworkMessage;

    fn encode_add_container_item(
        &self,
        cid: u8,
        slot: u16,
        args: ItemTemplateArgs,
    ) -> NetworkMessage;

    fn encode_update_container_item(
        &self,
        cid: u8,
        slot: u16,
        args: ItemTemplateArgs,
    ) -> NetworkMessage;

    /// `0x72` remove container item. 7.72 sends a `u8` slot; 10.98 sends a `u16` slot + optional item.
    fn encode_remove_container_item(&self, cid: u8, slot: u16) -> NetworkMessage;

    /// Standalone `0x6A` tile creature. 10.98 always includes `stackpos`; 7.72 omits it (OTCv8
    /// `GameTileAddThingWithStackpos` is 8.41+ only — `otclient_stackpos` is ignored on 772).
    fn encode_add_tile_creature(
        &self,
        pos: Position,
        stack_pos: u8,
        wire: &CreatureWire,
        otclient_stackpos: bool,
    ) -> NetworkMessage;

    fn encode_remove_tile_thing(&self, pos: Position, stackpos: u8) -> NetworkMessage;

    fn encode_remove_tile_creature_by_id(&self, creature_id: u32) -> NetworkMessage;

    fn encode_creature_light(
        &self,
        creature_id: u32,
        level: u8,
        color: u8,
        access_player: bool,
    ) -> NetworkMessage;

    fn encode_creature_turn(
        &self,
        creature_id: u32,
        stack_pos: u8,
        tile_pos: Position,
        direction: u8,
        can_walkthrough: bool,
    ) -> NetworkMessage;

    fn encode_cancel_walk(&self, direction: u8) -> NetworkMessage;

    /// `ProtocolGame::sendCancelTarget` — single byte `0xA3` (both eras).
    /// 772: `gameserver/src/protocolgame.cpp:1485-1490`; 1098: `src/protocolgame.cpp:2497-2500`.
    fn encode_clear_target(&self) -> NetworkMessage;

    fn encode_container_open(&self, c: &ContainerOpenWire) -> NetworkMessage;

    /// 7.72 `sendAnimatedText`; 10.98 has no equivalent (returns empty message).
    fn encode_animated_text(&self, w: &wire::AnimatedTextWire) -> NetworkMessage;

    fn encode_magic_effect(&self, w: &wire::MagicEffectWire) -> NetworkMessage;

    fn encode_distance_shoot(&self, w: &wire::DistanceShootWire) -> NetworkMessage;

    fn encode_creature_health(&self, w: &wire::CreatureHealthWire) -> NetworkMessage;

    /// `SendMarkCreature` / `sendCreatureSquare` — black square on the attacker.
    /// 772: `0x86` + id + color. 1098: `0x93` + id + `0x01` + color.
    fn encode_creature_square(&self, w: &wire::CreatureSquareWire) -> NetworkMessage;

    /// `SendCreatureSpeed` (772 `sending.cc:1028`) / `sendChangeSpeed` (1098).
    /// 772: `0x8F + u32 id + u16 speed`. 1098: `0x8F + u32 id + u16 base/2 + u16 speed/2`.
    fn encode_creature_speed(&self, w: &wire::CreatureSpeedWire) -> NetworkMessage;

    /// `ProtocolGame::sendCreatureOutfit` — `0x8E` + id + era `AddOutfit`.
    /// 772 has no addons/mount; using the 1098 outfit body desyncs and crashes 7.72 clients.
    fn encode_creature_outfit(
        &self,
        creature_id: u32,
        outfit: &CreatureOutfitWire,
    ) -> NetworkMessage;

    /// Player damage caption — simple text (772) vs damage block (1098).
    fn encode_combat_damage_text_message(&self, w: &wire::CombatDamageNotifyWire)
    -> NetworkMessage;

    /// `ProtocolGame::sendCreatureSay` — `0xAA` speech packet (1098 with `level`, 772 without).
    fn encode_creature_say(&self, statement_id: u32, w: &wire::CreatureSayWire) -> NetworkMessage;

    /// `ProtocolGame::sendToChannel` — `0xAA` channel speech (1098 with `level`, 772 without).
    /// 772: `gameserver/src/protocolgame.cpp:1442`; 1098: `src/protocolgame.cpp:1730`.
    fn encode_to_channel(&self, statement_id: u32, w: &wire::ToChannelWire) -> NetworkMessage;

    /// `ProtocolGame::sendPrivateMessage` — `0xAA` private message (1098 with `level`, 772 without).
    /// 772: `gameserver/src/protocolgame.cpp:1465`; 1098: `src/protocolgame.cpp:2480`.
    fn encode_private_message(
        &self,
        statement_id: u32,
        w: &wire::PrivateMessageWire,
    ) -> NetworkMessage;

    /// `ProtocolGame::sendChannelMessage` — `0xAA` anonymous channel message (1098 with author
    /// `level` field = 0, 772 without). 772: `gameserver/src/protocolgame.cpp:1306`;
    /// 1098: `src/protocolgame.cpp:1730`.
    fn encode_channel_message(&self, w: &wire::ChannelMessageWire) -> NetworkMessage;

    /// `ProtocolGame::sendChannelsDialog` — `0xAB` channel list dialog.
    /// Era-identical layout; both codecs emit `byte + u8 count + [u16 id + string name]*`.
    /// 772: `gameserver/src/protocolgame.cpp:1282`; 1098: `src/protocolgame.cpp:1687`.
    fn encode_channels_dialog(&self, w: &wire::ChannelsDialogWire) -> NetworkMessage;

    /// `ProtocolGame::sendChannel` — `0xAC` open-channel ack.
    /// 1098 appends `users` / `invited` name lists; 772 omits them.
    /// 772: `gameserver/src/protocolgame.cpp:1297`; 1098: `src/protocolgame.cpp:1702`.
    fn encode_channel_open(&self, w: &wire::ChannelOpenWire) -> NetworkMessage;

    /// `ProtocolGame::sendCreatePrivateChannel` — `0xB2` ack for a new private channel.
    /// 1098 appends `owner_name` + `invited` name list; 772 omits them.
    /// 772: `gameserver/src/protocolgame.cpp:1273`; 1098: `src/protocolgame.cpp:1675`.
    fn encode_create_private_channel(&self, w: &wire::CreatePrivateChannelWire) -> NetworkMessage;

    /// `ProtocolGame::sendTextWindow` template-item overload — `0x96`.
    /// 772 omits the date field and uses 772 `addItem` (no MARK); 1098 writes MARK + date.
    /// 772: `gameserver/src/protocolgame.cpp:1925`; 1098: `src/protocolgame.cpp:2999`.
    fn encode_text_window(&self, w: &wire::TextWindowWire) -> NetworkMessage;

    /// `ProtocolGame::sendHouseWindow` — `0x97 | 0x00 | u32 windowTextId | string`.
    /// Identical on 772 and 1098.
    fn encode_house_window(&self, window_text_id: u32, text: &str) -> NetworkMessage;

    /// Era-correct wire value for the "cancel / failure" text-message channel used by
    /// `sendCancelMessage` (1098) / `SendResult` (772).
    ///
    /// - **1098** (`src/const.h:190`): `MESSAGE_STATUS_SMALL = 21` — `sendCancelMessage` →
    ///   `sendTextMessage(MESSAGE_STATUS_SMALL, ...)`.
    /// - **772** (`sending.cc:339`, `enums.hh:674`): `TALK_FAILURE_MESSAGE = 23` — `SendResult` →
    ///   `SendMessage(TALK_FAILURE_MESSAGE, ...)`. The 772 TVP `const.h` names this
    ///   `MESSAGE_STATUS_SMALL = 0x17 = 23` — same wire byte, different name than 1098.
    fn failure_message_type(&self) -> u8;

    /// Era-correct wire value for status-style text messages sent to a single player
    /// (e.g. "You are poisoned." — `crcombat.cc:674-676` `SendMessage(TALK_STATUS_MESSAGE, ...)`).
    ///
    /// - **772** (`enums.hh:672`): `TALK_STATUS_MESSAGE = 21` — distinct from
    ///   `TALK_FAILURE_MESSAGE = 23` (see [`failure_message_type`]).
    /// - **1098** (`src/const.h:189`): `MESSAGE_STATUS_DEFAULT = 21` — same wire byte;
    ///   `MESSAGE_STATUS_SMALL` (also 21) is used by `sendCancelMessage`.
    fn status_message_type(&self) -> u8;

    /// Periodic keepalive ping packet — X1 (K1 inventory §X1).
    /// - **772** (`protocolgame.cpp:1516`): `0x1E` (`send_ping_back`) for non-OTClient,
    ///   `0x1D` (`send_ping`) for OTClient.
    /// - **1098** (`src/protocolgame.cpp:2530`): always `0x1D` (`send_ping`).
    fn periodic_ping_packet(&self, is_otclient: bool) -> NetworkMessage;
}

impl ProtocolCodec for Codec1098 {
    fn caps(&self) -> ProtocolCaps {
        Codec1098::caps(self)
    }

    fn write_tile_environment_prefix(&self, msg: &mut NetworkMessage) {
        // 10.98 `GetTileDescription` — `msg.add<uint16_t>(0x00)` environmental effects.
        msg.write_u16(0);
    }

    fn tile_environment_prefix_len(&self) -> usize {
        2
    }

    fn tile_description_caps_creatures(&self) -> bool {
        // 10.98 `GetTileDescription` (`src/protocolgame.cpp:669-682`) — creature loop has no
        // `count == 10` check; only top-items `break` and down-items `return` enforce the cap.
        false
    }

    fn write_item_template(
        &self,
        msg: &mut NetworkMessage,
        client_id: u16,
        count: u8,
        stackable: bool,
        is_splash_or_fluid: bool,
        is_animation: bool,
        with_description: bool,
    ) {
        Codec1098::write_item_template(
            self,
            msg,
            client_id,
            count,
            stackable,
            is_splash_or_fluid,
            is_animation,
            with_description,
        );
    }

    fn item_template_wire_len(
        &self,
        client_id: u16,
        count: u8,
        stackable: bool,
        is_splash_or_fluid: bool,
        is_animation: bool,
        with_description: bool,
    ) -> usize {
        Codec1098::item_template_wire_len(
            self,
            client_id,
            count,
            stackable,
            is_splash_or_fluid,
            is_animation,
            with_description,
        )
    }

    fn write_add_creature(&self, msg: &mut NetworkMessage, c: &CreatureWire) {
        Codec1098::write_add_creature(self, msg, c);
    }

    fn add_creature_wire_len(&self, c: &CreatureWire) -> usize {
        Codec1098::add_creature_wire_len(self, c)
    }

    fn write_outfit(&self, msg: &mut NetworkMessage, o: &CreatureOutfitWire) {
        Codec1098::write_outfit(self, msg, o);
    }

    fn encode_player_stats(&self, s: &PlayerStatsWire) -> NetworkMessage {
        Codec1098::encode_player_stats(self, s)
    }

    fn encode_player_skills(&self, s: &PlayerSkillsWire) -> NetworkMessage {
        Codec1098::encode_player_skills(self, s)
    }

    fn encode_basic_data(
        &self,
        is_premium: bool,
        premium_ends_at: u32,
        vocation_client_id: u8,
    ) -> NetworkMessage {
        Codec1098::encode_basic_data(self, is_premium, premium_ends_at, vocation_client_id)
    }

    fn encode_self_appear_login(&self, player_id: u32, server_beat: u16) -> NetworkMessage {
        Codec1098::encode_self_appear_login(self, player_id, server_beat)
    }

    fn encode_add_tile_item(
        &self,
        pos: Position,
        stack_pos: u8,
        args: ItemTemplateArgs,
        _otclient_stackpos: bool,
    ) -> NetworkMessage {
        Codec1098::encode_add_tile_item(self, pos, stack_pos, args)
    }

    fn encode_update_tile_item(
        &self,
        pos: Position,
        stack_pos: u8,
        args: ItemTemplateArgs,
    ) -> NetworkMessage {
        Codec1098::encode_update_tile_item(self, pos, stack_pos, args)
    }

    fn encode_inventory_item(&self, slot: u8, args: ItemTemplateArgs) -> NetworkMessage {
        Codec1098::encode_inventory_item(self, slot, args)
    }

    fn encode_add_container_item(
        &self,
        cid: u8,
        slot: u16,
        args: ItemTemplateArgs,
    ) -> NetworkMessage {
        Codec1098::encode_add_container_item(self, cid, slot, args)
    }

    fn encode_update_container_item(
        &self,
        cid: u8,
        slot: u16,
        args: ItemTemplateArgs,
    ) -> NetworkMessage {
        Codec1098::encode_update_container_item(self, cid, slot, args)
    }

    fn encode_remove_container_item(&self, cid: u8, slot: u16) -> NetworkMessage {
        Codec1098::encode_remove_container_item(self, cid, slot)
    }

    fn encode_add_tile_creature(
        &self,
        pos: Position,
        stack_pos: u8,
        wire: &CreatureWire,
        _otclient_stackpos: bool,
    ) -> NetworkMessage {
        Codec1098::encode_add_tile_creature(self, pos, stack_pos, wire)
    }

    fn encode_remove_tile_thing(&self, pos: Position, stackpos: u8) -> NetworkMessage {
        Codec1098::encode_remove_tile_thing(self, pos, stackpos)
    }

    fn encode_remove_tile_creature_by_id(&self, creature_id: u32) -> NetworkMessage {
        Codec1098::encode_remove_tile_creature_by_id(self, creature_id)
    }

    fn encode_creature_light(
        &self,
        creature_id: u32,
        level: u8,
        color: u8,
        access_player: bool,
    ) -> NetworkMessage {
        Codec1098::encode_creature_light(self, creature_id, level, color, access_player)
    }

    fn encode_creature_turn(
        &self,
        creature_id: u32,
        stack_pos: u8,
        tile_pos: Position,
        direction: u8,
        can_walkthrough: bool,
    ) -> NetworkMessage {
        Codec1098::encode_creature_turn(
            self,
            creature_id,
            stack_pos,
            tile_pos,
            direction,
            can_walkthrough,
        )
    }

    fn encode_cancel_walk(&self, direction: u8) -> NetworkMessage {
        Codec1098::encode_cancel_walk(self, direction)
    }

    fn encode_clear_target(&self) -> NetworkMessage {
        Codec1098::encode_clear_target(self)
    }

    fn encode_container_open(&self, c: &ContainerOpenWire) -> NetworkMessage {
        Codec1098::encode_container_open(self, c)
    }

    fn encode_animated_text(&self, w: &wire::AnimatedTextWire) -> NetworkMessage {
        Codec1098::encode_animated_text(self, w)
    }

    fn encode_magic_effect(&self, w: &wire::MagicEffectWire) -> NetworkMessage {
        Codec1098::encode_magic_effect(self, w)
    }

    fn encode_distance_shoot(&self, w: &wire::DistanceShootWire) -> NetworkMessage {
        Codec1098::encode_distance_shoot(self, w)
    }

    fn encode_creature_health(&self, w: &wire::CreatureHealthWire) -> NetworkMessage {
        Codec1098::encode_creature_health(self, w)
    }

    fn encode_creature_square(&self, w: &wire::CreatureSquareWire) -> NetworkMessage {
        Codec1098::encode_creature_square(self, w)
    }

    fn encode_creature_speed(&self, w: &wire::CreatureSpeedWire) -> NetworkMessage {
        Codec1098::encode_creature_speed(self, w)
    }

    fn encode_creature_outfit(
        &self,
        creature_id: u32,
        outfit: &CreatureOutfitWire,
    ) -> NetworkMessage {
        Codec1098::encode_creature_outfit(self, creature_id, outfit)
    }

    fn encode_combat_damage_text_message(
        &self,
        w: &wire::CombatDamageNotifyWire,
    ) -> NetworkMessage {
        Codec1098::encode_combat_damage_text_message(self, w)
    }

    fn encode_creature_say(&self, statement_id: u32, w: &wire::CreatureSayWire) -> NetworkMessage {
        Codec1098::encode_creature_say(self, statement_id, w)
    }

    fn encode_to_channel(&self, statement_id: u32, w: &wire::ToChannelWire) -> NetworkMessage {
        Codec1098::encode_to_channel(self, statement_id, w)
    }

    fn encode_private_message(
        &self,
        statement_id: u32,
        w: &wire::PrivateMessageWire,
    ) -> NetworkMessage {
        Codec1098::encode_private_message(self, statement_id, w)
    }

    fn encode_channel_message(&self, w: &wire::ChannelMessageWire) -> NetworkMessage {
        Codec1098::encode_channel_message(self, w)
    }

    fn encode_channels_dialog(&self, w: &wire::ChannelsDialogWire) -> NetworkMessage {
        Codec1098::encode_channels_dialog(self, w)
    }

    fn encode_channel_open(&self, w: &wire::ChannelOpenWire) -> NetworkMessage {
        Codec1098::encode_channel_open(self, w)
    }

    fn encode_create_private_channel(&self, w: &wire::CreatePrivateChannelWire) -> NetworkMessage {
        Codec1098::encode_create_private_channel(self, w)
    }

    fn encode_text_window(&self, w: &wire::TextWindowWire) -> NetworkMessage {
        Codec1098::encode_text_window(self, w)
    }

    fn encode_house_window(&self, window_text_id: u32, text: &str) -> NetworkMessage {
        crate::outgoing_extra::send_house_window(window_text_id, text)
    }

    fn failure_message_type(&self) -> u8 {
        21 // MESSAGE_STATUS_SMALL — `src/const.h:190`
    }

    fn status_message_type(&self) -> u8 {
        21 // MESSAGE_STATUS_DEFAULT — `src/const.h:189`
    }

    fn periodic_ping_packet(&self, _is_otclient: bool) -> NetworkMessage {
        // 1098: always 0x1D (`send_ping`) — `src/protocolgame.cpp:2530`.
        crate::outgoing::send_ping()
    }
}

impl ProtocolCodec for Codec772 {
    fn caps(&self) -> ProtocolCaps {
        Codec772::caps(self)
    }

    fn write_tile_environment_prefix(&self, _msg: &mut NetworkMessage) {
        // 7.72 `GetTileDescription` (`gameserver/src/protocolgame.cpp`) has no environmental-effects field.
    }

    fn tile_environment_prefix_len(&self) -> usize {
        0
    }

    fn tile_description_caps_creatures(&self) -> bool {
        // 7.72 `GetTileDescription` (`gameserver/src/protocolgame.cpp:572-574`) — creature loop
        // returns early once `count` hits 10, matching the down-items early return.
        true
    }

    fn write_item_template(
        &self,
        msg: &mut NetworkMessage,
        client_id: u16,
        count: u8,
        stackable: bool,
        is_splash_or_fluid: bool,
        is_animation: bool,
        with_description: bool,
    ) {
        Codec772::write_item_template(
            self,
            msg,
            client_id,
            count,
            stackable,
            is_splash_or_fluid,
            is_animation,
            with_description,
        );
    }

    fn item_template_wire_len(
        &self,
        client_id: u16,
        count: u8,
        stackable: bool,
        is_splash_or_fluid: bool,
        is_animation: bool,
        with_description: bool,
    ) -> usize {
        Codec772::item_template_wire_len(
            self,
            client_id,
            count,
            stackable,
            is_splash_or_fluid,
            is_animation,
            with_description,
        )
    }

    fn write_add_creature(&self, msg: &mut NetworkMessage, c: &CreatureWire) {
        Codec772::write_add_creature(self, msg, c);
    }

    fn add_creature_wire_len(&self, c: &CreatureWire) -> usize {
        Codec772::add_creature_wire_len(self, c)
    }

    fn write_outfit(&self, msg: &mut NetworkMessage, o: &CreatureOutfitWire) {
        Codec772::write_outfit(self, msg, o);
    }

    fn encode_player_stats(&self, s: &PlayerStatsWire) -> NetworkMessage {
        Codec772::encode_player_stats(self, s)
    }

    fn encode_player_skills(&self, s: &PlayerSkillsWire) -> NetworkMessage {
        Codec772::encode_player_skills(self, s)
    }

    fn encode_basic_data(
        &self,
        is_premium: bool,
        premium_ends_at: u32,
        vocation_client_id: u8,
    ) -> NetworkMessage {
        Codec772::encode_basic_data(self, is_premium, premium_ends_at, vocation_client_id)
    }

    fn encode_self_appear_login(&self, player_id: u32, server_beat: u16) -> NetworkMessage {
        Codec772::encode_self_appear_login(self, player_id, server_beat)
    }

    fn encode_add_tile_item(
        &self,
        pos: Position,
        stack_pos: u8,
        args: ItemTemplateArgs,
        otclient_stackpos: bool,
    ) -> NetworkMessage {
        Codec772::encode_add_tile_item(self, pos, stack_pos, args, otclient_stackpos)
    }

    fn encode_update_tile_item(
        &self,
        pos: Position,
        stack_pos: u8,
        args: ItemTemplateArgs,
    ) -> NetworkMessage {
        Codec772::encode_update_tile_item(self, pos, stack_pos, args)
    }

    fn encode_inventory_item(&self, slot: u8, args: ItemTemplateArgs) -> NetworkMessage {
        Codec772::encode_inventory_item(self, slot, args)
    }

    fn encode_add_container_item(
        &self,
        cid: u8,
        slot: u16,
        args: ItemTemplateArgs,
    ) -> NetworkMessage {
        Codec772::encode_add_container_item(self, cid, slot, args)
    }

    fn encode_update_container_item(
        &self,
        cid: u8,
        slot: u16,
        args: ItemTemplateArgs,
    ) -> NetworkMessage {
        Codec772::encode_update_container_item(self, cid, slot, args)
    }

    fn encode_remove_container_item(&self, cid: u8, slot: u16) -> NetworkMessage {
        Codec772::encode_remove_container_item(self, cid, slot)
    }

    fn encode_add_tile_creature(
        &self,
        pos: Position,
        stack_pos: u8,
        wire: &CreatureWire,
        otclient_stackpos: bool,
    ) -> NetworkMessage {
        Codec772::encode_add_tile_creature(self, pos, stack_pos, wire, otclient_stackpos)
    }

    fn encode_remove_tile_thing(&self, pos: Position, stackpos: u8) -> NetworkMessage {
        Codec772::encode_remove_tile_thing(self, pos, stackpos)
    }

    fn encode_remove_tile_creature_by_id(&self, creature_id: u32) -> NetworkMessage {
        Codec772::encode_remove_tile_creature_by_id(self, creature_id)
    }

    fn encode_creature_light(
        &self,
        creature_id: u32,
        level: u8,
        color: u8,
        access_player: bool,
    ) -> NetworkMessage {
        Codec772::encode_creature_light(self, creature_id, level, color, access_player)
    }

    fn encode_creature_turn(
        &self,
        creature_id: u32,
        stack_pos: u8,
        tile_pos: Position,
        direction: u8,
        can_walkthrough: bool,
    ) -> NetworkMessage {
        Codec772::encode_creature_turn(
            self,
            creature_id,
            stack_pos,
            tile_pos,
            direction,
            can_walkthrough,
        )
    }

    fn encode_cancel_walk(&self, direction: u8) -> NetworkMessage {
        Codec772::encode_cancel_walk(self, direction)
    }

    fn encode_clear_target(&self) -> NetworkMessage {
        Codec772::encode_clear_target(self)
    }

    fn encode_container_open(&self, c: &ContainerOpenWire) -> NetworkMessage {
        Codec772::encode_container_open(self, c)
    }

    fn encode_animated_text(&self, w: &wire::AnimatedTextWire) -> NetworkMessage {
        Codec772::encode_animated_text(self, w)
    }

    fn encode_magic_effect(&self, w: &wire::MagicEffectWire) -> NetworkMessage {
        Codec772::encode_magic_effect(self, w)
    }

    fn encode_distance_shoot(&self, w: &wire::DistanceShootWire) -> NetworkMessage {
        Codec772::encode_distance_shoot(self, w)
    }

    fn encode_creature_health(&self, w: &wire::CreatureHealthWire) -> NetworkMessage {
        Codec772::encode_creature_health(self, w)
    }

    fn encode_creature_square(&self, w: &wire::CreatureSquareWire) -> NetworkMessage {
        Codec772::encode_creature_square(self, w)
    }

    fn encode_creature_speed(&self, w: &wire::CreatureSpeedWire) -> NetworkMessage {
        Codec772::encode_creature_speed(self, w)
    }

    fn encode_creature_outfit(
        &self,
        creature_id: u32,
        outfit: &CreatureOutfitWire,
    ) -> NetworkMessage {
        Codec772::encode_creature_outfit(self, creature_id, outfit)
    }

    fn encode_combat_damage_text_message(
        &self,
        w: &wire::CombatDamageNotifyWire,
    ) -> NetworkMessage {
        Codec772::encode_combat_damage_text_message(self, w)
    }

    fn encode_creature_say(&self, statement_id: u32, w: &wire::CreatureSayWire) -> NetworkMessage {
        Codec772::encode_creature_say(self, statement_id, w)
    }

    fn encode_to_channel(&self, statement_id: u32, w: &wire::ToChannelWire) -> NetworkMessage {
        Codec772::encode_to_channel(self, statement_id, w)
    }

    fn encode_private_message(
        &self,
        statement_id: u32,
        w: &wire::PrivateMessageWire,
    ) -> NetworkMessage {
        Codec772::encode_private_message(self, statement_id, w)
    }

    fn encode_channel_message(&self, w: &wire::ChannelMessageWire) -> NetworkMessage {
        Codec772::encode_channel_message(self, w)
    }

    fn encode_channels_dialog(&self, w: &wire::ChannelsDialogWire) -> NetworkMessage {
        Codec772::encode_channels_dialog(self, w)
    }

    fn encode_channel_open(&self, w: &wire::ChannelOpenWire) -> NetworkMessage {
        Codec772::encode_channel_open(self, w)
    }

    fn encode_create_private_channel(&self, w: &wire::CreatePrivateChannelWire) -> NetworkMessage {
        Codec772::encode_create_private_channel(self, w)
    }

    fn encode_text_window(&self, w: &wire::TextWindowWire) -> NetworkMessage {
        Codec772::encode_text_window(self, w)
    }

    fn encode_house_window(&self, window_text_id: u32, text: &str) -> NetworkMessage {
        crate::outgoing_extra::send_house_window(window_text_id, text)
    }

    fn failure_message_type(&self) -> u8 {
        23 // TALK_FAILURE_MESSAGE — `sending.cc:339`, `enums.hh:674`
    }

    fn status_message_type(&self) -> u8 {
        21 // TALK_STATUS_MESSAGE — `enums.hh:672`
    }

    fn periodic_ping_packet(&self, is_otclient: bool) -> NetworkMessage {
        // TVP 772 (`protocolgame.cpp:1516`): 0x1E for non-OTClient, 0x1D for OTClient.
        if is_otclient {
            crate::outgoing::send_ping()
        } else {
            crate::outgoing::send_ping_back()
        }
    }
}

/// Zero-cost dispatcher for the active wire codec (A5: `V1098` + `V772`).
#[derive(Debug, Clone, Copy)]
pub enum Codec {
    V1098(Codec1098),
    V772(Codec772),
}

impl Codec {
    pub fn from_version(v: ProtocolVersion) -> Result<Self, String> {
        match v.raw() {
            1098 => Ok(Self::V1098(Codec1098)),
            772 => Ok(Self::V772(Codec772)),
            other => Err(format!(
                "unsupported clientVersion `{other}` for wire codec (supported: 772, 1098)"
            )),
        }
    }

    pub fn caps(&self) -> ProtocolCaps {
        match self {
            Self::V1098(c) => c.caps(),
            Self::V772(c) => c.caps(),
        }
    }
}

macro_rules! delegate_codec {
    ($($name:ident ( $($arg:ident : $ty:ty),* $(,)? ) -> $ret:ty);+ $(;)?) => {
        $(
            // Delegated wire encoders mirror C++ `NetworkMessage` field lists (parity); arg count
            // matches the trait method it forwards to.
            #[allow(clippy::too_many_arguments)]
            pub fn $name(&self, $($arg: $ty),*) -> $ret {
                match self {
                    Self::V1098(c) => ProtocolCodec::$name(c, $($arg),*),
                    Self::V772(c) => ProtocolCodec::$name(c, $($arg),*),
                }
            }
        )+
    };
}

impl Codec {
    delegate_codec! {
        write_tile_environment_prefix(msg: &mut NetworkMessage) -> ();

        tile_environment_prefix_len() -> usize;

        tile_description_caps_creatures() -> bool;

        write_item_template(
            msg: &mut NetworkMessage,
            client_id: u16,
            count: u8,
            stackable: bool,
            is_splash_or_fluid: bool,
            is_animation: bool,
            with_description: bool,
        ) -> ();

        item_template_wire_len(
            client_id: u16,
            count: u8,
            stackable: bool,
            is_splash_or_fluid: bool,
            is_animation: bool,
            with_description: bool,
        ) -> usize;

        write_add_creature(msg: &mut NetworkMessage, c: &CreatureWire) -> ();

        add_creature_wire_len(c: &CreatureWire) -> usize;

        write_outfit(msg: &mut NetworkMessage, o: &CreatureOutfitWire) -> ();

        encode_player_stats(s: &PlayerStatsWire) -> NetworkMessage;

        encode_player_skills(s: &PlayerSkillsWire) -> NetworkMessage;

        encode_basic_data(
            is_premium: bool,
            premium_ends_at: u32,
            vocation_client_id: u8,
        ) -> NetworkMessage;

        encode_self_appear_login(player_id: u32, server_beat: u16) -> NetworkMessage;

        encode_add_tile_item(
            pos: Position,
            stack_pos: u8,
            args: ItemTemplateArgs,
            otclient_stackpos: bool,
        ) -> NetworkMessage;

        encode_update_tile_item(pos: Position, stack_pos: u8, args: ItemTemplateArgs) -> NetworkMessage;

        encode_inventory_item(slot: u8, args: ItemTemplateArgs) -> NetworkMessage;

        encode_add_container_item(cid: u8, slot: u16, args: ItemTemplateArgs) -> NetworkMessage;

        encode_update_container_item(cid: u8, slot: u16, args: ItemTemplateArgs) -> NetworkMessage;

        encode_remove_container_item(cid: u8, slot: u16) -> NetworkMessage;

        encode_add_tile_creature(
            pos: Position,
            stack_pos: u8,
            wire: &CreatureWire,
            otclient_stackpos: bool,
        ) -> NetworkMessage;

        encode_remove_tile_thing(pos: Position, stackpos: u8) -> NetworkMessage;

        encode_remove_tile_creature_by_id(creature_id: u32) -> NetworkMessage;

        encode_creature_light(
            creature_id: u32,
            level: u8,
            color: u8,
            access_player: bool,
        ) -> NetworkMessage;

        encode_creature_turn(
            creature_id: u32,
            stack_pos: u8,
            tile_pos: Position,
            direction: u8,
            can_walkthrough: bool,
        ) -> NetworkMessage;

        encode_cancel_walk(direction: u8) -> NetworkMessage;

        encode_clear_target() -> NetworkMessage;

        encode_container_open(c: &ContainerOpenWire) -> NetworkMessage;

        encode_animated_text(w: &wire::AnimatedTextWire) -> NetworkMessage;

        encode_magic_effect(w: &wire::MagicEffectWire) -> NetworkMessage;

        encode_distance_shoot(w: &wire::DistanceShootWire) -> NetworkMessage;

        encode_creature_health(w: &wire::CreatureHealthWire) -> NetworkMessage;

        encode_creature_square(w: &wire::CreatureSquareWire) -> NetworkMessage;

        encode_creature_speed(w: &wire::CreatureSpeedWire) -> NetworkMessage;

        encode_creature_outfit(creature_id: u32, outfit: &CreatureOutfitWire) -> NetworkMessage;

        encode_combat_damage_text_message(w: &wire::CombatDamageNotifyWire) -> NetworkMessage;

        encode_creature_say(statement_id: u32, w: &wire::CreatureSayWire) -> NetworkMessage;

        encode_to_channel(statement_id: u32, w: &wire::ToChannelWire) -> NetworkMessage;

        encode_private_message(statement_id: u32, w: &wire::PrivateMessageWire) -> NetworkMessage;

        encode_channel_message(w: &wire::ChannelMessageWire) -> NetworkMessage;

        encode_channels_dialog(w: &wire::ChannelsDialogWire) -> NetworkMessage;

        encode_channel_open(w: &wire::ChannelOpenWire) -> NetworkMessage;

        encode_create_private_channel(w: &wire::CreatePrivateChannelWire) -> NetworkMessage;

        encode_text_window(w: &wire::TextWindowWire) -> NetworkMessage;

        encode_house_window(window_text_id: u32, text: &str) -> NetworkMessage;

        failure_message_type() -> u8;

        status_message_type() -> u8;

        periodic_ping_packet(is_otclient: bool) -> NetworkMessage;
    }
}

impl ProtocolCodec for Codec {
    fn caps(&self) -> ProtocolCaps {
        Codec::caps(self)
    }

    fn write_tile_environment_prefix(&self, msg: &mut NetworkMessage) {
        Codec::write_tile_environment_prefix(self, msg);
    }

    fn tile_environment_prefix_len(&self) -> usize {
        Codec::tile_environment_prefix_len(self)
    }

    fn tile_description_caps_creatures(&self) -> bool {
        Codec::tile_description_caps_creatures(self)
    }

    fn write_item_template(
        &self,
        msg: &mut NetworkMessage,
        client_id: u16,
        count: u8,
        stackable: bool,
        is_splash_or_fluid: bool,
        is_animation: bool,
        with_description: bool,
    ) {
        Codec::write_item_template(
            self,
            msg,
            client_id,
            count,
            stackable,
            is_splash_or_fluid,
            is_animation,
            with_description,
        );
    }

    fn item_template_wire_len(
        &self,
        client_id: u16,
        count: u8,
        stackable: bool,
        is_splash_or_fluid: bool,
        is_animation: bool,
        with_description: bool,
    ) -> usize {
        Codec::item_template_wire_len(
            self,
            client_id,
            count,
            stackable,
            is_splash_or_fluid,
            is_animation,
            with_description,
        )
    }

    fn write_add_creature(&self, msg: &mut NetworkMessage, c: &CreatureWire) {
        Codec::write_add_creature(self, msg, c);
    }

    fn add_creature_wire_len(&self, c: &CreatureWire) -> usize {
        Codec::add_creature_wire_len(self, c)
    }

    fn write_outfit(&self, msg: &mut NetworkMessage, o: &CreatureOutfitWire) {
        Codec::write_outfit(self, msg, o);
    }

    fn encode_player_stats(&self, s: &PlayerStatsWire) -> NetworkMessage {
        Codec::encode_player_stats(self, s)
    }

    fn encode_player_skills(&self, s: &PlayerSkillsWire) -> NetworkMessage {
        Codec::encode_player_skills(self, s)
    }

    fn encode_basic_data(
        &self,
        is_premium: bool,
        premium_ends_at: u32,
        vocation_client_id: u8,
    ) -> NetworkMessage {
        Codec::encode_basic_data(self, is_premium, premium_ends_at, vocation_client_id)
    }

    fn encode_self_appear_login(&self, player_id: u32, server_beat: u16) -> NetworkMessage {
        Codec::encode_self_appear_login(self, player_id, server_beat)
    }

    fn encode_add_tile_item(
        &self,
        pos: Position,
        stack_pos: u8,
        args: ItemTemplateArgs,
        otclient_stackpos: bool,
    ) -> NetworkMessage {
        Codec::encode_add_tile_item(self, pos, stack_pos, args, otclient_stackpos)
    }

    fn encode_update_tile_item(
        &self,
        pos: Position,
        stack_pos: u8,
        args: ItemTemplateArgs,
    ) -> NetworkMessage {
        Codec::encode_update_tile_item(self, pos, stack_pos, args)
    }

    fn encode_inventory_item(&self, slot: u8, args: ItemTemplateArgs) -> NetworkMessage {
        Codec::encode_inventory_item(self, slot, args)
    }

    fn encode_add_container_item(
        &self,
        cid: u8,
        slot: u16,
        args: ItemTemplateArgs,
    ) -> NetworkMessage {
        Codec::encode_add_container_item(self, cid, slot, args)
    }

    fn encode_update_container_item(
        &self,
        cid: u8,
        slot: u16,
        args: ItemTemplateArgs,
    ) -> NetworkMessage {
        Codec::encode_update_container_item(self, cid, slot, args)
    }

    fn encode_remove_container_item(&self, cid: u8, slot: u16) -> NetworkMessage {
        Codec::encode_remove_container_item(self, cid, slot)
    }

    fn encode_add_tile_creature(
        &self,
        pos: Position,
        stack_pos: u8,
        wire: &CreatureWire,
        otclient_stackpos: bool,
    ) -> NetworkMessage {
        Codec::encode_add_tile_creature(self, pos, stack_pos, wire, otclient_stackpos)
    }

    fn encode_remove_tile_thing(&self, pos: Position, stackpos: u8) -> NetworkMessage {
        Codec::encode_remove_tile_thing(self, pos, stackpos)
    }

    fn encode_remove_tile_creature_by_id(&self, creature_id: u32) -> NetworkMessage {
        Codec::encode_remove_tile_creature_by_id(self, creature_id)
    }

    fn encode_creature_light(
        &self,
        creature_id: u32,
        level: u8,
        color: u8,
        access_player: bool,
    ) -> NetworkMessage {
        Codec::encode_creature_light(self, creature_id, level, color, access_player)
    }

    fn encode_creature_turn(
        &self,
        creature_id: u32,
        stack_pos: u8,
        tile_pos: Position,
        direction: u8,
        can_walkthrough: bool,
    ) -> NetworkMessage {
        Codec::encode_creature_turn(
            self,
            creature_id,
            stack_pos,
            tile_pos,
            direction,
            can_walkthrough,
        )
    }

    fn encode_cancel_walk(&self, direction: u8) -> NetworkMessage {
        Codec::encode_cancel_walk(self, direction)
    }

    fn encode_clear_target(&self) -> NetworkMessage {
        Codec::encode_clear_target(self)
    }

    fn encode_container_open(&self, c: &ContainerOpenWire) -> NetworkMessage {
        Codec::encode_container_open(self, c)
    }

    fn encode_animated_text(&self, w: &wire::AnimatedTextWire) -> NetworkMessage {
        Codec::encode_animated_text(self, w)
    }

    fn encode_magic_effect(&self, w: &wire::MagicEffectWire) -> NetworkMessage {
        Codec::encode_magic_effect(self, w)
    }

    fn encode_distance_shoot(&self, w: &wire::DistanceShootWire) -> NetworkMessage {
        Codec::encode_distance_shoot(self, w)
    }

    fn encode_creature_health(&self, w: &wire::CreatureHealthWire) -> NetworkMessage {
        Codec::encode_creature_health(self, w)
    }

    fn encode_creature_square(&self, w: &wire::CreatureSquareWire) -> NetworkMessage {
        Codec::encode_creature_square(self, w)
    }

    fn encode_creature_speed(&self, w: &wire::CreatureSpeedWire) -> NetworkMessage {
        match self {
            Self::V1098(c) => ProtocolCodec::encode_creature_speed(c, w),
            Self::V772(c) => ProtocolCodec::encode_creature_speed(c, w),
        }
    }

    fn encode_creature_outfit(
        &self,
        creature_id: u32,
        outfit: &CreatureOutfitWire,
    ) -> NetworkMessage {
        match self {
            Self::V1098(c) => ProtocolCodec::encode_creature_outfit(c, creature_id, outfit),
            Self::V772(c) => ProtocolCodec::encode_creature_outfit(c, creature_id, outfit),
        }
    }

    fn encode_combat_damage_text_message(
        &self,
        w: &wire::CombatDamageNotifyWire,
    ) -> NetworkMessage {
        Codec::encode_combat_damage_text_message(self, w)
    }

    fn encode_creature_say(&self, statement_id: u32, w: &wire::CreatureSayWire) -> NetworkMessage {
        Codec::encode_creature_say(self, statement_id, w)
    }

    fn encode_to_channel(&self, statement_id: u32, w: &wire::ToChannelWire) -> NetworkMessage {
        Codec::encode_to_channel(self, statement_id, w)
    }

    fn encode_private_message(
        &self,
        statement_id: u32,
        w: &wire::PrivateMessageWire,
    ) -> NetworkMessage {
        Codec::encode_private_message(self, statement_id, w)
    }

    fn encode_channel_message(&self, w: &wire::ChannelMessageWire) -> NetworkMessage {
        Codec::encode_channel_message(self, w)
    }

    fn encode_channels_dialog(&self, w: &wire::ChannelsDialogWire) -> NetworkMessage {
        Codec::encode_channels_dialog(self, w)
    }

    fn encode_channel_open(&self, w: &wire::ChannelOpenWire) -> NetworkMessage {
        Codec::encode_channel_open(self, w)
    }

    fn encode_create_private_channel(&self, w: &wire::CreatePrivateChannelWire) -> NetworkMessage {
        Codec::encode_create_private_channel(self, w)
    }

    fn encode_text_window(&self, w: &wire::TextWindowWire) -> NetworkMessage {
        Codec::encode_text_window(self, w)
    }

    fn encode_house_window(&self, window_text_id: u32, text: &str) -> NetworkMessage {
        Codec::encode_house_window(self, window_text_id, text)
    }

    fn failure_message_type(&self) -> u8 {
        Codec::failure_message_type(self)
    }

    fn status_message_type(&self) -> u8 {
        Codec::status_message_type(self)
    }

    fn periodic_ping_packet(&self, is_otclient: bool) -> NetworkMessage {
        Codec::periodic_ping_packet(self, is_otclient)
    }
}
