//! Golden-byte checks for outgoing game packets against the C++ implementation in this repo.
// C++ reference: `src/protocolgame.cpp` (`ProtocolGame::send*`).

use tfs_rust_common::Position;
use tfs_rust_net::map_description::send_map_description_stub;
use tfs_rust_net::outgoing::{
    send_creature_health, send_extended_opcode, send_magic_effect, send_otcv8_features, send_ping,
    send_ping_back, send_text_message,
};
use tfs_rust_net::outgoing_extra::{send_player_skills_1098, send_unjustified_stats_stub};

/// `GameFeature::GameExtendedOpcode` / `GameItemTooltip` (`src/const.h`) — same pair as `ProtocolGame::sendFeatures`.
const GAME_EXTENDED_OPCODE: u8 = 80;
const GAME_ITEM_TOOLTIP: u8 = 93;

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
    let msg = send_player_skills_1098(&levels, &bases, &percents, &add_lv, &add_bs);
    let b = msg.as_bytes();
    assert_eq!(b.len(), 1 + 35 + 24);
    assert_eq!(b[0], 0xA1);
}
