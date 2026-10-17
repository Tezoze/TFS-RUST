//! Opcode `0x0B` banishment bytes.
//! Corpus: `sending.cc` `SendRights`; `info.cc` `CheckBanishmentRight`.
//!
//! The 32 bytes are reasons 0..=31 (`NAME_INSULTING`..=`INVALID_PAYMENT`, rights 18..=49).
//! Each byte's low 7 bits are actions 0..=6. Bit `0x80` is `IP_BANISHMENT`, and only when
//! at least one action bit is set. `FinishSendData` runs only when one of those bytes is
//! non-zero, so an all-zero mask is an empty message.

use crate::NetworkMessage;

/// Rights `CheckBanishmentRight` reads. `reasons` bit `n` is `CheckRight(n + 18)`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BanishmentRights {
    pub reasons: u32,
    pub notation: bool,
    pub namelock: bool,
    pub banishment: bool,
    pub final_warning: bool,
    pub statement_report: bool,
    pub ip_banishment: bool,
}

impl BanishmentRights {
    /// Every reason right plus the six action rights.
    ///
    /// The player row has no `Rights` bitset. `groups.lua` `access` is the pack stand-in
    /// already used for `CheckBanishmentRight` (`reporter_may_banish`), so an access group
    /// is treated as holding rights 1..=6 and 18..=49.
    pub const fn access_group() -> Self {
        Self {
            reasons: u32::MAX,
            notation: true,
            namelock: true,
            banishment: true,
            final_warning: true,
            statement_report: true,
            ip_banishment: true,
        }
    }
}

/// One action byte for `reason` in `0..=31`.
fn action_byte(reason: u32, rights: BanishmentRights) -> u8 {
    if rights.reasons & (1u32 << reason) == 0 {
        return 0;
    }
    let name = reason <= 8;
    let statement = (9..=27).contains(&reason) || reason == 29;
    let mut actions = 0u8;
    if rights.notation {
        actions |= 1 << 0;
    }
    if name && rights.namelock {
        actions |= 1 << 1;
    }
    if !name && rights.banishment {
        actions |= 1 << 2;
    }
    if name && rights.namelock && rights.banishment {
        actions |= 1 << 3;
    }
    if !name && rights.banishment && rights.final_warning {
        actions |= 1 << 4;
    }
    if name && rights.namelock && rights.banishment && rights.final_warning {
        actions |= 1 << 5;
    }
    if statement && rights.statement_report {
        actions |= 1 << 6;
    }
    if actions != 0 && rights.ip_banishment {
        actions |= 0x80;
    }
    actions
}

pub fn action_bytes(rights: BanishmentRights) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (reason, byte) in out.iter_mut().enumerate() {
        *byte = action_byte(reason as u32, rights);
    }
    out
}

/// `SV_CMD_RIGHTS` (`0x0B`) plus 32 action bytes. Empty when every byte is 0.
pub fn encode_rights(rights: BanishmentRights) -> NetworkMessage {
    let actions = action_bytes(rights);
    if actions.iter().all(|byte| *byte == 0) {
        return NetworkMessage::new();
    }
    let mut msg = NetworkMessage::new();
    msg.write_u8(0x0B);
    for byte in actions {
        msg.write_u8(byte);
    }
    msg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_reason_right_omits_the_packet() {
        let rights = BanishmentRights {
            notation: true,
            ip_banishment: true,
            ..BanishmentRights::default()
        };
        assert!(encode_rights(rights).into_bytes().is_empty());
    }

    #[test]
    fn notation_on_reason_zero_is_bit_zero() {
        let rights = BanishmentRights {
            reasons: 1,
            notation: true,
            ..BanishmentRights::default()
        };
        let bytes = action_bytes(rights);
        assert_eq!(bytes[0], 0x01);
        assert!(bytes[1..].iter().all(|byte| *byte == 0));
    }

    #[test]
    fn access_group_matches_check_banishment_right() {
        let bytes = action_bytes(BanishmentRights::access_group());
        assert!(bytes[..9].iter().all(|byte| *byte == 0xAB));
        assert!(bytes[9..28].iter().all(|byte| *byte == 0xD5));
        assert_eq!(bytes[28], 0x95);
        assert_eq!(bytes[29], 0xD5);
        assert_eq!(bytes[30], 0x95);
        assert_eq!(bytes[31], 0x95);
    }
}
