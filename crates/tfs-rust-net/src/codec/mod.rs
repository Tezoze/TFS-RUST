//! Protocol version codec seam (Track A — `docs/PROTOCOL_VERSIONING.md` §4.2).
//!
//! C++ reference: 10.98 `src/protocolgame.cpp`; 7.72 `gameserver/src/protocolgame.cpp` (Phase A5).

mod v1098;
mod v772;
pub mod wire;

pub use v1098::Codec1098;
pub use v772::Codec772;
pub use wire::{
    AddCreatureWire, ContainerOpenWire, ItemStack, ItemTemplateArgs, ItemWire, OutfitWire,
    PlayerSkillsWire, PlayerStatsWire,
};

use tfs_rust_common::{Position, ProtocolCaps, ProtocolVersion};

use crate::creature_encode::AddCreatureWire as CreatureWire;
use crate::creature_encode::OutfitWire as CreatureOutfitWire;
use crate::NetworkMessage;


/// Outgoing wire encoder — one impl per protocol family (A1: 1098 only).
pub trait ProtocolCodec {
    fn caps(&self) -> ProtocolCaps;

    /// Per-tile `GetTileDescription` prefix. 10.98 writes a `u16` "environmental effects" field
    /// (`src/protocolgame.cpp`); 7.72 (`gameserver/src/protocolgame.cpp`) writes nothing.
    fn write_tile_environment_prefix(&self, msg: &mut NetworkMessage);

    /// Byte length of [`Self::write_tile_environment_prefix`] (2 for 1098, 0 for 772).
    fn tile_environment_prefix_len(&self) -> usize;

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

    fn encode_self_appear_login(&self, player_id: u32) -> NetworkMessage;

    fn encode_add_tile_item(
        &self,
        pos: Position,
        stack_pos: u8,
        args: ItemTemplateArgs,
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

    fn encode_add_tile_creature(
        &self,
        pos: Position,
        stack_pos: u8,
        wire: &CreatureWire,
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

    fn encode_container_open(&self, c: &ContainerOpenWire) -> NetworkMessage;
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

    fn encode_self_appear_login(&self, player_id: u32) -> NetworkMessage {
        Codec1098::encode_self_appear_login(self, player_id)
    }

    fn encode_add_tile_item(
        &self,
        pos: Position,
        stack_pos: u8,
        args: ItemTemplateArgs,
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

    fn encode_add_tile_creature(
        &self,
        pos: Position,
        stack_pos: u8,
        wire: &CreatureWire,
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

    fn encode_container_open(&self, c: &ContainerOpenWire) -> NetworkMessage {
        Codec1098::encode_container_open(self, c)
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

    fn encode_self_appear_login(&self, player_id: u32) -> NetworkMessage {
        Codec772::encode_self_appear_login(self, player_id)
    }

    fn encode_add_tile_item(
        &self,
        pos: Position,
        stack_pos: u8,
        args: ItemTemplateArgs,
    ) -> NetworkMessage {
        Codec772::encode_add_tile_item(self, pos, stack_pos, args)
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

    fn encode_add_tile_creature(
        &self,
        pos: Position,
        stack_pos: u8,
        wire: &CreatureWire,
    ) -> NetworkMessage {
        Codec772::encode_add_tile_creature(self, pos, stack_pos, wire)
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

    fn encode_container_open(&self, c: &ContainerOpenWire) -> NetworkMessage {
        Codec772::encode_container_open(self, c)
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

        encode_self_appear_login(player_id: u32) -> NetworkMessage;

        encode_add_tile_item(pos: Position, stack_pos: u8, args: ItemTemplateArgs) -> NetworkMessage;

        encode_update_tile_item(pos: Position, stack_pos: u8, args: ItemTemplateArgs) -> NetworkMessage;

        encode_inventory_item(slot: u8, args: ItemTemplateArgs) -> NetworkMessage;

        encode_add_container_item(cid: u8, slot: u16, args: ItemTemplateArgs) -> NetworkMessage;

        encode_update_container_item(cid: u8, slot: u16, args: ItemTemplateArgs) -> NetworkMessage;

        encode_add_tile_creature(pos: Position, stack_pos: u8, wire: &CreatureWire) -> NetworkMessage;

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

        encode_container_open(c: &ContainerOpenWire) -> NetworkMessage;
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

    fn encode_self_appear_login(&self, player_id: u32) -> NetworkMessage {
        Codec::encode_self_appear_login(self, player_id)
    }

    fn encode_add_tile_item(
        &self,
        pos: Position,
        stack_pos: u8,
        args: ItemTemplateArgs,
    ) -> NetworkMessage {
        Codec::encode_add_tile_item(self, pos, stack_pos, args)
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

    fn encode_add_tile_creature(
        &self,
        pos: Position,
        stack_pos: u8,
        wire: &CreatureWire,
    ) -> NetworkMessage {
        Codec::encode_add_tile_creature(self, pos, stack_pos, wire)
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

    fn encode_container_open(&self, c: &ContainerOpenWire) -> NetworkMessage {
        Codec::encode_container_open(self, c)
    }
}
