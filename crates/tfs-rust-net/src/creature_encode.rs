//! `ProtocolGame::AddCreature` / `AddOutfit` wire format.
// C++ reference (this repo): `src/protocolgame.cpp` `AddCreature`, `AddOutfit`.

use crate::NetworkMessage;

/// Outfit block (`AddOutfit`).
#[derive(Debug, Clone, Default)]
pub struct OutfitWire {
    pub look_type: u16,
    pub look_head: u8,
    pub look_body: u8,
    pub look_legs: u8,
    pub look_feet: u8,
    pub look_addons: u8,
    pub look_mount: u16,
    /// When `look_type == 0`, `look_type_ex` is sent as item id (`addItemId`).
    pub look_type_ex: u16,
}

/// Serializes `AddOutfit` (used by `sendCreatureOutfit` and `AddCreature`).
pub fn write_outfit(msg: &mut NetworkMessage, o: &OutfitWire) {
    msg.write_u16(o.look_type);
    if o.look_type != 0 {
        msg.write_u8(o.look_head);
        msg.write_u8(o.look_body);
        msg.write_u8(o.look_legs);
        msg.write_u8(o.look_feet);
        msg.write_u8(o.look_addons);
    } else {
        msg.write_u16(o.look_type_ex);
    }
    msg.write_u16(o.look_mount);
}

/// Byte length of [`write_outfit`].
#[inline]
pub fn outfit_wire_len(o: &OutfitWire) -> usize {
    2 + if o.look_type != 0 { 5 } else { 2 } + 2
}

/// Byte length of [`write_add_creature`].
pub fn add_creature_wire_len(c: &AddCreatureWire) -> usize {
    let head = if c.known {
        2 + 4
    } else {
        2 + 4 + 4 + 1 + 2 + c.name.len()
    };
    head + 1
        + 1
        + outfit_wire_len(&c.outfit)
        + 1
        + 1
        + 2
        + 1
        + 1
        + if !c.known { 1 } else { 0 }
        + 1
        + 1
        + 1
        + 2
        + 1
}

/// Parameters for `AddCreature` (map / tooling — viewer defaults).
#[derive(Debug, Clone)]
pub struct AddCreatureWire {
    pub id: u32,
    pub remove_known: u32,
    pub known: bool,
    /// Final type byte including summon adjustments (`creatureType` in C++).
    pub creature_type: u8,
    pub name: String,
    pub health_percent: u8,
    pub direction: u8,
    pub outfit: OutfitWire,
    pub light_level: u8,
    pub light_color: u8,
    /// Full `Creature::getStepSpeed()` (max width — never pre-narrowed; design §9.5).
    /// 10.98 writes `step_speed / 2` (`ProtocolCaps::speed_halved`); 7.72 writes it in full.
    pub step_speed: u16,
    pub skull: u8,
    pub party_shield: u8,
    pub guild_emblem: u8,
    pub speech_bubble: u8,
    pub helpers: u16,
    pub walkthrough_blocked: u8,
    pub access_player: bool,
}

impl Default for AddCreatureWire {
    fn default() -> Self {
        Self {
            id: 0,
            remove_known: 0,
            known: false,
            creature_type: 0,
            name: String::new(),
            health_percent: 100,
            direction: 2,
            outfit: OutfitWire::default(),
            light_level: 0,
            light_color: 0,
            step_speed: 220,
            skull: 0,
            party_shield: 0,
            guild_emblem: 0,
            speech_bubble: 0,
            helpers: 0,
            walkthrough_blocked: 1,
            access_player: false,
        }
    }
}

/// `ProtocolGame::AddCreature` (inside tile description / login).
pub fn write_add_creature(msg: &mut NetworkMessage, c: &AddCreatureWire) {
    if c.known {
        msg.write_u16(0x62);
        msg.write_u32(c.id);
    } else {
        msg.write_u16(0x61);
        msg.write_u32(c.remove_known);
        msg.write_u32(c.id);
        msg.write_u8(c.creature_type);
        msg.write_string(&c.name);
    }

    msg.write_u8(c.health_percent);
    msg.write_u8(c.direction);
    write_outfit(msg, &c.outfit);

    let ll = if c.access_player { 0xFF } else { c.light_level };
    msg.write_u8(ll);
    msg.write_u8(c.light_color);

    // 10.98 writes `getStepSpeed() / 2` (`src/protocolgame.cpp` ~3210). 7.72 writes it in full
    // (`Codec772::write_add_creature`).
    msg.write_u16(c.step_speed / 2);
    msg.write_u8(c.skull);
    msg.write_u8(c.party_shield);

    if !c.known {
        msg.write_u8(c.guild_emblem);
    }

    msg.write_u8(c.creature_type);
    msg.write_u8(c.speech_bubble);
    msg.write_u8(0xFF); // MARK_UNMARKED

    msg.write_u16(c.helpers);
    msg.write_u8(c.walkthrough_blocked);
}
