//! Protocol version and wire capability flags.
//!
//! C++ reference: 7.72 `gameserver/src/protocolgame.cpp`, `networkmessage.cpp`;
//! 10.98 repo-root `src/protocolgame.cpp`, `networkmessage.cpp`.

use std::fmt;

/// Supported Tibia client protocol version (e.g. 772 = 7.72, 1098 = 10.98).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProtocolVersion(u16);

impl ProtocolVersion {
    pub const V772: Self = Self(772);
    pub const V1098: Self = Self(1098);

    pub fn raw(self) -> u16 {
        self.0
    }

    pub fn caps(self) -> ProtocolCaps {
        ProtocolCaps::for_version(self)
    }
}

impl fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<u16> for ProtocolVersion {
    type Error = String;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match value {
            772 => Ok(Self::V772),
            1098 => Ok(Self::V1098),
            other => Err(format!(
                "unsupported clientVersion `{other}` (supported: 772, 1098)"
            )),
        }
    }
}

/// Executable §2 wire-format matrix — one struct per connection, no scattered `if version` checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtocolCaps {
    pub adler_checksum: bool,
    pub prelogin_challenge: bool,
    pub account_name_login: bool,
    pub session_key_login: bool,
    pub item_mark_byte: bool,
    pub item_animation_byte: bool,
    pub creature_type_byte: bool,
    pub outfit_addons: bool,
    pub outfit_mount: bool,
    pub speed_halved: bool,
    pub stats_u64_experience: bool,
    pub stats_capacity_u32: bool,
    pub skills_u16: bool,
    pub icons_u16: bool,
    pub self_appear_opcode: u8,
    pub initial_buffer_position: u8,
    pub xtea_length_slack: u8,
}

impl ProtocolCaps {
    pub fn for_version(version: ProtocolVersion) -> Self {
        match version.raw() {
            772 => Self {
                adler_checksum: false,
                prelogin_challenge: false,
                account_name_login: false,
                session_key_login: false,
                item_mark_byte: false,
                item_animation_byte: false,
                creature_type_byte: false,
                outfit_addons: false,
                outfit_mount: false,
                speed_halved: false,
                stats_u64_experience: false,
                stats_capacity_u32: false,
                skills_u16: false,
                icons_u16: false,
                self_appear_opcode: 0x0A,
                initial_buffer_position: 4,
                xtea_length_slack: 4,
            },
            1098 => Self {
                adler_checksum: true,
                prelogin_challenge: true,
                account_name_login: true,
                session_key_login: true,
                item_mark_byte: true,
                item_animation_byte: true,
                creature_type_byte: true,
                outfit_addons: true,
                outfit_mount: true,
                speed_halved: true,
                stats_u64_experience: true,
                stats_capacity_u32: true,
                skills_u16: true,
                icons_u16: true,
                self_appear_opcode: 0x17,
                initial_buffer_position: 8,
                xtea_length_slack: 6,
            },
            other => unreachable!("unsupported protocol version {other}"),
        }
    }
}

/// Parse `clientVersion` / `TFS_PROTOCOL_VERSION` after range check.
pub fn protocol_version_from_raw(raw: i64) -> Result<ProtocolVersion, String> {
    if raw < 0 || raw > u16::MAX as i64 {
        return Err(format!("clientVersion out of range: {raw}"));
    }
    ProtocolVersion::try_from(raw as u16)
}

/// Like [`protocol_version_from_raw`] but maps integer conversion failure to a message.
pub fn protocol_version_from_u16(raw: u16) -> Result<ProtocolVersion, String> {
    ProtocolVersion::try_from(raw)
}

/// Helper for config paths that use `i64` then cast.
pub fn protocol_version_from_i64(raw: i64) -> Result<ProtocolVersion, String> {
    protocol_version_from_raw(raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_supported_versions() {
        for v in [ProtocolVersion::V772, ProtocolVersion::V1098] {
            let round = ProtocolVersion::try_from(v.raw()).expect("supported version");
            assert_eq!(round, v);
        }
    }

    #[test]
    fn unsupported_version_rejected() {
        assert!(ProtocolVersion::try_from(860).is_err());
    }
}
