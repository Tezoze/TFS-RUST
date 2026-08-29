//! Golden-byte checks for outgoing game packets against the C++ implementation in this repo.
//! Phase A1 regression gate — bytes must match pre-codec output.
// C++ reference: `src/protocolgame.cpp` (`ProtocolGame::send*`).

use std::collections::HashSet;
use tfs_rust_common::{Position, ProtocolVersion};
use tfs_rust_net::codec::{
    AddCreatureWire, AnimatedTextWire, ChannelOpenWire, ChannelsDialogWire, Codec, Codec1098,
    CombatDamageNotifyWire, ContainerOpenWire, CreatePrivateChannelWire, CreatureHealthWire,
    CreatureSpeedWire, DistanceShootWire, ItemTemplateArgs, MagicEffectWire, OutfitWire,
    PlayerSkillsWire, PlayerStatsWire, TextWindowWire,
};

use tfs_rust_net::creature_encode::write_add_creature;
use tfs_rust_net::map_description::{
    send_map_description_packet, send_map_description_stub, send_move_creature_player,
    send_notify_go,
};
use tfs_rust_net::outgoing::{
    send_creature_health, send_extended_opcode, send_magic_effect, send_otcv8_features, send_ping,
    send_ping_back, send_text_message,
};
use tfs_rust_net::outgoing_extra::send_unjustified_stats_stub;
use tfs_rust_net::{NetworkMessage, item_encode::write_item_template};

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

/// TVP `ProtocolGame::sendPing` (`gameserver/src/protocolgame.cpp:1516`): official 772 is `0x1E`.
/// Sending `0x1D` to the real client is `Control.cpp:1274` unknown packet type 29.
#[test]
fn v772_periodic_ping_opcode_by_client_family() {
    let codec = Codec::from_version(ProtocolVersion::V772).expect("772 codec");
    assert_eq!(codec.periodic_ping_packet(false).as_bytes(), &[0x1E]);
    assert_eq!(codec.periodic_ping_packet(true).as_bytes(), &[0x1D]);
    let c1098 = Codec::from_version(ProtocolVersion::V1098).expect("1098 codec");
    assert_eq!(c1098.periodic_ping_packet(false).as_bytes(), &[0x1D]);
    assert_eq!(c1098.periodic_ping_packet(true).as_bytes(), &[0x1D]);
}

#[test]
fn magic_effect_encoding() {
    let pos = Position::new(0x0102, 0x0304, 5);
    let m = send_magic_effect(pos, 7);
    assert_eq!(m.as_bytes(), &[0x83, 0x02, 0x01, 0x04, 0x03, 0x05, 0x07]);
    let via_codec = codec()
        .encode_magic_effect(&MagicEffectWire { pos, effect_id: 7 })
        .into_bytes();
    assert_eq!(via_codec, m.as_bytes());
}

#[test]
fn animated_text_1098_has_no_equivalent() {
    assert!(
        codec()
            .encode_animated_text(&AnimatedTextWire {
                pos: Position::new(1, 2, 3),
                color: 180,
                text: "9".to_string(),
            })
            .into_bytes()
            .is_empty()
    );
}

#[test]
fn combat_damage_text_message_1098_layout() {
    let pos = Position::new(0x0102, 0x0304, 5);
    let b = codec()
        .encode_combat_damage_text_message(&CombatDamageNotifyWire {
            pos,
            damage: 5,
            damage_color: 180,
            text: "You lose 5 hitpoints.".to_string(),
        })
        .into_bytes();
    assert_eq!(
        b,
        vec![
            0xB4, 24, 0x02, 0x01, 0x04, 0x03, 0x05, 5, 0, 0, 0, 180, 0, 0, 0, 0, 0, 0x15, 0x00,
            b'Y', b'o', b'u', b' ', b'l', b'o', b's', b'e', b' ', b'5', b' ', b'h', b'i', b't',
            b'p', b'o', b'i', b'n', b't', b's', b'.'
        ]
    );
}

#[test]
fn distance_shoot_encoding() {
    use tfs_rust_net::codec::wire::DistanceShootWire;

    let from = Position::new(0x0102, 0x0304, 5);
    let to = Position::new(0x0506, 0x0708, 5);
    // Golden bytes from the legacy `send_distance_shoot` encoder (now deprecated).
    let expected = &[
        0x85, 0x02, 0x01, 0x04, 0x03, 0x05, 0x06, 0x05, 0x08, 0x07, 0x05, 6,
    ];
    let via_codec = codec()
        .encode_distance_shoot(&DistanceShootWire {
            from,
            to,
            shoot_type: 6,
        })
        .into_bytes();
    assert_eq!(&via_codec, expected);
}

#[test]
fn creature_health_codec_matches_outgoing() {
    let m = send_creature_health(0x11223344, 73);
    let via_codec = codec()
        .encode_creature_health(&CreatureHealthWire {
            creature_id: 0x11223344,
            health_percent: 73,
        })
        .into_bytes();
    assert_eq!(via_codec, m.as_bytes());
}

#[test]
fn creature_health_encoding() {
    let m = send_creature_health(0x11223344, 88);
    assert_eq!(m.as_bytes(), &[0x8C, 0x44, 0x33, 0x22, 0x11, 0x58]);
}

/// 7.72 `SendCreatureSpeed` — `sending.cc:1028-1043`.
/// Wire: `0x8F + u32 creature_id + u16 GetSpeed()` (single full speed, no halving).
#[test]
fn creature_speed_encoding_772() {
    let codec = Codec::from_version(ProtocolVersion::V772).expect("772 codec");
    let m = codec.encode_creature_speed(&CreatureSpeedWire {
        creature_id: 0x11223344,
        speed: 818,
        base_speed: 818, // ignored by 772 encoder
    });
    // 0x8F + little-endian u32 + little-endian u16(818 = 0x0332)
    assert_eq!(m.as_bytes(), &[0x8F, 0x44, 0x33, 0x22, 0x11, 0x32, 0x03]);
}

/// 10.98 `sendChangeSpeed` — `src/protocolgame.cpp` ~2505.
/// Wire: `0x8F + u32 creature_id + u16 baseSpeed/2 + u16 speed/2` (two halved values).
#[test]
fn creature_speed_encoding_1098() {
    let codec = Codec::from_version(ProtocolVersion::V1098).expect("1098 codec");
    let m = codec.encode_creature_speed(&CreatureSpeedWire {
        creature_id: 0x11223344,
        speed: 818,
        base_speed: 220,
    });
    // 0x8F + u32 LE + u16(220/2=110=0x006E) + u16(818/2=409=0x0199)
    assert_eq!(
        m.as_bytes(),
        &[0x8F, 0x44, 0x33, 0x22, 0x11, 0x6E, 0x00, 0x99, 0x01]
    );
}

/// 7.72 `sendCreatureOutfit` — `gameserver/src/protocolgame.cpp` ~1119.
/// Invisible: empty outfit `look_type == 0` → `u16 0` + `u16 lookTypeEx` (no mount).
#[test]
fn creature_outfit_encoding_772_invisible() {
    let codec = Codec::from_version(ProtocolVersion::V772).expect("772 codec");
    let o = OutfitWire::default(); // look_type 0
    let m = codec.encode_creature_outfit(0x11223344, &o);
    // 0x8E + id LE + lookType 0 + lookTypeEx 0 — no addons, no mount
    assert_eq!(
        m.as_bytes(),
        &[0x8E, 0x44, 0x33, 0x22, 0x11, 0x00, 0x00, 0x00, 0x00]
    );
}

/// 7.72 `AddOutfit` lookType path — colors only (no addons / mount).
#[test]
fn creature_outfit_encoding_772_looktype() {
    let codec = Codec::from_version(ProtocolVersion::V772).expect("772 codec");
    let o = OutfitWire {
        look_type: 128,
        look_head: 1,
        look_body: 2,
        look_legs: 3,
        look_feet: 4,
        look_addons: 99, // ignored on 772
        look_mount: 9,   // ignored on 772
        look_type_ex: 0,
    };
    let m = codec.encode_creature_outfit(0x11223344, &o);
    assert_eq!(
        m.as_bytes(),
        &[0x8E, 0x44, 0x33, 0x22, 0x11, 128, 0, 1, 2, 3, 4]
    );
}

/// 10.98 `sendCreatureOutfit` — addons + mount after colors.
#[test]
fn creature_outfit_encoding_1098_looktype() {
    let codec = Codec::from_version(ProtocolVersion::V1098).expect("1098 codec");
    let o = OutfitWire {
        look_type: 128,
        look_head: 1,
        look_body: 2,
        look_legs: 3,
        look_feet: 4,
        look_addons: 5,
        look_mount: 9,
        look_type_ex: 0,
    };
    let m = codec.encode_creature_outfit(0x11223344, &o);
    assert_eq!(
        m.as_bytes(),
        &[0x8E, 0x44, 0x33, 0x22, 0x11, 128, 0, 1, 2, 3, 4, 5, 9, 0]
    );
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
    assert_eq!(m.as_bytes(), &[128, 0, 1, 2, 3, 4, 0, 0, 0]);
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
    assert_eq!(
        b.len(),
        1 + 2
            + 2
            + 4
            + 4
            + 8
            + 2
            + 1
            + 2
            + 2
            + 2
            + 2
            + 2
            + 2
            + 2
            + 1
            + 1
            + 1
            + 1
            + 2
            + 2
            + 2
            + 2
            + 2
            + 1
    );
}

#[test]
fn add_creature_known_header_via_codec() {
    let c = AddCreatureWire {
        id: 0x11223344,
        remove_known: 0,
        known: true,
        uptodate: false,
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
    let via_codec = codec()
        .encode_add_tile_item(pos, 2, args, false)
        .into_bytes();
    let via_legacy = Codec1098.encode_add_tile_item(pos, 2, args).into_bytes();
    assert_eq!(via_codec, via_legacy);
    assert_eq!(via_codec[0], 0x6A);
}

/// 1098 `sendContainer` (`0x6E`): cid + item + name + capacity + hasParent + unlocked + pagination
/// + `u16` size + `u16` firstIndex + count + items. Regression after routing through the codec.
#[test]
fn container_open_1098_layout() {
    let wire = ContainerOpenWire {
        cid: 3,
        header_item: ItemTemplateArgs {
            client_id: 0x0BBE,
            count: 1,
            stackable: false,
            is_splash_or_fluid: false,
            is_animation: false,
            with_description: false,
        },
        name: "bag".to_string(),
        capacity: 8,
        has_parent: false,
        unlocked: true,
        pagination: false,
        total_size: 1,
        first_index: 0,
        items: vec![ItemTemplateArgs {
            client_id: 0x0C00,
            count: 5,
            stackable: true,
            is_splash_or_fluid: false,
            is_animation: false,
            with_description: false,
        }],
    };
    let b = codec().encode_container_open(&wire).into_bytes();
    assert_eq!(
        b,
        vec![
            0x6E, 3, // opcode + cid
            0xBE, 0x0B, 0xFF, // header item: clientId + MARK
            0x03, 0x00, b'b', b'a', b'g', // name
            8,    // capacity
            0,    // hasParent
            1,    // unlocked
            0,    // pagination
            1, 0, // total size
            0, 0, // first index
            1, // items to send
            0x00, 0x0C, 0xFF, 5, // child item: clientId + MARK + count
        ]
    );
}

/// 1098 `sendRemoveContainerItem` (`0x72`): cid + `u16` slot + `u16` 0 when no `lastItem`.
/// C++ ref: `src/protocolgame.cpp` ~2952.
#[test]
fn remove_container_item_1098_layout() {
    let b = codec().encode_remove_container_item(4, 0x0201).into_bytes();
    assert_eq!(b, vec![0x72, 4, 0x01, 0x02, 0x00, 0x00]);
}

// ---------------------------------------------------------------------------
// CH-0 — chat-channel outgoing wire golden bytes.
// 1098 reference: `src/protocolgame.cpp` `sendChannelsDialog` (~1687),
//   `sendChannel` (~1702), `sendCreatePrivateChannel` (~1675),
//   `sendClosePrivate` (~1667), `sendOpenPrivateChannel` (~1296).
// 772  reference: `gameserver/src/protocolgame.cpp` `sendChannelsDialog` (~1282),
//   `sendChannel` (~1297), `sendCreatePrivateChannel` (~1273),
//   `sendClosePrivate` (~1265), `sendOpenPrivateChannel` (~1111).
// ---------------------------------------------------------------------------

/// 1098 `sendChannelsDialog` — `0xAB + u8 count + [u16 id + string name]*`. Era-identical.
#[test]
fn channels_dialog_1098_layout() {
    let b = codec()
        .encode_channels_dialog(&ChannelsDialogWire {
            channels: vec![(4, "Game-Chat".to_string()), (7, "Help".to_string())],
        })
        .into_bytes();
    assert_eq!(
        b,
        vec![
            0xAB, 2, // opcode + count
            4, 0, // channel id 4
            9, 0, b'G', b'a', b'm', b'e', b'-', b'C', b'h', b'a', b't', // name (9 chars)
            7, 0, // channel id 7
            4, 0, b'H', b'e', b'l', b'p', // name
        ]
    );
}

/// 1098 `sendChannel` — `0xAC + u16 id + string name + u16 usersCount + [string]* +
/// u16 invitedCount + [string]*`. Diverges from 772 (which omits both lists).
#[test]
fn channel_open_1098_with_user_lists() {
    let b = codec()
        .encode_channel_open(&ChannelOpenWire {
            channel_id: 4,
            name: "Game-Chat".to_string(),
            users: vec!["Alice".to_string(), "Bob".to_string()],
            invited: vec!["Carol".to_string()],
        })
        .into_bytes();
    assert_eq!(
        b,
        vec![
            0xAC, 4, 0, // opcode + channel id
            9, 0, b'G', b'a', b'm', b'e', b'-', b'C', b'h', b'a', b't', // name (9 chars)
            2, 0, // users count
            5, 0, b'A', b'l', b'i', b'c', b'e', // Alice
            3, 0, b'B', b'o', b'b', // Bob
            1, 0, // invited count
            5, 0, b'C', b'a', b'r', b'o', b'l', // Carol
        ]
    );
}

/// 1098 `sendChannel` with no user/invited lists — trailing `u16(0) + u16(0)`.
#[test]
fn channel_open_1098_empty_lists() {
    let b = codec()
        .encode_channel_open(&ChannelOpenWire {
            channel_id: 0xFFFF,
            name: "Private".to_string(),
            ..Default::default()
        })
        .into_bytes();
    assert_eq!(
        b,
        vec![
            0xAC, 0xFF, 0xFF, // opcode + channel id
            7, 0, b'P', b'r', b'i', b'v', b'a', b't', b'e', // name
            0, 0, // users count
            0, 0, // invited count
        ]
    );
}

/// 1098 `sendCreatePrivateChannel` — `0xB2 + u16 id + string name + u16(1) + string owner +
/// u16 invitedCount + [string]*`. Diverges from 772 (which omits owner + invited).
#[test]
fn create_private_channel_1098_layout() {
    let b = codec()
        .encode_create_private_channel(&CreatePrivateChannelWire {
            channel_id: 0x0100,
            name: "My Channel".to_string(),
            owner_name: "Alice".to_string(),
            invited: vec!["Bob".to_string()],
        })
        .into_bytes();
    assert_eq!(
        b,
        vec![
            0xB2, 0x00, 0x01, // opcode + channel id
            10, 0, b'M', b'y', b' ', b'C', b'h', b'a', b'n', b'n', b'e',
            b'l', // name (10 chars)
            1, 0, // owner count (always 1)
            5, 0, b'A', b'l', b'i', b'c', b'e', // owner name
            1, 0, // invited count
            3, 0, b'B', b'o', b'b', // invited name
        ]
    );
}

/// 1098 `sendTextWindow` template overload — MARK + empty writer + empty date
/// (`src/protocolgame.cpp:2999`). Matches the legacy helper so 1098 spellbooks stay intact.
#[test]
fn text_window_1098_has_mark_and_date() {
    let w = TextWindowWire {
        window_text_id: 1,
        item: ItemTemplateArgs {
            client_id: 0x1234,
            count: 1,
            stackable: false,
            is_splash_or_fluid: false,
            is_animation: false,
            with_description: false,
        },
        text: "hello".to_string(),
        writer: String::new(),
        written_date: None,
        can_write: false,
        max_text_len: 0,
    };
    let b = codec().encode_text_window(&w).into_bytes();
    assert_eq!(
        b,
        vec![
            0x96, 1, 0, 0, 0, // opcode + windowTextId
            0x34, 0x12, 0xFF, // clientId + MARK_UNMARKED
            5, 0, // maxlen = text.size()
            5, 0, b'h', b'e', b'l', b'l', b'o', // addString
            0, 0, // empty writer
            0, 0, // empty date
        ]
    );
    let helper = tfs_rust_net::outgoing_extra::send_text_window_simple_item(
        1, 0x1234, 1, false, false, false, false, "hello",
    )
    .into_bytes();
    assert_eq!(b, helper);
}

/// TVP `ProtocolGame::sendHouseWindow` — `0x97 | 0x00 | u32 windowTextId | string`.
#[test]
fn house_window_0x97_layout() {
    let m = tfs_rust_net::outgoing_extra::send_house_window(0x0102_0304, "alice");
    let b = m.as_bytes();
    assert_eq!(b[0], 0x97);
    assert_eq!(b[1], 0x00);
    assert_eq!(&b[2..6], &[0x04, 0x03, 0x02, 0x01]);
    assert_eq!(&b[6..8], &[5, 0]);
    assert_eq!(&b[8..], b"alice");
    let via = codec().encode_house_window(0x0102_0304, "alice").into_bytes();
    assert_eq!(via, b);
}

/// 1098 `sendClosePrivate` — `0xB3 + u16 channelId`. Era-identical.
#[test]
fn close_private_1098_layout() {
    let b = tfs_rust_net::outgoing_extra::send_close_private(0x1234).into_bytes();
    assert_eq!(b, vec![0xB3, 0x34, 0x12]);
}

/// 1098 `sendOpenPrivateChannel` — `0xAD + string receiver`. Era-identical.
#[test]
fn open_private_channel_1098_layout() {
    let b = tfs_rust_net::outgoing_extra::send_open_private_channel("Bob").into_bytes();
    assert_eq!(b, vec![0xAD, 3, 0, b'B', b'o', b'b']);
}

/// 7.72 golden bytes (Phase A5). Reference: `gameserver/src/` ONLY — `protocolgame.cpp`,
/// `networkmessage.cpp`, `tools.cpp`. Every assertion cites the C++ field list it mirrors.
mod v772 {
    use super::*;

    fn codec() -> Codec {
        Codec::from_version(ProtocolVersion::V772).expect("772 codec")
    }

    /// `networkmessage.cpp` `addItem`: `u16 clientId` only for a plain non-stackable item — no MARK.
    #[test]
    fn item_template_plain_is_two_bytes_no_mark() {
        let mut m = NetworkMessage::new();
        codec().write_item_template(&mut m, 0x1234, 1, false, false, false, false);
        assert_eq!(m.as_bytes(), &[0x34, 0x12]);
    }

    /// Description / animation flags are ignored in 7.72 (still 2 bytes).
    #[test]
    fn item_template_ignores_animation_and_description() {
        let mut m = NetworkMessage::new();
        codec().write_item_template(&mut m, 0x1234, 1, false, false, true, true);
        assert_eq!(m.as_bytes(), &[0x34, 0x12]);
    }

    /// Stackable: `u16 clientId` + `u8 count`.
    #[test]
    fn item_template_stackable_writes_count() {
        let mut m = NetworkMessage::new();
        codec().write_item_template(&mut m, 0x1234, 7, true, false, false, false);
        assert_eq!(m.as_bytes(), &[0x34, 0x12, 0x07]);
    }

    /// Fluid: `u16 clientId` + `u8 getLiquidColor(count)`. `tools.cpp`: type 3 → 3, type 6 → 4.
    #[test]
    fn item_template_fluid_uses_getliquidcolor_not_fluidmap() {
        let mut m = NetworkMessage::new();
        codec().write_item_template(&mut m, 0x1234, 6, false, true, false, false);
        // getLiquidColor(6) == 4 (differs from 10.x FLUID_MAP[6] == 9).
        assert_eq!(m.as_bytes(), &[0x34, 0x12, 0x04]);
    }

    /// `item_template_wire_len` must stay in sync with `write_item_template`.
    #[test]
    fn item_template_wire_len_matches_write() {
        for &(cid, count, stack, splash) in &[
            (0x1234u16, 1u8, false, false),
            (0x1234u16, 7u8, true, false),
            (0x1234u16, 6u8, false, true),
        ] {
            let mut m = NetworkMessage::new();
            codec().write_item_template(&mut m, cid, count, stack, splash, false, false);
            assert_eq!(
                m.as_bytes().len(),
                codec().item_template_wire_len(cid, count, stack, splash, false, false)
            );
        }
    }

    /// `AddOutfit` lookType path: `u16 lookType` + head/body/legs/feet — no addons, no mount.
    #[test]
    fn outfit_looktype_no_addons_no_mount() {
        let o = OutfitWire {
            look_type: 128,
            look_head: 1,
            look_body: 2,
            look_legs: 3,
            look_feet: 4,
            look_addons: 5,
            look_mount: 9,
            look_type_ex: 0,
        };
        let mut m = NetworkMessage::new();
        codec().write_outfit(&mut m, &o);
        assert_eq!(m.as_bytes(), &[128, 0, 1, 2, 3, 4]);
    }

    /// `AddOutfit` lookType==0 path: `u16 0` + `u16 lookTypeEx` (`addItemId`).
    #[test]
    fn outfit_item_outfit_writes_looktypeex() {
        let o = OutfitWire {
            look_type: 0,
            look_type_ex: 0x0456,
            ..Default::default()
        };
        let mut m = NetworkMessage::new();
        codec().write_outfit(&mut m, &o);
        assert_eq!(m.as_bytes(), &[0x00, 0x00, 0x56, 0x04]);
    }

    /// `sendTradeItemRequest` own offer: `0x7D` + name + `u8` count + items.
    /// 772: `gameserver/src/protocolgame.cpp`.
    #[test]
    fn trade_item_request_own_plain_item() {
        let args = ItemTemplateArgs {
            client_id: 0x1234,
            count: 1,
            stackable: false,
            is_splash_or_fluid: false,
            is_animation: false,
            with_description: false,
        };
        let b = codec()
            .encode_trade_item_request("Alice", true, &[args])
            .into_bytes();
        assert_eq!(
            b,
            vec![0x7D, 5, 0, b'A', b'l', b'i', b'c', b'e', 1, 0x34, 0x12]
        );
        assert_eq!(codec().encode_close_trade().into_bytes(), vec![0x7F]);
        let partner = codec()
            .encode_trade_item_request("Bob", false, &[args])
            .into_bytes();
        assert_eq!(partner[0], 0x7E);
    }

    /// `AddCreature` unknown header: `0x61` + removeId + id + name (no creature-type byte), health,
    /// direction, outfit, raw light, **full** step speed, skull, party shield. No emblem / 2nd
    /// type / bubble / MARK / helpers / walkthrough.
    #[test]
    fn add_creature_unknown_772_layout() {
        let c = AddCreatureWire {
            id: 0x11223344,
            remove_known: 0xAABBCCDD,
            known: false,
            uptodate: false,
            creature_type: 1,
            name: "Rat".to_string(),
            health_percent: 80,
            direction: 2,
            outfit: OutfitWire {
                look_type: 21,
                look_head: 1,
                look_body: 2,
                look_legs: 3,
                look_feet: 4,
                ..Default::default()
            },
            light_level: 7,
            light_color: 215,
            step_speed: 220,
            skull: 0,
            party_shield: 0,
            guild_emblem: 0,
            speech_bubble: 0,
            helpers: 0,
            walkthrough_blocked: 1,
            access_player: false,
        };
        let mut m = NetworkMessage::new();
        codec().write_add_creature(&mut m, &c);
        assert_eq!(
            m.as_bytes(),
            &[
                0x61, 0x00, // 0x61
                0xDD, 0xCC, 0xBB, 0xAA, // removeId
                0x44, 0x33, 0x22, 0x11, // id
                0x03, 0x00, b'R', b'a', b't', // name (no creature-type byte before it)
                80,   // health %
                2,    // direction
                21, 0, 1, 2, 3, 4, // outfit (no addons / mount)
                7, 215, // light level + color (raw, no 0xFF substitution)
                220, 0, // full step speed (not halved)
                0, // skull
                0, // party shield
            ]
        );
        assert_eq!(
            m.as_bytes().len(),
            codec().add_creature_wire_len(&c),
            "add_creature_wire_len must match write_add_creature"
        );
    }

    /// `AddCreature` known header: `0x62` + id, then the common tail.
    #[test]
    fn add_creature_known_772_wire_len_matches() {
        let c = AddCreatureWire {
            id: 0x11223344,
            known: true,
            uptodate: false,
            outfit: OutfitWire {
                look_type: 0,
                look_type_ex: 1234,
                ..Default::default()
            },
            step_speed: 300,
            ..Default::default()
        };
        let mut m = NetworkMessage::new();
        codec().write_add_creature(&mut m, &c);
        assert_eq!(m.as_bytes()[0], 0x62);
        assert_eq!(m.as_bytes().len(), codec().add_creature_wire_len(&c));
    }

    /// Decompile `SendMapObject` UPTODATE: `SendWord(99)` + id + direction (`sending.cc` ~218–221).
    #[test]
    fn add_creature_uptodate_772_is_0x63_id_direction_only() {
        let c = AddCreatureWire {
            id: 0x11223344,
            known: true,
            uptodate: true,
            direction: 3,
            outfit: OutfitWire {
                look_type: 21,
                ..Default::default()
            },
            health_percent: 40,
            name: "ShouldNotAppear".into(),
            ..Default::default()
        };
        let mut m = NetworkMessage::new();
        codec().write_add_creature(&mut m, &c);
        assert_eq!(
            m.as_bytes(),
            &[0x63, 0x00, 0x44, 0x33, 0x22, 0x11, 3],
            "0x63 is id + direction only — no name/HP/outfit"
        );
        assert_eq!(m.as_bytes().len(), codec().add_creature_wire_len(&c));
    }

    /// `AddPlayerStats` (`0xA0`): health/max u16, cap u16 (=free/100), exp u32, level u16 + %,
    /// mana/max u16, magic u8 + %, soul u8. 22 bytes after opcode.
    #[test]
    fn player_stats_772_layout() {
        let stats = PlayerStatsWire {
            health: 150,
            max_health: 150,
            free_capacity: 40000, // centi-oz → 400 on the wire
            total_capacity: 40000,
            experience: 4200,
            level: 8,
            level_percent: 50,
            mana: 35,
            max_mana: 35,
            magic_level: 3,
            base_magic_level: 3,
            magic_level_percent: 25,
            soul: 100,
            stamina_minutes: 2520,
            base_speed_half: 110,
            regeneration_ticks_sec: 0,
            offline_training_time: 0,
        };
        let b = codec().encode_player_stats(&stats).into_bytes();
        assert_eq!(
            b,
            vec![
                0xA0, //
                150, 0, // health
                150, 0, // max health
                0x90, 0x01, // capacity 400 (40000/100)
                0x68, 0x10, 0x00, 0x00, // experience 4200 u32
                8, 0,  // level
                50, // level %
                35, 0, // mana
                35, 0,   // max mana
                3,   // magic level
                25,  // magic level %
                100, // soul
            ]
        );
    }

    /// `AddPlayerStats` writes 0 for experience overflow.
    #[test]
    fn player_stats_772_experience_overflow_writes_zero() {
        let stats = PlayerStatsWire {
            health: 1,
            max_health: 1,
            free_capacity: 0,
            total_capacity: 0,
            experience: u32::MAX as u64, // >= u32::MAX - 1
            level: 1,
            level_percent: 0,
            mana: 0,
            max_mana: 0,
            magic_level: 0,
            base_magic_level: 0,
            magic_level_percent: 0,
            soul: 0,
            stamina_minutes: 0,
            base_speed_half: 0,
            regeneration_ticks_sec: 0,
            offline_training_time: 0,
        };
        let b = codec().encode_player_stats(&stats).into_bytes();
        // exp bytes are at offset 1+2+2+2 = 7..11
        assert_eq!(&b[7..11], &[0, 0, 0, 0]);
    }

    /// `AddPlayerSkills` (`0xA1`): 7 skills × (`u8` level + `u8`%) = 14 bytes after opcode.
    #[test]
    fn player_skills_772_layout() {
        let levels = [10u16, 11, 12, 13, 14, 15, 16];
        let percents = [1u8, 2, 3, 4, 5, 6, 7];
        let b = codec()
            .encode_player_skills(&PlayerSkillsWire {
                levels,
                bases: levels,
                percents,
                additional_levels: [0; 6],
                additional_bases: [0; 6],
            })
            .into_bytes();
        assert_eq!(
            b,
            vec![0xA1, 10, 1, 11, 2, 12, 3, 13, 4, 14, 5, 15, 6, 16, 7]
        );
    }

    /// Self-appear (`0x0A`): id + `u16` beat (from `MechanicsProfile::beat_ms`, 200 for 772) + `u8` canReportBugs.
    #[test]
    fn self_appear_772_layout() {
        let b = codec()
            .encode_self_appear_login(0x11223344, 200)
            .into_bytes();
        assert_eq!(b, vec![0x0A, 0x44, 0x33, 0x22, 0x11, 0xC8, 0x00, 0x00]);
    }

    /// `sendAddContainerItem` (`0x70`): cid + item — no slot index (10.x adds `u16`).
    #[test]
    fn add_container_item_772_no_slot_index() {
        let args = ItemTemplateArgs {
            client_id: 0x1234,
            count: 5,
            stackable: true,
            is_splash_or_fluid: false,
            is_animation: false,
            with_description: false,
        };
        let b = codec().encode_add_container_item(3, 99, args).into_bytes();
        assert_eq!(b, vec![0x70, 3, 0x34, 0x12, 5]);
    }

    /// `sendUpdateContainerItem` (`0x71`): cid + `u8` slot + item.
    #[test]
    fn update_container_item_772_u8_slot() {
        let args = ItemTemplateArgs {
            client_id: 0x1234,
            count: 1,
            stackable: false,
            is_splash_or_fluid: false,
            is_animation: false,
            with_description: false,
        };
        let b = codec()
            .encode_update_container_item(2, 6, args)
            .into_bytes();
        assert_eq!(b, vec![0x71, 2, 6, 0x34, 0x12]);
    }

    /// 7.72 `sendRemoveContainerItem` (`0x72`): cid + `u8` slot. TVP sends no item id.
    /// C++ ref: `gameserver/src/protocolgame.cpp` ~1890.
    #[test]
    fn remove_container_item_772_u8_slot() {
        let b = codec().encode_remove_container_item(2, 6).into_bytes();
        assert_eq!(b, vec![0x72, 2, 6]);
    }

    /// `sendInventoryItem` (`0x78`): slot + item.
    #[test]
    fn inventory_item_772_layout() {
        let args = ItemTemplateArgs {
            client_id: 0x1234,
            count: 1,
            stackable: false,
            is_splash_or_fluid: false,
            is_animation: false,
            with_description: false,
        };
        let b = codec().encode_inventory_item(5, args).into_bytes();
        assert_eq!(b, vec![0x78, 5, 0x34, 0x12]);
    }

    /// `sendAddTileItem` (`0x6A`): position + item (no stackpos on 7.72).
    #[test]
    fn add_tile_item_772_no_stackpos() {
        let pos = Position::new(0x0102, 0x0304, 5);
        let args = ItemTemplateArgs {
            client_id: 0x1234,
            count: 3,
            stackable: true,
            is_splash_or_fluid: false,
            is_animation: false,
            with_description: false,
        };
        let b = codec()
            .encode_add_tile_item(pos, 2, args, false)
            .into_bytes();
        assert_eq!(b, vec![0x6A, 0x02, 0x01, 0x04, 0x03, 0x05, 0x34, 0x12, 3]);
    }

    /// 772 ignores `otclient_stackpos` — OTCv8 772 does not read stackpos on `0x6A`.
    #[test]
    fn add_tile_item_772_ignores_otclient_stackpos_flag() {
        let pos = Position::new(0x0102, 0x0304, 5);
        let args = ItemTemplateArgs {
            client_id: 0x1234,
            count: 3,
            stackable: true,
            is_splash_or_fluid: false,
            is_animation: false,
            with_description: false,
        };
        let without = codec()
            .encode_add_tile_item(pos, 2, args, false)
            .into_bytes();
        let with_flag = codec()
            .encode_add_tile_item(pos, 2, args, true)
            .into_bytes();
        assert_eq!(without, with_flag);
    }

    /// `sendAddCreature` non-self (`0x6A`): position + creature marker immediately (no stackpos).
    #[test]
    fn add_tile_creature_772_no_stackpos() {
        let pos = Position::new(0x0102, 0x0304, 5);
        let wire = AddCreatureWire {
            id: 0x11223344,
            remove_known: 0,
            known: false,
            uptodate: false,
            name: "Orc".to_string(),
            health_percent: 100,
            direction: 2,
            outfit: OutfitWire {
                look_type: 5,
                ..Default::default()
            },
            step_speed: 200,
            ..Default::default()
        };
        let b = codec()
            .encode_add_tile_creature(pos, 1, &wire, false)
            .into_bytes();
        assert_eq!(b[0], 0x6A);
        // opcode + position (5) → creature marker `0x0061`
        assert_eq!(b[6], 0x61);
        assert_eq!(b[7], 0x00);
        assert_eq!(
            b,
            codec()
                .encode_add_tile_creature(pos, 1, &wire, true)
                .into_bytes()
        );
    }

    /// `sendUpdateTileItem` (`0x6B`): position + `u8` stackpos + item.
    #[test]
    fn update_tile_item_772_layout() {
        let pos = Position::new(0x0102, 0x0304, 5);
        let args = ItemTemplateArgs {
            client_id: 0x1234,
            count: 1,
            stackable: false,
            is_splash_or_fluid: false,
            is_animation: false,
            with_description: false,
        };
        let b = codec().encode_update_tile_item(pos, 2, args).into_bytes();
        assert_eq!(
            b,
            vec![0x6B, 0x02, 0x01, 0x04, 0x03, 0x05, 0x02, 0x34, 0x12]
        );
    }

    /// `RemoveTileThing` (`0x6C`): position + `u8` stackpos.
    #[test]
    fn remove_tile_thing_772_layout() {
        let pos = Position::new(0x0102, 0x0304, 5);
        let b = codec().encode_remove_tile_thing(pos, 3).into_bytes();
        assert_eq!(b, vec![0x6C, 0x02, 0x01, 0x04, 0x03, 0x05, 0x03]);
    }

    /// `AddCreatureLight` (`0x8D`): id + raw level + color (no access-player `0xFF`).
    #[test]
    fn creature_light_772_layout() {
        let b = codec()
            .encode_creature_light(0x11223344, 7, 215, true)
            .into_bytes();
        assert_eq!(b, vec![0x8D, 0x44, 0x33, 0x22, 0x11, 7, 215]);
    }

    /// `sendCreatureTurn` (`0x6B`): position + stackpos + `u16 0x63` + id + direction. No `0xFFFF`
    /// by-id branch, no walkthrough byte (10.x only).
    #[test]
    fn creature_turn_772_layout() {
        let pos = Position::new(0x0102, 0x0304, 5);
        let b = codec()
            .encode_creature_turn(0x11223344, 2, pos, 1, false)
            .into_bytes();
        assert_eq!(
            b,
            vec![
                0x6B, 0x02, 0x01, 0x04, 0x03, 0x05, 0x02, 0x63, 0x00, 0x44, 0x33, 0x22, 0x11, 1
            ]
        );
    }

    /// `sendCancelWalk` (`0xB5`): direction.
    #[test]
    fn cancel_walk_772_layout() {
        let b = codec().encode_cancel_walk(3).into_bytes();
        assert_eq!(b, vec![0xB5, 3]);
    }

    /// `sendContainer` (`0x6E`): cid + item + name + `u8` capacity + `u8` hasParent + `u8` count +
    /// items. No unlocked / pagination / `u16` size / firstIndex (10.x additions).
    #[test]
    fn container_open_772_layout() {
        let wire = ContainerOpenWire {
            cid: 3,
            header_item: ItemTemplateArgs {
                client_id: 0x0BBE,
                count: 1,
                stackable: false,
                is_splash_or_fluid: false,
                is_animation: false,
                with_description: false,
            },
            name: "bag".to_string(),
            capacity: 8,
            has_parent: false,
            // 10.x-only fields are filled by core but must be ignored by the 772 codec.
            unlocked: true,
            pagination: true,
            total_size: 99,
            first_index: 7,
            items: vec![ItemTemplateArgs {
                client_id: 0x0C00,
                count: 5,
                stackable: true,
                is_splash_or_fluid: false,
                is_animation: false,
                with_description: false,
            }],
        };
        let b = codec().encode_container_open(&wire).into_bytes();
        assert_eq!(
            b,
            vec![
                0x6E, 3, // opcode + cid
                0xBE, 0x0B, // header item: clientId only (no MARK in 7.72)
                0x03, 0x00, b'b', b'a', b'g', // name
                8,    // capacity
                0,    // hasParent
                1,    // items to send
                0x00, 0x0C, 5, // child: clientId + count (stackable, no MARK)
            ]
        );
    }

    /// 7.72 container window caps `count` at capacity (no pagination).
    #[test]
    fn container_open_772_caps_count_at_capacity() {
        let item = ItemTemplateArgs {
            client_id: 0x0C00,
            count: 1,
            stackable: false,
            is_splash_or_fluid: false,
            is_animation: false,
            with_description: false,
        };
        let wire = ContainerOpenWire {
            cid: 0,
            header_item: item,
            name: String::new(),
            capacity: 2,
            has_parent: false,
            unlocked: false,
            pagination: false,
            total_size: 4,
            first_index: 0,
            items: vec![item, item, item, item],
        };
        let b = codec().encode_container_open(&wire).into_bytes();
        // capacity 2 → count byte = 2, then 2 item bodies (2 bytes each) = 4.
        // header: 0x6E + cid + clientId(2) + name len(2) = 6; + cap + hasParent + count = 9; + 4.
        let count_byte_idx = 1 + 1 + 2 + 2 + 1 + 1;
        assert_eq!(b[count_byte_idx], 2);
        assert_eq!(b.len(), count_byte_idx + 1 + 2 * 2);
    }

    /// 7.72 has no `sendBasicData` / by-id tile removal — encoders return empty (skipped by core).
    #[test]
    fn no_equivalent_packets_are_empty() {
        assert!(
            codec()
                .encode_basic_data(true, 1234, 1)
                .into_bytes()
                .is_empty()
        );
        assert!(
            codec()
                .encode_remove_tile_creature_by_id(42)
                .into_bytes()
                .is_empty()
        );
    }

    /// `sendAnimatedText` (`0x84`): position + color + string.
    #[test]
    fn animated_text_772_layout() {
        let pos = Position::new(0x0102, 0x0304, 5);
        let b = codec()
            .encode_animated_text(&AnimatedTextWire {
                pos,
                color: 180,
                text: "42".to_string(),
            })
            .into_bytes();
        assert_eq!(
            b,
            vec![
                0x84, 0x02, 0x01, 0x04, 0x03, 0x05, 180, 0x02, 0x00, b'4', b'2'
            ]
        );
    }

    /// `sendDistanceShoot` (`0x85`): from + to + shoot type.
    #[test]
    fn distance_shoot_772_layout() {
        let from = Position::new(0x0102, 0x0304, 5);
        let to = Position::new(0x0506, 0x0708, 5);
        let b = codec()
            .encode_distance_shoot(&DistanceShootWire {
                from,
                to,
                shoot_type: 11,
            })
            .into_bytes();
        assert_eq!(
            b,
            vec![
                0x85, 0x02, 0x01, 0x04, 0x03, 0x05, 0x06, 0x05, 0x08, 0x07, 0x05, 11
            ]
        );
    }

    /// 772 combat damage uses simple `sendTextMessage` (`MESSAGE_EVENT_DEFAULT` = `0x14`).
    #[test]
    fn combat_damage_text_message_772_layout() {
        let b = codec()
            .encode_combat_damage_text_message(&CombatDamageNotifyWire {
                pos: Position::new(1, 2, 3),
                damage: 5,
                damage_color: 180,
                text: "You lose 5 hitpoints.".to_string(),
            })
            .into_bytes();
        assert_eq!(
            b,
            vec![
                0xB4, 0x14, 0x15, 0x00, b'Y', b'o', b'u', b' ', b'l', b'o', b's', b'e', b' ', b'5',
                b' ', b'h', b'i', b't', b'p', b'o', b'i', b'n', b't', b's', b'.'
            ]
        );
    }

    // CH-0 — 772 chat-channel outgoing wire golden bytes.
    // Reference: `gameserver/src/protocolgame.cpp` ONLY.

    /// 772 `sendChannelsDialog` — `0xAB + u8 count + [u16 id + string name]*`. Era-identical to 1098.
    /// (`gameserver/src/protocolgame.cpp:1282`)
    #[test]
    fn channels_dialog_772_layout() {
        let b = codec()
            .encode_channels_dialog(&ChannelsDialogWire {
                channels: vec![(4, "Game-Chat".to_string()), (7, "Help".to_string())],
            })
            .into_bytes();
        assert_eq!(
            b,
            vec![
                0xAB, 2, 4, 0, 9, 0, b'G', b'a', b'm', b'e', b'-', b'C', b'h', b'a', b't', 7, 0, 4,
                0, b'H', b'e', b'l', b'p',
            ]
        );
    }

    /// 772 `sendChannelsDialog` with an empty list — `0xAB + u8(0)`. Matches the legacy
    /// `send_channels_dialog_count` helper byte-for-byte.
    #[test]
    fn channels_dialog_772_empty_matches_legacy_count_helper() {
        let via_codec = codec()
            .encode_channels_dialog(&ChannelsDialogWire::default())
            .into_bytes();
        let via_legacy = tfs_rust_net::outgoing_extra::send_channels_dialog_count().into_bytes();
        assert_eq!(via_codec, vec![0xAB, 0]);
        assert_eq!(via_codec, via_legacy);
    }

    /// 772 `sendChannel` — `0xAC + u16 id + string name`. **No user/invited lists**
    /// (`gameserver/src/protocolgame.cpp:1297`). The `users` / `invited` fields are ignored.
    #[test]
    fn channel_open_772_omits_user_lists() {
        let b = codec()
            .encode_channel_open(&ChannelOpenWire {
                channel_id: 4,
                name: "Game-Chat".to_string(),
                users: vec!["Alice".to_string(), "Bob".to_string()],
                invited: vec!["Carol".to_string()],
            })
            .into_bytes();
        assert_eq!(
            b,
            vec![
                0xAC, 4, 0, 9, 0, b'G', b'a', b'm', b'e', b'-', b'C', b'h', b'a', b't',
            ]
        );
    }

    /// 772 `sendCreatePrivateChannel` — `0xB2 + u16 id + string name`. **No owner/invited lists**
    /// (`gameserver/src/protocolgame.cpp:1273`). The `owner_name` / `invited` fields are ignored.
    #[test]
    fn create_private_channel_772_omits_owner_and_invited() {
        let b = codec()
            .encode_create_private_channel(&CreatePrivateChannelWire {
                channel_id: 0x0100,
                name: "My Channel".to_string(),
                owner_name: "Alice".to_string(),
                invited: vec!["Bob".to_string()],
            })
            .into_bytes();
        assert_eq!(
            b,
            vec![
                0xB2, 0x00, 0x01, 10, 0, b'M', b'y', b' ', b'C', b'h', b'a', b'n', b'n', b'e',
                b'l',
            ]
        );
    }

    /// 772 `sendTextWindow` template overload (`gameserver/src/protocolgame.cpp:1925`):
    /// `addItem` is `u16 clientId` only (no MARK); writer `u16 0`; **no date**.
    /// The 1098 MARK + date bytes are what crashed the 7.72 client on spellbook open.
    #[test]
    fn text_window_772_no_mark_no_date() {
        let w = TextWindowWire {
            window_text_id: 1,
            item: ItemTemplateArgs {
                client_id: 0x1234,
                count: 1,
                stackable: false,
                is_splash_or_fluid: false,
                is_animation: false,
                with_description: false,
            },
            text: "hello".to_string(),
            writer: String::new(),
            written_date: Some("Jan 01 2026".to_string()),
            can_write: false,
            max_text_len: 0,
        };
        let b = codec().encode_text_window(&w).into_bytes();
        assert_eq!(
            b,
            vec![
                0x96, 1, 0, 0, 0, // opcode + windowTextId
                0x34, 0x12, // clientId only — no MARK
                5, 0, // maxlen = text.size()
                5, 0, b'h', b'e', b'l', b'l', b'o', // addString
                0, 0, // empty writer; no date follows
            ]
        );
        assert!(
            !b.contains(&0xFF),
            "772 addItem must not emit MARK_UNMARKED 0xFF"
        );
    }

    /// 772 `sendClosePrivate` — `0xB3 + u16 channelId`. Era-identical.
    /// (`gameserver/src/protocolgame.cpp:1265`)
    #[test]
    fn close_private_772_layout() {
        let b = tfs_rust_net::outgoing_extra::send_close_private(0x1234).into_bytes();
        assert_eq!(b, vec![0xB3, 0x34, 0x12]);
    }

    /// 772 `sendOpenPrivateChannel` — `0xAD + string receiver`. Era-identical.
    /// (`gameserver/src/protocolgame.cpp:1111`)
    #[test]
    fn open_private_channel_772_layout() {
        let b = tfs_rust_net::outgoing_extra::send_open_private_channel("Bob").into_bytes();
        assert_eq!(b, vec![0xAD, 3, 0, b'B', b'o', b'b']);
    }
}

// ---------------------------------------------------------------------------
// Phase 0 — 772 floor-change desync golden tests.
// See `docs/772_FLOOR_CHANGE_DESYNC.md` §14 Phase 0.
//
// These tests lock the *current* 1098 output (regression guard) and the
// *expected* 772 output (no spurious self-creature packet). The 772 tests
// FAIL until Phase 1 gates the self-packet on `codec.caps()`.
//
// Reference: 772 `NotifyGo` (`cract.cc:1400-1460`) — the player's own move
// never emits `0x6D`/`0x6C`; the viewport is updated purely via `SendFloors`
// (0xBE/0xBF) + `SendRow` (0x65-0x68). The map body (floor descriptions +
// edge rows) is byte-identical between the Rust 1098 path and 772's
// `SendFloors`/`SendRow` (audit #2, §16.1). The ONLY divergence is the
// leading self-creature packet.
//
// All tests use an empty map (`get_tile` → `None`) so the map body is
// deterministic skip-compression bytes, identical across eras.
// ---------------------------------------------------------------------------

/// Empty-map tile provider — all tiles return `None`. Makes the map body
/// deterministic (only skip-compression flush bytes) and era-independent.
fn empty_get_tile(_x: i32, _y: i32, _z: i32) -> Option<tfs_rust_net::map_description::TileContent> {
    None
}

/// `can_see_creature` stub — all creatures visible (no creature encoding in
/// empty-map tests anyway).
fn empty_can_see_creature(_id: u32) -> bool {
    true
}

/// Top-level creature ID for `notify_go_bytes` (the 772 self-move path).
const NOTIFY_GO_CID: u32 = 0x11223344;

/// Calls `send_move_creature_player` with an empty map and returns the raw
/// bytes for the given codec.
fn move_creature_player_bytes(
    codec: &Codec,
    old_pos: Position,
    new_pos: Position,
    old_stack: i32,
    creature_id: u32,
) -> Vec<u8> {
    let mut known = HashSet::new();
    let mut get_tile = empty_get_tile;
    let mut can_see = empty_can_see_creature;
    send_move_creature_player(
        codec,
        old_pos,
        new_pos,
        old_stack,
        creature_id,
        &mut get_tile,
        &mut known,
        &mut can_see,
        false,
    )
    .into_bytes()
}

/// Calls `send_notify_go` (the 772 self-move path) with an empty map and returns
/// the raw bytes for the given codec.
fn notify_go_bytes(codec: &Codec, orig: Position, dest: Position) -> Vec<u8> {
    let mut known = HashSet::new();
    let mut get_tile = empty_get_tile;
    let mut can_see = empty_can_see_creature;
    send_notify_go(
        codec,
        orig,
        dest,
        0,
        NOTIFY_GO_CID,
        &mut get_tile,
        &mut known,
        &mut can_see,
        false,
    )
    .into_bytes()
}

/// 1098 floor-change regression guard — current behavior WITH the self-packet.
/// These tests MUST pass before and after Phase 1 (the 1098 path is unchanged).
mod v1098_floor_change {
    use super::*;

    fn codec() -> Codec {
        Codec::from_version(ProtocolVersion::V1098).expect("1098 codec")
    }

    const CID: u32 = 0x11223344;

    /// Hole straight down (100,100,7)→(100,100,8).
    /// 1098 emits `0x6C` remove (7 bytes) + `0xBF` floors + `0x66` east col + `0x67` south row.
    /// Map body: 0xBF + 3-floor skip-compressed body(6) + 0x66 + col(2) + 0x67 + row(2) = 13.
    #[test]
    fn hole_down_in_place_1098_has_self_packet() {
        let old = Position::new(100, 100, 7);
        let new = Position::new(100, 100, 8);
        let b = move_creature_player_bytes(&codec(), old, new, 0, CID);
        // Leading byte must be 0x6C (remove) — the spurious self-packet.
        assert_eq!(b[0], 0x6C, "1098 surface→underground must emit 0x6C remove");
        // Self-packet is 7 bytes: 0x6C + position(5) + stack(1).
        assert_eq!(b.len(), 20, "1098 hole-down: self-packet(7) + map body(13)");
        // After self-packet, first map opcode must be 0xBF (floor down).
        assert_eq!(b[7], 0xBF);
    }

    /// Ladder straight up (100,100,8)→(100,100,7).
    /// 1098 emits `0x6D` move (12 bytes) + `0xBE` floors(6 floors) + `0x68` west col + `0x65` north row.
    /// Map body: 0xBE + 6-floor skip body(12) + 0x68 + col(2) + 0x65 + row(2) = 19.
    #[test]
    fn ladder_up_in_place_1098_has_self_packet() {
        let old = Position::new(100, 100, 8);
        let new = Position::new(100, 100, 7);
        let b = move_creature_player_bytes(&codec(), old, new, 0, CID);
        assert_eq!(b[0], 0x6D, "1098 underground→surface must emit 0x6D move");
        // Self-packet is 12 bytes: 0x6D + old_pos(5) + stack(1) + new_pos(5).
        assert_eq!(
            b.len(),
            31,
            "1098 ladder-up: self-packet(12) + map body(19)"
        );
        assert_eq!(b[12], 0xBE);
    }

    /// Stairs down diagonal (100,100,7)→(100,101,8).
    /// 1098 emits `0x6C` remove (7 bytes) + `0xBF` + `0x66` + `0x67` + outer `0x67`.
    /// Map body: 0xBF + floors(6) + 0x66 + col(2) + 0x67 + row(2) + outer 0x67 + row(2) = 16.
    #[test]
    fn stairs_down_diag_1098_has_self_packet() {
        let old = Position::new(100, 100, 7);
        let new = Position::new(100, 101, 8);
        let b = move_creature_player_bytes(&codec(), old, new, 0, CID);
        assert_eq!(b[0], 0x6C);
        assert_eq!(b[7], 0xBF);
        assert_eq!(
            b.len(),
            23,
            "1098 stairs-down-diag: self-packet(7) + map body(16)"
        );
        // The outer loop adds one more 0x67 (south) for oy < ny.
        let map_body = &b[7..];
        let south_count = map_body.iter().filter(|&&x| x == 0x67).count();
        assert_eq!(
            south_count, 2,
            "stairs-down-diag: append's 0x67 + outer oy<ny 0x67 = 2 south rows"
        );
    }

    /// Same-z east move (100,100,7)→(101,100,7).
    /// 1098 emits `0x6D` move (12 bytes) + `0x66` east column.
    /// Map body: 0x66 + col(2) = 3. Note: new_pos.x=101=0x65 appears in the
    /// self-packet's new_pos field — it is NOT an opcode.
    #[test]
    fn same_z_east_1098_has_self_packet() {
        let old = Position::new(100, 100, 7);
        let new = Position::new(101, 100, 7);
        let b = move_creature_player_bytes(&codec(), old, new, 0, CID);
        assert_eq!(b[0], 0x6D);
        assert_eq!(
            b.len(),
            15,
            "1098 same-z east: self-packet(12) + 0x66 + col body(2)"
        );
        // Byte 12 is the first map opcode (0x66). Bytes 7-11 are new_pos in the self-packet.
        assert_eq!(
            b[12], 0x66,
            "same-z east: after self-packet, first opcode is 0x66"
        );
    }

    /// Teleport (1098): remove + map description. Tested at the net level —
    /// `send_map_description_packet` produces `0x64` for both eras. The
    /// remove packet (`0x6C`) is emitted separately by `emit_teleport_move_packet`
    /// in `tfs-rust-core`; its suppression for 772 is Phase 2.
    #[test]
    fn teleport_map_description_1098_starts_with_0x64() {
        let pos = Position::new(100, 100, 7);
        let mut known = HashSet::new();
        let mut get_tile = empty_get_tile;
        let mut can_see = empty_can_see_creature;
        let m = send_map_description_packet(
            &codec(),
            pos,
            pos,
            &mut get_tile,
            &mut known,
            &mut can_see,
            false,
        );
        let b = m.into_bytes();
        assert_eq!(b[0], 0x64, "teleport map description starts with 0x64");
        // 0x64 + position(5) + 8-floor skip-compressed empty body (16 bytes).
        assert_eq!(b.len(), 22, "0x64 + pos(5) + empty-body skip flush(16)");
    }
}

/// 772 self-move golden tests — decompile `TCreature::NotifyGo` (`cract.cc:1400-1465`)
/// plus TVP surface→underground self-packet (`0x6C`, `protocolgame.cpp` ~1793–1805).
///
/// Adjacent moves stream `SendFloors`/`SendRow` after a leading self-packet:
/// - surface→underground (`z=7`→`z≥8`): `0x6C` remove (must not `0x6D` — client FloorDown
///   would double-apply z and assert `rz=-1` / bug0000013)
/// - otherwise: `0x6D` move (old+stack+new) then floor/row opcodes
///
/// Non-adjacent: `SendFullScreen` only (`0x64` + dest pos + map body; no `0x6D`/`0x6C`).
mod v772_floor_change {
    use super::*;

    fn codec_772() -> Codec {
        Codec::from_version(ProtocolVersion::V772).expect("772 codec")
    }

    /// 0x6D self-packet is 12 bytes: 1 (opcode) + 5 (old_pos) + 1 (old_stack) + 5 (new_pos).
    /// Surface→underground uses 0x6C remove (7 bytes) instead — see `hole_down_*`.
    const SELF_MOVE_PACKET_LEN: usize = 12;
    const SELF_REMOVE_PACKET_LEN: usize = 7;

    /// Assert the stream leads with a `0x6D` self-move and return the floor/row stream offset.
    fn assert_self_move_then_stream(bytes: &[u8]) -> usize {
        assert_eq!(
            bytes[0], 0x6D,
            "772 NotifyGo must lead with 0x6D self-move (got {:#04X})",
            bytes[0]
        );
        SELF_MOVE_PACKET_LEN
    }

    /// Hole straight down (100,100,7)→(100,100,8): `0x6C` remove (not `0x6D`), then `0xBF`.
    /// A leading `0x6D` pre-sets client z→8; FloorDown then yields `rz=-1` / bug0000013.
    #[test]
    fn hole_down_self_packet_then_floors() {
        let b = notify_go_bytes(
            &codec_772(),
            Position::new(100, 100, 7),
            Position::new(100, 100, 8),
        );
        assert_eq!(
            b[0], 0x6C,
            "surface→underground NotifyGo must lead with 0x6C remove (got {:#04X})",
            b[0]
        );
        assert_eq!(
            b[SELF_REMOVE_PACKET_LEN], 0xBF,
            "hole down leads with SendFloors down (0xBF)"
        );
    }

    /// Live crash repro (2026-08-01): (32380,32205,7)→(32380,32204,8) with `0x6D`+`0xBF`
    /// asserted `Map.cpp` `rz=-1` / bug0000013. Must use `0x6C` then `0xBF`.
    #[test]
    fn surface_to_underground_stairs_uses_remove_not_move() {
        let b = notify_go_bytes(
            &codec_772(),
            Position::new(32380, 32205, 7),
            Position::new(32380, 32204, 8),
        );
        assert_eq!(
            b[0], 0x6C,
            "must not pre-set client z with 0x6D before FloorDown"
        );
        assert_eq!(b[SELF_REMOVE_PACKET_LEN], 0xBF);
    }

    /// Ladder straight up (100,100,8)→(100,100,7): 0x6D self-packet, then `0xBE`.
    #[test]
    fn ladder_up_self_packet_then_floors() {
        let b = notify_go_bytes(
            &codec_772(),
            Position::new(100, 100, 8),
            Position::new(100, 100, 7),
        );
        let off = assert_self_move_then_stream(&b);
        assert_eq!(b[off], 0xBE, "ladder up leads with SendFloors up (0xBE)");
    }

    /// Same-z east step (100,100,7)→(101,100,7): 0x6D self-packet, then a lone `SendRow` east (0x66).
    #[test]
    fn same_z_east_self_packet_then_row() {
        let b = notify_go_bytes(
            &codec_772(),
            Position::new(100, 100, 7),
            Position::new(101, 100, 7),
        );
        let off = assert_self_move_then_stream(&b);
        assert_eq!(b[off], 0x66, "same-z east is a lone SendRow east (0x66)");
    }

    /// The reported repro. South-facing stairs at (100,100,z) send the climber to (100,101,z-1).
    /// Walking NORTH onto them → pure-vertical overall move (parallel to the stair shift).
    /// Walking WEST onto them → leftover delta on BOTH axes (perpendicular). Both must have
    /// the self-packet and produce *different* floor/row streams (the pre-fix per-segment path
    /// emitted the same buggy sequence regardless of approach).
    #[test]
    fn west_vs_north_onto_south_stairs_differ() {
        // North approach: start south of the stair, overall (100,101,8)→(100,101,7) — pure vertical.
        let north = notify_go_bytes(
            &codec_772(),
            Position::new(100, 101, 8),
            Position::new(100, 101, 7),
        );
        // West approach: start east of the stair, overall (101,100,8)→(100,101,7) — dx=1, dy=1, dz=1.
        let west = notify_go_bytes(
            &codec_772(),
            Position::new(101, 100, 8),
            Position::new(100, 101, 7),
        );

        let n_off = assert_self_move_then_stream(&north);
        let w_off = assert_self_move_then_stream(&west);
        assert_eq!(
            north[n_off], 0xBE,
            "north approach leads with SendFloors up"
        );
        assert_eq!(west[w_off], 0xBE, "west approach leads with SendFloors up");
        assert_ne!(
            north, west,
            "approach direction changes the leftover x/y delta → different row stream (§16.3)"
        );
    }

    /// Non-adjacent move (dz > 1): `SendFullScreen` only (`0x64`), no leading `0x6D`.
    /// `cract.cc` NotifyGo else → `SendFullScreen`; `sending.cc` `SendFullScreen`.
    #[test]
    fn non_adjacent_full_screen_only() {
        let dest = Position::new(100, 100, 9);
        let b = notify_go_bytes(&codec_772(), Position::new(100, 100, 7), dest);
        assert_eq!(
            b[0], 0x64,
            "non-adjacent NotifyGo must start with 0x64 (got {:#04X})",
            b[0]
        );
        assert_ne!(b[0], 0x6D);
        assert_eq!(u16::from_le_bytes([b[1], b[2]]), dest.x);
        assert_eq!(u16::from_le_bytes([b[3], b[4]]), dest.y);
        assert_eq!(b[5], dest.z);
    }

    /// Live crash repro (2026-08-16): (33211,31813,7)→(33211,31815,8) is |dy|=2 so
    /// non-adjacent. Leading `0x6D` then `0x64` crashed Map.cpp 378 `rz=-1` / bug0000013.
    #[test]
    fn non_adjacent_live_repro_full_screen_only() {
        let dest = Position::new(33211, 31815, 8);
        let b = notify_go_bytes(&codec_772(), Position::new(33211, 31813, 7), dest);
        assert_eq!(
            b[0], 0x64,
            "live repro must start with 0x64 (got {:#04X})",
            b[0]
        );
        assert_ne!(b[0], 0x6D, "must not lead with 0x6D");
        assert_eq!(u16::from_le_bytes([b[1], b[2]]), dest.x);
        assert_eq!(u16::from_le_bytes([b[3], b[4]]), dest.y);
        assert_eq!(b[5], dest.z);
    }
}
