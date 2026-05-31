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
//! PROTOCOL: this codec targets the **real 7.72 Tibia client**. TVP gates an extra `stackpos` byte on
//! `0x6A` (add-tile-item / add-creature) behind `CLIENTOS_OTCLIENT_LINUX` (OTClient-on-772). That byte
//! is **omitted** here for the canonical client — same way OTCv8 quirks are flagged separately for 1098
//! (`docs/OTCLIENT_INFO.md`).

use tfs_rust_common::{Position, ProtocolCaps, ProtocolVersion};

use crate::creature_encode::{AddCreatureWire, OutfitWire};
use crate::NetworkMessage;

use super::wire::{ItemTemplateArgs, PlayerSkillsWire, PlayerStatsWire};

/// Zero-sized 7.72 codec (stateless; caps from `ProtocolVersion::V772`).
#[derive(Debug, Clone, Copy, Default)]
pub struct Codec772;

/// 7.72 `getLiquidColor` (`gameserver/src/tools.cpp` ~L20). **Not** the 10.x `FLUID_MAP` table — the
/// 7.72 client uses a different liquid-color palette mapping.
fn liquid_color_772(fluid_type: u8) -> u8 {
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
            msg.write_u8(liquid_color_772(count));
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

    /// 7.72 `ProtocolGame::AddCreature` (`gameserver/src/protocolgame.cpp` ~L2051). No creature-type
    /// byte (unknown header), no guild emblem, no second creature-type, no speech bubble, no MARK,
    /// no helpers, no walkthrough byte; **full** `getStepSpeed()` (not halved); raw light level.
    pub fn write_add_creature(&self, msg: &mut NetworkMessage, c: &AddCreatureWire) {
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
    /// `0x0A` + `u32 id` + `u16` beat (`0x32`) + `u8` canReportBugs. Opcode is version-keyed via
    /// `protocol_opcodes::server::self_appear`. `canReportBugs` defaults to 0 (non-tutor) — account
    /// type is not threaded into this neutral signature.
    pub fn encode_self_appear_login(&self, player_id: u32) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(tfs_rust_common::protocol_opcodes::server::self_appear(
            ProtocolVersion::V772,
        ));
        m.write_u32(player_id);
        m.write_u16(0x32);
        m.write_u8(0x00); // canReportBugs (ACCOUNT_TYPE_TUTOR+) — default off
        m
    }

    /// 7.72 `sendAddTileItem` opcode `0x6A` (~L1591). Real client: no `stackpos` byte (OTClient-only).
    pub fn encode_add_tile_item(
        &self,
        pos: Position,
        _stack_pos: u8,
        args: ItemTemplateArgs,
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
    pub fn encode_update_container_item(
        &self,
        cid: u8,
        slot: u16,
        args: ItemTemplateArgs,
    ) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0x71);
        m.write_u8(cid);
        m.write_u8(slot as u8);
        self.write_item_template_args(&mut m, args);
        m
    }

    /// 7.72 `sendAddCreature` non-self branch opcode `0x6A` (~L1717). Real client: no `stackpos` byte.
    pub fn encode_add_tile_creature(
        &self,
        pos: Position,
        _stack_pos: u8,
        wire: &AddCreatureWire,
    ) -> NetworkMessage {
        let mut m = NetworkMessage::new();
        m.write_u8(0x6A);
        m.write_position(&pos);
        self.write_add_creature(&mut m, wire);
        m
    }

    /// 7.72 `RemoveTileThing` opcode `0x6C` (~L2161): position + `u8` stackpos.
    pub fn encode_remove_tile_thing(&self, pos: Position, stackpos: u8) -> NetworkMessage {
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
        let n = c
            .items
            .len()
            .min(c.capacity as usize)
            .min(u8::MAX as usize) as u8;
        m.write_u8(n);
        for args in c.items.iter().take(n as usize) {
            self.write_item_template_args(&mut m, *args);
        }
        m
    }
}
