//! Golden-byte checks for outgoing game packets against the C++ implementation in this repo.
//! Phase A1 regression gate — bytes must match pre-codec output.
// C++ reference: `src/protocolgame.cpp` (`ProtocolGame::send*`).

use tfs_rust_common::{Position, ProtocolVersion};
use tfs_rust_net::codec::{
    AddCreatureWire, Codec, Codec1098, ItemTemplateArgs, OutfitWire, PlayerSkillsWire,
    PlayerStatsWire,
};
use tfs_rust_net::creature_encode::write_add_creature;
use tfs_rust_net::map_description::send_map_description_stub;
use tfs_rust_net::outgoing::{
    send_creature_health, send_extended_opcode, send_magic_effect, send_otcv8_features, send_ping,
    send_ping_back, send_text_message,
};
use tfs_rust_net::outgoing_extra::send_unjustified_stats_stub;
use tfs_rust_net::{item_encode::write_item_template, NetworkMessage};

/// `GameFeature::GameExtendedOpcode` / `GameItemTooltip` (`src/const.h`) — same pair as `ProtocolGame::sendFeatures`.
const GAME_EXTENDED_OPCODE: u8 = 80;
const GAME_ITEM_TOOLTIP: u8 = 93;

fn codec() -> Codec {
    Codec::from_version(ProtocolVersion::V1098).expect("1098 codec")
}

#[test]
fn ping_and_ping_back() {
    assert_eq!(send_ping().as_bytes(), &[0x1D]);
    assert_eq!(send_ping_back().as_bytes(), &[0x1E]);
}

#[test]
fn magic_effect_encoding() {
    let pos = Position::new(0x0102, 0x0304, 5);
    let m = send_magic_effect(pos, 7);
    assert_eq!(m.as_bytes(), &[0x83, 0x02, 0x01, 0x04, 0x03, 0x05, 0x07]);
}

#[test]
fn creature_health_encoding() {
    let m = send_creature_health(0x11223344, 88);
    assert_eq!(m.as_bytes(), &[0x8C, 0x44, 0x33, 0x22, 0x11, 0x58]);
}

#[test]
fn text_message_encoding() {
    let m = send_text_message(0x16, "hello");
    assert_eq!(
        m.as_bytes(),
        &[0xB4, 0x16, 0x05, 0x00, b'h', b'e', b'l', b'l', b'o']
    );
}

#[test]
fn extended_opcode_encoding() {
    let m = send_extended_opcode(0xAB, "x");
    assert_eq!(m.as_bytes(), &[0x32, 0xAB, 0x01, 0x00, b'x']);
}

#[test]
fn otcv8_features_encoding_matches_send_features() {
    let m = send_otcv8_features(&[(GAME_EXTENDED_OPCODE, true), (GAME_ITEM_TOOLTIP, true)]);
    assert_eq!(
        m.as_bytes(),
        &[
            0x43,
            0x02,
            0x00,
            GAME_EXTENDED_OPCODE,
            0x01,
            GAME_ITEM_TOOLTIP,
            0x01
        ]
    );
}

#[test]
fn map_description_stub_encoding() {
    let p = Position::new(10, 20, 7);
    let m = send_map_description_stub(p, p);
    assert_eq!(m.as_bytes(), &[0x64, 10, 0, 20, 0, 7, 0xFF, 0xFF]);
}

/// `docs/OTCLIENT_INFO.md` §1 — `parseUnjustifiedStats`: opcode + 7× u8.
#[test]
fn unjustified_stats_stub_is_seven_payload_bytes() {
    let m = send_unjustified_stats_stub();
    let b = m.as_bytes();
    assert_eq!(b.len(), 1 + 7);
    assert_eq!(b[0], 0xB7);
    assert!(b[1..].iter().all(|&x| x == 0));
}

/// `docs/OTCLIENT_INFO.md` §2 — 13 skills with `GameAdditionalSkills`: 35 + 24 bytes after opcode.
#[test]
fn player_skills_1098_otc_thirteen_skill_layout_length() {
    let levels = [1u16, 2, 3, 4, 5, 6, 7];
    let bases = levels;
    let percents = [0u8; 7];
    let add_lv = [10u16; 6];
    let add_bs = [10u16; 6];
    let msg = codec().encode_player_skills(&PlayerSkillsWire {
        levels,
        bases,
        percents,
        additional_levels: add_lv,
        additional_bases: add_bs,
    });
    let b = msg.as_bytes();
    assert_eq!(b.len(), 1 + 35 + 24);
    assert_eq!(b[0], 0xA1);
}

#[test]
fn item_template_plain_via_codec_matches_legacy_bytes() {
    let mut legacy = NetworkMessage::new();
    write_item_template(&mut legacy, 0x1234, 1, false, false, false, false);
    let mut via_codec = NetworkMessage::new();
    codec().write_item_template(&mut via_codec, 0x1234, 1, false, false, false, false);
    assert_eq!(legacy.as_bytes(), via_codec.as_bytes());
    assert_eq!(legacy.as_bytes(), &[0x34, 0x12, 0xFF]);
}

#[test]
fn item_template_fluid_via_codec() {
    let mut m = NetworkMessage::new();
    codec().write_item_template(&mut m, 0x1234, 3, false, true, false, false);
    assert_eq!(m.as_bytes(), &[0x34, 0x12, 0xFF, 0x03]);
}

#[test]
fn outfit_looktype_via_codec() {
    let o = OutfitWire {
        look_type: 128,
        look_head: 1,
        look_body: 2,
        look_legs: 3,
        look_feet: 4,
        look_addons: 0,
        look_mount: 0,
        look_type_ex: 0,
    };
    let mut m = NetworkMessage::new();
    codec().write_outfit(&mut m, &o);
    assert_eq!(
        m.as_bytes(),
        &[128, 0, 1, 2, 3, 4, 0, 0, 0]
    );
}

#[test]
fn player_stats_packet_via_codec() {
    let stats = PlayerStatsWire {
        health: 100,
        max_health: 100,
        free_capacity: 40000,
        total_capacity: 40000,
        experience: 4200,
        level: 8,
        level_percent: 50,
        mana: 50,
        max_mana: 50,
        magic_level: 0,
        base_magic_level: 0,
        magic_level_percent: 0,
        soul: 100,
        stamina_minutes: 2520,
        base_speed_half: 110,
        regeneration_ticks_sec: 0,
        offline_training_time: 0,
    };
    let b = codec().encode_player_stats(&stats).as_bytes().to_vec();
    assert_eq!(b[0], 0xA0);
    assert_eq!(b.len(), 1 + 2 + 2 + 4 + 4 + 8 + 2 + 1 + 2 + 2 + 2 + 2 + 2 + 2 + 2 + 1 + 1 + 1 + 1 + 2 + 2 + 2 + 2 + 2 + 1);
}

#[test]
fn add_creature_known_header_via_codec() {
    let c = AddCreatureWire {
        id: 0x11223344,
        remove_known: 0,
        known: true,
        creature_type: 0,
        name: String::new(),
        health_percent: 100,
        direction: 2,
        outfit: OutfitWire::default(),
        light_level: 0,
        light_color: 0,
        speed_half: 110,
        skull: 0,
        party_shield: 0,
        guild_emblem: 0,
        speech_bubble: 0,
        helpers: 0,
        walkthrough_blocked: 1,
        access_player: false,
    };
    let mut legacy = NetworkMessage::new();
    write_add_creature(&mut legacy, &c);
    let mut via_codec = NetworkMessage::new();
    codec().write_add_creature(&mut via_codec, &c);
    assert_eq!(legacy.as_bytes(), via_codec.as_bytes());
}

#[test]
fn encode_add_tile_item_matches_deprecated_helper() {
    let pos = Position::new(10, 20, 7);
    let args = ItemTemplateArgs {
        client_id: 0x1234,
        count: 3,
        stackable: true,
        is_splash_or_fluid: false,
        is_animation: false,
        with_description: false,
    };
    let via_codec = codec().encode_add_tile_item(pos, 2, args).into_bytes();
    let via_legacy =
        Codec1098.encode_add_tile_item(pos, 2, args).into_bytes();
    assert_eq!(via_codec, via_legacy);
    assert_eq!(via_codec[0], 0x6A);
}
