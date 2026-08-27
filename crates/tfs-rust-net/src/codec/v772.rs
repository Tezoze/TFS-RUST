//! 7.72 (TVP / "The Violet Project") wire encoder — Phase A5.
//!
//! C++ reference (772 wire — `gameserver/src/` ONLY; never repo-root `src/` or `tibia-game-master`):
//! - `networkmessage.cpp` `NetworkMessage::addItem(uint16_t,uint8_t)` (~L82) — 2-byte min, no MARK /
//!   animation / description / duration; fluid via `tools.cpp` `getLiquidColor`.
//! - `protocolgame.cpp` `AddCreature` (~L2051), `AddPlayerStats` (~L2090), `AddPlayerSkills` (~L2118),
//!   `AddOutfit` (~L2128), `AddCreatureLight` (~L2149), `sendAddTileItem` / `sendUpdateTileItem` /
//!   `sendRemoveTileThing` (~L1591), `sendAddContainerItem` / `sendUpdateContainerItem` (~L1871),
//!   `sendInventoryItem` (~L1857), `sendAddCreature` self branch / self-appear `0x0A` (~L1694),
//!   `sendCreatureTurn` (~L1768), `sendCancelWalk` (~L1503), `RemoveTileThing` (~L2161).
//!
//! Standalone `0x6A` is opcode + position + thing (no stackpos byte). Real 7.72 clients place by
//! stack priority; OTCv8 `GameTileAddThingWithStackpos` is version >= 841 only. `otclient_stackpos`
//! on the codec API is always false for 772 (TVP `gameserver/` optional byte is not 7.72 wire).

use tfs_rust_common::protocol_opcodes::server;
use tfs_rust_common::{Position, ProtocolCaps, ProtocolVersion};

use crate::NetworkMessage;
use crate::creature_encode::{AddCreatureWire, OutfitWire};

use super::wire::{
    AnimatedTextWire, ChannelMessageWire, ChannelOpenWire, ChannelsDialogWire,
    CombatDamageNotifyWire, CreatePrivateChannelWire, CreatureHealthWire, CreatureSayWire,
    CreatureSpeedWire, CreatureSquareWire, DistanceShootWire, ItemTemplateArgs, MagicEffectWire,
    PlayerSkillsWire, PlayerStatsWire, PrivateMessageWire, TextWindowWire, ToChannelWire,
};

/// Zero-sized 7.72 codec (stateless; caps from `ProtocolVersion::V772`).
#[derive(Debug, Clone, Copy, Default)]
pub struct Codec772;

/// 7.72 `getLiquidColor` (`gameserver/src/tools.cpp` ~L20). **Not** the 10.x `FLUID_MAP` table — the
/// 7.72 client uses a different liquid-color palette mapping.
fn liquid_color(fluid_type: u8) -> u8 {
    match fluid_type {
        1 => 1,
        0 => 0,
        6 => 4,
        3 | 4 | 7 => 3,
        9 => 6,
        2 | 10 => 7,
        5 | 11 => 2,
        8 | 12 => 5,
        _ => 0,
    }
}

impl Codec772 {
    pub fn caps(&self) -> ProtocolCaps {
        ProtocolVersion::V772.caps()
    }

    /// 7.72 `NetworkMessage::addItem` template field list: `u16 clientId` + (stackable → `u8 count`)
    /// / (splash|fluid → `u8 getLiquidColor`). No MARK, animation, description, or duration.
    #[allow(clippy::too_many_arguments)] // mirrors C++ `NetworkMessage::addItem` field list (parity)
    pub fn write_item_template(
        &self,
        msg: &mut NetworkMessage,
        client_id: u16,
        count: u8,
        stackable: bool,
        is_splash_or_fluid: bool,
        _is_animation: bool,
        _with_description: bool,
    ) {
        msg.write_u16(client_id);
        if stackable {
            msg.write_u8(count);
        } else if is_splash_or_fluid {
            msg.write_u8(liquid_color(count));
        }
    }

    fn write_item_template_args(&self, msg: &mut NetworkMessage, args: ItemTemplateArgs) {
        self.write_item_template(
            msg,
            args.client_id,
            args.count,
            args.stackable,
            args.is_splash_or_fluid,
            args.is_animation,
            args.with_description,
        );
    }

    /// Byte length of [`Codec772::write_item_template`] for the same arguments.
    pub fn item_template_wire_len(
        &self,
        _client_id: u16,
        _count: u8,
        stackable: bool,
        is_splash_or_fluid: bool,
        _is_animation: bool,
        _with_description: bool,
    ) -> usize {
        // client id (2) + optional count/liquid byte (1).
        2 + usize::from(stackable || is_splash_or_fluid)
    }

    /// 7.72 map-object creature block — decompile `SendMapObject` (`sending.cc` ~217–268).
    ///
    /// - UPTODATE: `SendWord(99)` → `0x63` + id + direction only.
    /// - OUTDATED: `SendWord(98)` → `0x62` + id + HP/outfit/light/speed/skull/party (no name).
    /// - FREE: `SendWord(97)` → `0x61` + removeId + id + name + same tail as 0x62.
    ///
    /// TVP `AddCreature` has no 0x63; 1098 stays on `creature_encode::write_add_creature`.
    pub fn write_add_creature(&self, msg: &mut NetworkMessage, c: &AddCreatureWire) {
        if c.known && c.uptodate {
            msg.write_u16(0x63);
            msg.write_u32(c.id);
            msg.write_u8(c.direction);
            return;
        }
        if c.known {
            msg.write_u16(0x62);
            msg.write_u32(c.id);
        } else {
            msg.write_u16(0x61);
            msg.write_u32(c.remove_known);
            msg.write_u32(c.id);
            msg.write_string(&c.name);
        }

        msg.write_u8(c.health_percent);
        msg.write_u8(c.direction);
        self.write_outfit(msg, &c.outfit);

        // 7.72 `AddCreature` writes the raw creature light (no access-player `0xFF` substitution).
        msg.write_u8(c.light_level);
        msg.write_u8(c.light_color);

        msg.write_u16(c.step_speed);
        msg.write_u8(c.skull);
        msg.write_u8(c.party_shield);
    }

    /// Byte length of [`Codec772::write_add_creature`].
    pub fn add_creature_wire_len(&self, c: &AddCreatureWire) -> usize {
        if c.known && c.uptodate {
            return 2 + 4 + 1;
        }
        let head = if c.known {
            2 + 4
        } else {
            2 + 4 + 4 + 2 + c.name.len()
        };
        // health + direction + outfit + light(2) + speed(2) + skull + party shield
        head + 1 + 1 + self.outfit_wire_len(&c.outfit) + 2 + 2 + 1 + 1
    }

    /// 7.72 `ProtocolGame::AddOutfit` (~L2128): no addons byte, no trailing mount. `lookType == 0`
    /// path writes `addItemId(lookTypeEx)` (a `u16` client id, already resolved in the neutral wire).
    pub fn write_outfit(&self, msg: &mut NetworkMessage, o: &OutfitWire) {
        msg.write_u16(o.look_type);
        if o.look_type != 0 {
            msg.write_u8(o.look_head);
            msg.write_u8(o.look_body);
            msg.write_u8(o.look_legs);
            msg.write_u8(o.look_feet);
        } else {
            msg.write_u16(o.look_type_ex);
        }
    }

    fn outfit_wire_len(&self, o: &OutfitWire) -> usize {
        2 + if o.look_type != 0 { 4 } else { 2 }
    }

    /// 7.72 `ProtocolGame::AddPlayerStats` opcode `0xA0` (~L2090): `u16` capacity (`free/100`),
    /// `u32` experience, `u8` magic level + `u8`%; no base-magic / stamina / speed / training block.
    pub fn encode_player_stats(&self, s: &PlayerStatsWire) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0xA0);
        m.write_u16(s.health);
        m.write_u16(s.max_health);
        // C++ `static_cast<uint16_t>(getFreeCapacity() / 100.)` — neutral cap is centi-oz.
        m.write_u16((s.free_capacity / 100).min(u16::MAX as u32) as u16);
        // C++ writes 0 when experience would overflow `uint32_t`.
        if s.experience >= u32::MAX as u64 - 1 {
            m.write_u32(0);
        } else {
            m.write_u32(s.experience as u32);
        }
        m.write_u16(s.level);
        m.write_u8(s.level_percent);
        m.write_u16(s.mana);
        m.write_u16(s.max_mana);
        m.write_u8(s.magic_level);
        m.write_u8(s.magic_level_percent);
        m.write_u8(s.soul);
        m
    }

    /// 7.72 `ProtocolGame::AddPlayerSkills` opcode `0xA1` (~L2118): 7 skills × (`u8` level + `u8`%).
    pub fn encode_player_skills(&self, s: &PlayerSkillsWire) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0xA1);
        for i in 0..7 {
            // C++ `addByte(min<int32_t>(getSkillLevel(i), u16::MAX))` truncates to the low byte.
            m.write_u8(s.levels[i] as u8);
            m.write_u8(s.percents[i]);
        }
        m
    }

    /// 7.72 has no `sendBasicData` (`0x9F` is a 10.x packet). Returns an empty message — skipped by
    /// `enqueue_encoded` so nothing is written to the wire.
    pub fn encode_basic_data(
        &self,
        _is_premium: bool,
        _premium_ends_at: u32,
        _vocation_client_id: u8,
    ) -> NetworkMessage {
        NetworkMessage::new()
    }

    /// 7.72 self-appear (`gameserver/src/protocolgame.cpp` `sendAddCreature` self branch ~L1730):
    /// `0x0A` + `u32 id` + `u16` beat + `u8` canReportBugs. Opcode is version-keyed via
    /// `protocol_opcodes::server::self_appear`. `canReportBugs` defaults to 0 (non-tutor) — account
    /// type is not threaded into this neutral signature.
    ///
    /// `server_beat` is the beat duration in ms advertised to the client. TVP `gameserver` hardcodes
    /// `0x32` (50); the 772 mechanics decompile (`tibia-game-master/src/config.cc:102`) defaults
    /// `Beat = 200` and exposes it via `data/formulas/772.lua` `beatMs`. We send the profile value so
    /// the client walk clock matches the server's beat loop (`game_loop.rs` uses the same `beat_ms`).
    pub fn encode_self_appear_login(&self, player_id: u32, server_beat: u16) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(tfs_rust_common::protocol_opcodes::server::self_appear(
            ProtocolVersion::V772,
        ));
        m.write_u32(player_id);
        m.write_u16(server_beat);
        m.write_u8(0x00); // canReportBugs (ACCOUNT_TYPE_TUTOR+) — default off
        m
    }

    /// 7.72 `sendAddTileItem` opcode `0x6A` (~L1591).
    pub fn encode_add_tile_item(
        &self,
        pos: Position,
        _stack_pos: u8,
        args: ItemTemplateArgs,
        _otclient_stackpos: bool,
    ) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0x6A);
        m.write_position(&pos);
        self.write_item_template_args(&mut m, args);
        m
    }

    /// 7.72 `sendUpdateTileItem` opcode `0x6B` (~L1607): position + `u8` stackpos + item.
    pub fn encode_update_tile_item(
        &self,
        pos: Position,
        stack_pos: u8,
        args: ItemTemplateArgs,
    ) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0x6B);
        m.write_position(&pos);
        m.write_u8(stack_pos);
        self.write_item_template_args(&mut m, args);
        m
    }

    /// 7.72 `sendInventoryItem` opcode `0x78` (~L1857): `u8` slot + item.
    pub fn encode_inventory_item(&self, slot: u8, args: ItemTemplateArgs) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0x78);
        m.write_u8(slot);
        self.write_item_template_args(&mut m, args);
        m
    }

    /// 7.72 `sendAddContainerItem` opcode `0x70` (~L1871): cid + item. **No slot index** (10.x adds `u16`).
    pub fn encode_add_container_item(
        &self,
        cid: u8,
        _slot: u16,
        args: ItemTemplateArgs,
    ) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0x70);
        m.write_u8(cid);
        self.write_item_template_args(&mut m, args);
        m
    }

    /// 7.72 `sendUpdateContainerItem` opcode `0x71` (~L1880): cid + `u8` slot + item (10.x uses `u16`).
    /// Slots >= 36 are not addressable by the 7.72 client (`sending.cc:12`, `:792`), so drop them.
    pub fn encode_update_container_item(
        &self,
        cid: u8,
        slot: u16,
        args: ItemTemplateArgs,
    ) -> NetworkMessage {
        if slot >= 36 {
            return NetworkMessage::new();
        }
        let mut m = NetworkMessage::new();
        m.write_u8(0x71);
        m.write_u8(cid);
        m.write_u8(slot as u8);
        self.write_item_template_args(&mut m, args);
        m
    }

    /// 7.72 `sendRemoveContainerItem` opcode `0x72` (~L1890): cid + `u8` slot.
    /// TVP uses a single byte slot; 10.98 widens the slot to `u16` and appends an item.
    /// Slots >= 36 are not addressable by the 7.72 client (`sending.cc:772`), so drop them.
    pub fn encode_remove_container_item(&self, cid: u8, slot: u16) -> NetworkMessage {
        if slot >= 36 {
            return NetworkMessage::new();
        }
        let mut m = NetworkMessage::new();
        m.write_u8(0x72);
        m.write_u8(cid);
        m.write_u8(slot as u8);
        m
    }

    /// 7.72 `sendAddCreature` non-self branch opcode `0x6A` (~L1717).
    pub fn encode_add_tile_creature(
        &self,
        pos: Position,
        _stack_pos: u8,
        wire: &AddCreatureWire,
        _otclient_stackpos: bool,
    ) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0x6A);
        m.write_position(&pos);
        self.write_add_creature(&mut m, wire);
        m
    }

    /// 7.72 `RemoveTileThing` opcode `0x6C` (~L2161): position + `u8` stackpos.
    /// Silently returns an empty message when `stackpos >= 10` — C++ `RemoveTileThing`
    /// (`protocolgame.cpp:2162`) gates on `stackpos < 10`; items beyond the 10-slot
    /// client tile stack are invisible and must not be removed.
    pub fn encode_remove_tile_thing(&self, pos: Position, stackpos: u8) -> NetworkMessage {
        if stackpos >= 10 {
            return NetworkMessage::new();
        }
        let mut m = NetworkMessage::new();
        m.write_u8(0x6C);
        m.write_position(&pos);
        m.write_u8(stackpos);
        m
    }

    /// 7.72 has **no** by-id tile removal (`sendRemoveTileCreature` returns early when `stackpos >= 10`).
    /// Returns an empty message — skipped by `enqueue_encoded`.
    pub fn encode_remove_tile_creature_by_id(&self, _creature_id: u32) -> NetworkMessage {
        NetworkMessage::new()
    }

    /// 7.72 `ProtocolGame::AddCreatureLight` opcode `0x8D` (~L2149): id + `u8` level + `u8` color.
    /// Writes the raw light level (no access-player `0xFF` substitution).
    pub fn encode_creature_light(
        &self,
        creature_id: u32,
        level: u8,
        color: u8,
        _access_player: bool,
    ) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0x8D);
        m.write_u32(creature_id);
        m.write_u8(level);
        m.write_u8(color);
        m
    }

    /// 7.72 `ProtocolGame::sendCreatureTurn` opcode `0x6B` (~L1768): position + `u8` stackpos +
    /// `u16 0x63` + `u32` id + `u8` direction. No `0xFFFF` by-id branch, no walkthrough byte (10.x only).
    pub fn encode_creature_turn(
        &self,
        creature_id: u32,
        stack_pos: u8,
        tile_pos: Position,
        direction: u8,
        _can_walkthrough: bool,
    ) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0x6B);
        m.write_position(&tile_pos);
        m.write_u8(stack_pos);
        m.write_u16(0x63);
        m.write_u32(creature_id);
        m.write_u8(direction);
        m
    }

    /// 7.72 `ProtocolGame::sendCancelWalk` opcode `0xB5` (~L1503): `u8` direction.
    pub fn encode_cancel_walk(&self, direction: u8) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0xB5);
        m.write_u8(direction);
        m
    }

    /// 7.72 `ProtocolGame::sendCancelTarget` opcode `0xA3` (~L1485-1490): single byte.
    pub fn encode_clear_target(&self) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0xA3);
        m
    }

    /// 7.72 `ProtocolGame::sendContainer` opcode `0x6E` (~L1326): cid + container item + name +
    /// `u8` capacity + `u8` hasParent + `u8` count + items. No unlock / pagination / `u16` size /
    /// firstIndex (all 10.x additions). 7.72 never paginates, so `items` is the leading slice and
    /// `count = min(capacity, size, 0xFF)`.
    pub fn encode_container_open(&self, c: &super::wire::ContainerOpenWire) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0x6E);
        m.write_u8(c.cid);
        self.write_item_template_args(&mut m, c.header_item);
        m.write_string(&c.name);
        m.write_u8(c.capacity);
        m.write_u8(u8::from(c.has_parent));
        // 7.72 clients can only address the first 36 container slots (`sending.cc:12`, `:717`).
        let n = c
            .items
            .len()
            .min(c.capacity as usize)
            .min(36)
            .min(u8::MAX as usize) as u8;
        m.write_u8(n);
        for args in c.items.iter().take(n as usize) {
            self.write_item_template_args(&mut m, *args);
        }
        m
    }

    /// 7.72 `ProtocolGame::sendAnimatedText` — opcode [`server::ANIMATED_TEXT`] (~1255).
    pub fn encode_animated_text(&self, w: &AnimatedTextWire) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(server::ANIMATED_TEXT);
        m.write_position(&w.pos);
        m.write_u8(w.color);
        m.write_string(&w.text);
        m
    }

    /// 7.72 `ProtocolGame::sendMagicEffect` — opcode [`server::MAGIC_EFFECT`].
    pub fn encode_magic_effect(&self, w: &MagicEffectWire) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(server::MAGIC_EFFECT);
        m.write_position(&w.pos);
        m.write_u8(w.effect_id);
        m
    }

    /// 7.72 `ProtocolGame::sendDistanceShoot` — opcode [`server::DISTANCE_SHOOT`] (~1535).
    pub fn encode_distance_shoot(&self, w: &DistanceShootWire) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(server::DISTANCE_SHOOT);
        m.write_position(&w.from);
        m.write_position(&w.to);
        m.write_u8(w.shoot_type);
        m
    }

    /// 7.72 `ProtocolGame::sendCreatureHealth` — opcode [`server::CREATURE_HEALTH`].
    pub fn encode_creature_health(&self, w: &CreatureHealthWire) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(server::CREATURE_HEALTH);
        m.write_u32(w.creature_id);
        m.write_u8(w.health_percent);
        m
    }

    /// 7.72 `SendMarkCreature` / `sendCreatureSquare` — `sending.cc:962`, TVP `0x86` + id + color.
    pub fn encode_creature_square(&self, w: &CreatureSquareWire) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0x86);
        m.write_u32(w.creature_id);
        m.write_u8(w.color);
        m
    }

    /// 7.72 `SendCreatureSpeed` — `sending.cc:1028-1043`. Single `u16` `GetSpeed()` value.
    /// C++ reference: `sending.cc:1039-1041` `SendByte(SV_CMD_CREATURE_SPEED) + SendQuad(id) + SendWord(GetSpeed())`.
    pub fn encode_creature_speed(&self, w: &CreatureSpeedWire) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(server::CREATURE_SPEED);
        m.write_u32(w.creature_id);
        m.write_u16(w.speed);
        m
    }

    /// 7.72 `ProtocolGame::sendCreatureOutfit` — `gameserver/src/protocolgame.cpp` ~1119.
    /// `0x8E` + id + `AddOutfit` (no addons / mount). Empty `look_type` is invisibility.
    pub fn encode_creature_outfit(&self, creature_id: u32, outfit: &OutfitWire) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(server::CREATURE_OUTFIT);
        m.write_u32(creature_id);
        self.write_outfit(&mut m, outfit);
        m
    }

    /// 7.72 `Game::combatChangeHealth` — `sendTextMessage` simple branch (`gameserver/src/const.h` `MESSAGE_EVENT_DEFAULT`).
    pub fn encode_combat_damage_text_message(&self, w: &CombatDamageNotifyWire) -> NetworkMessage {
        const MESSAGE_EVENT_DEFAULT: u8 = 0x14;
        let mut m = NetworkMessage::new();
        m.write_u8(server::TEXT_MESSAGE);
        m.write_u8(MESSAGE_EVENT_DEFAULT);
        m.write_string(&w.text);
        m
    }

    /// 7.72 `ProtocolGame::sendCreatureSay` — opcode `0xAA` (`gameserver/src/protocolgame.cpp` ~1422):
    /// `u32 statementId + name + u8 speakType + pos + text`. **No `level` field** (10.98 adds it).
    pub fn encode_creature_say(&self, statement_id: u32, w: &CreatureSayWire) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0xAA);
        m.write_u32(statement_id);
        m.write_string(&w.speaker_name);
        m.write_u8(w.speak_type);
        m.write_position(&w.pos);
        m.write_string(&w.text);
        m
    }

    /// 7.72 `ProtocolGame::sendToChannel` — opcode `0xAA` (`gameserver/src/protocolgame.cpp:1442`):
    /// `u32 statementId + name + u8 speakType + u16 channelId + text`. **No `level` field** (10.98
    /// adds it after the name; writing it desyncs the 772 client's message-mode byte read).
    /// Anonymous (`speaker_name = None`) writes `u32 0` in place of the name.
    pub fn encode_to_channel(&self, statement_id: u32, w: &ToChannelWire) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0xAA);
        m.write_u32(statement_id);
        match &w.speaker_name {
            Some(name) => m.write_string(name),
            None => m.write_u32(0),
        }
        m.write_u8(w.speak_type);
        m.write_u16(w.channel_id);
        m.write_string(&w.text);
        m
    }

    /// 7.72 `ProtocolGame::sendPrivateMessage` — opcode `0xAA`
    /// (`gameserver/src/protocolgame.cpp:1465`): `u32 statementId + name + u8 speakType + text`.
    /// **No `level` field** (10.98 adds it). Anonymous (`speaker_name = None`) writes `u32 0`.
    pub fn encode_private_message(
        &self,
        statement_id: u32,
        w: &PrivateMessageWire,
    ) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0xAA);
        m.write_u32(statement_id);
        match &w.speaker_name {
            Some(name) => m.write_string(name),
            None => m.write_u32(0),
        }
        m.write_u8(w.speak_type);
        m.write_string(&w.text);
        m
    }

    /// 7.72 `ProtocolGame::sendChannelMessage` — opcode `0xAA`
    /// (`gameserver/src/protocolgame.cpp:1306`): `u32 0 + author + u8 speakType + u16 channel + text`.
    /// **No author-`level` field** (10.98 writes a `u16 0` after the author).
    pub fn encode_channel_message(&self, w: &ChannelMessageWire) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0xAA);
        m.write_u32(0);
        m.write_string(&w.author);
        m.write_u8(w.speak_type);
        m.write_u16(w.channel_id);
        m.write_string(&w.text);
        m
    }

    /// 7.72 `ProtocolGame::sendChannelsDialog` — opcode `0xAB`
    /// (`gameserver/src/protocolgame.cpp:1282`): `byte + u8 count + [u16 id + string name]*`.
    /// Era-identical to 1098 (`src/protocolgame.cpp:1687`); both codecs emit the same bytes.
    pub fn encode_channels_dialog(&self, w: &ChannelsDialogWire) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(server::CHANNELS_DIALOG);
        let n = w.channels.len().min(u8::MAX as usize) as u8;
        m.write_u8(n);
        for &(id, ref name) in w.channels.iter().take(n as usize) {
            m.write_u16(id);
            m.write_string(name);
        }
        m
    }

    /// 7.72 `ProtocolGame::sendChannel` — opcode `0xAC` (`gameserver/src/protocolgame.cpp:1297`):
    /// `byte + u16 channelId + string channelName`. **No user/invited lists** (10.98 adds them).
    pub fn encode_channel_open(&self, w: &ChannelOpenWire) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(server::CHANNEL_OPEN);
        m.write_u16(w.channel_id);
        m.write_string(&w.name);
        m
    }

    /// 7.72 `ProtocolGame::sendCreatePrivateChannel` — opcode `0xB2`
    /// (`gameserver/src/protocolgame.cpp:1273`): `byte + u16 channelId + string channelName`.
    /// **No owner/invited lists** (10.98 adds them).
    pub fn encode_create_private_channel(&self, w: &CreatePrivateChannelWire) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(server::CREATE_PRIVATE_CHANNEL);
        m.write_u16(w.channel_id);
        m.write_string(&w.name);
        m
    }

    /// 7.72 `ProtocolGame::sendTextWindow` template-item overload — opcode `0x96`
    /// (`gameserver/src/protocolgame.cpp:1925`): `byte + u32 windowTextId + addItem(itemId, 1)
    /// + u16 text.size() + addString(text) + u16 0` writer. **No date** (10.98 appends one).
    pub fn encode_text_window(&self, w: &TextWindowWire) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(server::TEXT_WINDOW);
        m.write_u32(w.window_text_id);
        self.write_item_template_args(&mut m, w.item);
        let maxlen = if w.can_write {
            w.max_text_len
        } else {
            w.text.len() as u16
        };
        m.write_u16(maxlen);
        m.write_string(&w.text);
        m.write_string(&w.writer);
        m
    }
}
