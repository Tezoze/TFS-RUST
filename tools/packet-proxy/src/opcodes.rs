//! Opcode → name for log lines (server vs client direction).

#[inline]
pub fn server_opcode_name(op: u8) -> &'static str {
    match op {
        0x0A => "PendingStateEntered",
        0x0F => "EnterWorld",
        0x17 => "LoginSuccess",
        0x1D => "Ping",
        0x1E => "PingBack",
        0x32 => "ExtendedOpcode",
        0x43 => "OTCV8Features",
        0x64 => "MapDescription",
        0x79 => "InventoryItem",
        0x82 => "WorldLight",
        0x83 => "MagicEffect",
        0x8D => "CreatureLight",
        0x8E => "CreatureOutfit",
        0x9F => "BasicData",
        0xA0 => "PlayerStats",
        0xA1 => "PlayerSkills",
        0xA2 => "Icons",
        0xA7 => "FightModes",
        0xB4 => "TextMessage",
        0xB7 => "UnjustifiedStats",
        0xD2 => "VIPEntries",
        0xF5 => "Items",
        _ => "Unknown",
    }
}

#[inline]
pub fn client_opcode_name(op: u8) -> &'static str {
    match op {
        0x0F => "EnterGame",
        0x14 => "Logout",
        0x1D => "PingBack",
        0x1E => "Ping",
        0x32 => "ExtendedOpcode",
        0x64 => "AutoWalk",
        0x65 => "WalkNorth",
        0x66 => "WalkEast",
        0x67 => "WalkSouth",
        0x68 => "WalkWest",
        0x69 => "StopAutoWalk",
        0x96 => "Say",
        _ => "Unknown",
    }
}
