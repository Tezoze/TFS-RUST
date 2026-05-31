//! Capability invariants for 772 vs 1098 (Phase A0 — docs/PROTOCOL_VERSIONING.md §2, §7).

use tfs_rust_common::protocol_opcodes::{client, server};
use tfs_rust_common::{ProtocolCaps, ProtocolVersion};

#[test]
fn caps_1098_matches_current_hardcoded_behavior() {
    let caps = ProtocolCaps::for_version(ProtocolVersion::V1098);

    assert!(caps.adler_checksum);
    assert!(caps.prelogin_challenge);
    assert!(caps.account_name_login);
    assert!(caps.session_key_login);
    assert!(caps.item_mark_byte);
    assert!(caps.item_animation_byte);
    assert!(caps.creature_type_byte);
    assert!(caps.outfit_addons);
    assert!(caps.outfit_mount);
    assert!(caps.speed_halved);
    assert!(caps.stats_u64_experience);
    assert!(caps.stats_capacity_u32);
    assert!(caps.skills_u16);
    assert!(caps.icons_u16);
    assert_eq!(caps.self_appear_opcode, 0x17);
    assert_eq!(caps.initial_buffer_position, 8);
    assert_eq!(caps.xtea_length_slack, 6);
}

#[test]
fn caps_772_inverse_invariants() {
    let caps = ProtocolCaps::for_version(ProtocolVersion::V772);

    assert!(!caps.adler_checksum);
    assert!(!caps.prelogin_challenge);
    assert!(!caps.account_name_login);
    assert!(!caps.session_key_login);
    assert!(!caps.item_mark_byte);
    assert!(!caps.item_animation_byte);
    assert!(!caps.creature_type_byte);
    assert!(!caps.outfit_addons);
    assert!(!caps.outfit_mount);
    assert!(!caps.speed_halved);
    assert!(!caps.stats_u64_experience);
    assert!(!caps.stats_capacity_u32);
    assert!(!caps.skills_u16);
    assert!(!caps.icons_u16);
    assert_eq!(caps.self_appear_opcode, 0x0A);
    assert_eq!(caps.initial_buffer_position, 4);
    assert_eq!(caps.xtea_length_slack, 4);
}

#[test]
fn version_caps_round_trip() {
    for version in [ProtocolVersion::V772, ProtocolVersion::V1098] {
        assert_eq!(version.caps(), ProtocolCaps::for_version(version));
        let round = ProtocolVersion::try_from(version.raw()).expect("supported");
        assert_eq!(round, version);
    }
}

/// Phase A2 — server self-appear opcode is version-keyed (`0x0A` in 772 vs `0x17` in 1098), sourced
/// from `ProtocolCaps::self_appear_opcode`. Executable §2.6 matrix entry.
#[test]
fn server_self_appear_opcode_is_version_keyed() {
    assert_eq!(server::self_appear(ProtocolVersion::V772), 0x0A);
    assert_eq!(server::self_appear(ProtocolVersion::V1098), 0x17);
}

/// Phase A2 — incoming opcode dispatch is version-keyed (§2.7).
#[test]
fn client_opcode_support_matrix() {
    // Shared commands accepted in both eras.
    for op in [
        client::MOVE_NORTH,
        client::SAY,
        client::ATTACK,
        client::SET_OUTFIT,
        client::USE_ITEM,
    ] {
        assert!(client::is_supported(op, ProtocolVersion::V772), "772 {op:#x}");
        assert!(
            client::is_supported(op, ProtocolVersion::V1098),
            "1098 {op:#x}"
        );
    }

    // 10.98-only blocks are rejected on 772 (shop / market / quest / mount / equip / wrap / VIP-edit).
    for op in [
        client::EQUIP_OBJECT,
        client::LOOK_IN_SHOP,
        client::PURCHASE,
        client::TOGGLE_MOUNT,
        client::WRAP_ITEM,
        client::QUEST_LOG,
        client::MARKET_LEAVE,
        client::MODAL_WINDOW_ANSWER,
        client::RULE_VIOLATION_REPORT, // 0xF2 — 772 uses the 0x9B trio instead
    ] {
        assert!(
            !client::is_supported(op, ProtocolVersion::V772),
            "772 must reject {op:#x}"
        );
        assert!(
            client::is_supported(op, ProtocolVersion::V1098),
            "1098 accepts {op:#x}"
        );
    }

    // 7.72-only rule-violation trio accepted on 772, rejected on 1098.
    for op in [
        client::v772::PROCESS_RULE_VIOLATION_REPORT,
        client::v772::CLOSE_RULE_VIOLATION_REPORT,
        client::v772::CANCEL_RULE_VIOLATION_REPORT,
    ] {
        assert!(
            client::is_supported(op, ProtocolVersion::V772),
            "772 accepts {op:#x}"
        );
        assert!(
            !client::is_supported(op, ProtocolVersion::V1098),
            "1098 rejects {op:#x}"
        );
    }
}
