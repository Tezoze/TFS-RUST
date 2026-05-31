//! Capability invariants for 772 vs 1098 (Phase A0 — docs/PROTOCOL_VERSIONING.md §2, §7).

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
